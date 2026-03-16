use candle_core::{Device, Tensor};
use candle_transformers::models::llama as model;
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;
use hf_hub::api::sync::Api;
use tracing::{info, error};
use std::path::PathBuf;

pub struct CandleBackend {
    device: Device,
    tokenizer: Tokenizer,
    model: Option<model::Llama>,
}

impl CandleBackend {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Detect Hardware
        let device = if candle_core::utils::cuda_is_available() {
            Device::new_cuda(0)?
        } else if candle_core::utils::metal_is_available() {
            Device::new_metal(0)?
        } else {
            Device::Cpu
        };
        
        info!("Inference Backend Initialized with device: {:?}", device);

        // Placeholder for tokenizer loading
        // In a real scenario, this would load from a path provided by ModelDownloader
        let tokenizer = Tokenizer::from_file("tokenizer.json").unwrap_or_else(|_| Tokenizer::from_pretrained("hf-internal-testing/llama-tokenizer", None).unwrap());

        Ok(Self {
            device,
            tokenizer,
            model: None,
        })
    }

    /// Loads the weights for a specific model assignment.
    pub fn load_model(&mut self, weights_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading model weights from: {:?}", weights_path);
        
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &self.device)?
        };

        // Standard Llama-3.2-1B Config (Instruct)
        let config = model::Config::config_1b_v3();
        let llama = model::Llama::load(vb, &config)?;

        self.model = Some(llama);
        info!("Model weights loaded successfully.");
        Ok(())
    }

    /// Executes a forward pass to generate text.
    pub fn generate_text(&mut self, prompt: &str, max_tokens: usize) -> Result<String, Box<dyn std::error::Error>> {
        let model = self.model.as_mut().ok_or("Model not loaded")?;
        
        info!("Generating response for prompt: '{}'", prompt);
        
        let tokens = self.tokenizer.encode(prompt, true).map_err(|e| e.to_string())?;
        let mut tokens = tokens.get_ids().to_vec();
        
        let mut generated_text = String::new();

        for _ in 0..max_tokens {
            let context_size = tokens.len();
            let input = Tensor::new(&tokens[..], &self.device)?.unsqueeze(0)?;
            let logits = model.forward(&input, context_size - 1)?;
            let logits = logits.squeeze(0)?;
            let next_token = logits.argmax(0)?.to_scalar::<u32>()?;
            
            tokens.push(next_token);
            let decoded = self.tokenizer.decode(&[next_token], true).map_err(|e| e.to_string())?;
            generated_text.push_str(&decoded);
            
            if next_token == 128001 { // EOS Token for Llama-3
                break;
            }
        }

        Ok(generated_text)
    }
}
