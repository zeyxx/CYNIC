//! Hub provider execution: direct `cynic-huggingface` consumption.
//!
//! No `cynic-inference` import here by construction (universe rule §3.1).
//! Secrets travel by NAME only; values never appear in argv or reports.

use std::process::ExitCode;

use cynic_huggingface::{
    HfCliAdapter, HfHubError, HfHubPort, HfJobKind, HfJobSpec, HfRepoType, HfScheduledJobRequest,
    HfUploadRequest,
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

fn hub_adapter() -> HfCliAdapter {
    let bin = std::env::var("CYNIC_HF_BIN").unwrap_or_else(|_| "hf".to_string());
    HfCliAdapter::new(bin)
}

fn refuse_mutation(json: bool, what: &str) -> ExitCode {
    if json {
        let _ = emit_json(&serde_json::json!({
            "schema": "cynic.hub.refusal.v1",
            "surface": "hub_cli",
            "ok": false,
            "message": format!("{what} requires explicit --yes"),
        }));
    } else {
        eprintln!("refused: {what} requires explicit --yes");
    }
    ExitCode::FAILURE
}

fn cli_error(json: bool, what: &str) -> ExitCode {
    if json {
        let _ = emit_json(&serde_json::json!({
            "schema": "cynic.hub.error.v1",
            "surface": "hub_cli",
            "ok": false,
            "message": what,
        }));
    } else {
        eprintln!("hub error: {what}");
    }
    ExitCode::FAILURE
}

fn port_error(json: bool, err: HfHubError) -> ExitCode {
    cli_error(json, &err.to_string())
}

pub(crate) struct HubUploadArgs<'a> {
    pub(crate) repo: &'a str,
    pub(crate) path: &'a str,
    pub(crate) path_in_repo: Option<&'a str>,
    pub(crate) repo_type: &'a str,
    pub(crate) revision: Option<&'a str>,
    pub(crate) private: bool,
    pub(crate) no_private: bool,
    pub(crate) include: &'a [String],
    pub(crate) exclude: &'a [String],
    pub(crate) commit_message: Option<&'a str>,
    pub(crate) create_pr: bool,
    pub(crate) yes: bool,
    pub(crate) json: bool,
}

pub(crate) fn execute_hub_upload(args: &HubUploadArgs<'_>) -> ExitCode {
    if !args.yes {
        return refuse_mutation(args.json, "hub upload");
    }
    if args.private && args.no_private {
        return cli_error(args.json, "--private and --no-private are exclusive");
    }
    let repo_type = match args.repo_type {
        "model" => HfRepoType::Model,
        "dataset" => HfRepoType::Dataset,
        "space" => HfRepoType::Space,
        other => return cli_error(args.json, &format!("--type must be model|dataset|space, got {other}")),
    };
    let req = HfUploadRequest {
        repo_id: args.repo.to_string(),
        local_path: args.path.to_string(),
        path_in_repo: args.path_in_repo.map(|s| s.to_string()),
        repo_type,
        revision: args.revision.map(|s| s.to_string()),
        private: if args.private {
            Some(true)
        } else if args.no_private {
            Some(false)
        } else {
            None
        },
        include: args.include.to_vec(),
        exclude: args.exclude.to_vec(),
        commit_message: args.commit_message.map(|s| s.to_string()),
        create_pr: args.create_pr,
    };
    match hub_adapter().upload(&req) {
        Ok(report) => {
            let code = if report.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
            if args.json {
                let _ = emit_json(&report);
            } else {
                println!("Hub Upload Report: ok={}", report.ok);
                if let Some(msg) = &report.message {
                    println!("  Message: {msg}");
                }
            }
            code
        }
        Err(err) => port_error(args.json, err),
    }
}

pub(crate) struct HubJobSubmitArgs<'a> {
    pub(crate) script: Option<&'a str>,
    pub(crate) image: Option<&'a str>,
    pub(crate) args: &'a [String],
    pub(crate) name: Option<&'a str>,
    pub(crate) flavor: Option<&'a str>,
    pub(crate) env: &'a [String],
    pub(crate) secret: &'a [String],
    pub(crate) volume: &'a [String],
    pub(crate) timeout: Option<&'a str>,
    pub(crate) label: &'a [String],
    pub(crate) namespace: Option<&'a str>,
    /// `Some` schedule = scheduled submission, `None` = one-shot.
    pub(crate) schedule: Option<&'a str>,
    pub(crate) yes: bool,
    pub(crate) json: bool,
}

fn parse_kv(entries: &[String], flag: &str) -> Result<Vec<(String, String)>, String> {
    entries
        .iter()
        .map(|e| {
            e.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| format!("{flag} entry must be K=V, got {e}"))
        })
        .collect()
}

fn build_job_spec(args: &HubJobSubmitArgs<'_>) -> Result<HfJobSpec, String> {
    let kind = match (args.script, args.image) {
        (Some(script), None) => {
            if script.trim().is_empty() {
                return Err("--script must not be empty".into());
            }
            HfJobKind::UvScript {
                script: script.to_string(),
                script_args: args.args.to_vec(),
            }
        }
        (None, Some(image)) => {
            if image.trim().is_empty() {
                return Err("--image must not be empty".into());
            }
            HfJobKind::Docker {
                image: image.to_string(),
                command: args.args.to_vec(),
            }
        }
        (None, None) => return Err("exactly one of --script/--image is required".into()),
        (Some(_), Some(_)) => return Err("--script and --image are exclusive".into()),
    };
    Ok(HfJobSpec {
        name: args.name.map(|s| s.to_string()),
        kind,
        flavor: args.flavor.map(|s| s.to_string()),
        env: parse_kv(args.env, "--env")?,
        secret_names: args.secret.to_vec(),
        volumes: args.volume.to_vec(),
        timeout: args.timeout.map(|s| s.to_string()),
        labels: args
            .label
            .iter()
            .map(|l| {
                l.split_once('=')
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .unwrap_or_else(|| (l.clone(), String::new()))
            })
            .collect(),
        namespace: args.namespace.map(|s| s.to_string()),
        detach: true,
    })
}

pub(crate) fn execute_hub_job_submit(args: &HubJobSubmitArgs<'_>) -> ExitCode {
    if !args.yes {
        return refuse_mutation(args.json, "hub job submit");
    }
    let spec = match build_job_spec(args) {
        Ok(spec) => spec,
        Err(msg) => return cli_error(args.json, &msg),
    };
    let adapter = hub_adapter();
    let result = match args.schedule {
        Some(schedule) => match HfScheduledJobRequest::validated(schedule, spec) {
            Ok(req) => adapter.job_schedule_submit(&req),
            Err(err) => return cli_error(args.json, &err.to_string()),
        },
        None => adapter.job_submit(&spec),
    };
    match result {
        Ok(report) => {
            let code = if report.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
            if args.json {
                let _ = emit_json(&report);
            } else {
                println!("Hub Job Report: ok={}", report.ok);
                if let Some(id) = &report.remote_id {
                    println!("  Remote id: {id}");
                }
                if let Some(msg) = &report.message {
                    println!("  Message: {msg}");
                }
            }
            code
        }
        Err(err) => port_error(args.json, err),
    }
}

pub(crate) fn execute_hub_job_poll(id: &str, namespace: Option<&str>, json: bool) -> ExitCode {
    match hub_adapter().job_poll(id, namespace) {
        Ok(info) => {
            if json {
                let _ = emit_json(&info);
            } else {
                println!("Hub Job: {} [{:?}] stage={}", info.remote_id, info.status, info.stage_raw);
            }
            ExitCode::SUCCESS
        }
        Err(err) => port_error(json, err),
    }
}

pub(crate) fn execute_hub_job_list(all: bool, namespace: Option<&str>, json: bool) -> ExitCode {
    match hub_adapter().job_list(all, namespace) {
        Ok(infos) => {
            if json {
                let _ = emit_json(&serde_json::json!({
                    "schema": "cynic.hub.job_list.v1",
                    "surface": "hub_cli",
                    "count": infos.len(),
                    "jobs": infos,
                }));
            } else {
                println!("Hub Jobs ({}):", infos.len());
                for info in &infos {
                    println!("  - {} [{:?}] stage={}", info.remote_id, info.status, info.stage_raw);
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => port_error(json, err),
    }
}

pub(crate) fn execute_hub_job_logs(id: &str, namespace: Option<&str>, json: bool) -> ExitCode {
    match hub_adapter().job_logs(id, namespace) {
        Ok(report) => {
            let code = if report.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
            if json {
                let _ = emit_json(&report);
            } else {
                print!("{}", report.logs);
            }
            code
        }
        Err(err) => port_error(json, err),
    }
}

pub(crate) fn execute_hub_job_cancel(
    id: &str,
    namespace: Option<&str>,
    yes: bool,
    json: bool,
) -> ExitCode {
    if !yes {
        return refuse_mutation(json, "hub job cancel");
    }
    match hub_adapter().job_cancel(id, namespace) {
        Ok(report) => {
            let code = if report.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
            if json {
                let _ = emit_json(&report);
            } else {
                println!("Hub Job Cancel: ok={}", report.ok);
            }
            code
        }
        Err(err) => port_error(json, err),
    }
}

pub(crate) fn execute_hub_sched_list(json: bool) -> ExitCode {
    match hub_adapter().job_schedule_list() {
        Ok(values) => {
            if json {
                let _ = emit_json(&serde_json::json!({
                    "schema": "cynic.hub.sched_list.v1",
                    "surface": "hub_cli",
                    "count": values.len(),
                    "scheduled": values,
                }));
            } else {
                println!("Hub Scheduled Jobs ({}):", values.len());
                for v in &values {
                    println!("  - {v}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => port_error(json, err),
    }
}

pub(crate) fn execute_hub_sched_delete(id: &str, yes: bool, json: bool) -> ExitCode {
    if !yes {
        return refuse_mutation(json, "hub sched delete");
    }
    match hub_adapter().job_schedule_delete(id) {
        Ok(report) => {
            let code = if report.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
            if json {
                let _ = emit_json(&report);
            } else {
                println!("Hub Sched Delete: ok={}", report.ok);
            }
            code
        }
        Err(err) => port_error(json, err),
    }
}
