//! Agent intent arm — thin seal/verify projection over protocol intent records.
//!
//! WHY: Field consumers (bootcamp teams) have no Rust toolchain. This arm lets
//! them seal and check IntentTransmission records natively instead of through
//! the transitional python adapter (scripts/intent-seal-TEMP.py, deleted when
//! this arm lands).
//! HOW: Parse argv, read the JSON file, delegate every check to
//! cynic-protocol intent.rs validate(), print human or JSON, return an exit
//! code. Exit code is the machine contract: 0 sealed/verified, 1 otherwise.
//! NEGATIVE BOUNDARY: No policy, no record creation wizard, no editing, no
//! journal, no persistence. Never recomputes validation rules locally —
//! intent.rs owns them; a disagreement here is a bug here.

use std::process::ExitCode;

use clap::Subcommand;
use serde_json::Value;

use cynic_protocol::intent::IntentTransmission;

use crate::cli_json::emit_pretty_json;

/// Report schema for a successful seal.
pub const AGENT_INTENT_SEAL_SCHEMA: &str = "cynic.cli.agent_intent_seal.v1";
/// Report schema for a verify verdict.
pub const AGENT_INTENT_VERIFY_SCHEMA: &str = "cynic.cli.agent_intent_verify.v1";

/// `cynic agent intent ...` — nested arm, zero new top-level hyphens.
#[derive(Subcommand)]
pub(crate) enum IntentCommand {
    /// Seal a record: compute its sha3-512 digest, print or write back
    Seal {
        /// Path to the unsealed record JSON (digest key ignored if present)
        path: String,
        /// Write the digest back into the file instead of printing it
        #[arg(long)]
        in_place: bool,
        /// Print the JSON report instead of the bare digest
        #[arg(long)]
        json: bool,
    },
    /// Verify a sealed record: recompute and compare the digest
    Verify {
        /// Path to the sealed record JSON
        path: String,
        /// Print the JSON verdict instead of human text
        #[arg(long)]
        json: bool,
    },
}

/// Load a record file and strip any claimed digest (never trust input seals).
fn load_unsealed(path: &str) -> Result<IntentTransmission, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("cannot_read_record_file: {e}"))?;
    let mut value: Value =
        serde_json::from_str(&raw).map_err(|e| format!("record_is_not_json: {e}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "record_must_be_a_json_object".to_string())?;
    object.remove("digest");
    object.insert("digest".to_string(), Value::String(String::new()));
    serde_json::from_value::<IntentTransmission>(value)
        .map_err(|e| format!("record_shape_invalid: {e}"))
}

/// Execute `cynic agent intent seal`.
pub(crate) fn execute_seal(path: &str, in_place: bool, json: bool) -> ExitCode {
    let mut record = match load_unsealed(path) {
        Ok(record) => record,
        Err(error) => {
            eprintln!("agent intent seal failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let digest = match record.expected_digest() {
        Ok(digest) => digest,
        Err(error) => {
            eprintln!("agent intent seal failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    record.digest = digest.clone();
    if let Err(violations) = record.validate() {
        eprintln!("agent intent seal failed: record invalid: {violations:?}");
        return ExitCode::FAILURE;
    }
    if in_place {
        match serde_json::to_string_pretty(&record) {
            Ok(payload) => {
                if let Err(error) = std::fs::write(path, payload + "\n") {
                    eprintln!("agent intent seal failed: cannot_write_back: {error}");
                    return ExitCode::FAILURE;
                }
            }
            Err(error) => {
                eprintln!("agent intent seal failed: serialize: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if json {
        emit_pretty_json(&serde_json::json!({
            "schema": AGENT_INTENT_SEAL_SCHEMA,
            "status": "sealed",
            "digest": record.digest,
        }))
    } else {
        println!("{}", record.digest);
        ExitCode::SUCCESS
    }
}

/// Execute `cynic agent intent verify`.
pub(crate) fn execute_verify(path: &str, json: bool) -> ExitCode {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("agent intent verify failed: cannot_read_record_file: {error}");
            return ExitCode::FAILURE;
        }
    };
    let record: IntentTransmission = match serde_json::from_str(&raw) {
        Ok(record) => record,
        Err(error) => {
            eprintln!("agent intent verify failed: record_shape_invalid: {error}");
            return ExitCode::FAILURE;
        }
    };
    match record.validate() {
        Ok(()) => {
            if json {
                emit_pretty_json(&serde_json::json!({
                    "schema": AGENT_INTENT_VERIFY_SCHEMA,
                    "status": "verified",
                    "digest": record.digest,
                }))
            } else {
                println!("OK {}", record.digest);
                ExitCode::SUCCESS
            }
        }
        Err(violations) => {
            if json {
                emit_pretty_json(&serde_json::json!({
                    "schema": AGENT_INTENT_VERIFY_SCHEMA,
                    "status": "mismatch",
                    "violations": violations,
                }));
            } else {
                eprintln!("MISMATCH {violations:?}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Dispatch `cynic agent intent ...`.
pub(crate) fn execute_intent(command: IntentCommand) -> ExitCode {
    match command {
        IntentCommand::Seal {
            path,
            in_place,
            json,
        } => execute_seal(&path, in_place, json),
        IntentCommand::Verify { path, json } => execute_verify(&path, json),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "cynic-intent-test-{name}-{}.json",
            std::process::id()
        ));
        path
    }

    fn write_fixture(name: &str, digest: &str) -> std::path::PathBuf {
        let path = fixture_path(name);
        let payload = serde_json::json!({
            "id": "0193f2a1-4b5c-4d6e-8f90-123456789abc",
            "schema": "cynic.intent_transmission.v1",
            "created_at": "2026-09-09T14:30:00Z",
            "emitter": "pole:cyber",
            "recipient": "role:pole_infra",
            "deliverable_ref": "M2-carto",
            "content": "VLAN 42 : 3 hôtes — accents, guillemets « », slash / ici.",
            "provenance_class": "observed_fact",
            "provenance_source": "scan terrain",
            "digest": digest,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&payload).unwrap()).unwrap();
        path
    }

    #[test]
    fn seal_then_verify_roundtrip() {
        let path = write_fixture("roundtrip", "");
        let path_str = path.to_string_lossy().to_string();
        assert_eq!(execute_seal(&path_str, true, false), ExitCode::SUCCESS);
        assert_eq!(execute_verify(&path_str, false), ExitCode::SUCCESS);
        let raw = std::fs::read_to_string(&path).unwrap();
        let record: IntentTransmission = serde_json::from_str(&raw).unwrap();
        assert!(record.digest.starts_with("sha3-512:"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn verify_rejects_tampered_content() {
        let path = write_fixture("tamper", "");
        let path_str = path.to_string_lossy().to_string();
        assert_eq!(execute_seal(&path_str, true, false), ExitCode::SUCCESS);
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value["content"] = Value::String("Altered after sealing.".to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
        assert_eq!(execute_verify(&path_str, false), ExitCode::FAILURE);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn seal_rejects_prefixless_addresses() {
        let path = write_fixture("prefix", "");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value["recipient"] = Value::String("everyone".to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
        let path_str = path.to_string_lossy().to_string();
        assert_eq!(execute_seal(&path_str, true, false), ExitCode::FAILURE);
        std::fs::remove_file(&path).ok();
    }
}
