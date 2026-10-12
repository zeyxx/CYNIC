//! Builder HTTP client — workload submission and builder URL resolution.
//!
//! The CLI driving adapter must not own HTTP: this module encapsulates
//! the builder's `/workloads/submit` and `/proofs/{id}` endpoints behind
//! plain functions, so the surface only parses argv and renders JSON.
//!
//! Resolution order for the builder URL: `CYNIC_BUILDER_URL` environment
//! variable first, then the `build_runner` node's service-plane address
//! from `infra/registry.json`. Fails closed when neither yields an URL.

use serde_json::json;

/// Resolve the builder HTTP endpoint.
///
/// `CYNIC_BUILDER_URL` wins when set and non-empty; otherwise the
/// `build_runner` node's service-plane address is read from the registry.
/// Returns `Err` when neither source yields an URL — never guesses.
pub fn resolve_builder_url() -> Result<String, String> {
    if let Ok(env_url) = std::env::var("CYNIC_BUILDER_URL") {
        if !env_url.trim().is_empty() {
            return Ok(env_url);
        }
    }
    let registry_path = std::path::Path::new("infra/registry.json");
    if registry_path.exists() {
        let content = std::fs::read_to_string(registry_path)
            .map_err(|e| format!("cannot read infra/registry.json: {e}"))?;
        let reg: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("invalid infra/registry.json format: {e}"))?;
        if let Some(nodes) = reg["nodes"].as_array() {
            for node in nodes {
                let role = node["role"].as_str().unwrap_or_default();
                if role == "build_runner" {
                    let ip = node.get("network").and_then(|network| {
                        cynic_network::plane_vocabulary::plane_address(network, "service")
                    });
                    if let Some(ip) = ip {
                        return Ok(format!("http://{ip}:9090"));
                    }
                }
            }
        }
    }
    Err("could not resolve builder URL dynamically from infra/registry.json or CYNIC_BUILDER_URL env".into())
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
pub fn submit_workload(
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
                    let state = pjson["state"].as_str().unwrap_or("unknown");
                    if state != last_state {
                        last_state = state.to_string();
                    }

                    if state == "completed" || state == "failed" {
                        return Ok(pjson);
                    }
                }
            }
        }
    }

    Err(format!("timed out waiting for proof of workload {wid}"))
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
