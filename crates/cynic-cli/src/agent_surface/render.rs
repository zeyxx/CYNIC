//! Text projection for the HumanAdmin governance observer.

use super::types::AgentSocReport;

#[allow(dead_code)]
pub(crate) fn render_text(report: &AgentSocReport) {
    println!(
        "cynic agent soc — RBAC read-back (schema {})",
        report.schema
    );
    println!();
    println!(
        "HumanAdmin zero-touch runtime: {}",
        if report.human_admin_role.zero_touch_runtime {
            "OK"
        } else {
            "BLOCKED"
        }
    );
    for never in &report.human_admin_role.never_touches_runtime {
        println!("  - {}", never);
    }
    println!();
    println!("=== Roles ===");
    for (name, entry) in &report.roles {
        println!(
            "  {} [{}] granted={} zero_touch={} via={}",
            name,
            entry.authority_class,
            entry.granted,
            entry.zero_touch_runtime,
            entry.granted_via
        );
    }
    println!();
    println!("=== Domains ===");
    for (name, domain) in &report.domains {
        let obs = if domain.observed { "observed" } else { "NOT YET GRANTED" };
        println!(
            "  {} -> agent={} action={} approval={} [{}] {}",
            name,
            domain.agent_role,
            domain.agent_action,
            domain.human_approval_required,
            domain.mutation_policy,
            obs
        );
    }
    println!();
    if !report.residual_unknowns.is_empty() {
        println!("=== Residual unknowns ===");
        for u in &report.residual_unknowns {
            println!("  - {}", u);
        }
    }
}
