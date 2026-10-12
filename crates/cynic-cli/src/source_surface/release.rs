//! Release gate evaluation and publication to remote relay.

use std::path::Path;
use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;
use crate::source_surface::SourceExit;

fn observe_build_proof(sha: &str) -> cynic_release::build_proof::BuildProofObservation {
    use cynic_release::build_proof::BuildProofObservation;

    let log_path = match crate::build_proof_log::default_proof_log_path() {
        Ok(path) => path,
        Err(_) => return BuildProofObservation::NeverTested,
    };
    let (proofs, skipped) = match crate::build_proof_log::read_proofs(&log_path, Some(sha), 0) {
        Ok(reading) => reading,
        Err(_) => return BuildProofObservation::NeverTested,
    };
    cynic_release::reduce_newest_proof(&proofs, skipped.len())
}

pub(crate) fn execute_release(root: &Path, _dry_run: bool, apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let mut proofs = cynic_release::adapters::release_gate_adapter::collect_host_gate_proofs(root);

    let head_sha = cynic_vcs::observe_head_sha(root);
    let build_observation = match head_sha.as_deref() {
        Some(sha) => observe_build_proof(sha),
        None => cynic_release::build_proof::BuildProofObservation::NeverTested,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    proofs.push(build_observation.to_gate_record(now));

    let report = cynic_release::evaluate_gate_records(proofs);

    if !report.all_passed {
        let exit = SourceExit {
            status: "blocked",
            summary: format!(
                "gate failed — {} checks failed: {}",
                report.blockers.len(),
                report.blockers.join("; ")
            ),
        };
        return if json {
            emit_pretty_json(&serde_json::json!({
                "schema": "cynic.source_release_report.v1",
                "status": "blocked",
                "commit": head_sha,
                "build_proof": build_observation,
                "gate_records": report.records,
                "blockers": report.blockers,
            }))
        } else {
            println!("{}", exit.summary);
            ExitCode::FAILURE
        };
    }

    if apply {
        let remote = "github";
        let branch = "main";
        let push_result = cynic_release::adapters::release_relay_adapter::push_branch_to_remote(
            root, remote, branch,
        );
        let readback = cynic_release::adapters::release_relay_adapter::verify_branch_readback(
            root, remote, branch,
        );

        if json {
            emit_pretty_json(&serde_json::json!({
                "schema": "cynic.source_release_report.v1",
                "status": if push_result.is_ok() && readback.matches { "success" } else { "failed" },
                "remote": remote,
                "branch": branch,
                "readback": {
                    "local_sha": readback.local_sha,
                    "remote_sha": readback.remote_sha,
                    "matches": readback.matches,
                },
                "push_error": push_result.as_ref().err(),
            }))
        } else {
            match push_result {
                Ok(_) => {
                    println!("Release published to relay '{remote}' branch '{branch}'.");
                    println!(
                        "Read-back proof: local={} remote={} matches={}",
                        readback.local_sha.as_deref().unwrap_or("none"),
                        readback.remote_sha.as_deref().unwrap_or("none"),
                        readback.matches
                    );
                    if readback.matches {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("Release push failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    } else {
        let exit = SourceExit {
            status: "ready",
            summary: format!(
                "gate passed — forgejo ahead by {} commits, ready for release (pass --apply to execute)",
                observation.forgejo.commits_ahead.unwrap_or(0)
            ),
        };
        if json {
            emit_pretty_json(&exit)
        } else {
            println!("{}", exit.summary);
            ExitCode::SUCCESS
        }
    }
}
