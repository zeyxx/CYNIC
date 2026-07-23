//! Inference CLI: **CYNIC native system** vs **ODS environment bootstrap**.
//!
//! [Osmantic ODS](https://github.com/Osmantic/ODS) is an external maintained
//! framework we use to bootstrap a full local AI stack. CYNIC still builds its
//! own organism inference control (vault, units, routing SSOT).
//!
//! | Command | Surface | Mutates |
//! |---|---|---|
//! | `runtime inference` | native — routing SSOT report | no |
//! | `runtime inference-vault` | native — declared vault/profiles | no |
//! | `runtime inference-status` | native — local unit lifecycle | no |
//! | `runtime inference-start` | native — start local profile | yes |
//! | `runtime inference-stop` | native — stop local unit | yes |
//! | `runtime inference-discover` | peer attach (ODS optional) — probe; `--apply` publishes | optional |
//! | `runtime inference-models` | native — centralized model store | optional persist |
//! | `runtime inference-models-consolidate` | native — move stray GGUF into vault | yes (`--yes`) |
//! | `runtime inference-hf-status` | HF CLI — version/auth vs vault layout | no |
//! | `runtime inference-hf-download` | HF CLI — download into vault | yes |
//!
//! Native does not install ODS. Discover does not start CYNIC vault models.

use std::process::ExitCode;

use serde::Serialize;

use cynic_inference::{InferenceConfigError, load_inference_config};

mod hf;
mod local_runtime;

/// One backend entry in the JSON output.
#[derive(Serialize)]
struct BackendView {
    name: String,
    sovereign: bool,
    base_url: String,
    model: String,
    backend_type: String,
    context_size: u32,
    timeout_secs: u64,
    max_tokens: u32,
    /// Whether an api_key is present (value redacted — never emitted).
    has_api_key: bool,
}

/// Top-level JSON output shape.
#[derive(Serialize)]
struct InferenceConfigView {
    schema: &'static str,
    config_path: Option<String>,
    status: &'static str,
    /// Human-readable message (populated on error).
    message: Option<String>,
    backends: Vec<BackendView>,
    /// Number of sovereign backends in the list.
    sovereign_count: usize,
}

pub(crate) fn execute_inference(json: bool) -> ExitCode {
    match load_inference_config() {
        Ok(loaded) => {
            let sovereign_count = loaded.backends.iter().filter(|b| b.sovereign).count();
            let backends: Vec<BackendView> = loaded
                .backends
                .iter()
                .map(|b| BackendView {
                    name: b.name.clone(),
                    sovereign: b.sovereign,
                    base_url: b.base_url.clone(),
                    model: b.model.clone(),
                    backend_type: format!("{:?}", b.backend_type).to_lowercase(),
                    context_size: b.context_size,
                    timeout_secs: b.timeout_secs,
                    max_tokens: b.max_tokens,
                    has_api_key: b.api_key.is_some(),
                })
                .collect();

            let view = InferenceConfigView {
                schema: "cynic.inference_config.v1",
                config_path: loaded.path.map(|p| p.to_string_lossy().to_string()),
                status: "ok",
                message: None,
                backends,
                sovereign_count,
            };

            if json {
                emit_json(&view)
            } else {
                println!(
                    "Sovereign Inference Routing SSOT ({} backends, {} sovereign):",
                    view.backends.len(),
                    view.sovereign_count
                );
                if let Some(path) = &view.config_path {
                    println!("  Config Path: {}", path);
                }
                println!();
                println!(
                    "{:<24} {:<10} {:<10} {:<30} {}",
                    "NAME", "SOVEREIGN", "TYPE", "MODEL", "BASE URL"
                );
                println!("{}", "-".repeat(90));
                for b in &view.backends {
                    println!(
                        "{:<24} {:<10} {:<10} {:<30} {}",
                        b.name,
                        if b.sovereign { "yes" } else { "no" },
                        b.backend_type,
                        b.model,
                        b.base_url
                    );
                }
                ExitCode::SUCCESS
            }
        }

        Err(InferenceConfigError::NotFound { path }) => {
            let view = InferenceConfigView {
                schema: "cynic.inference_config.v1",
                config_path: Some(path.to_string_lossy().to_string()),
                status: "not_found",
                message: Some(format!(
                    "No inference config found. Copy infra/inference.toml.example to {}",
                    path.display()
                )),
                backends: vec![],
                sovereign_count: 0,
            };
            if json {
                emit_json(&view)
            } else {
                eprintln!(
                    "{}",
                    view.message
                        .as_deref()
                        .unwrap_or("Inference config not found.")
                );
                ExitCode::SUCCESS
            }
        }

        Err(error) => {
            let view = InferenceConfigView {
                schema: "cynic.inference_config.v1",
                config_path: None,
                status: "error",
                message: Some(error.to_string()),
                backends: vec![],
                sovereign_count: 0,
            };
            if json {
                let _ = serde_json::to_string_pretty(&view).map(|payload| println!("{payload}"));
            } else {
                eprintln!("inference config error: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

pub(crate) use hf::{
    HfDownloadArgs, execute_inference_hardware, execute_inference_hf_categories,
    execute_inference_hf_download, execute_inference_hf_search, execute_inference_hf_status,
    execute_inference_hf_suggest,
};

pub(crate) use local_runtime::{
    execute_inference_discover, execute_inference_models, execute_inference_models_consolidate,
    execute_inference_plan, execute_inference_start, execute_inference_status,
    execute_inference_stop, execute_inference_vault,
};

fn emit_json(value: &impl serde::Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => {
            println!("{payload}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("JSON serialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}
