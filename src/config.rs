use std::path::PathBuf;

pub const DEFAULT_MODEL_ID: &str = "qwen2.5-coder-3b";
pub const DEFAULT_REPO_ID: &str = "bartowski/Qwen2.5-Coder-3B-Instruct-GGUF";
pub const DEFAULT_GGUF_FILENAME: &str = "Qwen2.5-Coder-3B-Instruct-Q4_K_M.gguf";
pub const TOKENIZER_REPO: &str = "Qwen/Qwen2.5-Coder-3B-Instruct";
pub const DEFAULT_MAX_TOKENS: usize = 200;

#[allow(dead_code)]
pub struct InferenceConfig {
    pub model_id: String,
    pub max_tokens: usize,
    pub temperature: f64,
    pub top_p: f64,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            model_id: DEFAULT_MODEL_ID.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: 0.7,
            top_p: 0.9,
        }
    }
}

pub fn get_tokenizer_path() -> PathBuf {
    PathBuf::from("tokenizer.json")
}

pub fn get_prompt_template(prompt: &str) -> String {
    format!("<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", prompt)
}

pub fn is_eos_token(token_id: u32) -> bool {
    // Qwen2.5 EOS tokens: 151643 (<|endoftext|>), 151645 (<|im_end|>)
    token_id == 151643 || token_id == 151645
}
