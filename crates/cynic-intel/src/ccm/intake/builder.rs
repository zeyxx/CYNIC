//! Observation builders and domain inference logic.

use super::hash::content_hash;
use cynic_protocol::{Observation, sanitize_observation_target};

/// Infer domain from file extension or target pattern.
pub fn infer_domain(target: Option<&str>, tool: Option<&str>) -> String {
    if let Some(t) = tool
        && t == "self-probe"
    {
        return "infra".to_string();
    }

    let target = match target {
        Some(t) if !t.is_empty() => t,
        _ => return "general".to_string(),
    };

    if target.starts_with("cynic-")
        || target.contains("llama-server")
        || target.contains("surrealdb")
    {
        return "infra".to_string();
    }

    target
        .rsplit('.')
        .next()
        .map(|ext| match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "md" => "docs",
            "toml" | "json" | "yaml" | "yml" => "config",
            "service" | "timer" => "infra",
            _ => "general",
        })
        .unwrap_or("general")
        .to_string()
}

/// Build an `Observation` from raw API parameters.
#[allow(clippy::too_many_arguments)]
pub fn build_observation(
    tool: String,
    target: Option<String>,
    domain: Option<String>,
    status: Option<String>,
    context: Option<String>,
    project: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
    tags: Option<Vec<String>>,
) -> Observation {
    build_observation_with_ledger(
        tool, target, domain, status, context, project, agent_id, session_id, tags,
        None, // value
        None, // confidence
        None, // consumer
        None, // action
        None, // depends_on
        None, // maturity
    )
}

/// Build an `Observation` with full ledger fields.
#[allow(clippy::too_many_arguments)]
pub fn build_observation_with_ledger(
    tool: String,
    target: Option<String>,
    domain: Option<String>,
    status: Option<String>,
    context: Option<String>,
    project: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
    tags: Option<Vec<String>>,
    value: Option<serde_json::Value>,
    confidence: Option<String>,
    consumer: Option<String>,
    action: Option<String>,
    depends_on: Option<Vec<String>>,
    maturity: Option<f64>,
) -> Observation {
    let resolved_domain = domain.unwrap_or_else(|| infer_domain(target.as_deref(), Some(&tool)));
    let resolved_agent_id = agent_id.unwrap_or_else(|| "unknown".into());

    let hash_input = format!(
        "{}:{}:{}:{}:{}:{}",
        tool,
        target.as_deref().unwrap_or(""),
        resolved_domain,
        status.as_deref().unwrap_or(""),
        context.as_deref().unwrap_or(""),
        value.as_ref().map(|v| v.to_string()).unwrap_or_default(),
    );
    let hash = format!("{:x}", content_hash(&hash_input));

    let confidence_str = confidence.unwrap_or_else(|| "observed".into());
    let consensus_score = match confidence_str.as_str() {
        "observed" => Some(1.0),
        "deduced" => Some(0.8),
        "inferred" => Some(0.6),
        "conjecture" => Some(0.4),
        _ => None,
    };

    let maturity_val = maturity.or_else(|| match confidence_str.as_str() {
        "observed" => Some(1.0),
        "deduced" => Some(0.8),
        _ => None,
    });

    Observation {
        project: project.unwrap_or_else(|| "cynic".into()),
        agent_id: resolved_agent_id.clone(),
        tool,
        target: sanitize_observation_target(&target.unwrap_or_default()),
        domain: resolved_domain,
        status: status.unwrap_or_else(|| "success".into()),
        context: context.unwrap_or_default(),
        session_id: session_id.unwrap_or_default(),
        timestamp: chrono::Utc::now().timestamp(),
        tags: tags.unwrap_or_default(),
        source_tier: String::new(),
        value,
        confidence: Some(confidence_str),
        consumer,
        action,
        depends_on: depends_on.unwrap_or_default(),
        maturity: maturity_val,
        hash,
        prev_hash: String::new(),
        observers: vec![resolved_agent_id],
        consensus_score,
    }
}
