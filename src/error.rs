use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum SocialKubeError {
    #[error("Inference engine error: {0}")]
    Inference(String),

    #[error("Model download error: {0}")]
    Download(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Networking error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unexpected error: {0}")]
    Unknown(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SocialKubeError>;
