use serde::{Deserialize, Serialize};
use crate::engine::benchmark::HardwareProfile;
use tracing::info;
use crate::config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    pub model_id: String,
    pub start_layer: usize,
    pub end_layer: usize,
    pub is_full: bool,
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
        config::DEFAULT_MODEL_ID => ModelConfig {
            model_id: model_id.to_string(),
            total_layers: 36,
            layer_cost_gb: 0.15, // ~5.4GB total (FP16)
        },
        "qwen2.5-7b" => ModelConfig {
            model_id: "qwen2.5-7b".to_string(),
            total_layers: 28,
            layer_cost_gb: 0.5, // ~14GB total (FP16)
        },
        _ => ModelConfig {
            model_id: config::DEFAULT_MODEL_ID.to_string(),
            total_layers: 36,
            layer_cost_gb: 0.15,
        },
    }
}

/// Calculates which models/layers this node should host based on hardware.
pub fn calculate_shard_assignment(
    hw: &HardwareProfile
) -> Vec<ShardMetadata> {
    let mut assignments = Vec::new();
    let mut remaining_vram = hw.vram_gb.unwrap_or(0) as f32;
    let mut remaining_ram = hw.total_ram_gb as f32 * 0.8; // Use up to 80% of RAM total

    // 1. Base Layer: Primary model (Full)
    let base_config = get_model_config(config::DEFAULT_MODEL_ID);
    let base_cost = base_config.total_layers as f32 * base_config.layer_cost_gb;
    
    if remaining_vram >= base_cost || remaining_ram >= base_cost {
        assignments.push(ShardMetadata {
            model_id: base_config.model_id.clone(),
            start_layer: 0,
            end_layer: base_config.total_layers,
            is_full: true,
        });

        // Deduct resource cost
        if remaining_vram >= base_cost {
            remaining_vram -= base_cost;
        } else {
            remaining_ram -= base_cost;
        }
        info!("Assigned Base Model: {} (Full)", base_config.model_id);
    }

    // 2. Swarm Layer: Larger model (Sharded)
    // ONLY if we are already hosting the base model (for local reliability)
    if !assignments.is_empty() {
        let swarm_config = get_model_config("qwen2.5-7b");
        let layer_cost = swarm_config.layer_cost_gb;
        
        let max_layers_vram = (remaining_vram / layer_cost).floor() as usize;
        let max_layers_ram = (remaining_ram / layer_cost).floor() as usize;
        
        let total_allocatable = std::cmp::max(max_layers_vram, max_layers_ram);
        let layers_to_host = std::cmp::min(total_allocatable, swarm_config.total_layers);

        if layers_to_host > 0 {
            assignments.push(ShardMetadata {
                model_id: swarm_config.model_id.clone(),
                start_layer: 0, 
                end_layer: layers_to_host,
                is_full: layers_to_host == swarm_config.total_layers,
            });
            info!("Assigned Swarm Model: {} ({} layers)", swarm_config.model_id, layers_to_host);
        }
    }

    assignments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_assignment_high_end_gpu() {
        let hw = HardwareProfile {
            cpu_model: "test".to_string(),
            cpu_cores: 8,
            total_ram_gb: 32,
            gpu_name: Some("RTX 4090".to_string()),
            vram_gb: Some(24),
        };

        let assignments = calculate_shard_assignment(&hw);
        
        // Should have both models
        assert!(assignments.len() >= 1);
        assert_eq!(assignments[0].model_id, config::DEFAULT_MODEL_ID);
        assert!(assignments[0].is_full);
        
        // Should also have some layers of the larger model
        assert!(assignments.iter().any(|a| a.model_id == "qwen2.5-7b"));
    }

    #[test]
    fn test_shard_assignment_low_end_cpu_only() {
        let hw = HardwareProfile {
            cpu_model: "test".to_string(),
            cpu_cores: 4,
            total_ram_gb: 8,
            gpu_name: None,
            vram_gb: None,
        };

        let assignments = calculate_shard_assignment(&hw);
        
        // 8GB * 0.8 = 6.4GB available.
        // Base model (3B) is ~5.4GB.
        // Remaining 1GB can fit 2 layers of Qwen 7B (0.5GB each).
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].model_id, config::DEFAULT_MODEL_ID);
        assert!(assignments[0].is_full);
        assert_eq!(assignments[1].model_id, "qwen2.5-7b");
    }

    #[test]
    fn test_shard_assignment_very_low_ram() {
        let hw = HardwareProfile {
            cpu_model: "test".to_string(),
            cpu_cores: 2,
            total_ram_gb: 4,
            gpu_name: None,
            vram_gb: None,
        };

        let assignments = calculate_shard_assignment(&hw);
        
        // 4GB * 0.8 = 3.2GB. Base model (5.4GB) won't fit.
        assert!(assignments.is_empty());
    }
}
