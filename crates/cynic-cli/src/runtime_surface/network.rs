use std::process::ExitCode;

use cynic_host::adapters::local_network::LocalNetworkAdapter;
use cynic_host::domain::network::NetworkObservationPort;
use cynic_host::domain::network::types::NetworkReport;

pub fn execute_network(json: bool) -> ExitCode {
    // The port view is infallible: partial failures land in report.unknowns.
    let report: NetworkReport = LocalNetworkAdapter::new().observe_network();
    if json {
        crate::cli_json::emit_pretty_json(&report);
    } else {
        print_network_report(&report);
    }
    ExitCode::SUCCESS
}

fn print_network_report(r: &NetworkReport) {
    println!("── Network Report ───────────────────────────");
    println!("schema: {}", r.schema);
    println!("── Physical Interfaces (ip -j link, L1) ──");
    if r.interfaces.is_empty() {
        println!("  (none observed)");
    }
    for iface in &r.interfaces {
        println!(
            "  {} — mac: {:?}, mtu: {}, state: {}, speed: {} Mbps, duplex: {}",
            iface.name,
            iface.mac,
            iface.mtu,
            iface.state,
            iface.speed_mbps.map_or("?".to_string(), |s| s.to_string()),
            iface.duplex.as_deref().unwrap_or("unknown"),
        );
    }
    println!("── Addresses (ip -j addr, L2) ──");
    if r.addresses.is_empty() {
        println!("  (none observed)");
    }
    for a in &r.addresses {
        println!("  {}: {}", a.iface, a.address);
    }
    println!("── Routes (ip -j route, L2) ──");
    if r.routes.is_empty() {
        println!("  (none observed)");
    }
    for rt in &r.routes {
        println!(
        "  dst={} gateway={} dev={}",
        rt.dst,
        rt.gateway.as_deref().unwrap_or("none"),
        rt.device
    );
    }
    println!("── Usage (/proc/net/dev, L3) ──");
    for u in &r.usage {
        println!(
            "  {}: rx_bytes={} rx_pkts={} tx_bytes={} tx_pkts={}",
            u.iface, u.rx_bytes, u.rx_packets, u.tx_bytes, u.tx_packets
        );
    }
    println!("── Pressure / Fidelity (drops+errors, L4) ──");
    for p in &r.pressure {
        println!(
            "  {}: rx_drops={} tx_drops={} rx_errs={} tx_errs={} saturated={}",
            p.iface, p.rx_drop, p.tx_drop, p.rx_errs, p.tx_errs, p.saturated
        );
    }
    if !r.unknowns.is_empty() {
        println!("── Residual unknowns ──");
        for u in &r.unknowns {
            println!("  ⚠ {u}");
        }
    }
}
