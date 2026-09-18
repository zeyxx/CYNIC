//! Constitutional Court surface — operator-facing projection of the
//! constitutional court meta-agent (Phase A: manual audit only).
//!
//! Thin surface: parse → delegate to executor → emit.
//! Domain policy lives in cynic-protocol.

use std::process::ExitCode;

use clap::Subcommand;

use crate::cli_json::emit_pretty_json;

#[derive(Subcommand)]
pub(crate) enum CourtCommand {
    /// Run constitutional court audit across all surfaces (Phase A).
    Audit {
        /// Comma-separated surfaces to audit. Omit for all.
        #[arg(long, value_parser = parse_surfaces)]
        scope: Option<Vec<cynic_protocol::constitutional_court::SurfaceType>>,
        /// Machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
}

/// Parse a comma-separated list of surfaces into SurfaceType variants.
fn parse_surfaces(
    s: &str,
) -> Result<Vec<cynic_protocol::constitutional_court::SurfaceType>, String> {
    let mut result = Vec::new();
    for part in s.split(',') {
        match part.trim() {
            "contracts" => {
                result.push(cynic_protocol::constitutional_court::SurfaceType::Contracts)
            }
            "runtime_authority" | "runtime-authority" => {
                result.push(cynic_protocol::constitutional_court::SurfaceType::RuntimeAuthority)
            }
            "protocol_types" | "protocol-types" | "protocol" => {
                result.push(cynic_protocol::constitutional_court::SurfaceType::ProtocolTypes)
            }
            "cli_surface" | "cli-surface" | "cli" => {
                result.push(cynic_protocol::constitutional_court::SurfaceType::CliSurface)
            }
            "mcp_tool" | "mcp-tool" | "mcp" => {
                result.push(cynic_protocol::constitutional_court::SurfaceType::McpTool)
            }
            "skill" | "skills" => {
                result.push(cynic_protocol::constitutional_court::SurfaceType::Skill)
            }
            "runtime" => result.push(cynic_protocol::constitutional_court::SurfaceType::Runtime),
            other => {
                return Err(format!(
                    "unknown surface: {other} (valid: contracts, runtime_authority, protocol_types, cli_surface, mcp_tool, skill, runtime)"
                ));
            }
        }
    }
    Ok(result)
}

pub(crate) fn execute_court(command: CourtCommand) -> ExitCode {
    match command {
        CourtCommand::Audit { scope, json } => execute_audit(scope, json),
    }
}

fn execute_audit(
    scope: Option<Vec<cynic_protocol::constitutional_court::SurfaceType>>,
    json: bool,
) -> ExitCode {
    let surfaces =
        scope.unwrap_or_else(|| cynic_protocol::constitutional_court::SurfaceType::ALL.to_vec());

    let mut report =
        cynic_protocol::constitutional_court::CourtAuditReport::new(surfaces.clone(), "manual");

    // Phase A: demonstration report (no real audit logic yet).
    // In Phase C this will be driven by cynic-inference.
    if json {
        let _ = emit_pretty_json(&report);
    } else {
        println!("⚖️ Constitutional Court Audit Report (Phase A)");
        println!(
            "    Surfaces: {}",
            surfaces
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("    Violations found: {}", report.violations.len());
        println!("    Coherence score: {:.1}%", report.coherence_score);
        println!();
        println!("    This is a Phase A stub. Run `cynic dev court-audit` from a Hermes session");
        println!(
            "    to invoke the full LLM-assisted audit via skill `cynic-constitutional-court`."
        );
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_surfaces_valid() {
        let s = "contracts,cli,mcp_tool".to_string();
        let parsed = parse_surfaces(&s).unwrap();
        assert_eq!(parsed.len(), 3);
    }

    #[test]
    fn parse_surfaces_invalid() {
        let s = "invalid_surface".to_string();
        assert!(parse_surfaces(&s).is_err());
    }
}
