use serde::{Deserialize, Serialize};

// ============================================================
// NODE CONFIG — Single Source of Truth
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeConfig {
    pub probed_at: String,
    pub hardware: HardwareInfo,
    pub compute: ComputeInfo,
    pub llm: LlmConfig,
    pub env: EnvInfo,
    pub suggestions: Vec<String>, // Sovereignty Advisor output
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareInfo {
    pub total_ram_gb: f64,
    pub cpu_cores: usize,
    pub cpu_model: String,
}

/// Universal compute backend — determined by Ring 0 from hardware reality
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ComputeBackend {
    Cuda,   // NVIDIA GPU — highest throughput
    ROCm,   // AMD dGPU via ROCm (/dev/kfd)
    Vulkan, // AMD/Intel iGPU or dGPU via Vulkan — universal fallback for GPU
    Metal,  // Apple Silicon — always available on macOS
    #[default]
    Cpu, // Fallback — always works, AVX2 accelerated if available
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeInfo {
    pub backend: ComputeBackend,
    pub gpu_name: String,
    pub vram_gb: f64, // dedicated VRAM (or shared RAM for iGPU)
    pub cpu_threads: usize,
    pub avx2: bool,
    pub is_igpu: bool, // true = shares system RAM
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    /// CYNIC-managed models directory — always exists after first boot
    pub models_dir: String,
    /// All GGUF models found on this machine (all tiers combined)
    pub gguf_models: Vec<GgufModel>,
    /// Running inference server URL (Ollama, llama-server, vLLM…)
    pub running_server_url: Option<String>,
    /// Currently selected model name
    pub active_model: Option<String>,
    /// Optimal llama-server flags for this machine's compute
    pub llama_server_flags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GgufModel {
    pub path: String,
    pub size_gb: f64,
    pub name: String,
    pub source: String, // "cynic" | "ollama" | "filesystem"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvInfo {
    pub os: String,
    pub is_wsl2: bool,
    pub is_docker: bool,
    pub is_proxmox_lxc: bool,
    pub wsl2_windows_host: Option<String>,
    pub io_benchmark_mbps: f64, // Real sequential I/O speed
}

// ============================================================
// SOVEREIGNTY ADVISOR
// ============================================================

#[derive(Debug)]
pub struct SovereigntyAdvisor;

impl SovereigntyAdvisor {
    pub fn analyze(cfg: &NodeConfig) -> Vec<String> {
        let mut suggestions = vec![];

        // 1. Storage Friction
        if cfg.env.io_benchmark_mbps < 200.0 {
            suggestions.push(format!(
                "SLOW IO ({:.1} MB/s): Modèles sur filesystem lent (Virtual/Network?). Migrez vers un disque local pour +500%% de perf.",
                cfg.env.io_benchmark_mbps
            ));
        }

        // 2. Compute Friction
        if cfg.compute.backend == ComputeBackend::Cpu {
            if cfg.env.is_wsl2 {
                suggestions.push("GPU HIDDEN: GPU détecté sur Windows mais invisible par WSL2. Activez le passthrough ou utilisez le backend Vulkan.".to_string());
            } else {
                suggestions.push("NO HW ACCEL: Aucune accélération GPU trouvée. Vérifiez vos pilotes CUDA/ROCm/Vulkan.".to_string());
            }
        }

        // 3. Virtualization Friction
        if cfg.compute.gpu_name.to_lowercase().contains("parsec")
            || cfg.compute.gpu_name.to_lowercase().contains("virtual")
        {
            suggestions.push("VIRTUAL DISPLAY DETECTED: CYNIC utilise un adaptateur virtuel. Forcez l'usage du GPU physique (Radeon/NVIDIA) pour l'inférence.".to_string());
        }

        // 4. Memory Calibration
        if cfg.hardware.total_ram_gb < 16.0 {
            suggestions.push("CRITICAL RAM: Mémoire faible pour les modèles Tier S. Activez le swap ou l'IOMMU pour éviter les crashs.".to_string());
        }

        // 5. Disk Metabolic Check (Site Reliability Lens)
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "C:".to_string());
        if (home.starts_with("C:") || home.starts_with("/"))
            && cfg.env.os == "windows"
            && cfg.llm.models_dir.starts_with("C:")
        {
            suggestions.push("STORAGE CHOKING: C: drive est saturé. La redirection vers D:\\CYNIC-v2 est fortement recommandée pour la stabilité.".to_string());
        }

        suggestions
    }
}
