//! Handler and projection for `cynic dev opportunities`.
//!
//! Conforms to Library-First Invariant (§472) and Anti-Monolith rules:
//! thin facade only, domain logic lives in `cynic_host::organic_opportunity`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cynic_host::organic_opportunity::collect_default_organic_opportunities;
use cynic_protocol::organic_opportunity::{OrganicOpportunityReport, ValueFrame};

use crate::emit_json;

pub(crate) fn execute_opportunities(
    json: bool,
    frame: Option<String>,
    output: Option<PathBuf>,
) -> ExitCode {
    let now_unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let workspace_root = Path::new(".");

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
        "{:<4} {:<30} {:<18} {:<15} {:<10} {}",
        "#", "SCOPE", "CLASS", "NEXT STEP", "CONFIDENCE", "EXPECTED UNLOCK"
    );
    println!("{:-<100}", "");

    for (idx, opp) in report.opportunities.iter().enumerate() {
        println!(
            "{:<4} {:<30} {:<18} {:<15} {:<10.2} {}",
            idx + 1,
            truncate(&opp.scope, 28),
            format!("{:?}", opp.class),
            format!("{:?}", opp.smallest_next_step),
            opp.confidence,
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
}
