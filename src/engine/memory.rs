use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::engine::types::ChatTurn;

pub struct ConversationMemory {
    // Session ID -> List of turns
    sessions: Arc<Mutex<HashMap<String, Vec<ChatTurn>>>>,
    max_turns: usize,
}

impl ConversationMemory {
    pub fn new(max_turns: usize) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            max_turns,
        }
    }

    /// Adds a turn to the session history.
    pub fn add_turn(&self, session_id: &str, user_prompt: String, assistant_response: String) {
        let mut sessions = self.sessions.lock().unwrap();
        let history = sessions.entry(session_id.to_string()).or_insert_with(Vec::new);
        
        history.push(ChatTurn {
            user: user_prompt,
            assistant: assistant_response,
        });

        // Limit history size
        if history.len() > self.max_turns {
            history.remove(0);
        }
    }

    /// Gets history for a session.
    pub fn get_history(&self, session_id: &str) -> Option<Vec<ChatTurn>> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    pub fn clear_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id);
    }
}

impl Default for ConversationMemory {
    fn default() -> Self {
        Self::new(10) // Store last 10 turns by default
    }
}
