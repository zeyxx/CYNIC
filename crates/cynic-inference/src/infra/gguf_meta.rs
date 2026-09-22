//! Minimal GGUF metadata reader (architecture / name).
//!
//! Used for preflight before launching llama-server so we fail closed on
//! unsupported arches (e.g. `dspark` on older llama.cpp) instead of a false
//! systemd "active" then crash loop.

use crate::domain::engine_caps::EngineCapabilities;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GgufMeta {
    pub path: String,
    pub gguf_version: u32,
    pub architecture: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GgufMetaError {
    #[error("not a GGUF file: {0}")]
    NotGguf(String),
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("gguf parse error for {path}: {detail}")]
    Parse { path: String, detail: String },
}

/// Read `general.architecture` / `general.name` from a GGUF header.
pub fn read_gguf_meta(path: &Path) -> Result<GgufMeta, GgufMetaError> {
    let path_s = path.display().to_string();
    let mut f = File::open(path).map_err(|source| GgufMetaError::Io {
        path: path_s.clone(),
        source,
    })?;

    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .map_err(|source| GgufMetaError::Io {
            path: path_s.clone(),
            source,
        })?;
    if &magic != b"GGUF" {
        return Err(GgufMetaError::NotGguf(path_s));
    }

    let ver = read_u32(&mut f, &path_s)?;
    let _n_tensors = read_u64(&mut f, &path_s)?;
    let n_kv = read_u64(&mut f, &path_s)?;

    let mut architecture = None;
    let mut name = None;

    for _ in 0..n_kv.min(512) {
        let key = read_string(&mut f, &path_s)?;
        let vtype = read_u32(&mut f, &path_s)?;
        let val = read_value(&mut f, vtype, &path_s)?;
        match key.as_str() {
            "general.architecture" => {
                if let Some(s) = val.as_str() {
                    architecture = Some(s.to_string());
                }
            }
            "general.name" => {
                if let Some(s) = val.as_str() {
                    name = Some(s.to_string());
                }
            }
            _ => {}
        }
        if architecture.is_some() && name.is_some() {
            break;
        }
    }

    Ok(GgufMeta {
        path: path_s,
        gguf_version: ver,
        architecture,
        name,
    })
}

/// Architectures this workspace's llama-server build generation is known to load.
/// Expand when engine is upgraded; keep conservative.
pub fn default_supported_arches_for_llama_b9444() -> &'static [&'static str] {
    &[
        "llama",
        "qwen2",
        "qwen2vl",
        "qwen3",
        "qwen35", // Bonsai-27B-Q1_0 etc.
        "phi3",
        "phi2",
        "gemma",
        "gemma2",
        "gemma3",
        "mistral",
        "mixtral",
        "command-r",
        "deepseek2",
        "deepseek",
        "gpt2",
        "gptneox",
        "starcoder",
        "refact",
        "bert",
        "nomic-bert",
        "rwkv",
        "rwkv6",
        "mamba",
        "jamba",
        "cohere",
        "dbrx",
        "olmo",
        "arctic",
        "chatglm",
        "glm4",
        "stablelm",
        "internlm2",
        "minicpm",
        "minicpm3",
        "granite",
        "openelm",
    ]
}

/// Return Ok(meta) if arch is supported, Err with reason otherwise.
///
/// When `store` is provided, uses observed capabilities (a posteriori learning).
/// Falls back to static list when no store is available.
pub fn preflight_gguf_with_store(
    path: &Path,
    store: Option<&EngineCapabilities>,
) -> Result<GgufMeta, LocalRuntimeCompatError> {
    let meta = read_gguf_meta(path).map_err(|e| LocalRuntimeCompatError::Meta(e.to_string()))?;
    let Some(arch) = meta.architecture.as_deref() else {
        return Err(LocalRuntimeCompatError::UnknownArch {
            path: meta.path,
            detail: "GGUF missing general.architecture".into(),
        });
    };

    // Path 1: observed capabilities (highest confidence)
    if let Some(store) = store {
        match store.preflight_decision(arch) {
            crate::domain::engine_caps::PreflightDecision::Allow => {
                return Ok(meta);
            }
            crate::domain::engine_caps::PreflightDecision::Reject { reason } => {
                return Err(LocalRuntimeCompatError::UnsupportedArch {
                    path: meta.path,
                    architecture: arch.to_string(),
                    hint: reason,
                });
            }
            crate::domain::engine_caps::PreflightDecision::Warn { .. } => {
                // Fall through to static fallback check
            }
        }
    }

    // Path 2: static fallback (filet de sécurité)
    if crate::domain::engine_caps::static_fallback_arches().contains(&arch) {
        return Ok(meta);
    }

    // Path 3: unknown — reject
    Err({
        let meta_path = meta.path;
        LocalRuntimeCompatError::UnsupportedArch {
            path: meta_path,
            architecture: arch.to_string(),
            hint: format!(
                "arch '{arch}' not in observed capabilities or static fallback. \
                 Launch once to register, or add to fallback list. Supported examples: {}",
                crate::domain::engine_caps::static_fallback_arches()
                    .iter()
                    .take(12)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    })
}

/// Return Ok(meta) if arch is supported, Err with reason otherwise.
///
/// Legacy API — uses static fallback only. Prefer `preflight_gguf_with_store`
/// when an `EngineCapabilityStore` is available.
pub fn preflight_gguf_for_engine(
    path: &Path,
    supported: &[&str],
) -> Result<GgufMeta, LocalRuntimeCompatError> {
    let meta = read_gguf_meta(path).map_err(|e| LocalRuntimeCompatError::Meta(e.to_string()))?;
    let Some(arch) = meta.architecture.as_deref() else {
        return Err(LocalRuntimeCompatError::UnknownArch {
            path: meta.path,
            detail: "GGUF missing general.architecture".into(),
        });
    };
    if supported.contains(&arch) {
        return Ok(meta);
    }
    Err({
        let meta_path = meta.path;
        LocalRuntimeCompatError::UnsupportedArch {
            path: meta_path,
            architecture: arch.to_string(),
            hint: format!(
                "engine build does not list arch '{arch}'. Upgrade llama-server or pick another quant \
                 (e.g. non-dspark). Supported examples: {}",
                supported
                    .iter()
                    .take(12)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    })
}

#[derive(Debug, thiserror::Error)]
pub enum LocalRuntimeCompatError {
    #[error("{0}")]
    Meta(String),
    #[error("unsupported GGUF architecture '{architecture}' for {path}: {hint}")]
    UnsupportedArch {
        path: String,
        architecture: String,
        hint: String,
    },
    #[error("unknown GGUF architecture for {path}: {detail}")]
    UnknownArch { path: String, detail: String },
}

fn read_u32(f: &mut File, path: &str) -> Result<u32, GgufMetaError> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b).map_err(|source| GgufMetaError::Io {
        path: path.to_string(),
        source,
    })?;
    Ok(u32::from_le_bytes(b))
}

fn read_u64(f: &mut File, path: &str) -> Result<u64, GgufMetaError> {
    let mut b = [0u8; 8];
    f.read_exact(&mut b).map_err(|source| GgufMetaError::Io {
        path: path.to_string(),
        source,
    })?;
    Ok(u64::from_le_bytes(b))
}

fn read_string(f: &mut File, path: &str) -> Result<String, GgufMetaError> {
    let n = read_u64(f, path)?;
    if n > 1024 * 1024 {
        return Err(GgufMetaError::Parse {
            path: path.to_string(),
            detail: format!("string length too large: {n}"),
        });
    }
    let mut buf = vec![0u8; n as usize];
    f.read_exact(&mut buf).map_err(|source| GgufMetaError::Io {
        path: path.to_string(),
        source,
    })?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[derive(Debug)]
enum Val {
    Str(String),
    Other,
}

impl Val {
    fn as_str(&self) -> Option<&str> {
        match self {
            Val::Str(s) => Some(s),
            _ => None,
        }
    }
}

fn read_value(f: &mut File, vtype: u32, path: &str) -> Result<Val, GgufMetaError> {
    // GGUF value types: 0=UINT8 1=INT8 2=UINT16 3=INT16 4=UINT32 5=INT32
    // 6=FLOAT32 7=BOOL 8=STRING 9=ARRAY 10=UINT64 11=INT64 12=FLOAT64
    match vtype {
        0 | 1 | 7 => {
            let mut b = [0u8; 1];
            f.read_exact(&mut b).map_err(|source| GgufMetaError::Io {
                path: path.to_string(),
                source,
            })?;
            Ok(Val::Other)
        }
        2 | 3 => {
            let mut b = [0u8; 2];
            f.read_exact(&mut b).map_err(|source| GgufMetaError::Io {
                path: path.to_string(),
                source,
            })?;
            Ok(Val::Other)
        }
        4..=6 => {
            let mut b = [0u8; 4];
            f.read_exact(&mut b).map_err(|source| GgufMetaError::Io {
                path: path.to_string(),
                source,
            })?;
            Ok(Val::Other)
        }
        8 => Ok(Val::Str(read_string(f, path)?)),
        9 => {
            let at = read_u32(f, path)?;
            let n = read_u64(f, path)?;
            for _ in 0..n {
                let _ = read_value(f, at, path)?;
            }
            Ok(Val::Other)
        }
        10..=12 => {
            let mut b = [0u8; 8];
            f.read_exact(&mut b).map_err(|source| GgufMetaError::Io {
                path: path.to_string(),
                source,
            })?;
            Ok(Val::Other)
        }
        other => Err(GgufMetaError::Parse {
            path: path.to_string(),
            detail: format!(
                "unknown value type {other} at pos {:?}",
                f.stream_position()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_list_includes_qwen35_not_dspark() {
        let s = default_supported_arches_for_llama_b9444();
        assert!(s.contains(&"qwen35"));
        assert!(!s.contains(&"dspark"));
    }
}
