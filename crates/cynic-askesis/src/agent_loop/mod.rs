use std::collections::BTreeSet;

use serde_json::Value;

pub mod catalog;
pub mod types;

pub use types::{AgentLoopAuditReport, AgentLoopCapability, CapabilityStatus};

const TARGET_PERCENT: f64 = 80.0;

pub fn audit_agent_loop(exposure: &Value, ecosystem: &Value) -> AgentLoopAuditReport {
    let mut tools = BTreeSet::new();
    collect_tool_names(exposure, &mut tools);
    let hermes_component_pinned = contains_string(ecosystem, "hermes-agent");

    let mut capabilities = catalog::default_capabilities(&tools, hermes_component_pinned);
    capabilities.sort_by_key(|capability| (capability.phase, capability.id));

    let declared = capabilities
        .iter()
        .filter(|capability| matches!(capability.status, CapabilityStatus::Declared))
        .count();
    let declared_percent = percent(declared, capabilities.len());
    let has_critical_gap = capabilities.iter().any(|capability| {
        capability.critical && !matches!(capability.status, CapabilityStatus::Declared)
    });

    AgentLoopAuditReport {
        schema_version: "cynic.agent-learning-loop-audit.v2",
        scope: vec!["agent_execution", "data_flywheel", "industrial_ml"],
        status: if declared_percent >= TARGET_PERCENT && !has_critical_gap {
            "ready"
        } else {
            "gap"
        },
        authority: "declared_projection_only",
        target_percent: TARGET_PERCENT,
        declared_percent,
        observed_percent: None,
        hermes_component_pinned,
        capabilities,
        residual_risks: vec![
            "Declared tool exposure does not prove that an MCP client can discover or execute a tool.",
            "No real mutable action is exercised by this static audit.",
            "Prompt injection, tool poisoning, secret exfiltration, and output sanitization require conformance trials.",
            "Existing observations, audit rows, verdicts, and knowledge records are not training-ready without lineage, rights, redaction, and immutable dataset versions.",
            "Teacher outputs are candidate labels, not ground truth; distillation must preserve disagreement and uncertainty.",
            "Hermes and ODS remain replaceable adapters; neither is runtime authority.",
        ],
    }
}

fn collect_tool_names(value: &Value, tools: &mut BTreeSet<String>) {
    match value {
        Value::String(candidate) => {
            if candidate.contains('_')
                && !candidate.chars().any(char::is_whitespace)
                && candidate.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
                })
            {
                tools.insert(candidate.to_ascii_lowercase());
            }
        }
        Value::Array(values) => values
            .iter()
            .for_each(|value| collect_tool_names(value, tools)),
        Value::Object(values) => values
            .values()
            .for_each(|value| collect_tool_names(value, tools)),
        _ => {}
    }
}

fn contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(candidate) => candidate == needle,
        Value::Array(values) => values.iter().any(|value| contains_string(value, needle)),
        Value::Object(values) => values.values().any(|value| contains_string(value, needle)),
        _ => false,
    }
}

fn percent(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
}
