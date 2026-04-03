use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_qwen2 as model;
use tokenizers::Tokenizer;
use tracing::info;
use std::path::PathBuf;
use std::time::Instant;

use crate::error::{Result, SocialKubeError};
use crate::config;
use crate::engine::types::ChatTurn;

/// Trait for different LLM inference backends.
pub trait ModelBackend: Send + Sync {
    /// Resets the internal state (KV cache) of the model.
    fn clear_kv_cache(&mut self);

    /// Loads the model weights from the provided paths.
    fn load_model(&mut self, weights_paths: Vec<PathBuf>) -> Result<()>;

    /// Generates text from a prompt, optionally including session history.
    fn generate_text(&mut self, prompt: &str, max_tokens: usize, history: Option<&[ChatTurn]>) -> Result<String>;
}

/// A specialized backend for Qwen2.5 models using GGUF quantization.
pub struct QwenBackend {
    device: Device,
    tokenizer: Tokenizer,
    model: Option<model::ModelWeights>,
}

impl QwenBackend {
    /// Creates a new QwenBackend with an appropriate device.
    pub fn new() -> Result<Self> {
        let device = if candle_core::utils::cuda_is_available() {
            Device::new_cuda(0).map_err(|e| SocialKubeError::Inference(format!("CUDA error: {:?}", e)))?
        } else if candle_core::utils::metal_is_available() {
            Device::new_metal(0).map_err(|e| SocialKubeError::Inference(format!("Metal error: {:?}", e)))?
        } else {
            Device::Cpu
        };
        
        info!("Inference Backend Initialized with device: {:?}", device);

        let tokenizer_path = config::get_tokenizer_path();
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| SocialKubeError::Inference(format!("Failed to load {:?}: {:?}", tokenizer_path, e)))?;

        Ok(Self {
            device,
            tokenizer,
            model: None,
        })
    }
}

impl ModelBackend for QwenBackend {
    fn clear_kv_cache(&mut self) {
        // Quantized models in candle-transformers reset the cache when index_pos is 0.
        // We ensure pos 0 is used during prefill.
    }

    fn load_model(&mut self, weights_paths: Vec<PathBuf>) -> Result<()> {
        let gguf_path = weights_paths.iter()
            .find(|p| p.extension().map_or(false, |ext| ext == "gguf"))
            .ok_or_else(|| SocialKubeError::Inference("No GGUF file found in downloaded paths".into()))?;

        info!("Loading Quantized Qwen2.5 (GGUF) from: {:?}", gguf_path);
        
        let mut file = std::fs::File::open(gguf_path)
            .map_err(|e| SocialKubeError::Inference(format!("Failed to open GGUF file: {:?}", e)))?;
        
        let model = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(|e| SocialKubeError::Inference(format!("Failed to read GGUF content: {:?}", e)))?;
        
        let weights = model::ModelWeights::from_gguf(model, &mut file, &self.device)
            .map_err(|e| SocialKubeError::Inference(format!("Failed to load ModelWeights from GGUF: {:?}", e)))?;
        
        self.model = Some(weights);
        
        info!("Qwen2.5 (Quantized) model loaded successfully on {:?}", self.device);
        Ok(())
    }

    fn generate_text(&mut self, prompt: &str, max_tokens: usize, history: Option<&[ChatTurn]>) -> Result<String> {
        let model = self.model.as_mut().ok_or_else(|| SocialKubeError::Inference("Model not loaded".into()))?;
        
        let mut formatted_prompt = String::new();
        if let Some(history) = history {
            for turn in history {
                formatted_prompt.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>\n", turn.user, turn.assistant));
            }
        }
        formatted_prompt.push_str(&config::get_prompt_template(prompt));

        let tokens = self.tokenizer.encode(formatted_prompt, true)
            .map_err(|e| SocialKubeError::Inference(format!("Tokenizer error: {:?}", e)))?;
        
        let prompt_tokens = tokens.get_ids();
        let mut generated_text = String::new();
        
        info!("Starting quantized prefill for {} tokens on {:?}...", prompt_tokens.len(), self.device);
        let start_prefill = Instant::now();
        
        // 1. Prefill (index_pos=0 resets KV cache)
        let input = Tensor::new(prompt_tokens, &self.device)
            .map_err(|e| SocialKubeError::Inference(format!("Tensor creation error: {:?}", e)))?
            .unsqueeze(0)
            .map_err(|e| SocialKubeError::Inference(format!("Tensor unsqueeze error: {:?}", e)))?;
        
        let logits = model.forward(&input, 0)
            .map_err(|e| SocialKubeError::Inference(format!("Model forward (prefill) error: {:?}", e)))?;
        
        let mut next_token = logits.flatten_all()
            .map_err(|e| SocialKubeError::Inference(format!("Logits flatten error: {:?}", e)))?
            .argmax(0)
            .map_err(|e| SocialKubeError::Inference(format!("Argmax error: {:?}", e)))?
            .to_scalar::<u32>()
            .map_err(|e| SocialKubeError::Inference(format!("Scalar conversion error: {:?}", e)))?;
        
        info!("Prefill finished in {:?}.", start_prefill.elapsed());
        
        let mut pos = prompt_tokens.len();
        let start_gen = Instant::now();

        for i in 0..max_tokens {
            if config::is_eos_token(next_token) {
                break;
            }

            let decoded = self.tokenizer.decode(&[next_token], true)
                .map_err(|e| SocialKubeError::Inference(format!("Decoding error: {:?}", e)))?;
            generated_text.push_str(&decoded);

            // 2. Incremental Step
            let input = Tensor::new(&[next_token], &self.device)
                .map_err(|e| SocialKubeError::Inference(format!("Tensor creation error: {:?}", e)))?
                .unsqueeze(0)
                .map_err(|e| SocialKubeError::Inference(format!("Tensor unsqueeze error: {:?}", e)))?;
            
            let logits = model.forward(&input, pos)
                .map_err(|e| SocialKubeError::Inference(format!("Model forward step error: {:?}", e)))?;
            
            next_token = logits.flatten_all()
                .map_err(|e| SocialKubeError::Inference(format!("Logits flatten error: {:?}", e)))?
                .argmax(0)
                .map_err(|e| SocialKubeError::Inference(format!("Argmax error: {:?}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| SocialKubeError::Inference(format!("Scalar conversion error: {:?}", e)))?;
            
            pos += 1;

            if (i + 1) % 10 == 0 {
                let elapsed = start_gen.elapsed().as_secs_f32();
                if elapsed > 0.0 {
                    info!("Quantized speed: {:.2} tokens/sec", (i + 1) as f32 / elapsed);
                }
            }
        }

        Ok(generated_text)
    }
}
