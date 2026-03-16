use axum::{
    routing::{get, post},
    Json, Router, extract::State,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::engine::sharder::ShardMetadata;
use crate::engine::backend::CandleBackend;
use crate::engine::benchmark::HardwareProfile;

#[derive(Clone)]
pub struct AppState {
    pub shard_assignments: Vec<ShardMetadata>,
    pub hw_profile: HardwareProfile,
    pub backend: Arc<Mutex<Option<CandleBackend>>>,
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
}

#[derive(Serialize)]
pub struct InferenceResponse {
    pub result: String,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/models", get(get_models))
        .route("/api/inference", post(do_inference))
        .with_state(state)
}

async fn get_models(State(state): State<AppState>) -> Json<Vec<ModelInfo>> {
    let models = state.shard_assignments.iter().map(|a| {
        ModelInfo {
            id: a.model_id.clone(),
            layers: a.end_layer - a.start_layer,
            is_local: a.model_id == "llama-3.2-1b" && a.end_layer == a.total_layers,
        }
    }).collect();
    Json(models)
}

async fn do_inference(
    State(state): State<AppState>,
    Json(payload): Json<InferenceRequest>,
) -> Json<InferenceResponse> {
    let mut backend_lock = state.backend.lock().await;
    
    if let Some(backend) = backend_lock.as_mut() {
        // For now, always use the 1B model if requested and local
        if payload.model_id == "llama-3.2-1b" {
             match backend.generate_text(&payload.prompt, 100) {
                 Ok(result) => return Json(InferenceResponse { result }),
                 Err(e) => return Json(InferenceResponse { result: format!("Error: {:?}", e) }),
             }
        }
    }

    Json(InferenceResponse {
        result: format!("Inference for {} is not yet fully implemented for swarm-wide shards.", payload.model_id),
    })
}
