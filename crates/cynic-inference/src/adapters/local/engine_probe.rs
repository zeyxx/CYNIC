//! Probe installed llama.cpp / ggml plugins without hardcoding a host.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::hardware_profile::HardwareProfile;
use crate::domain::runtime_plan::EnginePlugins;

/// Default plugin search roots (relative / FHS) — not machine-local home paths.
const PLUGIN_CANDIDATES: &[&str] = &["/usr/local/lib/llama", "/usr/lib/llama", "/usr/local/lib"];

const BIN_CANDIDATES: &[&str] = &[
    "llama-server",
    "/usr/local/bin/llama-server",
    "/usr/bin/llama-server",
];

pub fn probe_engine_plugins(hw: &HardwareProfile) -> EnginePlugins {
    let llama_server_bin = resolve_llama_bin();
    let plugin_dir = find_plugin_dir(&llama_server_bin);
    let cpu_backends = plugin_dir
        .as_ref()
        .map(|d| list_cpu_backends(d))
        .unwrap_or_default();
    let has_vulkan = plugin_dir
        .as_ref()
        .is_some_and(|d| has_library(d, "libggml-vulkan.so", "ggml-vulkan.dll"))
        || Path::new("/usr/local/lib/llama/libggml-vulkan.so").exists();
    let has_cuda = plugin_dir.as_ref().is_some_and(|d| {
        has_library(d, "libggml-cuda.so", "ggml-cuda.dll")
            || has_library(d, "libggml-hip.so", "ggml-hip.dll")
    });
    let has_metal = plugin_dir
        .as_ref()
        .is_some_and(|d| has_library(d, "libggml-metal.so", "ggml-metal.dll"));

    let likely = match hw.cpu_feature_class() {
        "avx512" => {
            if cpu_backends.iter().any(|b| {
                b.contains("zen4") || b.contains("sapphirerapids") || b.contains("icelake")
            }) {
                "avx512-class-plugin"
            } else {
                "avx512-features-no-plugin"
            }
        }
        "avx2" => {
            // Zen3 lands here: no dedicated zen3.so; haswell/AVX2 plugin is correct.
            if cpu_backends
                .iter()
                .any(|b| b.contains("haswell") || b.contains("skylakex") || b.contains("zen4"))
            {
                "avx2-class (haswell/skylakex/zen4 plugin via CPUID — zen3 has no separate so)"
            } else {
                "avx2-features"
            }
        }
        other => other,
    }
    .to_string();

    EnginePlugins {
        llama_server_bin,
        plugin_dir: plugin_dir.map(|p| p.display().to_string()),
        cpu_backends,
        has_vulkan,
        has_cuda,
        has_metal,
        likely_cpu_backend_class: likely,
    }
}

fn resolve_llama_bin() -> String {
    if let Ok(p) = std::env::var("LLAMA_SERVER_BIN") {
        return p;
    }
    if let Ok(p) = std::env::var("CYNIC_LLAMA_SERVER_BIN") {
        return p;
    }
    for c in BIN_CANDIDATES {
        if c.starts_with('/') {
            if Path::new(c).is_file() {
                return c.to_string();
            }
        } else if let Ok(out) = Command::new(path_lookup_command()).arg(c).output()
            && out.status.success()
        {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return p;
            }
        }
    }
    "llama-server".into()
}

#[cfg(windows)]
fn path_lookup_command() -> &'static str {
    "where.exe"
}

#[cfg(not(windows))]
fn path_lookup_command() -> &'static str {
    "which"
}

fn find_plugin_dir(bin: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CYNIC_LLAMA_PLUGIN_DIR") {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Some(pb);
        }
    }
    // sibling of binary: /usr/local/bin/llama-server → try /usr/local/lib/llama
    if let Some(parent) = Path::new(bin).parent() {
        if has_library(parent, "libggml-base.so", "ggml-base.dll") {
            return Some(parent.to_path_buf());
        }
        let candidate = parent.join("../lib/llama");
        if let Ok(c) = candidate.canonicalize()
            && c.is_dir()
        {
            return Some(c);
        }
    }
    for c in PLUGIN_CANDIDATES {
        let p = PathBuf::from(c);
        if p.is_dir()
            && (p.join("libggml-base.so").exists()
                || p.join("libggml-vulkan.so").exists()
                || p.join("libllama.so").exists())
        {
            return Some(p);
        }
    }
    None
}

fn list_cpu_backends(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(rest) = name
            .strip_prefix("libggml-cpu-")
            .and_then(|s| s.strip_suffix(".so"))
            .or_else(|| {
                name.strip_prefix("ggml-cpu-")
                    .and_then(|s| s.strip_suffix(".dll"))
            })
        {
            out.push(rest.to_string());
        }
    }
    out.sort();
    out
}

fn has_library(dir: &Path, unix_name: &str, windows_name: &str) -> bool {
    dir.join(unix_name).is_file() || dir.join(windows_name).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_windows_plugins_beside_injected_engine() {
        let temp = tempfile::tempdir().expect("tempdir");
        let engine = temp.path().join("llama-server.exe");
        std::fs::write(&engine, []).expect("engine fixture");
        std::fs::write(temp.path().join("ggml-base.dll"), []).expect("base plugin fixture");
        std::fs::write(temp.path().join("ggml-cpu-avx2.dll"), []).expect("cpu plugin fixture");

        let plugin_dir = find_plugin_dir(engine.to_string_lossy().as_ref()).expect("plugin dir");
        assert_eq!(plugin_dir, temp.path());
        assert_eq!(list_cpu_backends(&plugin_dir), vec!["avx2"]);
    }
}
