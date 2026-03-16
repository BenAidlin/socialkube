use hf_hub::api::tokio::Api;
use std::path::PathBuf;
use tracing::{info, error};
use crate::engine::sharder::ShardMetadata;

/// Handles model downloading and caching.
pub struct ModelDownloader {
    api: Api,
}

impl ModelDownloader {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api = Api::new()?;
        Ok(Self { api })
    }

    /// Downloads a specific model file from HuggingFace.
    pub async fn download_file(&self, repo_id: &str, filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        info!("Checking for model file: {} in repo: {}", filename, repo_id);
        let repo = self.api.model(repo_id.to_string());
        let path = repo.get(filename).await?;
        info!("Model file ready at: {:?}", path);
        Ok(path)
    }

    /// Iterates through shard assignments and ensures necessary files are downloaded.
    pub async fn check_and_download_models(&self, assignments: &[ShardMetadata]) {
        for assignment in assignments {
            let (repo_id, filename) = match assignment.model_id.as_str() {
                "llama-3.2-1b" => (
                    "meta-llama/Llama-3.2-1B-Instruct", 
                    "model.safetensors" // or "llama-3.2-1b-instruct-q4_k_m.gguf" for GGUF
                ),
                "llama-3-8b" => (
                    "meta-llama/Meta-Llama-3-8B-Instruct",
                    "model.safetensors"
                ),
                _ => continue,
            };

            // Download Tokenizer (Needed for all models)
            if let Err(e) = self.download_file(repo_id, "tokenizer.json").await {
                error!("Failed to download tokenizer for {}: {:?}", repo_id, e);
            }

            // Download Weights
            if let Err(e) = self.download_file(repo_id, filename).await {
                error!("Failed to download weights for {}: {:?}", repo_id, e);
            }
        }
    }
}
