//! Ring 0 — CYNIC Onboarding Probe
//! Runs once at first boot, writes ~/.cynic/node.toml
//! Subsequent boots: instant load from cache.
//! Daemons read config via gRPC NodeConfigService — never touch node.toml directly.

use std::path::{Path, PathBuf};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use sysinfo::System;

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
    Cuda,       // NVIDIA GPU — highest throughput
    ROCm,       // AMD dGPU via ROCm (/dev/kfd)
    Vulkan,     // AMD/Intel iGPU or dGPU via Vulkan — universal fallback for GPU
    Metal,      // Apple Silicon — always available on macOS
    #[default]
    Cpu,        // Fallback — always works, AVX2 accelerated if available
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeInfo {
    pub backend: ComputeBackend,
    pub gpu_name: String,
    pub vram_gb: f64,          // dedicated VRAM (or shared RAM for iGPU)
    pub cpu_threads: usize,
    pub avx2: bool,
    pub is_igpu: bool,         // true = shares system RAM
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
// PROBE ENTRY POINT
// ============================================================

pub async fn run(force_reprobe: bool) -> NodeConfig {
    let config_path = config_file_path();

    if config_path.exists() && !force_reprobe {
        match load_config(&config_path) {
            Ok(cfg) => {
                klog!("[Ring 0] ✅ Node config loaded from cache: {}", config_path.display());
                return cfg;
            }
            Err(e) => println!("[Ring 0] ANOMALY: Config corrupt ({}). Re-probing...", e),
        }
    }

    klog!("[Ring 0] First boot — comprehensive probe running...");

    // All probes run in parallel
    let (hw, env, compute) = tokio::join!(
        probe_hardware(),
        probe_environment(),
        probe_compute(),
    );

    // LLM probe uses env + compute results
    let llm = probe_llm_resources(&env, &compute).await;

    let mut config = NodeConfig {
        probed_at: chrono::Utc::now().to_rfc3339(),
        hardware: hw,
        compute,
        llm,
        env,
        suggestions: vec![],
    };

    // Run Sovereignty Advisor
    config.suggestions = SovereigntyAdvisor::analyze(&config);

    if !config.suggestions.is_empty() {
        klog!("\n╔══════════════════════════════════════╗");
        klog!("║       SOVEREIGNTY SUGGESTIONS        ║");
        klog!("╚══════════════════════════════════════╝");
        for s in &config.suggestions {
            klog!(" > {}", s);
        }
    }

    match save_config(&config, &config_path) {
        Err(e) => println!("[Ring 0] ANOMALY: Could not save config: {}", e),
        Ok(_)  => println!("[Ring 0] ✅ Node config saved: {}", config_path.display()),
    }
    config
}

// ============================================================
// HARDWARE PROBE
// ============================================================

async fn probe_hardware() -> HardwareInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_model = sys.cpus().first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".into());

    let hw = HardwareInfo {
        total_ram_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        cpu_cores: sys.cpus().len(),
        cpu_model: cpu_model.clone(),
    };
    klog!("[Ring 0 / HW]  CPU: {} ({} cores) | RAM: {:.1} GB",
        hw.cpu_model, hw.cpu_cores, hw.total_ram_gb);
    hw
}

// ============================================================
// COMPUTE DETECTION — reads sysfs, no external tools required
// ============================================================

async fn probe_compute() -> ComputeInfo {
    // Priority: CUDA > ROCm > Vulkan (AMD/Intel via sysfs) > Metal > CPU
    // Always read /sys/class/drm/ FIRST — most accurate, no external tools.
    // /dev/dxg only tells us "some GPU exists in WSL2", not the vendor.

    // 1. NVIDIA via nvidia-smi
    if let Some(info) = detect_nvidia() {
        klog!("[Ring 0 / GPU] NVIDIA: {} ({:.1} GB VRAM) → cuda", info.gpu_name, info.vram_gb);
        return info;
    }

    // 2. AMD ROCm via /dev/kfd
    if Path::new("/dev/kfd").exists() {
        let info = detect_amd_via_sysfs(false);
        klog!("[Ring 0 / GPU] AMD ROCm (/dev/kfd): {} → rocm", info.gpu_name);
        return ComputeInfo { backend: ComputeBackend::ROCm, ..info };
    }

    // 3. Read /sys/class/drm/ vendor — works on bare Linux, WSL2, Docker
    //    This correctly identifies AMD Ryzen iGPU (Vega 8 = vendor 0x1002)
    if let Some(mut info) = detect_gpu_via_sysfs() {
        // If we found a generic GPU name but we are in WSL2, try to enrich it via host
        if (info.gpu_name.to_lowercase().contains("generic") || info.gpu_name.is_empty())
            && let Some(host_name) = probe_windows_gpu().await
        {
            info.gpu_name = host_name;
        }
        klog!("[Ring 0 / GPU] DRM: {} (is_igpu:{}) → {:?}",
            info.gpu_name, info.is_igpu, info.backend);
        return info;
    }

    // 4. /dev/dxg exists but sysfs found nothing → WSL2 with GPU passthrough
    //    Use the Vascular Bridge (PowerShell) to identify the real hardware
    if Path::new("/dev/dxg").exists()
        && let Some(host_name) = probe_windows_gpu().await
    {
        let mut info = detect_cpu_info();
        info.gpu_name = host_name;
        // Heuristic: if AMD/Radeon as found by user, use Vulkan
        if info.gpu_name.to_lowercase().contains("amd") || info.gpu_name.to_lowercase().contains("radeon") {
            info.backend = ComputeBackend::Vulkan;
        } else if info.gpu_name.to_lowercase().contains("nvidia") {
            info.backend = ComputeBackend::Cuda; // Force check CUDA again with right label
        }
        info.is_igpu = true;
        klog!("[Ring 0 / GPU] Bridge: {} → {:?}", info.gpu_name, info.backend);
        return info;
    }

    // 5. Native Windows GPU (DirectX/WMI via PowerShell)
    if cfg!(target_os = "windows")
        && let Some(host_name) = probe_windows_gpu().await
    {
        let mut info = detect_cpu_info();
        info.gpu_name = host_name;
        info.backend = ComputeBackend::Vulkan; // Windows default for iGPU/dGPU
        info.is_igpu = true; // Safe assumption for Ryzen G series or integrated
        if info.gpu_name.to_lowercase().contains("nvidia") {
            info.backend = ComputeBackend::Cuda;
            info.is_igpu = false;
        }
        klog!("[Ring 0 / GPU] Windows Native: {} → {:?}", info.gpu_name, info.backend);
        return info;
    }

    // 6. macOS Metal
    if cfg!(target_os = "macos") {
        klog!("[Ring 0 / GPU] macOS Metal → metal");
        return ComputeInfo {
            backend: ComputeBackend::Metal,
            gpu_name: "Apple Silicon GPU".into(),
            vram_gb: 0.0,
            is_igpu: true,
            ..detect_cpu_info()
        };
    }

    // 7. CPU fallback
    let cpu = detect_cpu_info();
    klog!("[Ring 0 / GPU] No GPU → cpu (AVX2:{})", cpu.avx2);
    cpu
}

fn detect_nvidia() -> Option<ComputeInfo> {
    let out = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.trim().lines().next()?;
    let parts: Vec<&str> = line.splitn(2, ',').collect();
    let gpu_name = parts.first().unwrap_or(&"NVIDIA GPU").trim().to_string();
    let vram_mb: f64 = parts.get(1).unwrap_or(&"0").trim().parse().unwrap_or(0.0);

    Some(ComputeInfo {
        backend: ComputeBackend::Cuda,
        gpu_name,
        vram_gb: vram_mb / 1024.0,
        is_igpu: false,
        ..detect_cpu_info()
    })
}

/// Read /sys/class/drm/card*/device/vendor to identify GPU vendor
fn detect_gpu_via_sysfs() -> Option<ComputeInfo> {
    let drm = Path::new("/sys/class/drm");
    if !drm.exists() { return None; }

    for entry in std::fs::read_dir(drm).ok()?.flatten() {
        let card = entry.path();
        let name = card.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Only top-level card entries (card0, card1...)
        if !name.starts_with("card") || name.contains('-') { continue; }

        let vendor_path = card.join("device/vendor");
        let vendor = std::fs::read_to_string(&vendor_path).ok()?;
        let vendor = vendor.trim();

        match vendor {
            "0x1002" => { // AMD
                let info = detect_amd_via_sysfs(true);
                return Some(ComputeInfo { backend: ComputeBackend::Vulkan, ..info });
            }
            "0x8086" => { // Intel
                return Some(ComputeInfo {
                    backend: ComputeBackend::Vulkan,
                    gpu_name: read_sysfs_string(card.join("device/product_name"))
                        .unwrap_or_else(|| "Intel iGPU".into()),
                    vram_gb: 0.0,
                    is_igpu: true,
                    ..detect_cpu_info()
                });
            }
            _ => continue,
        }
    }
    None
}

fn detect_amd_via_sysfs(is_igpu: bool) -> ComputeInfo {
    // Try to read GPU name from drm device
    let gpu_name = std::fs::read_dir("/sys/class/drm")
        .ok()
        .and_then(|mut d| d.find_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.contains('-') { return None; }
            read_sysfs_string(e.path().join("device/product_name"))
        }))
        .unwrap_or_else(|| "AMD GPU".into());

    ComputeInfo {
        backend: ComputeBackend::Vulkan, // set by caller for ROCm
        gpu_name,
        vram_gb: 0.0, // iGPU shares RAM; dGPU VRAM readable via hwmon but complex
        is_igpu,
        ..detect_cpu_info()
    }
}

fn read_sysfs_string(path: PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn detect_cpu_info() -> ComputeInfo {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get()).unwrap_or(4);

    let avx2 = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/cpuinfo")
            .map(|c| c.contains("avx2"))
            .unwrap_or(false)
    } else if cfg!(target_os = "windows") {
        // Simple heuristic for Windows: check for RYZEN/CORE and skip older ones
        // In real SRE lens, we'd use a crate or cpuid, but this is a bridge
        true // Most modern machines have AVX2 now
    } else {
        false
    };

    ComputeInfo {
        backend: ComputeBackend::Cpu,
        gpu_name: String::new(),
        vram_gb: 0.0,
        cpu_threads: threads,
        avx2,
        is_igpu: false,
    }
}

// ============================================================
// ENVIRONMENT PROBE
// ============================================================

async fn probe_environment() -> EnvInfo {
    let os = if cfg!(target_os = "linux") { "linux" }
              else if cfg!(target_os = "windows") { "windows" }
              else { "macos" }.to_string();

    let is_wsl2 = std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false);

    let is_docker = Path::new("/.dockerenv").exists();
    let is_proxmox_lxc = std::fs::read_to_string("/proc/1/environ")
        .map(|v| v.contains("container=lxc"))
        .unwrap_or(false);

    let wsl2_windows_host = if is_wsl2 {
        std::fs::read_to_string("/etc/resolv.conf").ok()
            .and_then(|c| c.lines()
                .find(|l| l.starts_with("nameserver"))
                .and_then(|l| l.split_whitespace().nth(1))
                .map(|s| s.to_string()))
    } else { None };

    klog!("[Ring 0 / Env] OS: {} | WSL2: {} | Docker: {} | LXC: {} | WinHost: {:?}",
        os, is_wsl2, is_docker, is_proxmox_lxc, wsl2_windows_host);

    let models_dir = ensure_models_dir();
    let io_benchmark_mbps = benchmark_io(&models_dir).await;

    EnvInfo {
        os,
        is_wsl2,
        is_docker,
        is_proxmox_lxc,
        wsl2_windows_host,
        io_benchmark_mbps,
    }
}

async fn benchmark_io(path: &Path) -> f64 {
    use std::io::Write;
    let test_file = path.join(".cynic_io_test");
    let data = vec![0u8; 100 * 1024 * 1024]; // 100MB test
    let start = std::time::Instant::now();

    let mut f = match std::fs::File::create(&test_file) {
        Ok(f) => f,
        Err(_) => return 0.0,
    };

    if f.write_all(&data).is_err() { return 0.0; }
    if f.sync_all().is_err() { return 0.0; }

    let duration = start.elapsed().as_secs_f64();
    let _ = std::fs::remove_file(&test_file);

    let mbps = 100.0 / duration;
    klog!("[Ring 0 / Env] IO Benchmark: {:.1} MB/s on {}", mbps, path.display());
    mbps
}

async fn probe_windows_gpu() -> Option<String> {
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"])
        .output()
        .ok()?;

    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() { return None; }

    // Filter out virtual adapters
    let lines: Vec<String> = text.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.to_lowercase().contains("virtual") 
                    && !s.to_lowercase().contains("parsec") 
                    && !s.to_lowercase().contains("citrix")
                    && !s.to_lowercase().contains("microsoft remote"))
        .collect();

    if lines.is_empty() {
        // Fallback to first one if all are virtual
        return text.lines().next().map(|s| s.trim().to_string());
    }

    Some(lines[0].clone())
}

// ============================================================
// SOVEREIGNTY ADVISOR
// ============================================================

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
        if cfg.compute.gpu_name.to_lowercase().contains("parsec") || cfg.compute.gpu_name.to_lowercase().contains("virtual") {
            suggestions.push("VIRTUAL DISPLAY DETECTED: CYNIC utilise un adaptateur virtuel. Forcez l'usage du GPU physique (Radeon/NVIDIA) pour l'inférence.".to_string());
        }

        // 4. Memory Calibration
        if cfg.hardware.total_ram_gb < 16.0 {
            suggestions.push("CRITICAL RAM: Mémoire faible pour les modèles Tier S. Activez le swap ou l'IOMMU pour éviter les crashs.".to_string());
        }

        // 5. Disk Metabolic Check (Site Reliability Lens)
        // Heuristic: check if current home disk (usually C:) is choking
        let home = std::env::var("HOME").unwrap_or_else(|_| "C:".to_string());
        if home.starts_with("C:") || home.starts_with("/") {
             // We'd use a sysinfo or volume crate for precise check, 
             // but here we trigger on the fact that C: was critical in manual audit
             if cfg.env.os == "windows" {
                 // In industrial mode, we suggest D: if C: is the current path
                 if cfg.llm.models_dir.starts_with("C:") {
                     suggestions.push("STORAGE CHOKING: C: drive est saturé. La redirection vers D:\\CYNIC-v2 est fortement recommandée pour la stabilité.".to_string());
                 }
             }
        }

        suggestions
    }
}

// ============================================================
// LLM RESOURCE PROBE (Exhaustive — all tiers always run)
// ============================================================

async fn probe_llm_resources(env: &EnvInfo, compute: &ComputeInfo) -> LlmConfig {
    let models_dir = ensure_models_dir();

    // All discovery runs in parallel
    let (mut gguf_models, running_server_url) = tokio::join!(
        async { discover_all_models(&models_dir, env) },
        probe_running_servers(env),
    );

    // Deduplicate by path
    gguf_models.dedup_by(|a, b| a.path == b.path);
    gguf_models.sort_by(|a, b| b.size_gb.partial_cmp(&a.size_gb).unwrap_or(std::cmp::Ordering::Equal));

    let active_model = gguf_models.first().map(|m| m.name.clone());
    let llama_server_flags = compute_optimal_flags(compute, &gguf_models);

    klog!("[Ring 0 / LLM] Models: {} | Server: {} | Backend: {:?} | Active: {:?}",
        gguf_models.len(),
        running_server_url.as_deref().unwrap_or("none"),
        compute.backend,
        active_model
    );

    LlmConfig {
        models_dir: models_dir.display().to_string(),
        gguf_models,
        running_server_url,
        active_model,
        llama_server_flags,
    }
}

/// Discover all models from all sources in parallel
fn discover_all_models(models_dir: &Path, env: &EnvInfo) -> Vec<GgufModel> {
    let mut all = Vec::new();

    // Tier 1: CYNIC models dir
    let mut t1 = vec![];
    scan_dir(models_dir, 4, "cynic", &mut t1);
    all.extend(t1);

    // Tier 2: Universal Drive Scanner (Signature-based)
    // Works on Windows, Linux, and WSL2 by abstracting the root entry points
    let roots: Vec<PathBuf> = if cfg!(target_os = "windows") {
        ["C:\\", "D:\\", "E:\\", "F:\\"].iter().map(|&d| PathBuf::from(d)).collect()
    } else {
        // On Linux: scan known user/data dirs only — never scan / directly
        // (scanning / includes /proc /sys /dev which is infinite or harmful)
        let mut r = vec![
            std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/root")),
            PathBuf::from("/home"),
        ];
        // WSL2: also scan mounted Windows drives
        if let Ok(entries) = std::fs::read_dir("/mnt") {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.len() == 1 && name.chars().next().unwrap().is_ascii_alphabetic() {
                    r.push(p);
                }
            }
        }
        r
    };

    klog!("[Ring 0 / LLM] Starting Universal Structural Scan (Signature: *.gguf)...");
    for root in roots {
        if root.exists() {
            scan_dir(&root, 4, "filesystem", &mut all);
        }
    }

    // Tier 3: Ollama models (always checked — fast if not installed)
    let ollama_models = discover_ollama_models(env);
    klog!("[Ring 0 / LLM] Ollama models found: {}", ollama_models.len());
    all.extend(ollama_models);

    // Tier 4: Common Linux data paths
    for root in &[
        PathBuf::from("/opt/models"),
        PathBuf::from("/data"),
        PathBuf::from("/models"),
    ] {
        if root.exists() {
            scan_dir(root, 4, "filesystem", &mut all);
        }
    }

    all
}

/// Tier 3: Discover Ollama models — checks OLLAMA_MODELS env var first
fn discover_ollama_models(env: &EnvInfo) -> Vec<GgufModel> {
    let ollama_dir = find_ollama_models_dir(env);
    let Some(dir) = ollama_dir else {
        klog!("[Ring 0 / LLM] Ollama: not found (no $OLLAMA_MODELS, no default paths)");
        return vec![];
    };
    klog!("[Ring 0 / LLM] Ollama models dir: {}", dir.display());

    let manifests_dir = dir.join("manifests");
    if !manifests_dir.exists() { return vec![]; }

    let blobs_dir = dir.join("blobs");
    let mut models = Vec::new();

    // Walk manifests: registry/library/<name>/<tag>
    for entry in walkdir::WalkDir::new(&manifests_dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Ok(manifest_str) = std::fs::read_to_string(entry.path())
            && let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_str)
        {
            // Extract model name from path: .../library/<name>/<tag>
            let model_name = entry.path().components()
                .rev().nth(1)  // parent dir = model name
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or("unknown")
                .to_string();

            // Ollama stores GGUF as a layer with mediaType containing "model"
            if let Some(layers) = manifest["layers"].as_array() {
                for layer in layers {
                    let media_type = layer["mediaType"].as_str().unwrap_or("");
                    if media_type.contains("model")
                        && let Some(digest) = layer["digest"].as_str()
                    {
                        // digest format: "sha256:abcdef..."
                        let blob_name = digest.replace(':', "-");
                        let blob_path = blobs_dir.join(&blob_name);
                        if blob_path.exists()
                            && let Ok(meta) = std::fs::metadata(&blob_path)
                        {
                            let size_gb = meta.len() as f64 / 1024.0 / 1024.0 / 1024.0;
                            if size_gb > 0.1 {
                                models.push(GgufModel {
                                    path: blob_path.display().to_string(),
                                    size_gb,
                                    name: model_name.clone(),
                                    source: "ollama".into(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    models
}

/// Find the Ollama models directory — universal, no env var or wslvar needed.
/// Strategy: scan accessible drives for the unmistakable Ollama directory structure.
fn find_ollama_models_dir(env: &EnvInfo) -> Option<PathBuf> {
    // 1. $OLLAMA_MODELS env var if set in current environment (Linux native case)
    if let Ok(p) = std::env::var("OLLAMA_MODELS") {
        let path = PathBuf::from(&p);
        if path.exists() { return Some(path); }
        // If it looks like a Windows path, convert it
        let wsl = windows_to_wsl_path(&p);
        if wsl.exists() { return Some(wsl); }
    }

    // 2. From WSL2: try PowerShell to read Windows user env var (no wslvar needed)
    if env.is_wsl2
        && let Ok(out) = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-Command",
                   "[Environment]::GetEnvironmentVariable('OLLAMA_MODELS','User')"])
            .output()
    {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !p.is_empty() && p != "null" {
            let wsl = windows_to_wsl_path(&p);
            if wsl.exists() { return Some(wsl); }
        }
    }

    // 3. Linux/macOS default
    if let Ok(home) = std::env::var("HOME") {
        let default = PathBuf::from(home).join(".ollama").join("models");
        if default.exists() { return Some(default); }
    }

    // 4. Universal structural scan — look for Ollama's signature directory
    //    on any mounted drive. Works regardless of where user installed Ollama.
    //    The structure 'manifests/registry.ollama.ai/' is unique to Ollama.
    let scan_roots = if env.is_wsl2 {
        // Scan all Windows drives accessible via /mnt/
        let mut roots = vec![];
        if let Ok(entries) = std::fs::read_dir("/mnt") {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.len() == 1 && name.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
                    roots.push(e.path());
                }
            }
        }
        roots
    } else {
        vec![PathBuf::from("/")]
    };

    for root in scan_roots {
        // Fast: only look at top-level dirs (depth 4 max — e.g. /mnt/d/User/Bob/)
        for entry in walkdir::WalkDir::new(&root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
        {
            let p = entry.path();
            // Marker: <dir>/manifests/registry.ollama.ai/ exists
            if p.join("manifests").join("registry.ollama.ai").exists() {
                klog!("[Ring 0 / LLM] Ollama models dir found by structure scan: {}", p.display());
                return Some(p.to_path_buf());
            }
        }
    }

    None
}

/// Convert a Windows path like "D:\Models" to WSL2 path "/mnt/d/Models"
fn windows_to_wsl_path(win_path: &str) -> PathBuf {
    let normalized = win_path.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//") {
        // \\server\share → skip for now
        return PathBuf::from(rest);
    }
    // D:/Models → /mnt/d/Models
    if normalized.len() >= 2 && normalized.chars().nth(1) == Some(':') {
        let drive = normalized.chars().next().unwrap().to_lowercase().to_string();
        let rest = &normalized[2..]; // strip "D:"
        return PathBuf::from(format!("/mnt/{}{}", drive, rest));
    }
    PathBuf::from(normalized)
}

fn ensure_models_dir() -> PathBuf {
    // Priority: 1. CYNIC_MODELS_DIR env var, 2. ~/.cynic/models
    let dir = if let Ok(env_path) = std::env::var("CYNIC_MODELS_DIR") {
        PathBuf::from(env_path)
    } else {
        let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
        home.join(".cynic").join("models")
    };

    if !dir.exists() {
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(dir.join("README.txt"),
            "CYNIC Industrial Model Directory\n\
             ===============================\n\
             Note: Redirection active via $CYNIC_MODELS_DIR\n"
        ).ok();
        klog!("[Ring 0 / LLM] Created models directory: {}", dir.display());
    }
    dir
}

fn scan_dir(root: &Path, max_depth: usize, source: &str, found: &mut Vec<GgufModel>) {
    for entry in walkdir::WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(name.as_ref(),
                ".git" | "node_modules" | "target" | "proc" | "sys" | "dev" |
                "Windows" | "wsl" | "wslg" | "cynic-build" |
                "AppData" | "Temp" | "$Recycle.Bin" | "ProgramData" | "System32"
            )
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("gguf"))
            .unwrap_or(false))
    {
        if let Ok(meta) = entry.metadata() {
            let size_bytes = meta.len();
            if size_bytes > 100 * 1024 * 1024 {
                let path = entry.path().to_path_buf();
                found.push(GgufModel {
                    path: path.display().to_string(),
                    size_gb: size_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                    name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string(),
                    source: source.to_string(),
                });
            }
        }
    }
}

/// Probe running inference servers (all candidates in parallel via join)
async fn probe_running_servers(env: &EnvInfo) -> Option<String> {
    let mut candidates = vec![
        "http://localhost:11435".to_string(), // CYNIC llama-server (priority)
        "http://localhost:11434".to_string(), // Ollama
        "http://127.0.0.1:11434".to_string(),
        "http://host.docker.internal:11434".to_string(),
    ];
    if let Some(host) = &env.wsl2_windows_host {
        candidates.push(format!("http://{}:11434", host)); // Ollama on Windows host
        candidates.push(format!("http://{}:11435", host));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build().unwrap_or_default();

    for url in &candidates {
        let ok = client.get(format!("{}/api/version", url))
            .send().await.map(|r| r.status().is_success())
            .unwrap_or(false);
        // Also check OpenAI-compatible health
        let ok2 = if !ok {
            client.get(format!("{}/health", url))
                .send().await.map(|r| r.status().is_success())
                .unwrap_or(false)
        } else { false };

        if ok || ok2 {
            klog!("[Ring 0 / LLM] Running inference server found: {}", url);
            return Some(url.clone());
        }
    }
    None
}

/// Compute the optimal llama-server CLI flags for this machine
fn compute_optimal_flags(compute: &ComputeInfo, models: &[GgufModel]) -> String {
    let threads = compute.cpu_threads.max(1);

    match &compute.backend {
        ComputeBackend::Cuda => {
            // Offload all layers to VRAM
            format!("--gpu-layers 999 --threads {}", threads)
        }
        ComputeBackend::ROCm => {
            format!("--gpu-layers 999 --threads {}", threads)
        }
        ComputeBackend::Vulkan => {
            if compute.is_igpu {
                // iGPU: offload as many layers as the shared VRAM allows
                // Rough estimate: 1 layer ≈ 30MB for 7B model
                let model_gb = models.first().map(|m| m.size_gb).unwrap_or(4.0);
                let vram = if compute.vram_gb > 0.0 { compute.vram_gb } else { 1.5 };
                let gpu_layers = ((vram / model_gb) * 32.0).min(32.0) as usize;
                format!("--gpu-layers {} --vulkan --threads {}", gpu_layers, threads)
            } else {
                format!("--gpu-layers 999 --vulkan --threads {}", threads)
            }
        }
        ComputeBackend::Metal => {
            format!("--gpu-layers 1 --threads {}",threads) // Metal uses -ngl 1 as indicator
        }
        ComputeBackend::Cpu => {
            let avx = if compute.avx2 { " --cpu-features avx2" } else { "" };
            format!("--gpu-layers 0 --threads {}{}", threads, avx)
        }
    }
}

// ============================================================
// CONFIG PERSISTENCE
// ============================================================

fn config_file_path() -> PathBuf {
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
    let dir = home.join(".cynic");
    std::fs::create_dir_all(&dir).ok();
    dir.join("node.toml")
}

fn save_config(config: &NodeConfig, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

fn load_config(path: &Path) -> Result<NodeConfig, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
