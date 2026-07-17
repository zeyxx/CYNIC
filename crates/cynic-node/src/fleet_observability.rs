#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetEventValidationError {
    InvalidTool,
    InvalidNode,
}

impl FleetEventValidationError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidTool => "tool must be 1-64 characters",
            Self::InvalidNode => "node must be 1-64 characters",
        }
    }
}

pub fn validate_event_identity(tool: &str, node: &str) -> Result<(), FleetEventValidationError> {
    if tool.is_empty() || tool.len() > 64 {
        return Err(FleetEventValidationError::InvalidTool);
    }
    if node.is_empty() || node.len() > 64 {
        return Err(FleetEventValidationError::InvalidNode);
    }
    Ok(())
}

pub fn classify_node_quality(
    success_rate: f64,
    last_seen_secs: u64,
    failure_reason: &str,
) -> &'static str {
    match failure_reason {
        "none" => match (last_seen_secs, success_rate) {
            (0..=5, sr) if sr >= 0.95 => "excellent",
            (0..=60, sr) if sr >= 0.9 => "good",
            (_, sr) if sr >= 0.7 => "degraded",
            _ => "dead",
        },
        "port_conflict" | "config_error" | "firewall" | "unknown" => "degraded",
        _ => "dead",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_event_identity_bounds() {
        assert!(validate_event_identity("probe", "node-a").is_ok());
        assert_eq!(
            validate_event_identity("", "node-a"),
            Err(FleetEventValidationError::InvalidTool)
        );
        assert_eq!(
            validate_event_identity("probe", ""),
            Err(FleetEventValidationError::InvalidNode)
        );
    }

    #[test]
    fn classifies_node_quality_from_failure_and_freshness() {
        assert_eq!(classify_node_quality(0.98, 2, "none"), "excellent");
        assert_eq!(classify_node_quality(0.91, 30, "none"), "good");
        assert_eq!(classify_node_quality(0.75, 600, "none"), "degraded");
        assert_eq!(classify_node_quality(0.4, 1, "none"), "dead");
        assert_eq!(classify_node_quality(1.0, 1, "firewall"), "degraded");
        assert_eq!(classify_node_quality(1.0, 1, "process_crash"), "dead");
    }

    #[test]
    fn projects_storage_rows_with_node_owned_quality() {
        let rows = vec![
            ("node-a".to_string(), 120, 0.98, 2, "none".to_string()),
            (
                "node-b".to_string(),
                800,
                0.4,
                1,
                "process_crash".to_string(),
            ),
        ];

        assert_eq!(
            project_fleet_stats(rows),
            vec![
                FleetNodeStats {
                    node: "node-a".to_string(),
                    avg_latency_ms: 120,
                    success_rate: 0.98,
                    last_seen_secs: 2,
                    quality: "excellent".to_string(),
                },
                FleetNodeStats {
                    node: "node-b".to_string(),
                    avg_latency_ms: 800,
                    success_rate: 0.4,
                    last_seen_secs: 1,
                    quality: "dead".to_string(),
                },
            ]
        );
    }
}
use serde::Serialize;

/// Transport-neutral fleet statistics projected from runtime observations.
/// Storage adapters may use tuples, but callers should not own quality mapping.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetNodeStats {
    pub node: String,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub last_seen_secs: u64,
    pub quality: String,
}

pub type FleetStatsRow = (String, u64, f64, u64, String);

pub fn project_fleet_stats(rows: impl IntoIterator<Item = FleetStatsRow>) -> Vec<FleetNodeStats> {
    rows.into_iter()
        .map(
            |(node, avg_latency_ms, success_rate, last_seen_secs, failure_reason)| FleetNodeStats {
                node,
                avg_latency_ms,
                success_rate,
                last_seen_secs,
                quality: classify_node_quality(success_rate, last_seen_secs, &failure_reason)
                    .to_string(),
            },
        )
        .collect()
}
