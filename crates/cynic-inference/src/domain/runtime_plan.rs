//! Probe-driven native launch plan (agnostic — not host-frozen flags).
//!
//! Profile files declare **what** to run (model path, alias).
//! This module decides **how** from live hardware + installed engine plugins.
//!
//! ## CPU backends note (llama.cpp / ggml)
//! ggml ships **feature-level** CPU plugins (haswell/AVX2, skylakex, zen4, …),
//! not one `.so` per marketing microarch. Zen3 (Ryzen 5000G) has AVX2+FMA+BMI2
//! and is correctly served by the AVX2-class plugin selected via CPUID — a
//! missing `libggml-cpu-zen3.so` is **not** a missing Zen3 path.

use serde::{Deserialize, Serialize};

use super::hardware_profile::{GpuClass, HardwareProfile};

const INFERENCE_DEVICE_ENV: &str = "CYNIC_INFERENCE_DEVICE";

/// Observed engine plugin inventory on the host (paths may vary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnginePlugins {
    pub llama_server_bin: String,
    pub plugin_dir: Option<String>,
    pub cpu_backends: Vec<String>,
    pub has_vulkan: bool,
    pub has_cuda: bool,
    pub has_metal: bool,
    /// Vulkan/CUDA device name discovered from `--list-devices` (e.g. "Vulkan0").
    /// Empty/None when no GPU backend is available or listing fails.
    pub detected_gpu_device: Option<String>,
    /// Best-effort: which CPU backend ggml is likely to pick (by feature class).
    pub likely_cpu_backend_class: String,
}

/// Full launch plan derived from probe + optional profile overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeLaunchPlan {
    pub schema: &'static str,
    pub surface: &'static str,
    pub alias: String,
    pub model_path: String,
    pub model_size_bytes: Option<u64>,
    pub hardware: HardwareProfile,
    pub plugins: EnginePlugins,
    pub host: String,
    pub port: u16,
    pub ctx_size: u32,
    /// Final argv fragment after model/host/port (engine-specific).
    pub engine_args: Vec<String>,
    /// Joined form for env `LLAMA_EXTRA_ARGS` compatibility.
    pub llama_extra_args: String,
    pub threads: u32,
    pub threads_batch: u32,
    pub n_gpu_layers: i32,
    pub flash_attn: String,
    pub memory_max: String,
    /// Empty string => do not apply CPUQuota (uncapped).
    pub cpu_quota: String,
    pub rationale: Vec<String>,
    /// Operator overrides that were applied (from profile), if any.
    pub overrides_applied: Vec<String>,
}

impl RuntimeLaunchPlan {
    pub const SCHEMA: &'static str = "cynic.inference.runtime_launch_plan.v1";
    pub const SURFACE: &'static str = "native_probe";
}

/// Optional frozen overrides from a profile (everything else is probed).
#[derive(Debug, Clone, Default)]
pub struct ProfileOverrides {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub ctx_size: Option<u32>,
    /// If set, replaces probed engine args entirely (escape hatch).
    pub force_extra_args: Option<String>,
    pub force_threads: Option<u32>,
    pub force_n_gpu_layers: Option<i32>,
    pub force_memory_max: Option<String>,
    pub force_cpu_quota: Option<String>,
}

/// Build a launch plan from live probe data.
pub fn plan_launch(
    alias: &str,
    model_path: &str,
    model_size_bytes: Option<u64>,
    hw: &HardwareProfile,
    plugins: &EnginePlugins,
    overrides: &ProfileOverrides,
) -> RuntimeLaunchPlan {
    let mut rationale = Vec::new();
    let mut overrides_applied = Vec::new();

    let host = overrides
        .host
        .clone()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    if overrides.host.is_some() {
        overrides_applied.push("host".into());
    }
    let port = overrides.port.unwrap_or(8080);
    if overrides.port.is_some() {
        overrides_applied.push("port".into());
    }

    // Context: scale with available RAM and model size
    let ctx_size = if let Some(c) = overrides.ctx_size {
        overrides_applied.push("ctx_size".into());
        c
    } else {
        let ctx = default_ctx(hw, model_size_bytes);
        rationale.push(format!("ctx_size={ctx} from available_ram + model size"));
        ctx
    };

    // Threads: leave headroom for OS; batch can use more
    let threads = if let Some(t) = overrides.force_threads {
        overrides_applied.push("threads".into());
        t
    } else {
        let t = ((hw.cpu_threads as f32) * 0.5).ceil() as u32;
        t.clamp(2, hw.cpu_threads.max(2))
    };
    let threads_batch = (threads * 2).min(hw.cpu_threads.max(threads));
    rationale.push(format!(
        "threads={threads} threads_batch={threads_batch} (cpu_threads={})",
        hw.cpu_threads
    ));

    // GPU layers: full offload when a GPU backend is present
    let n_gpu_layers = if let Some(n) = overrides.force_n_gpu_layers {
        overrides_applied.push("n_gpu_layers".into());
        n
    } else if plugins.has_vulkan || plugins.has_cuda || plugins.has_metal {
        rationale.push("n_gpu_layers=99 — GPU backend plugin present".into());
        99
    } else {
        rationale.push("n_gpu_layers=0 — no GPU plugin detected".into());
        0
    };

    let flash_attn = if n_gpu_layers > 0 {
        "on".to_string()
    } else {
        "auto".to_string()
    };

    // KV quant always useful on constrained hosts
    let use_kv_q4 = matches!(hw.gpu.class, GpuClass::Integrated)
        || hw.available_ram_bytes < 24 * 1024 * 1024 * 1024;
    if use_kv_q4 {
        rationale.push("KV cache q4_0 — integrated GPU or <24Gi available RAM".into());
    }

    let batch = if matches!(hw.gpu.class, GpuClass::Integrated) {
        512
    } else {
        2048
    };
    let ubatch = if matches!(hw.gpu.class, GpuClass::Integrated) {
        256
    } else {
        512
    };

    // parallel: keep 1 on integrated (stability); 2+ on discrete with RAM
    let parallel = if matches!(hw.gpu.class, GpuClass::Integrated) {
        rationale.push("parallel=1 — integrated/APU stability default".into());
        1
    } else if hw.available_ram_bytes > 20 * 1024 * 1024 * 1024 {
        2
    } else {
        1
    };

    let mut engine_args: Vec<String> = Vec::new();
    if let Some(force) = &overrides.force_extra_args {
        overrides_applied.push("force_extra_args".into());
        // split conservatively on whitespace
        engine_args.extend(force.split_whitespace().map(|s| s.to_string()));
        rationale.push("using profile force_extra_args (operator override)".into());
    } else {
        engine_args.push("--threads".into());
        engine_args.push(threads.to_string());
        engine_args.push("--threads-batch".into());
        engine_args.push(threads_batch.to_string());
        engine_args.push("--parallel".into());
        engine_args.push(parallel.to_string());
        engine_args.push("--n-gpu-layers".into());
        engine_args.push(n_gpu_layers.to_string());
        engine_args.push("--flash-attn".into());
        engine_args.push(flash_attn.clone());
        engine_args.push("--batch-size".into());
        engine_args.push(batch.to_string());
        engine_args.push("--ubatch-size".into());
        engine_args.push(ubatch.to_string());
        if use_kv_q4 {
            engine_args.push("-ctk".into());
            engine_args.push("q4_0".into());
            engine_args.push("-ctv".into());
            engine_args.push("q4_0".into());
        }
        // Auto-detect GPU device name from `llama-server --list-devices` probe.
        // Falls back to operator override (CYNIC_INFERENCE_DEVICE) or skips if no GPU.
        let device = plugins
            .detected_gpu_device
            .as_ref()
            .or_else(|| explicit_device());
        if let Some(device) = device {
            engine_args.push("--device".into());
            engine_args.push(device.clone());
            rationale.push(format!(
                "device={device} (auto-detected from llama-server --list-devices or {INFERENCE_DEVICE_ENV})"
            ));
        }
    }

    let memory_max = if let Some(m) = &overrides.force_memory_max {
        overrides_applied.push("memory_max".into());
        m.clone()
    } else {
        // model + KV headroom + 2G OS/engine
        let model = model_size_bytes.unwrap_or(2 * 1024 * 1024 * 1024);
        let budget = model
            .saturating_add(2 * 1024 * 1024 * 1024)
            .saturating_add((ctx_size as u64) * 2 * 1024 * 1024 / 4);
        let gb = (budget / (1024 * 1024 * 1024)).clamp(2, 64);
        // never claim more than ~90% available
        let cap_gb = (hw.available_ram_bytes * 9 / 10) / (1024 * 1024 * 1024);
        let gb = gb.min(cap_gb.max(2));
        rationale.push(format!("MemoryMax={gb}G from model+ctx vs available RAM"));
        format!("{gb}G")
    };

    let cpu_quota = if let Some(q) = &overrides.force_cpu_quota {
        overrides_applied.push("cpu_quota".into());
        q.clone()
    } else {
        // Allow all logical CPUs for the service — do not freeze 400%
        let pct = hw.cpu_threads.saturating_mul(100);
        rationale.push(format!(
            "CPUQuota={pct}% (= cpu_threads * 100) — not a frozen 400%"
        ));
        format!("{pct}%")
    };

    rationale.push(format!(
        "cpu_backend_class={} (Zen3⇒AVX2/haswell-class plugins; no zen3.so required)",
        plugins.likely_cpu_backend_class
    ));

    let llama_extra_args = engine_args.join(" ");

    RuntimeLaunchPlan {
        schema: RuntimeLaunchPlan::SCHEMA,
        surface: RuntimeLaunchPlan::SURFACE,
        alias: alias.to_string(),
        model_path: model_path.to_string(),
        model_size_bytes,
        hardware: hw.clone(),
        plugins: plugins.clone(),
        host,
        port,
        ctx_size,
        engine_args,
        llama_extra_args,
        threads,
        threads_batch,
        n_gpu_layers,
        flash_attn,
        memory_max,
        cpu_quota,
        rationale,
        overrides_applied,
    }
}

fn default_ctx(hw: &HardwareProfile, model_size: Option<u64>) -> u32 {
    let avail = hw.available_ram_bytes;
    let model = model_size.unwrap_or(0);
    let leftover = avail.saturating_sub(model);
    // rough: more free → more ctx
    if leftover > 16 * 1024 * 1024 * 1024 {
        16384
    } else if leftover > 8 * 1024 * 1024 * 1024 {
        8192
    } else if leftover > 4 * 1024 * 1024 * 1024 {
        4096
    } else {
        2048
    }
}

fn explicit_device() -> Option<String> {
    let raw = match std::env::var(INFERENCE_DEVICE_ENV) {
        Ok(raw) => raw.trim().to_string(),
        Err(_) => return None,
    };
    if raw.is_empty() || raw.eq_ignore_ascii_case("auto") {
        return None;
    }
    Some(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hardware_profile::{GpuClass, GpuInfo};

    fn fake_hw(threads: u32, avail_gb: u64, integrated: bool) -> HardwareProfile {
        HardwareProfile {
            schema: HardwareProfile::SCHEMA,
            total_ram_bytes: avail_gb * 2 * 1024 * 1024 * 1024,
            available_ram_bytes: avail_gb * 1024 * 1024 * 1024,
            cpu_threads: threads,
            gpu: GpuInfo {
                class: if integrated {
                    GpuClass::Integrated
                } else {
                    GpuClass::None
                },
                vendor_id: None,
                vendor_name: None,
                notes: vec![],
            },
            recommended_max_weight_bytes: avail_gb * 1024 * 1024 * 1024 / 2,
            absolute_max_weight_bytes: avail_gb * 1024 * 1024 * 1024 * 3 / 4,
            tier: "test".into(),
            backends: vec!["cpu".into()],
            cpu_flags: vec!["avx2".into(), "fma".into()],
            cpu_governor: None,
            gtt_total_bytes: None,
        }
    }

    #[test]
    fn zen_style_plan_uses_gpu_layers_when_vulkan() {
        let hw = fake_hw(16, 10, true);
        let plugins = EnginePlugins {
            llama_server_bin: "llama-server".into(),
            plugin_dir: None,
            cpu_backends: vec!["haswell".into(), "zen4".into()],
            has_vulkan: true,
            has_cuda: false,
            has_metal: false,
            detected_gpu_device: None,
            likely_cpu_backend_class: "avx2".into(),
        };
        let plan = plan_launch(
            "t",
            "/m.gguf",
            Some(1_700_000_000),
            &hw,
            &plugins,
            &ProfileOverrides::default(),
        );
        assert_eq!(plan.n_gpu_layers, 99);
        assert_eq!(plan.cpu_quota, "1600%");
        assert!(plan.llama_extra_args.contains("--n-gpu-layers"));
    }
}
