use serde::{Serialize, Deserialize};
use crate::engine::benchmark::HardwareProfile;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    pub model_id: String,
    pub total_layers: usize,
    pub start_layer: usize,
    pub end_layer: usize,
    pub vram_required_gb: f32,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub model_id: String,
    pub total_layers: usize,
    pub layer_cost_gb: f32,
}

/// Returns the configuration for a given model.
pub fn get_model_config(model_id: &str) -> ModelConfig {
    match model_id {
        "llama-3.2-1b" => ModelConfig {
            model_id: "llama-3.2-1b".to_string(),
            total_layers: 16,
            layer_cost_gb: 0.1, // ~1.6GB total for 1B model (quantized)
        },
        "llama-3.2-3b" => ModelConfig {
            model_id: "llama-3.2-3b".to_string(),
            total_layers: 28,
            layer_cost_gb: 0.15, // ~4.2GB total
        },
        "llama-3-8b" => ModelConfig {
            model_id: "llama-3-8b".to_string(),
            total_layers: 32,
            layer_cost_gb: 0.25, // ~8GB total (4-bit quantization)
        },
        _ => ModelConfig {
            model_id: "generic-7b".to_string(),
            total_layers: 32,
            layer_cost_gb: 0.25,
        },
    }
}

/// Calculates which models/layers this node should host.
/// 1. Every node hosts the full "Lightest" model (1B) for local fallback.
/// 2. Every node hosts as many layers of the "Standard" model (8B) as remaining RAM/VRAM allows.
pub fn calculate_shard_assignment(
    hw: &HardwareProfile
) -> Vec<ShardMetadata> {
    let mut assignments = Vec::new();
    let mut remaining_vram = hw.vram_gb.unwrap_or(0) as f32;
    let mut remaining_ram = hw.total_ram_gb as f32 * 0.8; // Use up to 80% of RAM total

    // 1. Base Layer: Llama-3.2-1B (Full)
    let base_config = get_model_config("llama-3.2-1b");
    let base_cost = base_config.total_layers as f32 * base_config.layer_cost_gb;
    
    if remaining_vram >= base_cost || remaining_ram >= base_cost {
        assignments.push(ShardMetadata {
            model_id: base_config.model_id.clone(),
            total_layers: base_config.total_layers,
            start_layer: 0,
            end_layer: base_config.total_layers,
            vram_required_gb: base_cost,
        });
        
        if remaining_vram >= base_cost {
            remaining_vram -= base_cost;
        } else {
            remaining_ram -= base_cost;
        }
        info!("Assigned Base Model: {} (Full)", base_config.model_id);
    }

    // 2. Swarm Layer: Llama-3-8B (As many layers as fit)
    let swarm_config = get_model_config("llama-3-8b");
    let layer_cost = swarm_config.layer_cost_gb;
    
    let max_layers_vram = (remaining_vram / layer_cost).floor() as usize;
    let max_layers_ram = (remaining_ram / layer_cost).floor() as usize;
    let layers_to_host = std::cmp::min(
        std::cmp::max(max_layers_vram, max_layers_ram),
        swarm_config.total_layers
    );

    if layers_to_host > 0 {
        assignments.push(ShardMetadata {
            model_id: swarm_config.model_id.clone(),
            total_layers: swarm_config.total_layers,
            start_layer: 0,
            end_layer: layers_to_host,
            vram_required_gb: layers_to_host as f32 * layer_cost,
        });
        info!("Assigned Swarm Shard: {} ({} layers)", swarm_config.model_id, layers_to_host);
    }

    assignments
}
