use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::engine::benchmark::HardwareProfile;
use crate::engine::sharder::ShardMetadata;
use crate::engine::backend::ModelBackend;
use crate::engine::memory::ConversationMemory;
use crate::config;

#[derive(Clone)]
pub struct AppState {
    pub shard_assignments: Vec<ShardMetadata>,
    pub hw_profile: HardwareProfile,
    pub backend: Arc<Mutex<Option<Box<dyn ModelBackend>>>>,
    pub memory: Arc<ConversationMemory>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub peer_id: String,
    pub assignments: Vec<ShardMetadata>,
    pub hardware: HardwareProfile,
}

#[derive(Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub layers: usize,
    pub is_local: bool,
}

#[derive(Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub prompt: String,
    pub session_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct InferenceResponse {
    pub result: String,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct ClearSessionRequest {
    pub session_id: String,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Legacy/Frontend expected routes
        .route("/api/models", get(get_models))
        .route("/api/inference", post(do_inference))
        .route("/api/clear_session", post(clear_session))
        // New management routes
        .route("/status", get(get_status))
        .with_state(state)
}

async fn get_models(State(state): State<AppState>) -> Json<Vec<ModelInfo>> {
    let models = state.shard_assignments.iter().map(|a| {
        ModelInfo {
            id: a.model_id.clone(),
            layers: a.end_layer - a.start_layer,
            is_local: a.is_full,
        }
    }).collect();
    Json(models)
}

async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "online".to_string(),
        peer_id: "local-node".to_string(), // TODO: Get actual PeerID
        assignments: state.shard_assignments,
        hardware: state.hw_profile,
    })
}

async fn clear_session(
    State(state): State<AppState>,
    Json(payload): Json<ClearSessionRequest>,
) -> Json<StatusResponse> {
    state.memory.clear_session(&payload.session_id);
    get_status(State(state)).await
}

async fn do_inference(
    State(state): State<AppState>,
    Json(payload): Json<InferenceRequest>,
) -> Json<InferenceResponse> {
    info!("Received local inference request for model: {}", payload.model_id);

    let mut backend_lock = state.backend.lock().await;
    
    if let Some(backend) = backend_lock.as_mut() {
        if payload.model_id == config::DEFAULT_MODEL_ID {
             let history = payload.session_id.as_ref().and_then(|id| state.memory.get_history(id));
             
             match backend.generate_text(&payload.prompt, config::DEFAULT_MAX_TOKENS, history.as_deref()) {
                 Ok(result) => {
                     if let Some(session_id) = &payload.session_id {
                         state.memory.add_turn(session_id, payload.prompt.clone(), result.clone());
                     }
                     return Json(InferenceResponse { result, error: None });
                 },
                 Err(e) => return Json(InferenceResponse { 
                     result: String::new(), 
                     error: Some(format!("Generation Error: {:?}", e)) 
                 }),
             }
        }
    }

    Json(InferenceResponse {
        result: String::new(),
        error: Some("Inference backend is not yet initialized or model is still downloading...".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt; 
    use crate::error::Result;
    use crate::engine::types::ChatTurn;

    struct MockBackend {
        should_fail: bool,
    }

    impl ModelBackend for MockBackend {
        fn clear_kv_cache(&mut self) {}
        fn load_model(&mut self, _paths: Vec<std::path::PathBuf>) -> Result<()> { Ok(()) }
        fn generate_text(&mut self, _prompt: &str, _max_tokens: usize, _history: Option<&[ChatTurn]>) -> Result<String> {
            if self.should_fail {
                Err(crate::error::SocialKubeError::Inference("Mock failure".into()))
            } else {
                Ok("Mocked response".into())
            }
        }
    }

    fn setup_test_state(backend: Option<Box<dyn ModelBackend>>) -> AppState {
        AppState {
            shard_assignments: vec![],
            hw_profile: HardwareProfile {
                cpu_model: "test".into(),
                cpu_cores: 1,
                total_ram_gb: 8,
                gpu_name: None,
                vram_gb: None,
            },
            backend: Arc::new(Mutex::new(backend)),
            memory: Arc::new(ConversationMemory::default()),
        }
    }

    #[tokio::test]
    async fn test_get_status() {
        let state = setup_test_state(None);
        let app = create_router(state);

        let response = app
            .oneshot(Request::builder().uri("/status").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_inference_success() {
        let backend = Box::new(MockBackend { should_fail: false });
        let state = setup_test_state(Some(backend));
        let app = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/inference")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(format!(
                r#"{{"model_id": "{}", "prompt": "hello"}}"#,
                config::DEFAULT_MODEL_ID
            )))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let res: InferenceResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(res.result, "Mocked response");
        assert!(res.error.is_none());
    }

    #[tokio::test]
    async fn test_inference_no_backend() {
        let state = setup_test_state(None);
        let app = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/inference")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(format!(
                r#"{{"model_id": "{}", "prompt": "hello"}}"#,
                config::DEFAULT_MODEL_ID
            )))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let res: InferenceResponse = serde_json::from_slice(&body).unwrap();
        
        assert!(res.result.is_empty());
        assert!(res.error.unwrap().contains("not yet initialized"));
    }
}
