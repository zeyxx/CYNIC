use std::path::Path;
use std::process::ExitCode;

use cynic_host::domain::storage::apply_client_policy_file;
use cynic_storage::client_policy::{
    ClientPolicy, STORAGE_CLIENT_POLICY_SCHEMA, StorageClientPolicy,
};

pub(crate) fn execute_client_policy_set(
    path: &str,
    endpoints: &[String],
    timeout_ms: u64,
    write_quorum: u32,
    apply: bool,
    json: bool,
) -> ExitCode {
    let policy = StorageClientPolicy {
        schema: STORAGE_CLIENT_POLICY_SCHEMA.to_string(),
        client_policy: ClientPolicy {
            endpoints: endpoints.to_vec(),
            failover_mode: "ordered_fallback".to_string(),
            write_quorum,
            timeout_ms,
        },
    };
    match apply_client_policy_file(Path::new(path), &policy, apply) {
        Ok(()) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "schema": "cynic.storage_client_policy_apply.v1",
                        "status": "proven",
                        "policy_path": path,
                        "endpoints": endpoints,
                    })
                );
            } else {
                println!("storage client policy applied and read back: {path}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if json {
                println!(
                    "{{\"schema\":\"cynic.storage_client_policy_apply.v1\",\"status\":\"blocked\",\"error\":\"{}\"}}",
                    error
                );
            } else {
                eprintln!("storage client policy blocked: {error}");
            }
            ExitCode::from(1)
        }
    }
}
