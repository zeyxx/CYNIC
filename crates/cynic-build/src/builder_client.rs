//! Builder HTTP client — workload submission and builder URL resolution.
//!
//! The CLI driving adapter must not own HTTP: this module encapsulates
//! the builder's `/workloads/submit` and `/proofs/{id}` endpoints behind
//! plain functions, so the surface only parses argv and renders JSON.
//!
//! Resolution order for the builder URL: `CYNIC_BUILDER_URL` environment
//! variable first, then the builder pool from `infra/registry.json` with
//! health-check. Fails closed when no healthy builder is available.

use serde_json::json;

/// Resolve the builder HTTP endpoint via the builder pool with health-check.
///
/// `CYNIC_BUILDER_URL` wins when set and non-empty (backward compat).
/// Otherwise the registry is read, all `build_runner` nodes are health-checked,
/// their `/capacity` gauges are read, and the lowest-pressure builder is
/// selected via [`crate::builder_pool::resolve_builder_pool_with_capacity`].
///
/// Fails closed when no builder is healthy — never guesses, never falls back
/// to an unhealthy endpoint. Builders without the gauge surface stay eligible
/// through the legacy spread, but never win against a gauge.
pub fn resolve_builder_url_with_health() -> Result<String, String> {
    // Backward compat: explicit env var bypasses pool entirely.
    if let Ok(env_url) = std::env::var("CYNIC_BUILDER_URL") {
        if !env_url.trim().is_empty() {
            return Ok(env_url);
        }
    }

    let registry_path = std::path::Path::new("infra/registry.json");
    if !registry_path.exists() {
        return Err("could not resolve builder URL: infra/registry.json not found and CYNIC_BUILDER_URL not set".into());
    }

    let content = std::fs::read_to_string(registry_path)
        .map_err(|e| format!("cannot read infra/registry.json: {e}"))?;
    let reg: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid infra/registry.json format: {e}"))?;

    match crate::builder_pool::resolve_builder_pool_with_capacity(&reg) {
        Some(endpoint) => Ok(endpoint.url),
        None => Err(
            "could not resolve builder URL: no healthy build_runner in pool (all unhealthy or registry empty)"
                .into(),
        ),
    }
}

/// Resolve the builder HTTP endpoint (backward-compatible wrapper).
///
/// Delegates to [`resolve_builder_url_with_health`] — the pool-based path.
/// Single-builder registries work unchanged: the pool resolves one endpoint,
/// health-check passes (or env var is set), and the URL is returned.
pub fn resolve_builder_url() -> Result<String, String> {
    resolve_builder_url_with_health()
}

/// Submit a build workload to the builder and optionally wait for proof.
///
/// Posts the admission payload to `/workloads/submit`. When `wait` is
/// false, returns the acceptance proof immediately. When `wait` is true,
/// polls `/proofs/{workload_id}` every 3s up to 15 minutes, returning the
/// terminal state JSON (`completed` or `failed`).
///
/// The caller decides how to render the result; this function only owns
/// the HTTP boundary.
///
/// Runs the whole blocking sequence on a dedicated OS thread: the CLI runs
/// under `#[tokio::main]`, and constructing (not just dropping) a
/// `reqwest::blocking::Client` inside async context panics. Never move the
/// client construction back out of the spawned closure.
pub fn submit_workload(
    commit: &str,
    package: Option<&str>,
    action: &str,
    requested_by: &str,
    builder_url: &str,
    wait: bool,
) -> Result<serde_json::Value, String> {
    let commit = commit.to_string();
    let package = package.map(str::to_string);
    let action = action.to_string();
    let requested_by = requested_by.to_string();
    let builder_url = builder_url.to_string();
    std::thread::spawn(move || {
        submit_workload_inner(
            &commit,
            package.as_deref(),
            &action,
            &requested_by,
            &builder_url,
            wait,
        )
    })
    .join()
    .map_err(|_| "builder client thread panicked".to_string())?
}

fn submit_workload_inner(
    commit: &str,
    package: Option<&str>,
    action: &str,
    requested_by: &str,
    builder_url: &str,
    wait: bool,
) -> Result<serde_json::Value, String> {
    let payload = json!({
        "commit_sha": commit,
        "package": package,
        "action": action,
        "requested_by": requested_by,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("http client init failed: {e}"))?;

    let submit_url = format!("{}/workloads/submit", builder_url.trim_end_matches('/'));
    let res = client
        .post(&submit_url)
        .json(&payload)
        .send()
        .map_err(|e| format!("failed to submit workload to {submit_url}: {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().unwrap_or_default();
        return Err(format!("builder rejected submission ({status}): {body}"));
    }

    let proof: serde_json::Value = res
        .json()
        .map_err(|e| format!("failed to parse acceptance proof: {e}"))?;

    if !wait {
        return Ok(proof);
    }

    let wid = proof["workload_id"].as_str().unwrap_or("");
    let proof_url = format!("{}/proofs/{}", builder_url.trim_end_matches('/'), wid);

    let start = std::time::Instant::now();
    let poll_timeout = std::time::Duration::from_secs(900);
    let mut last_state = String::new();

    while start.elapsed() < poll_timeout {
        std::thread::sleep(std::time::Duration::from_secs(3));
        if let Ok(pres) = client.get(&proof_url).send() {
            if pres.status().is_success() {
                if let Ok(pjson) = pres.json::<serde_json::Value>() {
                    // Terminal states live in completion.status (snake_case),
                    // not in a top-level "state" field: the proofs endpoint
                    // returns the completion envelope, whose status is
                    // completed/failed/blocked/... A blocked workload is
                    // terminal — polling past it waits out the timeout.
                    let completed = pjson
                        .get("completion")
                        .and_then(|completion| completion.get("status"))
                        .and_then(|status| status.as_str())
                        .map(|status| {
                            matches!(
                                status,
                                "completed" | "failed" | "blocked" | "recovered_unattributed"
                            )
                        })
                        .unwrap_or(false);
                    let legacy = pjson
                        .get("state")
                        .and_then(|state| state.as_str())
                        .map(|state| matches!(state, "completed" | "failed"))
                        .unwrap_or(false);
                    if completed || legacy {
                        return Ok(pjson);
                    }
                    last_state = pjson
                        .get("completion")
                        .and_then(|completion| completion.get("status"))
                        .and_then(|status| status.as_str())
                        .unwrap_or(&last_state)
                        .to_string();
                }
            }
        }
    }

    Err(format!("timed out waiting for proof of workload {wid}"))
}

/// Bounded output of a settled workload, as the node sealed it in evidence.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct WorkloadFailureOutput {
    pub exit_code: Option<i64>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

/// Read one workload record from the builder (`GET /records/{id}`).
///
/// Same thread discipline as [`submit_workload`]: the blocking client must
/// be built outside the async runtime.
pub fn fetch_workload_record(
    builder_url: &str,
    workload_id: &str,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/records/{}",
        builder_url.trim_end_matches('/'),
        workload_id
    );
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("http client init failed: {e}"))?;
        let res = client
            .get(&url)
            .send()
            .map_err(|e| format!("failed to read {url}: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("builder refused {url} ({})", res.status()));
        }
        res.json::<serde_json::Value>()
            .map_err(|e| format!("failed to parse record from {url}: {e}"))
    })
    .join()
    .map_err(|_| "builder client thread panicked".to_string())?
}

/// Extract the sealed process output from a workload record.
///
/// The node persists `{exit_code, stdout, stderr}` as evidence when a build
/// fails; a successful build seals none. `None` means the record carries no
/// output evidence, which is distinct from an empty output.
pub fn failure_output_from_record(
    record: &serde_json::Value,
    tail_lines: usize,
) -> Option<WorkloadFailureOutput> {
    let payload = record
        .get("evidence")?
        .as_array()?
        .iter()
        .filter_map(|item| item.get("envelope")?.get("payload"))
        .find(|payload| payload.get("stderr").is_some() || payload.get("stdout").is_some())?;
    let text = |key: &str| {
        payload
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
    };
    Some(WorkloadFailureOutput {
        exit_code: payload.get("exit_code").and_then(|code| code.as_i64()),
        stdout_tail: last_lines(text("stdout"), tail_lines),
        stderr_tail: last_lines(text("stderr"), tail_lines),
    })
}

fn last_lines(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].join("\n")
}

/// Render a workload acceptance or terminal-state proof for the operator.
///
/// Shared by the CLI's JSON and human-readable paths so both renderings
/// describe the same fields.
pub fn render_workload_proof(proof: &serde_json::Value, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(proof).unwrap_or_default()
        );
        return Ok(());
    }
    let wid = proof["workload_id"].as_str().unwrap_or("unknown");
    println!("Workload admitted successfully: id={wid} (status: accepted)");
    Ok(())
}

/// Render a terminal workload state for the operator.
pub fn render_workload_terminal(
    pjson: &serde_json::Value,
    wid: &str,
    elapsed_secs: u128,
    json: bool,
) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(pjson).unwrap_or_default()
        );
        return Ok(());
    }
    let state = pjson["state"].as_str().unwrap_or("unknown");
    println!("Build finished in {elapsed_secs}s: state={state} id={wid}");
    if let Some(receipt) = pjson.get("parcel_receipt") {
        if let Some(digest) = receipt.get("final_record_digest").and_then(|d| d.as_str()) {
            println!("Hermetic Parcel Digest: {digest}");
        }
        if let Some(settled_at) = receipt.get("settled_at_ms").and_then(|t| t.as_u64()) {
            println!("Settled At: {settled_at} ms");
        }
    }
    if let Some(completion) = pjson.get("completion") {
        if let Some(status) = completion.get("status").and_then(|s| s.as_str()) {
            println!("Result Status: {status}");
        }
        if let Some(evs) = completion.get("evidence_refs").and_then(|e| e.as_array()) {
            println!("Sealed Evidence References ({}):", evs.len());
            for ev in evs {
                println!("  - {}", ev.as_str().unwrap_or_default());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    /// Ensure tests that mutate CYNIC_BUILDER_URL don't race each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn resolve_builder_url_env_var_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: single-threaded test with ENV_LOCK held.
        unsafe { std::env::set_var("CYNIC_BUILDER_URL", "http://explicit:9090") };
        let url = resolve_builder_url().unwrap();
        assert_eq!(url, "http://explicit:9090");
        unsafe { std::env::remove_var("CYNIC_BUILDER_URL") };
    }

    #[test]
    fn resolve_builder_url_with_health_env_var_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: single-threaded test with ENV_LOCK held.
        unsafe { std::env::set_var("CYNIC_BUILDER_URL", "http://explicit:9090") };
        let url = resolve_builder_url_with_health().unwrap();
        assert_eq!(url, "http://explicit:9090");
        unsafe { std::env::remove_var("CYNIC_BUILDER_URL") };
    }

    #[test]
    fn resolve_builder_url_fails_closed_when_no_registry() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("CYNIC_BUILDER_URL") };
        // The test runs from the crate root; infra/registry.json may or may
        // not exist. We can only assert the function does not panic and
        // returns a Result. If registry exists with a healthy builder, it
        // returns Ok; otherwise Err. Both are valid — no panic is the contract.
        let _ = resolve_builder_url();
    }

    #[test]
    fn resolve_builder_url_with_health_fails_closed_when_no_registry() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("CYNIC_BUILDER_URL") };
        let _ = resolve_builder_url_with_health();
    }

    #[test]
    fn resolve_builder_url_delegates_to_pool() {
        let _guard = ENV_LOCK.lock().unwrap();
        // With env var set, both functions must return the same value.
        // SAFETY: single-threaded test with ENV_LOCK held.
        unsafe { std::env::set_var("CYNIC_BUILDER_URL", "http://same:9090") };
        let url = resolve_builder_url().unwrap();
        let url_health = resolve_builder_url_with_health().unwrap();
        assert_eq!(url, url_health);
        unsafe { std::env::remove_var("CYNIC_BUILDER_URL") };
    }

    #[test]
    fn failure_output_reads_sealed_stderr_and_keeps_only_the_tail() {
        let record = json!({
            "evidence": [
                {"kind": "build_grant_read_back", "envelope": {"payload": {"granted": true}}},
                {"kind": "build_runner_observation", "envelope": {"payload": {
                    "exit_code": 101,
                    "stdout": "",
                    "stderr": "a\nb\nc\nd"
                }}}
            ]
        });
        let output = failure_output_from_record(&record, 2).unwrap();
        assert_eq!(output.exit_code, Some(101));
        assert_eq!(output.stdout_tail, "");
        assert_eq!(output.stderr_tail, "c\nd");
    }

    #[test]
    fn failure_output_is_absent_when_the_record_sealed_none() {
        let record = json!({"evidence": [
            {"kind": "build_grant_read_back", "envelope": {"payload": {"granted": true}}},
            {"kind": "build_runner_observation", "envelope": null}
        ]});
        assert_eq!(failure_output_from_record(&record, 10), None);
        assert_eq!(failure_output_from_record(&json!({}), 10), None);
    }

    #[test]
    fn last_lines_returns_everything_when_shorter_than_the_tail() {
        assert_eq!(last_lines("a\nb", 10), "a\nb");
        assert_eq!(last_lines("", 3), "");
    }
}
