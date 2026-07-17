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
//! | `runtime inference-discover` | **ODS integration** — probe ODS/peers + publish | yes (config) |
//! | `runtime inference-models` | native — centralized model store | optional persist |
//! | `runtime inference-hf-status` | HF CLI — version/auth vs vault layout | no |
//! | `runtime inference-hf-download` | HF CLI — download into vault | yes |
//!
//! Native does not install ODS. Discover does not start CYNIC vault models.

use std::process::ExitCode;

use serde::Serialize;

use cynic_inference::{
    HardwareProfile, HfDiscoveryService, InferenceConfigError, LocalRuntimeServiceAny,
    ModelStoreService, RuntimePlanner, category_catalog, load_inference_config,
};

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
    if !json {
        // This surface is machine-only; human-friendly output is out of scope.
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }

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

            emit_json(&view)
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
            // Emit JSON and exit 0 — "not found" is a valid state, not an error
            // for machine consumers (MCP, CI) that must distinguish it from a
            // parse failure.
            emit_json(&view)
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
            eprintln!("inference config error: {error}");
            // Emit JSON for machine consumers even on error, then fail.
            let _ = serde_json::to_string_pretty(&view).map(|payload| println!("{payload}"));
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_vault(json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = LocalRuntimeServiceAny::from_env();
    match service.list_inventory() {
        Ok(inventory) => emit_json(&inventory),
        Err(error) => {
            eprintln!("inference vault error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_status(json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = LocalRuntimeServiceAny::from_env();
    match service.status() {
        Ok(status) => emit_json(&status),
        Err(error) => {
            eprintln!("inference status error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_start(alias: &str, json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = LocalRuntimeServiceAny::from_env();
    match service.start(alias) {
        Ok(report) => {
            let code = if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
            let _ = emit_json(&report);
            code
        }
        Err(error) => {
            eprintln!("inference start error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_stop(alias: Option<&str>, json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = LocalRuntimeServiceAny::from_env();
    match service.stop(alias) {
        Ok(report) => {
            let code = if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
            let _ = emit_json(&report);
            code
        }
        Err(error) => {
            eprintln!("inference stop error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_discover(json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to start tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    let results = rt.block_on(cynic_inference::adapters::ods::run_sync_pipeline(
        "local-node",
    ));
    let status = if results.iter().any(|r| r.error.is_some()) {
        "degraded"
    } else {
        "ok"
    };

    let report = serde_json::json!({
        "schema": "cynic.inference.multi_publish.v1",
        "status": status,
        "results": results,
    });

    emit_json(&report)
}

pub(crate) fn execute_inference_models(json: bool, persist: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = ModelStoreService::from_env();
    let result = if persist {
        service.scan_and_persist()
    } else {
        service.scan()
    };
    match result {
        Ok(report) => emit_json(&report),
        Err(error) => {
            eprintln!("inference models error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_status(json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = ModelStoreService::from_env();
    match service.hf_status() {
        Ok(status) => emit_json(&status),
        Err(error) => {
            eprintln!("hf status error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_download(
    repo: &str,
    json: bool,
    revision: Option<&str>,
    include: &[String],
    exclude: &[String],
    repo_type: Option<&str>,
    cache_only: bool,
    alias: Option<&str>,
    register_profile: bool,
) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = HfDiscoveryService::from_env();
    match service.download_and_register(
        repo,
        revision,
        include,
        exclude,
        repo_type,
        cache_only,
        alias,
        register_profile,
    ) {
        Ok(report) => {
            // Refresh centralized registry after successful download
            if report.ok {
                let _ = ModelStoreService::from_env().scan_and_persist();
            }
            let code = if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
            let _ = emit_json(&report);
            code
        }
        Err(error) => {
            eprintln!("hf download error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_categories(json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let body = serde_json::json!({
        "schema": "cynic.inference.hf_categories.v1",
        "surface": "hf_cli",
        "categories": category_catalog(),
    });
    emit_json(&body)
}

pub(crate) fn execute_inference_hardware(json: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    emit_json(&HardwareProfile::probe())
}

pub(crate) fn execute_inference_hf_search(
    json: bool,
    category: Option<&str>,
    query: Option<&str>,
    author: Option<&str>,
    limit: u32,
) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = HfDiscoveryService::from_env();
    match service.search(category, query, limit, author) {
        Ok(report) => emit_json(&report),
        Err(error) => {
            eprintln!("hf search error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_suggest(json: bool, category: &str, limit: u32) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let service = HfDiscoveryService::from_env();
    match service.suggest(Some(category), limit) {
        Ok(report) => emit_json(&report),
        Err(error) => {
            eprintln!("hf suggest error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_plan(alias: &str, json: bool, apply: bool) -> ExitCode {
    if !json {
        eprintln!("Use --json. This command is designed for machine consumption.");
        return ExitCode::FAILURE;
    }
    let planner = RuntimePlanner::from_env();
    match planner.plan_for_alias(alias) {
        Ok(plan) => {
            if apply {
                match planner.materialize(&plan) {
                    Ok(mat) => {
                        let body = serde_json::json!({
                            "plan": plan,
                            "materialized": mat,
                            "applied": true,
                        });
                        emit_json(&body)
                    }
                    Err(e) => {
                        eprintln!("plan apply error: {e}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                emit_json(&plan)
            }
        }
        Err(error) => {
            eprintln!("inference plan error: {error}");
            ExitCode::FAILURE
        }
    }
}

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
