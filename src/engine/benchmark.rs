use sysinfo::System;
use serde::{Serialize, Deserialize};
use tracing::info;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HardwareProfile {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub total_ram_gb: u64,
    pub gpu_name: Option<String>,
    pub vram_gb: Option<u64>,
}

/// Detects the local hardware and returns a HardwareProfile.
pub fn detect_hardware() -> HardwareProfile {
    let mut sys = System::new_all();
    sys.refresh_all();

    // CPU Info (Works on Linux, macOS, Windows, Docker)
    let cpu_model = sys.cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());
    let cpu_cores = sys.cpus().len();

    // RAM Info (bytes -> GB)
    let total_ram_gb = sys.total_memory() / 1024 / 1024 / 1024;

    // GPU Info - Cross-Platform Detection
    let (gpu_name, vram_gb) = detect_gpu();

    let profile = HardwareProfile {
        cpu_model,
        cpu_cores,
        total_ram_gb,
        gpu_name,
        vram_gb,
    };

    info!("Hardware detected: {:?}", profile);
    profile
}

/// Universal GPU detection
fn detect_gpu() -> (Option<String>, Option<u64>) {
    // 1. Try NVIDIA SMI (Standard for Linux, Windows, and Docker with NVIDIA Passthrough)
    if let Some((name, vram)) = detect_nvidia_gpu() {
        return (Some(name), vram);
    }

    // 2. Try macOS specific detection
    if cfg!(target_os = "macos") {
        if let Some((name, vram)) = detect_darwin_gpu() {
            return (Some(name), vram);
        }
    }

    // 3. Fallback for Generic Linux (No NVIDIA)
    if cfg!(target_os = "linux") {
        if let Some((name, vram)) = detect_linux_generic_gpu() {
            return (Some(name), vram);
        }
    }

    (None, None)
}

/// Detects NVIDIA GPU using nvidia-smi.
fn detect_nvidia_gpu() -> Option<(String, Option<u64>)> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total")
        .arg("--format=csv,noheader,nounits")
        .output();

    if let Ok(output) = output {
        let content = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = content.lines().next() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let vram = parts[1].parse::<u64>().ok().map(|m| m / 1024); // MiB to GiB
                return Some((name, vram));
            }
        }
    }
    None
}

/// Detects GPU on macOS using system_profiler.
fn detect_darwin_gpu() -> Option<(String, Option<u64>)> {
    let output = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output();

    if let Ok(output) = output {
        let content = String::from_utf8_lossy(&output.stdout);
        let mut model = None;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("Chipset Model:") {
                model = Some(line.replace("Chipset Model:", "").trim().to_string());
                break;
            }
        }
        if let Some(m) = model {
            return Some((m, None)); // VRAM detection on Mac is complex due to unified memory
        }
    }
    None
}

/// Detects Generic GPU on Linux by checking sysfs.
fn detect_linux_generic_gpu() -> Option<(String, Option<u64>)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("lspci | grep -i vga")
        .output();

    if let Ok(output) = output {
        let content = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = content.lines().next() {
            return Some((line.to_string(), None));
        }
    }
    None
}
