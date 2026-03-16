use candle_core::{Device, Tensor};
use candle_transformers::models::llama as model;
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;
use tracing::info;
use std::path::PathBuf;

pub struct CandleBackend {
    device: Device,
    tokenizer: Tokenizer,
    model: Option<model::Llama>,
    cache: model::Cache,
    config: model::Config,
}

impl CandleBackend {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let device = if candle_core::utils::cuda_is_available() {
            Device::new_cuda(0)?
        } else if candle_core::utils::metal_is_available() {
            Device::new_metal(0)?
        } else {
            Device::Cpu
        };
        
        info!("Inference Backend Initialized with device: {:?}", device);

        // Load tokenizer from a local file or fallback
        let tokenizer = Tokenizer::from_file("tokenizer.json")
            .map_err(|_| "tokenizer.json not found. Please ensure ModelDownloader has run.")?;

        // Default config for initialization
        let config = model::Config::config_7b_v2(false);
        let cache = model::Cache::new(true, candle_core::DType::F32, &config, &device)?;

        Ok(Self {
            device,
            tokenizer,
            model: None,
            cache,
            config,
        })
    }

    /// Loads the weights for a specific model assignment.
    pub fn load_model(&mut self, weights_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading model weights from: {:?}", weights_path);
        
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &self.device)?
        };

        // Llama-3.2-1B Config (Corrected for candle-transformers 0.8)
        let config = model::Config {
            hidden_size: 2048,
            num_hidden_layers: 16,
            num_attention_heads: 32,
            num_key_value_heads: 8,
            intermediate_size: 8192,
            vocab_size: 128256,
            rms_norm_eps: 1e-5,
            rope_theta: 500000.0,
            use_flash_attn: false,
            bos_token_id: Some(128000),
            eos_token_id: Some(model::LlamaEosToks::Single(128001)),
            max_position_embeddings: 131072,
            rope_scaling: None,
            tie_word_embeddings: false,
        };

        let llama = model::Llama::load(vb, &config)?;
        self.cache = model::Cache::new(true, candle_core::DType::F32, &config, &self.device)?;
        self.model = Some(llama);
        self.config = config;
        
        info!("Model weights loaded successfully.");
        Ok(())
    }

    /// Executes a forward pass to generate text.
    pub fn generate_text(&mut self, prompt: &str, max_tokens: usize) -> Result<String, Box<dyn std::error::Error>> {
        let model = self.model.as_mut().ok_or("Model not loaded")?;
        
        let tokens = self.tokenizer.encode(prompt, true).map_err(|e| e.to_string())?;
        let mut tokens = tokens.get_ids().to_vec();
        let mut generated_text = String::new();

        for i in 0..max_tokens {
            let context_size = tokens.len();
            let input = Tensor::new(&tokens[context_size - 1..], &self.device)?.unsqueeze(0)?;
            let logits = model.forward(&input, i, &mut self.cache)?;
            let logits = logits.squeeze(0)?;
            let next_token = logits.argmax(0)?.to_scalar::<u32>()?;
            
            // Check EOS
            if let Some(eos) = &self.config.eos_token_id {
                let is_eos = match eos {
                    model::LlamaEosToks::Single(id) => next_token == *id,
                    model::LlamaEosToks::Multiple(ids) => ids.contains(&next_token),
                };
                if is_eos { break; }
            }

            tokens.push(next_token);
            let decoded = self.tokenizer.decode(&[next_token], true).map_err(|e| e.to_string())?;
            generated_text.push_str(&decoded);
        }

        Ok(generated_text)
    }
}
