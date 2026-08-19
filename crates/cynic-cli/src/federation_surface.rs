#![allow(dead_code)]
use clap::Parser;
use clap::Subcommand;

use cynic_protocol::federation::FederationReconciliation;

/// Federation CLI command — subcommands for federation operations.
#[derive(Debug, Subcommand)]
pub(crate) enum FederationCommand {
    /// Show the current federation reconciliation status.
    Status(FederationStatusCommand),
}

/// CLI arguments for `cynic federation status`.
#[derive(Debug, Parser)]
pub(crate) struct FederationStatusCommand {
    /// Output as JSON (machine-readable).
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

impl FederationCommand {
    /// Execute the federation command.
    pub(crate) fn execute(&self, reconciliation: &FederationReconciliation) -> Result<(), String> {
        match self {
            FederationCommand::Status(cmd) => cmd.execute(reconciliation),
        }
    }
}

impl FederationStatusCommand {
    /// Execute the federation status command.
    pub(crate) fn execute(&self, reconciliation: &FederationReconciliation) -> Result<(), String> {
        if self.json {
            let json = serde_json::to_string_pretty(reconciliation)
                .map_err(|e| format!("failed to serialize reconciliation: {e}"))?;
            println!("{json}");
        } else {
            self.render_human(reconciliation);
        }
        Ok(())
    }

    fn render_human(&self, reconciliation: &FederationReconciliation) {
        let label = reconciliation.label();
        let mode = reconciliation.mode();
        let capabilities = reconciliation.retained_capabilities();
        let unknowns = reconciliation.residual_unknowns();

        println!("Federation Status");
        println!("=================");
        println!("Mode:         {label} ({mode:?})");

        println!(
            "Capabilities: {}",
            if capabilities.is_empty() {
                "(none)".to_string()
            } else {
                capabilities.join(", ")
            }
        );

        if !unknowns.is_empty() {
            println!("Unknowns:");
            for u in &unknowns {
                println!("  - {u}");
            }
        }

        match reconciliation {
            FederationReconciliation::Coordinated {
                node_control_plan, ..
            } => {
                println!("Kernel plan:  {}", node_control_plan.status);
                println!("Connection:   {:?}", node_control_plan.connection);
            }
            FederationReconciliation::Autonomous { assumed_roles, .. } => {
                if assumed_roles.is_empty() {
                    println!("Roles:        (none elected)");
                } else {
                    println!("Roles:");
                    for role in assumed_roles {
                        println!(
                            "  - {} ({})",
                            role.assumed_role.as_deref().unwrap_or("?"),
                            role.reasoning
                        );
                    }
                }
            }
            FederationReconciliation::Promoted { takeover, .. } => {
                println!("Takeover:     {}", takeover.status);
                println!("Fencing token: {}", takeover.fencing_token);
            }
            FederationReconciliation::Degraded { .. } => {
                println!("Degraded mode — identity and health only.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::autonomy::{AutonomyDecision, AutonomyMode, RetainedCapabilities};

    fn autonomy_decision(mode: AutonomyMode) -> AutonomyDecision {
        AutonomyDecision {
            schema: "cynic.local_autonomy_decision.v1".to_string(),
            mode,
            retained_capabilities: RetainedCapabilities::for_mode(mode),
            blockers: vec![],
            evidence_refs: vec![],
        }
    }

    #[test]
    fn human_renders_coordinated() {
        let reconciliation = FederationReconciliation::Coordinated {
            node_control_plan: cynic_protocol::node_control::NodeControlPlan {
                schema: "cynic.node_control_plan.v2".to_string(),
                status: "converged".to_string(),
                node_id: "node-a".to_string(),
                role: "build_runner".to_string(),
                desired_generation: 1,
                observed_generation: Some(1),
                connection: cynic_protocol::node_control::NodeControlConnectionState::Connected,
                decision: cynic_protocol::node_control::NodeControlDecision::Converged,
                execution: None,
                evidence_refs: vec![],
                unknowns: vec![],
                epoch: None,
            },
            autonomy: autonomy_decision(AutonomyMode::Coordinated),
        };
        let cmd = FederationCommand::Status(FederationStatusCommand { json: false });
        cmd.execute(&reconciliation).unwrap();
    }

    #[test]
    fn json_renders_all_status_variants() {
        let cases = vec![
            FederationReconciliation::Coordinated {
                node_control_plan: cynic_protocol::node_control::NodeControlPlan {
                    schema: "cynic.node_control_plan.v2".to_string(),
                    status: "converged".to_string(),
                    node_id: "node-a".to_string(),
                    role: "build_runner".to_string(),
                    desired_generation: 1,
                    observed_generation: Some(1),
                    connection: cynic_protocol::node_control::NodeControlConnectionState::Connected,
                    decision: cynic_protocol::node_control::NodeControlDecision::Converged,
                    execution: None,
                    evidence_refs: vec![],
                    unknowns: vec![],
                    epoch: None,
                },
                autonomy: autonomy_decision(AutonomyMode::Coordinated),
            },
            FederationReconciliation::Degraded {
                autonomy: autonomy_decision(AutonomyMode::Degraded),
                retained_capabilities: vec!["identity".to_string(), "health".to_string()],
            },
        ];

        for reconciliation in cases {
            let cmd = FederationCommand::Status(FederationStatusCommand { json: true });
            cmd.execute(&reconciliation).unwrap();
        }
    }
}
