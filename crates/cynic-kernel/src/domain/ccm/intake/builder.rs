//! Observation builders and domain inference logic.

use crate::json::JsonValue;
use super::hash::content_hash;

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
) -> crate::domain::storage::Observation {
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

/// Extended build_observation with ledger system support.
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
    value: Option<JsonValue>,
    confidence: Option<String>,
    consumer: Option<String>,
    action: Option<String>,
    depends_on: Option<Vec<String>>,
    maturity: Option<f64>,
) -> crate::domain::storage::Observation {
    let resolved_domain = domain.unwrap_or_else(|| infer_domain(target.as_deref(), Some(&tool)));
    let resolved_agent_id = agent_id.unwrap_or_else(|| "unknown".into());

    let hash_input = format!(
        "{:}|{:}|{:}|{:}",
        resolved_domain,
        tool,
        target.as_deref().unwrap_or(""),
        chrono::Utc::now().timestamp_millis()
    );
    let hash = format!("{:x}", content_hash(&hash_input));

    let consensus_score = confidence.as_deref().and_then(|c| match c {
        "observed" => Some(0.9),
        "deduced" => Some(0.7),
        "inferred" => Some(0.5),
        "conjecture" => Some(0.3),
        _ => None,
    });

    crate::domain::storage::Observation {
        project: project.unwrap_or_else(|| "CYNIC".into()),
        agent_id: resolved_agent_id.clone(),
        tool,
        target: crate::domain::sanitize::sanitize_observation_target(&target.unwrap_or_default()),
        domain: resolved_domain,
        status: status.unwrap_or_else(|| "success".into()),
        context: context
            .map(|c| c.chars().take(2000).collect())
            .unwrap_or_default(),
        session_id: session_id.unwrap_or_default(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        tags: tags.unwrap_or_default(),
        source_tier: String::new(),
        value,
        confidence,
        consumer,
        action,
        depends_on: depends_on.unwrap_or_default(),
        maturity,
        hash,
        prev_hash: String::new(),
        observers: vec![resolved_agent_id],
        consensus_score,
    }
}
