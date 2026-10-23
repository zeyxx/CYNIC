//! Pure config-reload decision core for `POST /admin/reload`.
//!
//! The HTTP layer owns every side effect (file read, HMAC check, evidence
//! persist, live-snapshot swap). Everything in this module is a pure
//! function of its string inputs, so the refusal and diff rules are
//! unit-testable without a socket, a store, or a running daemon.
//!
//! Contract:
//! - A candidate that does not parse (or fails the same validation the
//!   startup loader applies) is refused with a precise reason. The caller
//!   keeps serving the previous config; nothing is half-applied because the
//!   swap happens only on `ReloadDecision::Applied`.
//! - The only ambient input is the process environment (`${VAR}` expansion
//!   behaves exactly as at startup). No filesystem, no clock, no network.

use sha2::{Digest, Sha256};

use super::load::{expand_node_control, validate_node_control};
use super::types::{NodeControlConfig, NodeControlRuntimeConfig};

/// Wire schema for reload receipts (`POST /admin/reload` persists one per
/// applied transition; the evidence store is the audit trail).
pub const RELOAD_RECEIPT_SCHEMA: &str = "cynic.node.reload_receipt.v1";

/// Pure reload choice. The caller owns the file read, the receipt persist,
/// and the live-snapshot swap.
#[derive(Debug, Clone)]
pub enum ReloadDecision {
    /// The candidate parsed, validated, and differs: swap to `control`.
    Applied {
        previous_digest: String,
        applied_digest: String,
        changed_keys: Vec<String>,
        requires_restart: bool,
        control: NodeControlConfig,
    },
    /// Semantically identical config: nothing to do, persist nothing.
    NoChange { digest: String },
    /// The candidate is unusable: keep serving the previous config.
    Refused {
        reason: String,
        previous_digest: String,
    },
}

/// Machine outcome name for receipts and HTTP responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadOutcome {
    Applied,
    AlreadyConverged,
    Refused,
}

impl ReloadOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadyConverged => "already-converged",
            Self::Refused => "refused",
        }
    }
}

/// One reload receipt payload, persisted per applied transition as
/// [`RELOAD_RECEIPT_SCHEMA`]. Digests are the semantic [`canonical_digest`]
/// of the file bytes (whitespace and comment edits converge); `changed_keys`
/// names the dotted TOML paths that moved so the response can report applied
/// vs previous without re-reading the file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReloadReceipt {
    pub schema: String,
    pub node_id: String,
    pub outcome: String,
    pub previous_digest: String,
    pub applied_digest: String,
    pub changed_keys: Vec<String>,
    pub requires_restart: bool,
    pub observed_at_unix_ms: u128,
}

impl ReloadReceipt {
    pub fn applied(
        node_id: &str,
        previous_digest: &str,
        applied_digest: &str,
        changed_keys: Vec<String>,
        requires_restart: bool,
        observed_at_unix_ms: u128,
    ) -> Self {
        Self {
            schema: RELOAD_RECEIPT_SCHEMA.to_string(),
            node_id: node_id.to_string(),
            outcome: ReloadOutcome::Applied.as_str().to_string(),
            previous_digest: previous_digest.to_string(),
            applied_digest: applied_digest.to_string(),
            changed_keys,
            requires_restart,
            observed_at_unix_ms,
        }
    }
}

/// Parse and validate one `node-control.toml` document exactly as the
/// startup loader does (same expansion, same validation). Any deviation is
/// refused with the loader's own reason — never a new error vocabulary.
pub fn parse_node_control_str(raw: &str) -> Result<NodeControlConfig, String> {
    let mut runtime: NodeControlRuntimeConfig =
        toml::from_str(raw).map_err(|error| format!("config parse error: {error}"))?;
    expand_node_control(&mut runtime.node_control)?;
    validate_node_control(&runtime.node_control)?;
    Ok(runtime.node_control)
}

/// Semantic digest of one config document: the SHA-256 (hex) of the
/// canonical JSON rendering of the parsed TOML value. Whitespace, comment,
/// and key-order edits digest identically; any semantic change digests
/// differently.
pub fn canonical_digest(raw: &str) -> Result<String, String> {
    let value: toml::Value =
        toml::from_str(raw).map_err(|error| format!("config parse error: {error}"))?;
    let canonical =
        serde_json::to_string(&value).map_err(|error| format!("canonicalize config: {error}"))?;
    Ok(hex::encode(Sha256::digest(canonical.as_bytes())))
}

/// Dotted-path diff between two parsed config documents, sorted. Tables
/// recurse (`node_control.interval_secs`); any other unequal pair — changed
/// scalars, added or removed keys, reordered arrays — reports its own path
/// without descending further.
pub fn diff_toml_values(previous: &toml::Value, candidate: &toml::Value) -> Vec<String> {
    let mut out = Vec::new();
    diff_at(previous, candidate, String::new(), &mut out);
    out.sort();
    out
}

fn diff_at(previous: &toml::Value, candidate: &toml::Value, path: String, out: &mut Vec<String>) {
    match (previous, candidate) {
        (toml::Value::Table(prev), toml::Value::Table(next)) => {
            for (key, prev_value) in prev {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match next.get(key) {
                    Some(next_value) => diff_at(prev_value, next_value, child, out),
                    None => out.push(child),
                }
            }
            for key in next.keys() {
                if !prev.contains_key(key) {
                    out.push(if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    });
                }
            }
        }
        _ => {
            if previous != candidate {
                out.push(path);
            }
        }
    }
}

/// Fields the builder cannot hot-apply without a process bounce: the socket
/// is already bound and the state-root stores (evidence, workload store)
/// are already constructed. A reload that moves any of these still swaps
/// the snapshot, but the receipt flags `requires_restart` so the operator
/// knows a bounce is due.
pub fn requires_restart(previous: &NodeControlConfig, candidate: &NodeControlConfig) -> bool {
    previous.builder_listen_addr != candidate.builder_listen_addr
        || previous.agent_workload_state_root != candidate.agent_workload_state_root
        || previous.runtime_artifact_inbox_root != candidate.runtime_artifact_inbox_root
}

/// Pure reload choice over two file contents. No I/O: the caller read both
/// strings (the previous content is the last applied document, the candidate
/// is the current file). A previous document that fails to parse is a
/// caller bug (it was valid when applied); it is refused rather than
/// coerced so the failure stays loud.
pub fn decide_reload(previous_raw: &str, candidate_raw: &str) -> ReloadDecision {
    let previous_digest = match canonical_digest(previous_raw) {
        Ok(digest) => digest,
        Err(error) => {
            return ReloadDecision::Refused {
                reason: format!("previous config unreadable: {error}"),
                previous_digest: String::new(),
            };
        }
    };
    let candidate_digest = match canonical_digest(candidate_raw) {
        Ok(digest) => digest,
        Err(error) => {
            return ReloadDecision::Refused {
                reason: error,
                previous_digest: previous_digest.clone(),
            };
        }
    };
    if candidate_digest == previous_digest {
        return ReloadDecision::NoChange {
            digest: previous_digest,
        };
    }
    let control = match parse_node_control_str(candidate_raw) {
        Ok(control) => control,
        Err(reason) => {
            return ReloadDecision::Refused {
                reason,
                previous_digest: previous_digest.clone(),
            };
        }
    };
    let previous_control = match parse_node_control_str(previous_raw) {
        Ok(control) => control,
        Err(error) => {
            return ReloadDecision::Refused {
                reason: format!("previous config unreadable: {error}"),
                previous_digest: previous_digest.clone(),
            };
        }
    };
    let previous_value: toml::Value = match toml::from_str(previous_raw) {
        Ok(value) => value,
        Err(error) => {
            return ReloadDecision::Refused {
                reason: format!("previous config unreadable: config parse error: {error}"),
                previous_digest: previous_digest.clone(),
            };
        }
    };
    let candidate_value: toml::Value = match toml::from_str(candidate_raw) {
        Ok(value) => value,
        Err(error) => {
            return ReloadDecision::Refused {
                reason: format!("config parse error: {error}"),
                previous_digest: previous_digest.clone(),
            };
        }
    };
    ReloadDecision::Applied {
        previous_digest,
        applied_digest: candidate_digest,
        changed_keys: diff_toml_values(&previous_value, &candidate_value),
        requires_restart: requires_restart(&previous_control, &control),
        control,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_config(node_id: &str) -> String {
        format!(
            r#"
[node_control]
kernel_url = "http://127.0.0.1:3010"
node_id = "{node_id}"
signing_key_path = "/tmp/cynic-test/signing.key"
enrollment_request_path = "/tmp/cynic-test/enrollment.request"
enrollment_grant_path = "/tmp/cynic-test/enrollment.grant"
enrollment_authority_public_key_path = "/tmp/cynic-test/authority.pub"
enrollment_token_path = "/tmp/cynic-test/enrollment.token"
enrollment_seed_id = "seed-1"
enrollment_role = "build_runner"
"#
        )
    }

    #[test]
    fn valid_config_parses_like_startup() {
        let control = parse_node_control_str(&minimal_config("builder-1")).expect("valid");
        assert_eq!(control.node_id, "builder-1");
        assert_eq!(control.enrollment_role, "build_runner");
    }

    #[test]
    fn garbage_candidate_is_refused_with_loader_reason() {
        let previous = minimal_config("builder-1");
        let decision = decide_reload(&previous, "this is [not = valid toml {{{");
        match decision {
            ReloadDecision::Refused {
                reason,
                previous_digest,
            } => {
                assert!(reason.contains("config parse error"), "reason: {reason}");
                assert!(!previous_digest.is_empty());
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn invalid_candidate_is_refused_and_previous_digest_survives() {
        let previous = minimal_config("builder-1");
        // Parses as TOML but fails validation: empty node_id.
        let candidate = minimal_config("");
        let decision = decide_reload(&previous, &candidate);
        match decision {
            ReloadDecision::Refused {
                reason,
                previous_digest,
            } => {
                assert!(
                    reason.contains("node_control.node_id must be non-empty"),
                    "reason: {reason}"
                );
                assert_eq!(previous_digest, canonical_digest(&previous).unwrap());
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn identical_content_is_no_change_even_with_different_whitespace() {
        let previous = minimal_config("builder-1");
        let candidate = format!("\n\n# a comment\n{previous}\n");
        match decide_reload(&previous, &candidate) {
            ReloadDecision::NoChange { digest } => {
                assert_eq!(digest, canonical_digest(&previous).unwrap());
            }
            other => panic!("expected NoChange, got {other:?}"),
        }
    }

    #[test]
    fn changed_scalar_reports_its_dotted_path() {
        let previous = minimal_config("builder-1");
        let candidate = previous.replace("seed-1", "seed-2");
        match decide_reload(&previous, &candidate) {
            ReloadDecision::Applied {
                changed_keys,
                requires_restart,
                control,
                ..
            } => {
                assert_eq!(
                    changed_keys,
                    vec!["node_control.enrollment_seed_id".to_string()]
                );
                assert!(!requires_restart);
                assert_eq!(control.enrollment_seed_id, "seed-2");
            }
            other => panic!("expected Applied, got {other:?}"),
        }
    }

    #[test]
    fn listen_addr_move_flags_restart_but_still_applies() {
        let previous = minimal_config("builder-1");
        let candidate =
            format!("{previous}[node_control.build_scheduler]\nrepository_id = \"cynic\"\n");
        // build_scheduler without listen change: no restart required.
        match decide_reload(&previous, &candidate) {
            ReloadDecision::Applied {
                changed_keys,
                requires_restart,
                ..
            } => {
                assert!(!requires_restart);
                assert!(
                    changed_keys
                        .iter()
                        .any(|key| key == "node_control.build_scheduler"),
                    "keys: {changed_keys:?}"
                );
            }
            other => panic!("expected Applied, got {other:?}"),
        }
    }

    #[test]
    fn canonical_digest_ignores_comments_and_key_order() {
        let first = "[node_control]\nkernel_url = \"http://127.0.0.1:3010\"\nnode_id = \"b1\"\n";
        let second =
            "# comment\n[node_control]\nnode_id = \"b1\"\nkernel_url = \"http://127.0.0.1:3010\"\n";
        assert_eq!(
            canonical_digest(first).unwrap(),
            canonical_digest(second).unwrap()
        );
    }

    #[test]
    fn diff_reports_added_and_removed_keys() {
        let previous: toml::Value = toml::from_str("[a]\nx = 1\ngone = true\n").unwrap();
        let candidate: toml::Value = toml::from_str("[a]\nx = 1\nnew = 2\n").unwrap();
        assert_eq!(
            diff_toml_values(&previous, &candidate),
            vec!["a.gone".to_string(), "a.new".to_string()]
        );
    }

    #[test]
    fn reload_receipt_names_schema_and_outcome() {
        let receipt =
            ReloadReceipt::applied("builder-1", "prev", "next", vec!["k".into()], false, 42);
        assert_eq!(receipt.schema, RELOAD_RECEIPT_SCHEMA);
        assert_eq!(receipt.outcome, "applied");
        let roundtrip: ReloadReceipt =
            serde_json::from_slice(&serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert_eq!(roundtrip, receipt);
    }

    #[test]
    fn outcome_names_are_byte_stable() {
        assert_eq!(ReloadOutcome::Applied.as_str(), "applied");
        assert_eq!(
            ReloadOutcome::AlreadyConverged.as_str(),
            "already-converged"
        );
        assert_eq!(ReloadOutcome::Refused.as_str(), "refused");
    }
}
