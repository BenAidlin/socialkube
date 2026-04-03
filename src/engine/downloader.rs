use hf_hub::api::tokio::{Api, ApiBuilder};
use hf_hub::{Repo, RepoType};
use std::path::PathBuf;
use tracing::{info, error};

use crate::error::{Result, SocialKubeError};
use crate::engine::sharder::ShardMetadata;
use crate::config;

/// Handles downloading model weights and tokenizers from HuggingFace.
pub struct ModelDownloader {
    api: Api,
}

impl ModelDownloader {
    /// Creates a new ModelDownloader.
    pub fn new() -> Result<Self> {
        let api = ApiBuilder::new()
            .build()
            .map_err(|e| SocialKubeError::Download(format!("Failed to build HF API: {:?}", e)))?;
        Ok(Self { api })
    }

    /// Downloads a specific model file from HuggingFace.
    pub async fn download_file(&self, repo_id: &str, filename: &str) -> Result<PathBuf> {
        info!("Checking for model file: {} in repo: {}", filename, repo_id);
        let repo = self.api.repo(Repo::new(repo_id.to_string(), RepoType::Model));
        let path = repo.get(filename).await
            .map_err(|e| SocialKubeError::Download(format!("Failed to download {}: {:?}", filename, e)))?;
        info!("Model file ready at: {:?}", path);
        Ok(path)
    }

    /// Iterates through shard assignments and ensures necessary files are downloaded.
    /// Returns a list of (model_id, Vec<weights_path>, tokenizer_path) for successfully downloaded models.
    pub async fn check_and_download_models(&self, assignments: &[ShardMetadata]) -> Vec<(String, Vec<PathBuf>, PathBuf)> {
        let mut downloaded = Vec::new();
        for assignment in assignments {
            let (repo_id, filenames) = match assignment.model_id.as_str() {
                config::DEFAULT_MODEL_ID => (
                    config::DEFAULT_REPO_ID, 
                    vec![config::DEFAULT_GGUF_FILENAME]
                ),
                _ => continue,
            };

            // Download Tokenizer
            let tokenizer_repo = config::TOKENIZER_REPO;
            let tokenizer_path = match self.download_file(tokenizer_repo, "tokenizer.json").await {
                Ok(path) => path,
                Err(e) => {
                    error!("Failed to download tokenizer for {}: {:?}", tokenizer_repo, e);
                    continue; // Skip model if tokenizer fails
                }
            };

            let mut paths = Vec::new();
            let mut success = true;
            for filename in filenames {
                match self.download_file(repo_id, filename).await {
                    Ok(path) => paths.push(path),
                    Err(e) => {
                        error!("Failed to download {} for {}: {:?}", filename, repo_id, e);
                        success = false;
                        break;
                    }
                }
            }

            if success {
                downloaded.push((assignment.model_id.clone(), paths, tokenizer_path));
            }
        }
        downloaded
    }
}
