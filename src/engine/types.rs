use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub user: String,
    pub assistant: String,
}
