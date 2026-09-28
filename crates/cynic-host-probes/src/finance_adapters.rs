//! Finance adapters for cynic-host-probes.
//!
//! Implements cynic-finance GpuMeter port using Vulkan, ROCm, and CUDA.

use cynic_finance::ports::{GpuMeter, MeterError};
use std::fs;

/// GPU meter — detects and measures GPU usage via Vulkan, ROCm, or CUDA.
pub struct ProbesGpuMeter;

impl GpuMeter for ProbesGpuMeter {
    fn measure_gpu_ms(&self, agent_id: &str) -> Result<u64, MeterError> {
        // Try Vulkan first (universal fallback)
        if let Ok(ms) = self.measure_vulkan_gpu_ms(agent_id) {
            return Ok(ms);
        }

        // Try ROCm
        if let Ok(ms) = self.measure_rocm_gpu_ms(agent_id) {
            return Ok(ms);
        }

        // Try CUDA
        if let Ok(ms) = self.measure_cuda_gpu_ms(agent_id) {
            return Ok(ms);
        }

        Err(MeterError::Unknown(
            "no GPU backend available (Vulkan, ROCm, CUDA)".to_string(),
        ))
    }

    fn measure_vram_bytes(&self, agent_id: &str) -> Result<u64, MeterError> {
        // Try Vulkan
        if let Ok(bytes) = self.measure_vulkan_vram(agent_id) {
            return Ok(bytes);
        }

        // Try ROCm
        if let Ok(bytes) = self.measure_rocm_vram(agent_id) {
            return Ok(bytes);
        }

        // Try CUDA
        if let Ok(bytes) = self.measure_cuda_vram(agent_id) {
            return Ok(bytes);
        }

        Err(MeterError::Unknown(
            "no GPU backend available (Vulkan, ROCm, CUDA)".to_string(),
        ))
    }

    fn measure_gpu_utilization(&self, agent_id: &str) -> Result<f64, MeterError> {
        // Try Vulkan
        if let Ok(util) = self.measure_vulkan_utilization(agent_id) {
            return Ok(util);
        }

        // Try ROCm
        if let Ok(util) = self.measure_rocm_utilization(agent_id) {
            return Ok(util);
        }

        // Try CUDA
        if let Ok(util) = self.measure_cuda_utilization(agent_id) {
            return Ok(util);
        }

        Err(MeterError::Unknown(
            "no GPU backend available (Vulkan, ROCm, CUDA)".to_string(),
        ))
    }
}

impl ProbesGpuMeter {
    fn measure_vulkan_gpu_ms(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // Vulkan GPU time requires VK_EXT_tooling_info or similar
        // For now, return unknown — this needs a Vulkan loader integration
        Err(MeterError::Unknown(
            "Vulkan GPU time measurement requires VK_EXT_tooling_info".to_string(),
        ))
    }

    fn measure_vulkan_vram(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // Vulkan VRAM requires VK_EXT_memory_budget or similar
        Err(MeterError::Unknown(
            "Vulkan VRAM measurement requires VK_EXT_memory_budget".to_string(),
        ))
    }

    fn measure_vulkan_utilization(&self, _agent_id: &str) -> Result<f64, MeterError> {
        Err(MeterError::Unknown(
            "Vulkan utilization measurement requires VK_EXT_tooling_info".to_string(),
        ))
    }

    fn measure_rocm_gpu_ms(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // ROCm SMI provides GPU use time
        // rocm-smi --showuse --csv
        Err(MeterError::Unknown(
            "ROCm GPU time measurement requires rocm-smi".to_string(),
        ))
    }

    fn measure_rocm_vram(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // ROCm SMI provides VRAM usage
        // rocm-smi --showmemuse --csv
        if let Ok(content) = fs::read_to_string("/sys/class/drm/card0/device/mem_info_vram_used") {
            if let Ok(bytes) = content.trim().parse::<u64>() {
                return Ok(bytes);
            }
        }
        Err(MeterError::Unknown(
            "ROCm VRAM measurement requires rocm-smi or /sys/class/drm".to_string(),
        ))
    }

    fn measure_rocm_utilization(&self, _agent_id: &str) -> Result<f64, MeterError> {
        // ROCm SMI provides GPU utilization
        // rocm-smi --showuse --csv
        if let Ok(content) = fs::read_to_string("/sys/class/drm/card0/device/gpu_busy_percent") {
            if let Ok(pct) = content.trim().parse::<f64>() {
                return Ok(pct / 100.0);
            }
        }
        Err(MeterError::Unknown(
            "ROCm utilization measurement requires rocm-smi or /sys/class/drm".to_string(),
        ))
    }

    fn measure_cuda_gpu_ms(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // CUDA GPU time requires NVML
        Err(MeterError::Unknown(
            "CUDA GPU time measurement requires NVML".to_string(),
        ))
    }

    fn measure_cuda_vram(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // CUDA VRAM requires NVML
        Err(MeterError::Unknown(
            "CUDA VRAM measurement requires NVML".to_string(),
        ))
    }

    fn measure_cuda_utilization(&self, _agent_id: &str) -> Result<f64, MeterError> {
        // CUDA utilization requires NVML
        Err(MeterError::Unknown(
            "CUDA utilization measurement requires NVML".to_string(),
        ))
    }
}
