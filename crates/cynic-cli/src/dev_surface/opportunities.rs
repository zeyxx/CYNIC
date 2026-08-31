//! Handler and projection for `cynic dev opportunities`.
//!
//! Conforms to Library-First Invariant (§472) and Anti-Monolith rules:
//! thin facade only, domain logic lives in `cynic_host::organic_opportunity`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cynic_host::organic_opportunity::{
    OpportunityDecisionStore, collect_default_organic_opportunities,
};
use cynic_protocol::organic_opportunity::{
    OPPORTUNITY_DECISION_SCHEMA, OperatorDisposition, OpportunityDecisionRecord,
    OrganicOpportunityReport, ValueFrame,
};

use crate::emit_json;

pub(crate) fn execute_opportunities(
    json: bool,
    frame: Option<String>,
    output: Option<PathBuf>,
    decide: Option<String>,
    action: Option<String>,
    reason: Option<String>,
) -> ExitCode {
    let now_unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let workspace_root = Path::new(".");

    // Phase 4: Handle explicit operator decision recording if requested
    if let Some(target_id) = decide {
        return execute_record_decision(
            &target_id,
            action.as_deref(),
            reason.as_deref(),
            workspace_root,
            now_unix_s,
            json,
        );
    }

    let custom_frame = frame.map(|frame_id| ValueFrame {
        schema: cynic_protocol::organic_opportunity::ORGANIC_VALUE_FRAME_SCHEMA.to_string(),
        frame_id: frame_id.clone(),
        description: format!("Custom value frame '{}'", frame_id),
        tie_break: vec![
            cynic_protocol::organic_opportunity::ValueDimension::SafetyBlock,
            cynic_protocol::organic_opportunity::ValueDimension::UnlockFanOut,
            cynic_protocol::organic_opportunity::ValueDimension::Freshness,
            cynic_protocol::organic_opportunity::ValueDimension::ConfidenceHigh,
            cynic_protocol::organic_opportunity::ValueDimension::EnergyLow,
        ],
    });

    let report = collect_default_organic_opportunities(workspace_root, custom_frame, now_unix_s);

    if let Some(dest) = output {
        if let Ok(json_str) = serde_json::to_string_pretty(&report) {
            let _ = std::fs::write(dest, json_str);
        }
    }

    if json {
        emit_json(&report, false)
    } else {
        render_human_opportunities(&report);
        ExitCode::SUCCESS
    }
}

fn execute_record_decision(
    target_id: &str,
    action: Option<&str>,
    reason: Option<&str>,
    workspace_root: &Path,
    now_unix_s: u64,
    json: bool,
) -> ExitCode {
    let reason_str = reason
        .unwrap_or("operator decision recorded via CLI")
        .to_string();

    let disposition = match action.unwrap_or("defer").to_lowercase().as_str() {
        "accept" | "accepted" => OperatorDisposition::Accepted {
            at_unix_s: now_unix_s,
            reason: reason_str.clone(),
        },
        "reject" | "rejected" => OperatorDisposition::Rejected {
            at_unix_s: now_unix_s,
            reason: reason_str.clone(),
        },
        "defer" | "deferred" => OperatorDisposition::Deferred {
            at_unix_s: now_unix_s,
            reason: reason_str.clone(),
        },
        other => {
            eprintln!(
                "error: unknown decision action '{}'. Valid actions: accept | defer | reject",
                other
            );
            return ExitCode::FAILURE;
        }
    };

    let decision_id = format!(
        "dec:{}:{}:{}",
        sanitize_id(target_id),
        action.unwrap_or("defer"),
        now_unix_s
    );

    let record = OpportunityDecisionRecord {
        schema: OPPORTUNITY_DECISION_SCHEMA.to_string(),
        decision_id: decision_id.clone(),
        opportunity_digest: target_id.to_string(),
        operator_id: "operator:cli".to_string(),
        decided_at_unix_s: now_unix_s,
        disposition,
        reason: reason_str,
        action_intent: None,
    };

    let store = OpportunityDecisionStore::new(workspace_root.join(".hermes").join("decisions"));
    match store.record_decision(&record) {
        Ok(path) => {
            if json {
                emit_json(&record, false)
            } else {
                println!("Recorded operator decision: {}", record.decision_id);
                println!("Target:     {}", record.opportunity_digest);
                println!("Saved to:   {}", path.display());
                ExitCode::SUCCESS
            }
        }
        Err(err) => crate::fail(err),
    }
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn render_human_opportunities(report: &OrganicOpportunityReport) {
    println!("CYNIC Organic Opportunities Compass");
    println!("===================================");
    println!(
        "Frame:       {} ({})",
        report.frame.frame_id, report.frame.description
    );
    println!("Total opps:  {}", report.opportunities.len());

    if !report.unavailable_sources.is_empty() {
        println!("Unavailable: {:?}", report.unavailable_sources);
    }
    println!();

    if report.opportunities.is_empty() {
        println!("No organic opportunities currently observed.");
        return;
    }

    println!(
        "{:<4} {:<30} {:<18} {:<15} {:<10} {:<12} {}",
        "#", "SCOPE", "CLASS", "NEXT STEP", "CONFIDENCE", "DISPOSITION", "EXPECTED UNLOCK"
    );
    println!("{:-<115}", "");

    for (idx, opp) in report.opportunities.iter().enumerate() {
        let disp_str = match &opp.operator_disposition {
            OperatorDisposition::Unreviewed => "unreviewed".to_string(),
            OperatorDisposition::Accepted { .. } => "accepted".to_string(),
            OperatorDisposition::Deferred { .. } => "deferred".to_string(),
            OperatorDisposition::Rejected { .. } => "rejected".to_string(),
        };

        println!(
            "{:<4} {:<30} {:<18} {:<15} {:<10.2} {:<12} {}",
            idx + 1,
            truncate(&opp.scope, 28),
            format!("{:?}", opp.class),
            format!("{:?}", opp.smallest_next_step),
            opp.confidence,
            disp_str,
            opp.expected_unlock
        );
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", &s[..max_chars.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_helper_bounds_strings() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a_very_long_string_here", 10), "a_very_...");
    }

    #[test]
    fn sanitize_id_cleans_non_alphanumeric() {
        assert_eq!(
            sanitize_id("opp:session_friction:1234"),
            "opp_session_friction_1234"
        );
        assert_eq!(sanitize_id("debt/Sec1"), "debt_Sec1");
    }
}
