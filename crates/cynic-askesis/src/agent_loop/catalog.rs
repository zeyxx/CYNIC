use super::types::{AgentLoopCapability, CapabilityStatus};
use std::collections::BTreeSet;

pub(crate) fn default_capabilities(
    tools: &BTreeSet<String>,
    hermes_component_pinned: bool,
) -> Vec<AgentLoopCapability> {
    vec![
        matched(
            tools,
            "surface_discovery",
            "discover",
            true,
            &[&["mcp", "surface"], &["api", "contract"]],
            "The agent can discover MCP tools and their machine contracts before acting.",
        ),
        matched(
            tools,
            "runtime_identity",
            "enroll",
            true,
            &[&["identity", "attest"], &["auth"]],
            "Every agent session is authenticated and bound to an attested runtime identity.",
        ),
        matched(
            tools,
            "inference_control",
            "infer",
            true,
            &[&["infer"], &["inference", "backend"]],
            "Inference is routed through declared backends with observable runtime state.",
        ),
        matched(
            tools,
            "task_lifecycle",
            "claim",
            true,
            &[
                &["agent", "task", "dispatch"],
                &["agent", "task", "claim"],
                &["agent", "task", "update"],
            ],
            "Tasks can be dispatched, exclusively claimed, and completed with a result.",
        ),
        matched(
            tools,
            "coordination_lease",
            "claim",
            true,
            &[
                &["coord", "register"],
                &["coord", "claim"],
                &["coord", "release"],
            ],
            "Concurrent agents coordinate through explicit registration and bounded leases.",
        ),
        matched(
            tools,
            "observation",
            "observe",
            true,
            &[&["observe"]],
            "Effects are observed after execution rather than inferred from tool success.",
        ),
        matched(
            tools,
            "judgment",
            "judge",
            true,
            &[&["judge"]],
            "Observed evidence is judged against an explicit expected result.",
        ),
        matched(
            tools,
            "proof_publication",
            "prove",
            true,
            &[
                &["coord", "artifact", "share"],
                &["coord", "decision", "record"],
            ],
            "Artifacts and decisions can be attached to the task as durable proof.",
        ),
        matched(
            tools,
            "canonical_validation",
            "prove",
            true,
            &[&["validate"]],
            "The loop can invoke the canonical validation gate and preserve its output.",
        ),
        matched(
            tools,
            "agent_evaluation",
            "evaluate",
            false,
            &[&["agent", "eval"]],
            "Models and agent policies can be measured with repeated controlled trials.",
        ),
        matched(
            tools,
            "knowledge_retrieval",
            "learn",
            false,
            &[&["library", "query"], &["crystal"]],
            "Agents can retrieve accumulated observations and coherence patterns through CYNIC MCP.",
        ),
        matched(
            tools,
            "governed_trace_capture",
            "collect",
            true,
            &[&["trace", "record"], &["dataset", "candidate"]],
            "Agent prompts, outputs, tool calls, proofs, judgments, and outcomes are captured as governed training candidates rather than raw chat logs.",
        ),
        matched(
            tools,
            "data_provenance_and_rights",
            "govern",
            true,
            &[
                &["data", "lineage"],
                &["data", "rights"],
                &["secret", "redact"],
            ],
            "Every retained sample records origin, ownership, license or consent, retention, sensitivity, and redaction proof.",
        ),
        matched(
            tools,
            "dataset_registry",
            "curate",
            true,
            &[
                &["dataset", "registry"],
                &["dataset", "version"],
                &["dataset", "split"],
            ],
            "Immutable dataset versions expose lineage, deduplication, labels, train/eval splits, and content hashes.",
        ),
        matched(
            tools,
            "quality_labeling",
            "curate",
            true,
            &[&["sample", "label"], &["sample", "judge"]],
            "Deterministic checks, runtime proof, judges, and human review produce separable labels with uncertainty.",
        ),
        matched(
            tools,
            "code_dependency_corpus",
            "curate",
            false,
            &[&["code", "corpus"], &["dependency", "snapshot"]],
            "Owned code, build results, dependency graphs, incidents, and patches can become versioned task-specific corpora.",
        ),
        matched(
            tools,
            "training_and_distillation",
            "train",
            true,
            &[&["training", "plan"], &["distill"], &["checkpoint"]],
            "Training and distillation are planned executions with pinned inputs, teacher identity, hyperparameters, checkpoints, and reproducible proof.",
        ),
        matched(
            tools,
            "model_artifact_lineage",
            "train",
            true,
            &[&["model", "lineage"], &["model", "artifact"]],
            "A model artifact links base model, quantization or adapter, datasets, code revision, environment, metrics, and checksums.",
        ),
        matched(
            tools,
            "model_promotion",
            "promote",
            true,
            &[&["model", "promot"], &["model", "rollback"]],
            "Only models passing capability, safety, latency, memory, and regression gates can move from candidate to canary or runtime authority.",
        ),
        matched(
            tools,
            "model_drift_monitoring",
            "observe",
            false,
            &[&["model", "drift"], &["model", "metric"]],
            "Production outcomes detect capability, data, latency, and calibration drift without training directly on uncontrolled outputs.",
        ),
        matched(
            tools,
            "product_ml_contract",
            "productize",
            false,
            &[
                &["ml", "dataset", "contract"],
                &["ml", "deployment", "proof"],
            ],
            "Future product ML uses the same governed lineage and proof interfaces without inheriting agent-chat assumptions.",
        ),
        matched(
            tools,
            "harness_binding",
            "preflight",
            true,
            &[&["harness"]],
            "Every run declares harness kind, authority, isolation, target scope, and network mutation.",
        ),
        matched(
            tools,
            "capability_policy",
            "preflight",
            true,
            &[&["agent", "capabilit"], &["agent", "policy"]],
            "The gateway grants least-privilege tool capabilities and explicit mutation approval.",
        ),
        matched(
            tools,
            "trace_correlation",
            "observe",
            true,
            &[&["trace"]],
            "Inference, MCP calls, tasks, mutations, evidence, and final answers share one trace identity.",
        ),
        matched(
            tools,
            "cancellation",
            "recover",
            false,
            &[&["cancel"]],
            "A running task and its owned execution can be cancelled without orphaning work.",
        ),
        matched(
            tools,
            "rollback",
            "recover",
            true,
            &[&["rollback"]],
            "Mutable actions expose and can invoke a bounded rollback intent.",
        ),
        matched(
            tools,
            "context_memory",
            "resume",
            false,
            &[&["agent", "memory"], &["agent", "context"]],
            "The loop can resume from compacted task state without relying on chat history alone.",
        ),
        unknown(
            "idempotency_and_deduplication",
            "execute",
            true,
            "Tool names cannot prove idempotency keys, deduplication, or safe retry semantics.",
        ),
        AgentLoopCapability {
            id: "hermes_gateway_projection",
            phase: "enroll",
            critical: true,
            status: CapabilityStatus::Missing,
            evidence: hermes_component_pinned
                .then(|| "ecosystem pin: hermes-agent".to_string())
                .into_iter()
                .collect(),
            expectation: "Hermes is replaceably configured to use CYNIC MCP as its capability gateway; a package pin is not an integration proof.",
        },
    ]
}

fn matched(
    tools: &BTreeSet<String>,
    id: &'static str,
    phase: &'static str,
    critical: bool,
    required_patterns: &[&[&str]],
    expectation: &'static str,
) -> AgentLoopCapability {
    let evidence: Vec<String> = required_patterns
        .iter()
        .filter_map(|pattern| {
            tools
                .iter()
                .find(|tool| pattern.iter().all(|token| tool.contains(token)))
                .cloned()
        })
        .collect();
    let status = if evidence.len() == required_patterns.len() {
        CapabilityStatus::Declared
    } else {
        CapabilityStatus::Missing
    };
    AgentLoopCapability {
        id,
        phase,
        critical,
        status,
        evidence,
        expectation,
    }
}

fn unknown(
    id: &'static str,
    phase: &'static str,
    critical: bool,
    expectation: &'static str,
) -> AgentLoopCapability {
    AgentLoopCapability {
        id,
        phase,
        critical,
        status: CapabilityStatus::Unknown,
        evidence: Vec::new(),
        expectation,
    }
}
