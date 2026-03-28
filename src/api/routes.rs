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
use crate::config;

#[derive(Clone)]
pub struct AppState {
    pub shard_assignments: Vec<ShardMetadata>,
    pub hw_profile: HardwareProfile,
    pub backend: Arc<Mutex<Option<Box<dyn ModelBackend>>>>,
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
}

#[derive(Serialize)]
pub struct InferenceResponse {
    pub result: String,
    pub error: Option<String>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Legacy/Frontend expected routes
        .route("/api/models", get(get_models))
        .route("/api/inference", post(do_inference))
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

async fn do_inference(
    State(state): State<AppState>,
    Json(payload): Json<InferenceRequest>,
) -> Json<InferenceResponse> {
    info!("Received local inference request for model: {}", payload.model_id);

    let mut backend_lock = state.backend.lock().await;
    
    if let Some(backend) = backend_lock.as_mut() {
        if payload.model_id == config::DEFAULT_MODEL_ID {
             match backend.generate_text(&payload.prompt, config::DEFAULT_MAX_TOKENS) {
                 Ok(result) => return Json(InferenceResponse { result, error: None }),
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
