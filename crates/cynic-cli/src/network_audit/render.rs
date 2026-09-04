use std::process::ExitCode;

use crate::network_audit::types::NetworkAuditReport;

pub(crate) fn render_text(report: &NetworkAuditReport) -> ExitCode {
    println!("Network Audit Report");
    println!("====================");
    println!("Status: {}", report.status);
    println!(
        "Nodes: {} total, {} healthy, {} degraded, {} failed",
        report.total_nodes, report.healthy_nodes, report.degraded_nodes, report.failed_nodes
    );
    println!();

    println!("DNS ({})", report.dns.zone);
    println!(
        "  Resolved: {}/{}",
        report.dns.resolved_records, report.dns.total_records
    );
    for record in &report.dns.records {
        let icon = if record.status == "ok" { "✓" } else { "✗" };
        println!("  {} {} -> {}", icon, record.name, record.declared_address);
    }
    println!();

    println!("WireGuard ({})", report.wireguard.interface);
    println!(
        "  Peers: {} ok, {} stale, {} dead",
        report.wireguard.ok_peers, report.wireguard.stale_peers, report.wireguard.dead_peers
    );
    for peer in &report.wireguard.peers {
        let icon = match peer.status.as_str() {
            "ok" => "✓",
            "stale" => "~",
            _ => "✗",
        };
        println!(
            "  {} {}",
            icon,
            peer.public_key.chars().take(20).collect::<String>()
        );
    }
    println!();

    println!("Nodes");
    for node in &report.nodes {
        println!("  {} ({}) - {}", node.node_id, node.role, node.outcome);
        for svc in &node.services {
            let icon = if svc.reachable { "✓" } else { "✗" };
            println!("    {} {} -> {}", icon, svc.service, svc.declared_endpoint);
        }
    }
    println!();

    if !report.alerts.is_empty() {
        println!("Alerts");
        for alert in &report.alerts {
            println!(
                "  [{:?}] {}: {}",
                alert.level, alert.component, alert.message
            );
        }
    }

    if !report.residual_unknowns.is_empty() {
        println!("Residual Unknowns: {:?}", report.residual_unknowns);
    }

    if report.status == "critical" {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
