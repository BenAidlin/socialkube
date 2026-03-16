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

/// Calculates which layers of a model this node should host based on its hardware.
pub fn calculate_shard_assignment(model_id: &str, total_layers: usize, hw: &HardwareProfile) -> ShardMetadata {
    let available_vram = hw.vram_gb.unwrap_or(0);
    let available_ram = hw.total_ram_gb;

    // Very rough heuristic:
    // Assume each layer takes ~0.5GB (for a 7B-30B model quantized)
    let layer_cost_gb = 0.5;
    
    let max_layers_vram = if available_vram > 0 {
        (available_vram as f32 / layer_cost_gb).floor() as usize
    } else {
        0
    };

    let max_layers_ram = (available_ram as f32 * 0.5 / layer_cost_gb).floor() as usize; // Use only 50% of system RAM
    
    let can_host_layers = std::cmp::max(max_layers_vram, max_layers_ram);
    let layers_to_host = std::cmp::min(can_host_layers, total_layers);

    // For simplicity in this version, host the first N layers.
    // In a real swarm, this would be negotiated based on what's missing.
    let shard = ShardMetadata {
        model_id: model_id.to_string(),
        total_layers,
        start_layer: 0,
        end_layer: layers_to_host,
        vram_required_gb: layers_to_host as f32 * layer_cost_gb,
    };

    info!("Calculated Shard Assignment: host {} to {} of {} layers", shard.start_layer, shard.end_layer, total_layers);
    shard
}

/// Helper to parse GGUF metadata (Placeholder for actual candle logic)
pub fn get_model_layer_count(_path: &str) -> usize {
    // For now, return a default for Llama-3-8B
    32
}
