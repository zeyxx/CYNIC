use std::path::{Path, PathBuf};
use std::time::Duration;

use super::types::*;
use super::environment::{ensure_models_dir, windows_to_wsl_path};

// ============================================================
// LLM RESOURCE PROBE (Exhaustive — all tiers always run)
// ============================================================

pub(super) async fn probe_llm_resources(env: &EnvInfo, compute: &ComputeInfo) -> LlmConfig {
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
