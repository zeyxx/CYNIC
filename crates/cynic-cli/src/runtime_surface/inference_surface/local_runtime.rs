use std::fs;
use std::{env, io::Read, process::ExitCode};

use cynic_inference::{
    LocalRuntimeServiceAny, ModelStoreService, RuntimePlanner, consolidate_vault,
    forge_profile_from_benchmark_report,
};

use super::emit_json;

pub(crate) fn execute_inference_vault(json: bool) -> ExitCode {
    let service = LocalRuntimeServiceAny::from_env();
    match service.list_inventory() {
        Ok(inventory) => {
            if json {
                emit_json(&inventory)
            } else {
                println!("Sovereign Vault Models Inventory:");
                println!("  Vault Directory: {}", inventory.vault_dir);
                println!("  Profiles Directory: {}", inventory.profiles_dir);
                println!("  Unit Template: {}", inventory.unit_template);
                println!();
                println!(
                    "{:<24} {:<8} {:<10} {:<10} {}",
                    "ALIAS", "EXISTS", "PORT", "CTX", "MODEL PATH"
                );
                println!("{}", "-".repeat(90));
                for m in &inventory.models {
                    println!(
                        "{:<24} {:<8} {:<10} {:<10} {}",
                        m.alias,
                        if m.model_exists { "yes" } else { "no" },
                        m.port,
                        m.ctx_size,
                        m.model_path
                    );
                }
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("inference vault error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_status(json: bool, tail: usize) -> ExitCode {
    let service = LocalRuntimeServiceAny::from_env();
    match service.status() {
        Ok(status) => {
            if json {
                emit_json(&status)
            } else {
                println!("Local Inference Runtime Status:");
                println!("  Supervisor: {}", status.surface);
                println!("  State:      {:?}", status.state);
                if let Some(alias) = &status.alias {
                    println!("  Model Alias: {}", alias);
                }
                if let Some(pid) = status.main_pid {
                    println!("  Main PID:    {}", pid);
                }
                if let Some(endpoint) = &status.endpoint {
                    println!("  Endpoint:    {}", endpoint);
                }
                if let Some(msg) = &status.message {
                    println!("  Message:     {}", msg);
                    if tail > 0 {
                        let log_path = extract_log_path(msg);
                        print_recent_log_tail(&log_path, tail);
                    }
                }
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("inference status error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn extract_log_path(msg: &str) -> String {
    msg.split(';')
        .find_map(|part| {
            let part = part.trim();
            part.strip_prefix("log=").map(|value| value.to_string())
        })
        .or_else(|| {
            msg.split_once("log=")
                .map(|(_, v)| v.split_whitespace().next().unwrap_or("").trim().to_string())
        })
        .unwrap_or_default()
}

fn print_recent_log_tail(log_path: &str, tail: usize) {
    let mut file = match fs::File::open(log_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("  Logs: unable to open {log_path} ({error})");
            return;
        }
    };
    let mut raw = String::new();
    let _ = file.read_to_string(&mut raw);
    if raw.is_empty() {
        println!("  Logs: no content in {log_path}");
        return;
    }

    let keep = tail.max(1);
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(keep);
    let preview = lines[start..]
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    println!("  Logs (last {keep} lines):");
    for line in preview.lines() {
        println!("    {line}");
    }
}

pub(crate) fn execute_inference_start(
    alias: Option<&str>,
    json: bool,
    yes: bool,
    no_wait: bool,
) -> ExitCode {
    if !yes {
        eprintln!(
            "refusing to start a local model without --yes; inspect first with `cynic models list --json`"
        );
        return ExitCode::FAILURE;
    }
    cynic_inference::progress::progress(
        "resolve",
        "selecting startable vault profile (auto ensures profiles if needed)",
    );
    let resolved = match cynic_inference::resolve_start_alias(alias) {
        Ok(a) => a,
        Err(error) => {
            eprintln!("inference start resolve: {error}");
            return ExitCode::FAILURE;
        }
    };
    cynic_inference::progress::progress("resolve", format!("alias={resolved}"));
    let service = LocalRuntimeServiceAny::from_env();
    cynic_inference::progress::progress(
        "supervisor",
        format!("{:?}", service.supervisor_kind().as_str()),
    );

    let auto_mode = alias.is_none()
        || alias.is_some_and(|a| {
            matches!(
                a.to_ascii_lowercase().as_str(),
                "auto" | "default" | "any" | "" | "_" | "-"
            )
        });

    let mut candidates = vec![resolved];
    if auto_mode {
        if let Ok(ensured) = cynic_inference::ensure_profiles_from_vault() {
            for a in ensured.startable_aliases {
                if !candidates.contains(&a) {
                    candidates.push(a);
                }
            }
        }
    }

    let wait_for_ready = if no_wait {
        false
    } else {
        env::var("CYNIC_INFERENCE_WAIT_FOR_READY")
            .map(|raw| {
                let lower = raw.to_ascii_lowercase();
                !matches!(lower.as_str(), "0" | "false" | "off" | "no")
            })
            .unwrap_or(true)
    };

    let mut last_report = None;
    for (i, try_alias) in candidates.iter().enumerate() {
        if i > 0 {
            cynic_inference::progress::progress(
                "fallback",
                format!("previous alias failed; trying {try_alias}"),
            );
        }
        match service.start_with_readiness(try_alias, wait_for_ready) {
            Ok(mut report) => {
                if let Some(msg) = report.message.take() {
                    report.message = Some(format!("resolved_alias={try_alias}; {msg}"));
                } else {
                    report.message = Some(format!("resolved_alias={try_alias}"));
                }
                if report.ok {
                    if json {
                        let _ = emit_json(&report);
                    } else {
                        println!("Model started successfully!");
                        println!("  Alias:    {}", report.alias);
                        println!("  Unit:     {}", report.unit);
                        println!(
                            "  Endpoint: {}",
                            report.status.endpoint.as_deref().unwrap_or("?")
                        );
                        if let Some(msg) = &report.message {
                            println!("  Message:  {}", msg);
                        }
                    }
                    return ExitCode::SUCCESS;
                }
                last_report = Some(report);
            }
            Err(error) => {
                cynic_inference::progress::progress("error", format!("{try_alias}: {error}"));
                if !auto_mode {
                    eprintln!("inference start error: {error}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    if let Some(report) = last_report {
        if json {
            let _ = emit_json(&report);
        } else {
            eprintln!("Model start failed!");
            eprintln!("  Alias:   {}", report.alias);
            if let Some(msg) = &report.message {
                eprintln!("  Message: {}", msg);
            }
        }
        ExitCode::FAILURE
    } else {
        eprintln!("inference start error: no startable alias succeeded");
        ExitCode::FAILURE
    }
}

pub(crate) fn execute_inference_stop(alias: Option<&str>, json: bool, yes: bool) -> ExitCode {
    if !yes {
        eprintln!(
            "refusing to stop a local model without --yes; inspect first with `cynic models status --json`"
        );
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
            if json {
                let _ = emit_json(&report);
            } else if report.ok {
                println!("Model stopped successfully: {}", report.alias);
            } else {
                eprintln!("Failed to stop model: {}", report.alias);
                if let Some(msg) = &report.message {
                    eprintln!("  Message: {}", msg);
                }
            }
            code
        }
        Err(error) => {
            eprintln!("inference stop error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_discover(json: bool, apply: bool, open_webui: bool) -> ExitCode {
    if open_webui && !apply {
        eprintln!("--open-webui requires --apply (opt-in mutation).");
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

    if apply {
        cynic_inference::progress::progress(
            "attach",
            if open_webui {
                "probe peers + publish SSOT + open-webui"
            } else {
                "probe peers + publish SSOT (inference.toml)"
            },
        );
    } else {
        cynic_inference::progress::progress("attach", "probe peers only (no mutation)");
    }

    let report = rt.block_on(cynic_inference::peer_attach(
        cynic_inference::PeerAttachOptions {
            apply,
            open_webui,
            node_id: "local-node".into(),
        },
    ));

    let code = match report.status.as_str() {
        "ok" | "empty" => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    };
    if json {
        let _ = emit_json(&report);
    } else {
        println!("Peer discovery status: {}", report.status);
        println!("  Services discovered: {}", report.services.len());
        for svc in &report.services {
            println!("    - Service: {}", svc.name);
            println!("      Endpoint: {}", svc.endpoint);
            println!("      Models:    {:?}", svc.models);
        }
        if !report.notes.is_empty() {
            println!("  Notes: {}", report.notes.join("; "));
        }
    }
    code
}

pub(crate) fn execute_inference_models(
    json: bool,
    persist: bool,
    ensure_profiles: bool,
) -> ExitCode {
    if ensure_profiles {
        match cynic_inference::ensure_profiles_from_vault() {
            Ok(ensured) => {
                let service = ModelStoreService::from_env();
                match service.scan() {
                    Ok(_report) => {
                        if json {
                            let _ = emit_json(&ensured);
                        } else {
                            println!("Ensure profiles: {}", ensured.message);
                        }
                        if persist {
                            let _ = service.scan_and_persist();
                        }
                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        if json {
                            let _ = emit_json(&ensured);
                        } else {
                            println!("Ensure profiles: {}", ensured.message);
                        }
                        eprintln!("scan after ensure: {e}");
                        return ExitCode::SUCCESS;
                    }
                }
            }
            Err(error) => {
                eprintln!("ensure-profiles error: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    let service = ModelStoreService::from_env();
    let result = if persist {
        service.scan_and_persist()
    } else {
        service.scan()
    };
    match result {
        Ok(report) => {
            if json {
                emit_json(&report)
            } else {
                println!("Model Store Scan Report:");
                println!("  Vault models scanned: {}", report.artifacts.len());
                println!("  Profiles scanned:     {}", report.profiles.len());
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("inference models error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_models_consolidate(json: bool, yes: bool) -> ExitCode {
    match consolidate_vault(yes) {
        Ok(report) => {
            let code = match report.status {
                "ok" | "planned" => ExitCode::SUCCESS,
                "degraded" => ExitCode::FAILURE,
                _ => ExitCode::FAILURE,
            };
            if json {
                let _ = emit_json(&report);
            } else {
                println!("Inference Models Consolidate (status={}):", report.status);
                println!("  Moved count: {}", report.moved);
                println!("  Rewritten count: {}", report.profiles_rewritten);
            }
            code
        }
        Err(error) => {
            eprintln!("inference-models-consolidate error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_plan(alias: &str, json: bool, apply: bool) -> ExitCode {
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
                        if json {
                            emit_json(&body)
                        } else {
                            println!("Plan applied and materialized successfully!");
                            println!("  Env: {}", mat.runtime_env);
                            ExitCode::SUCCESS
                        }
                    }
                    Err(e) => {
                        eprintln!("plan apply error: {e}");
                        ExitCode::FAILURE
                    }
                }
            } else if json {
                emit_json(&plan)
            } else {
                println!("Inference Launch Plan for alias={}:", alias);
                println!("  Model path:  {}", plan.model_path);
                println!("  Port:        {}", plan.port);
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("inference plan error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `cynic runtime inference-forge --benchmark <report>`: derive a vault profile
/// from a winning benchmark run and (when `--yes`) hand off to the inference
/// supervisor.
pub(crate) fn execute_inference_forge(
    report_path: &str,
    source_note: &str,
    json: bool,
    yes: bool,
) -> ExitCode {
    match forge_profile_from_benchmark_report(report_path, source_note) {
        Ok(report) => {
            let body = serde_json::json!({
                "report": report,
                "applied": yes,
            });
            if json {
                emit_json(&body);
            } else {
                println!("Inference Forge (plan):");
                println!("  Source:   {}  status={}", report.source, report.status);
                println!("  Plan ID:  {}", report.plan_id);
                println!("  Run ID:   {}", report.run_id);
                println!("  Alias:    {}", report.alias);
                println!("  Model ID: {}", report.model_id);
                println!("  Model Path: {}", report.model_path);
                println!("  Profile:  {}", report.profile_path);
                if yes {
                    println!("--yes set: handing off to inference-start.");
                    return execute_inference_start(
                        Some(report.alias.as_str()),
                        json,
                        /* yes */ true,
                        /* no_wait */ false,
                    );
                }
                println!("  (pass --yes to apply and launch.)");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("inference forge error: {error}");
            ExitCode::FAILURE
        }
    }
}
