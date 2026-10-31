//! Pipeline build proof and build runner port contracts.
//!
//! Provides pure domain types for distributed build triggers, pipeline proofs,
//! layer status, verdicts, and runner health observations.

use std::fmt;

/// Build phase requested on the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BuildPhase {
    /// `cargo check` only — fast convergence signal.
    Check,
    /// Full release build of runtime artifacts.
    Build,
    /// Build + activate (enroll node, install service).
    Activate,
    /// `cargo test` — the test proof.
    Test,
}

impl BuildPhase {
    /// CLI / argv token for the runner's `--target-phase`.
    pub fn as_arg(&self) -> &'static str {
        match self {
            BuildPhase::Check => "check",
            BuildPhase::Build => "build",
            BuildPhase::Activate => "activate",
            BuildPhase::Test => "test",
        }
    }
}

impl fmt::Display for BuildPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_arg())
    }
}

/// Network mode the runner may use to resolve dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NetworkMode {
    Offline,
    Online,
}

impl NetworkMode {
    pub fn as_arg(&self) -> &'static str {
        match self {
            NetworkMode::Offline => "offline",
            NetworkMode::Online => "online",
        }
    }
}

/// A build request: an exact source commit and the phase to run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildTrigger {
    pub source_commit: String,
    pub phase: BuildPhase,
    pub network_mode: NetworkMode,
}

impl BuildTrigger {
    /// Validate the commit is an exact 40-char hex SHA — fail-closed before
    /// any runner round-trip.
    pub fn validate(&self) -> Result<(), BuildError> {
        if !self.source_commit.chars().all(|c| c.is_ascii_hexdigit())
            || self.source_commit.len() != 40
        {
            return Err(BuildError::InvalidCommit(self.source_commit.clone()));
        }
        Ok(())
    }
}

/// Per-layer build result, mirroring `cynic.layer_build_proof.v1`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LayerProof {
    pub layer: String,
    pub status: LayerStatus,
    /// If blocked, the upstream layer that blocked it.
    pub blocked_by: Option<String>,
}

/// Status of a single layer in the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerStatus {
    /// Layer check/test passed.
    Converged,
    /// Layer failed its own check/test.
    Failed,
    /// Layer not attempted because an upstream layer blocked it.
    BlockedUpstream,
}

/// Aggregated pipeline proof, mirroring `cynic.pipeline_build_proof.v1`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildProofBundle {
    pub schema: String,
    pub node: String,
    pub source_commit: String,
    pub action: String,
    pub status: PipelineStatus,
    pub broken_layer: Option<String>,
    pub layers: Vec<LayerProof>,
    /// Wall time of the remote build command in milliseconds, measured
    /// around the runner round-trip with `Instant::now()`.
    pub duration_ms: u64,
    /// Builder CPU time consumed by the remote build in milliseconds.
    #[serde(default)]
    pub cpu_ms: Option<u64>,
    /// Why the pipeline blocked, in the runner's own words, truncated.
    #[serde(default)]
    pub verdict_reason: Option<String>,
}

/// Whole-pipeline convergence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Converged,
    Blocked,
}

impl BuildProofBundle {
    /// Machine-readable JSON projection (the read-back proof surface).
    pub fn to_json(&self) -> String {
        let layers: Vec<String> = self
            .layers
            .iter()
            .map(|l| {
                format!(
                    "{{\"layer\":\"{}\",\"status\":\"{}\",\"blocked_by\":{}}}",
                    l.layer,
                    match l.status {
                        LayerStatus::Converged => "converged",
                        LayerStatus::Failed => "failed",
                        LayerStatus::BlockedUpstream => "blocked_upstream",
                    },
                    match &l.blocked_by {
                        Some(b) => format!("\"{}\"", b),
                        None => "null".to_string(),
                    }
                )
            })
            .collect();
        format!(
            "{{\"schema\":\"{}\",\"node\":\"{}\",\"source_commit\":\"{}\",\"action\":\"{}\",\"status\":\"{}\",\"broken_layer\":{},\"duration_ms\":{},\"cpu_ms\":{},\"verdict_reason\":{},\"layers\":[{}]}}",
            self.schema,
            self.node,
            self.source_commit,
            self.action,
            match self.status {
                PipelineStatus::Converged => "converged",
                PipelineStatus::Blocked => "blocked",
            },
            match &self.broken_layer {
                Some(b) => format!("\"{}\"", escape_json_string(b)),
                None => "null".to_string(),
            },
            self.duration_ms,
            match self.cpu_ms {
                Some(ms) => ms.to_string(),
                None => "null".to_string(),
            },
            match &self.verdict_reason {
                Some(r) => format!("\"{}\"", escape_json_string(r)),
                None => "null".to_string(),
            },
            layers.join(",")
        )
    }
}

fn escape_json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Health of the network build runner, observed before triggering.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunnerHealth {
    pub node_id: String,
    pub reachable: bool,
    /// Whether the runner has a warm incremental cache for the requested commit.
    pub cache_warm: bool,
    pub toolchain: String,
    /// Source SHA the runner binary reports via `cynic version --json`.
    pub binary_sha: Option<String>,
}

/// How a trigger ended when the runner RENDERED A VERDICT on the commit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BuildVerdict {
    /// The runner ran the requested action and the pipeline converged.
    Converged,
    /// The runner ran (or refused to run) the requested action and the commit
    /// did not pass. Carries the broken layer and the truncated evidence.
    Blocked {
        broken_layer: Option<String>,
        reason: String,
    },
}

impl BuildVerdict {
    /// Project the verdict onto the bundle's pipeline status.
    pub fn pipeline_status(&self) -> PipelineStatus {
        match self {
            BuildVerdict::Converged => PipelineStatus::Converged,
            BuildVerdict::Blocked { .. } => PipelineStatus::Blocked,
        }
    }

    pub fn is_converged(&self) -> bool {
        matches!(self, BuildVerdict::Converged)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            BuildVerdict::Converged => None,
            BuildVerdict::Blocked { reason, .. } => Some(reason.as_str()),
        }
    }
}

/// Errors from the build runner port: the cases where NO verdict exists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BuildError {
    InvalidCommit(String),
    RunnerUnreachable(String),
    ConfigurationRefused(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::InvalidCommit(c) => write!(f, "invalid source commit: {c}"),
            BuildError::RunnerUnreachable(e) => write!(f, "runner unreachable: {e}"),
            BuildError::ConfigurationRefused(e) => {
                write!(f, "build trigger configuration refused: {e}")
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// Port: trigger a distributed build and observe runner health.
pub trait BuildRunnerPort {
    fn trigger(&self, trigger: &BuildTrigger) -> Result<BuildProofBundle, BuildError>;
    fn observe_runner(&self) -> RunnerHealth;
}

/// Aggregate pipeline proofs for one commit into a metabolic invoice.
pub fn invoice_from_proofs(
    source_commit: &str,
    proofs: &[BuildProofBundle],
) -> cynic_protocol::throughput_invoice::ThroughputInvoice {
    use cynic_protocol::energy_ledger::EnergyDimensions;
    use cynic_protocol::throughput_invoice::{THROUGHPUT_INVOICE_SCHEMA, ThroughputInvoiceLine};

    let mut lines = Vec::new();
    let mut unknowns = Vec::new();
    let mut total_ms: u64 = 0;
    let mut total_cpu_ms: u64 = 0;
    let mut cpu_complete = true;

    for proof in proofs {
        total_ms = total_ms.saturating_add(proof.duration_ms);
        match proof.cpu_ms {
            Some(ms) => total_cpu_ms = total_cpu_ms.saturating_add(ms),
            None => cpu_complete = false,
        }
        if proof.layers.len() == 1 {
            let layer = &proof.layers[0];
            lines.push(ThroughputInvoiceLine {
                layer: layer.layer.clone(),
                action: proof.action.clone(),
                status: match layer.status {
                    LayerStatus::Converged => "converged".to_string(),
                    LayerStatus::Failed => "failed".to_string(),
                    LayerStatus::BlockedUpstream => "blocked_upstream".to_string(),
                },
                wall_time_ms: proof.duration_ms,
                wall_time_observed: true,
            });
        } else {
            unknowns.push(format!("{}:{}:layers_unsplit", proof.action, proof.duration_ms));
        }
    }

    let cpu_dimensions = if cpu_complete && !proofs.is_empty() {
        EnergyDimensions {
            wall_time_ms: total_ms,
            cpu_ms: Some(total_cpu_ms),
            network_bytes: None,
            disk_bytes: None,
            estimated_joules: 0.0,
        }
    } else {
        EnergyDimensions {
            wall_time_ms: total_ms,
            cpu_ms: None,
            network_bytes: None,
            disk_bytes: None,
            estimated_joules: 0.0,
        }
    };

    cynic_protocol::throughput_invoice::ThroughputInvoice {
        schema: THROUGHPUT_INVOICE_SCHEMA.to_string(),
        source_commit: source_commit.to_string(),
        proofs_observed: proofs.len(),
        proofs_invoiced: lines.len(),
        total: cpu_dimensions,
        lines,
        residual_unknowns: unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_json_carries_duration_ms() {
        let proof = BuildProofBundle {
            schema: "cynic.pipeline_build_proof.v1".to_string(),
            node: "runner-1".to_string(),
            source_commit: "a".repeat(40),
            action: "cargo:check".to_string(),
            status: PipelineStatus::Converged,
            broken_layer: None,
            layers: vec![LayerProof {
                layer: "cynic-host".to_string(),
                status: LayerStatus::Converged,
                blocked_by: None,
            }],
            duration_ms: 1234,
            cpu_ms: Some(900),
            verdict_reason: None,
        };
        let json = proof.to_json();
        assert!(json.contains("\"duration_ms\":1234"), "{json}");
        assert!(json.contains("\"cpu_ms\":900"), "{json}");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("proof is JSON");
        assert_eq!(parsed["schema"], "cynic.pipeline_build_proof.v1");
        assert_eq!(parsed["duration_ms"], 1234);
        assert_eq!(parsed["cpu_ms"], 900);
    }

    fn single_layer_proof(action: &str, ms: u64) -> BuildProofBundle {
        single_layer_proof_with_cpu(action, ms, None)
    }

    fn single_layer_proof_with_cpu(action: &str, ms: u64, cpu_ms: Option<u64>) -> BuildProofBundle {
        BuildProofBundle {
            schema: "cynic.pipeline_build_proof.v1".to_string(),
            node: "runner-1".to_string(),
            source_commit: "a".repeat(40),
            action: action.to_string(),
            status: PipelineStatus::Converged,
            broken_layer: None,
            layers: vec![LayerProof {
                layer: "cynic-host".to_string(),
                status: LayerStatus::Converged,
                blocked_by: None,
            }],
            duration_ms: ms,
            cpu_ms,
            verdict_reason: None,
        }
    }

    #[test]
    fn invoice_sums_observed_wall_time() {
        let proofs = vec![
            single_layer_proof("cargo:check", 1000),
            single_layer_proof("cargo:build", 2500),
        ];
        let invoice = invoice_from_proofs(&"a".repeat(40), &proofs);
        assert!(invoice.validate().is_ok());
        assert_eq!(invoice.proofs_observed, 2);
        assert_eq!(invoice.proofs_invoiced, 2);
        assert_eq!(invoice.total.wall_time_ms, 3500);
        assert_eq!(invoice.total.cpu_ms, None);
        assert_eq!(invoice.residual_unknowns.len(), 1);
    }

    #[test]
    fn invoice_sums_cpu_when_fully_observed() {
        let proofs = vec![
            single_layer_proof_with_cpu("cargo:check", 1000, Some(800)),
            single_layer_proof_with_cpu("cargo:build", 2500, Some(2200)),
        ];
        let invoice = invoice_from_proofs(&"a".repeat(40), &proofs);
        assert_eq!(invoice.total.cpu_ms, Some(3000));
        assert!(invoice.residual_unknowns.is_empty());
    }

    #[test]
    fn invoice_cpu_is_unknown_when_partially_observed() {
        let proofs = vec![
            single_layer_proof_with_cpu("cargo:check", 1000, Some(800)),
            single_layer_proof("cargo:build", 2500),
        ];
        let invoice = invoice_from_proofs(&"a".repeat(40), &proofs);
        assert_eq!(invoice.total.cpu_ms, None);
        assert_eq!(invoice.total.wall_time_ms, 3500);
        assert_eq!(invoice.residual_unknowns.len(), 1);
    }

    #[test]
    fn invoice_without_proofs_is_empty_and_honest() {
        let invoice = invoice_from_proofs(&"a".repeat(40), &[]);
        assert_eq!(invoice.total.wall_time_ms, 0);
        assert!(invoice.lines.is_empty());
        assert!(invoice.residual_unknowns.is_empty());
    }

    #[test]
    fn proof_json_round_trips_through_serde() {
        let proof = single_layer_proof("cargo:check", 27735);
        let json = proof.to_json();
        let back: BuildProofBundle = serde_json::from_str(&json).expect("emitted proof must parse");
        assert_eq!(back, proof);
    }
}
