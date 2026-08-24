//! ML Data Engine — Structured Agent Trace Collection & Multi-Domain Fine-Tuning Synthesis
//!
//! Transforms live agent interactions, tool calls, and verified outcomes into
//! canonical, provenance-backed training pairs for continuous local fine-tuning
//! partitioned by domain:
//! 1. Rust Architecture & Contract-First Coding
//! 2. Epistemic Judgment & Jury Axioms (FIDELITY, PHI, VERIFY)
//! 3. Economic & Onchain Solana / Anchor Analysis
//! 4. Infrastructure & Fleet Supervision (WireGuard, Kademlia, Lease Fencing)

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const TRAINING_TRACE_SCHEMA: &str = "cynic.ml.training_trace.v1";

/// The 4 specialized domains for agent fine-tuning and distillation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningDomain {
    RustArchitecture,
    EpistemicJury,
    SolanaOnchain,
    FleetOrchestration,
}

impl FineTuningDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RustArchitecture => "rust_architecture",
            Self::EpistemicJury => "epistemic_jury",
            Self::SolanaOnchain => "solana_onchain",
            Self::FleetOrchestration => "fleet_orchestration",
        }
    }

    pub fn dataset_filename(&self) -> &'static str {
        match self {
            Self::RustArchitecture => "cynic-rust-architecture-v1.jsonl",
            Self::EpistemicJury => "cynic-epistemic-jury-v1.jsonl",
            Self::SolanaOnchain => "cynic-solana-onchain-v1.jsonl",
            Self::FleetOrchestration => "cynic-fleet-orchestration-v1.jsonl",
        }
    }
}

/// A structured training pair derived from verified execution traces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainingTrace {
    pub schema: String,
    pub trace_id: String,
    pub domain: String,
    pub context_evidence_digests: Vec<String>,
    pub input_prompt: String,
    pub action_or_prediction: serde_json::Value,
    pub observed_outcome: serde_json::Value,
    pub verdict: String,
    pub q_score: Option<f64>,
    pub timestamp_unix_ms: u128,
}

impl TrainingTrace {
    pub fn new(
        trace_id: impl Into<String>,
        domain: impl Into<String>,
        input_prompt: impl Into<String>,
        action_or_prediction: serde_json::Value,
        observed_outcome: serde_json::Value,
        verdict: impl Into<String>,
        q_score: Option<f64>,
        context_evidence_digests: Vec<String>,
    ) -> Self {
        let timestamp_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        Self {
            schema: TRAINING_TRACE_SCHEMA.to_string(),
            trace_id: trace_id.into(),
            domain: domain.into(),
            context_evidence_digests,
            input_prompt: input_prompt.into(),
            action_or_prediction,
            observed_outcome,
            verdict: verdict.into(),
            q_score,
            timestamp_unix_ms,
        }
    }

    /// Converts this trace into an OpenAI/ChatML compatible fine-tuning JSON pair.
    pub fn to_fine_tuning_format(&self) -> serde_json::Value {
        serde_json::json!({
            "messages": [
                {
                    "role": "system",
                    "content": format!("You are a CYNIC sovereign AI agent operating in the domain: {}.", self.domain)
                },
                {
                    "role": "user",
                    "content": self.input_prompt
                },
                {
                    "role": "assistant",
                    "content": serde_json::to_string(&self.action_or_prediction).unwrap_or_default()
                }
            ],
            "metadata": {
                "trace_id": self.trace_id,
                "q_score": self.q_score,
                "verdict": self.verdict,
                "evidence_digests": self.context_evidence_digests
            }
        })
    }
}

/// Trace storage and dataset export manager.
pub struct TraceEngine;

impl TraceEngine {
    /// Appends a new training trace to the specified JSONL trace log.
    pub fn append_trace(path: &Path, trace: &TrainingTrace) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let line = serde_json::to_string(trace)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{}", line)
    }

    /// Loads and filters high-quality training traces (e.g. Q-score >= min_q_score).
    pub fn load_filtered_traces(
        path: &Path,
        min_q_score: Option<f64>,
    ) -> std::io::Result<Vec<TrainingTrace>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut traces = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            if let Ok(trace) = serde_json::from_str::<TrainingTrace>(&line) {
                if let Some(min_q) = min_q_score {
                    if trace.q_score.unwrap_or(0.0) < min_q {
                        continue;
                    }
                }
                traces.push(trace);
            }
        }
        Ok(traces)
    }

    /// Exports partitioned fine-tuning datasets by domain into an output directory.
    pub fn export_domain_datasets(
        source_path: &Path,
        output_dir: &Path,
        min_q_score: Option<f64>,
    ) -> std::io::Result<std::collections::BTreeMap<String, usize>> {
        let traces = Self::load_filtered_traces(source_path, min_q_score)?;
        std::fs::create_dir_all(output_dir)?;

        let mut counts = std::collections::BTreeMap::new();
        let mut handles = std::collections::BTreeMap::new();

        for trace in &traces {
            let filename = match trace.domain.as_str() {
                "rust_architecture" | "rust-coding" | "cynic-runtime" => {
                    FineTuningDomain::RustArchitecture.dataset_filename()
                }
                "epistemic_jury" | "epistemic" | "judgment" => {
                    FineTuningDomain::EpistemicJury.dataset_filename()
                }
                "solana_onchain" | "solana" | "anchor" => {
                    FineTuningDomain::SolanaOnchain.dataset_filename()
                }
                "fleet_orchestration" | "fleet" | "network" => {
                    FineTuningDomain::FleetOrchestration.dataset_filename()
                }
                _ => "cynic-general-traces-v1.jsonl",
            };

            let file = handles.entry(filename.to_string()).or_insert_with(|| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(output_dir.join(filename))
                    .expect("open dataset file")
            });

            let formatted = trace.to_fine_tuning_format();
            writeln!(
                file,
                "{}",
                serde_json::to_string(&formatted).unwrap_or_default()
            )?;
            *counts.entry(filename.to_string()).or_insert(0) += 1;
        }

        Ok(counts)
    }

    /// Generates canonical gold-standard synthetic traces for all 4 domains.
    pub fn generate_canonical_seed_traces() -> Vec<TrainingTrace> {
        vec![
            TrainingTrace::new(
                "seed-rust-01",
                FineTuningDomain::RustArchitecture.as_str(),
                "How do you implement an execution boundary without shell injection?",
                serde_json::json!({
                    "design": "Argv-based Command with explicit args array, no shell wrapper",
                    "code": "Command::new(\"cynic-node\").args([\"control\", \"--check\"]).output()"
                }),
                serde_json::json!({"security": "no_shell_injection", "deterministic": true}),
                "HOWL",
                Some(1.0),
                vec!["contract:foundation.json".into()],
            ),
            TrainingTrace::new(
                "seed-jury-01",
                FineTuningDomain::EpistemicJury.as_str(),
                "Evaluate this unverified claim: 'The node is 100% healthy without reading socket or logs'",
                serde_json::json!({
                    "verdict": "BARK",
                    "violation": "Axiom FIDELITY & VERIFY violated. Claiming success without read-back proof is forbidden."
                }),
                serde_json::json!({"accepted": false, "reason": "unproven_claim"}),
                "HOWL",
                Some(1.0),
                vec!["axiom:fidelity".into()],
            ),
            TrainingTrace::new(
                "seed-solana-01",
                FineTuningDomain::SolanaOnchain.as_str(),
                "How do you compress N offchain evidence records for Solana settlement?",
                serde_json::json!({
                    "solution": "PoH Merkle Checkpointing O(1)",
                    "instruction": "record_poh_checkpoint",
                    "pda_seeds": "[\"cynic_poh\", node_authority, epoch]"
                }),
                serde_json::json!({"onchain_cost_cu": 15000, "merkle_root_bytes": 32}),
                "HOWL",
                Some(0.98),
                vec!["cynic.anchor_client.v1".into()],
            ),
            TrainingTrace::new(
                "seed-fleet-01",
                FineTuningDomain::FleetOrchestration.as_str(),
                "What happens if a node's disk usage exceeds 75% during a heavy compilation workload?",
                serde_json::json!({
                    "policy": "ResourceBurstPolicy",
                    "action": "Pre-emptive DiskCleanupPlan triggered at high watermark threshold (75%) to avoid ENOSPC OOM crash"
                }),
                serde_json::json!({"preemptive_cleanup": true, "reclaimed_bytes": 8900000000u64}),
                "HOWL",
                Some(0.99),
                vec!["cynic.resource_allocation_policy.v1".into()],
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn trace_engine_partitions_by_domain() {
        let temp_dir = tempdir().unwrap();
        let trace_file = temp_dir.path().join("all-traces.jsonl");
        let output_dir = temp_dir.path().join("datasets");

        let seed_traces = TraceEngine::generate_canonical_seed_traces();
        for trace in &seed_traces {
            TraceEngine::append_trace(&trace_file, trace).unwrap();
        }

        let counts =
            TraceEngine::export_domain_datasets(&trace_file, &output_dir, Some(0.80)).unwrap();
        assert_eq!(
            counts.get(FineTuningDomain::RustArchitecture.dataset_filename()),
            Some(&1)
        );
        assert_eq!(
            counts.get(FineTuningDomain::EpistemicJury.dataset_filename()),
            Some(&1)
        );
        assert_eq!(
            counts.get(FineTuningDomain::SolanaOnchain.dataset_filename()),
            Some(&1)
        );
        assert_eq!(
            counts.get(FineTuningDomain::FleetOrchestration.dataset_filename()),
            Some(&1)
        );
    }
}
