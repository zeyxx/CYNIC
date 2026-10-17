//! Fleet arms: thin I/O adapter over the `cynic-fleet` domain crate.
//!
//! This module only reads files (registry, coordination log) and maps them
//! onto domain inputs. Projection (`assemble`), HTML rendering, and the live
//! TUI all live in `cynic-fleet`. No domain logic here by construction.

use clap::{Args, Subcommand};
use std::process::ExitCode;

use cynic_fleet::{LogEntry, NodeEntry};

use crate::coordination_log::{default_coordination_log_path, read_entries};

/// Fleet-wide read-only projections.
#[derive(Debug, Clone, Subcommand)]
pub enum NodeFleetCommand {
    /// Render the fleet dashboard (registry + coordination log) as one HTML file.
    Report(NodeFleetReportArgs),
    /// Watch the fleet dashboard live (fullscreen TUI; falls back to text when piped).
    Live {
        /// Registry path (else /etc/cynic/registry.json, else the repo registry).
        #[arg(long)]
        registry: Option<String>,
        /// Coordination log path (else CYNIC_COORDINATION_LOG, else local share).
        #[arg(long)]
        log: Option<String>,
    },
}

#[derive(Debug, Clone, Args)]
pub struct NodeFleetReportArgs {
    /// Registry path (else /etc/cynic/registry.json, else the repo registry).
    #[arg(long)]
    pub registry: Option<String>,
    /// Coordination log path (else CYNIC_COORDINATION_LOG, else local share).
    #[arg(long)]
    pub log: Option<String>,
    /// Output HTML path.
    #[arg(long, default_value = "fleet.html")]
    pub out: String,
    /// Also print the summary as JSON.
    #[arg(long, default_value = "false")]
    pub json: bool,
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn resolve_registry(explicit: Option<String>) -> Result<String, String> {
    if let Some(path) = explicit {
        if !path.trim().is_empty() {
            return Ok(path);
        }
    }
    let system = "/etc/cynic/registry.json";
    if std::path::Path::new(system).exists() {
        return Ok(system.to_string());
    }
    Ok("infra/registry.json".to_string())
}

/// Read both inputs and build the domain projection. The single I/O point
/// for both surfaces.
pub(crate) fn load_report(
    registry: Option<String>,
    log: Option<String>,
) -> Result<cynic_fleet::FleetReport, String> {
    let registry_path = resolve_registry(registry)?;
    let bytes =
        std::fs::read(&registry_path).map_err(|e| format!("read registry {registry_path}: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse registry: {e}"))?;
    let nodes: Vec<NodeEntry> = value
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter()
                .map(|n| {
                    let enrollment = n.get("enrollment");
                    NodeEntry {
                        id: n
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        role: n
                            .get("role")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unassigned")
                            .to_string(),
                        status: n
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        certificate_id: enrollment
                            .and_then(|e| e.get("certificate_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("—")
                            .to_string(),
                        authority_id: enrollment
                            .and_then(|e| e.get("authority_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("—")
                            .to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let log_path = match log {
        Some(path) if !path.trim().is_empty() => path,
        _ => default_coordination_log_path()?,
    };
    let (entries, _) = read_entries(&log_path, None, usize::MAX);
    let log_entries: Vec<LogEntry> = entries
        .into_iter()
        .map(|e| LogEntry {
            ts_unix_s: e.ts_unix_s,
            agent: e.agent,
            kind: e.kind,
            body: e.body,
        })
        .collect();

    Ok(cynic_fleet::assemble(
        &registry_path,
        now_unix_s(),
        nodes,
        probe_services(&value),
        log_entries,
    ))
}

/// Live service discovery (read-only): for every registry node with a
/// known IP, TCP-probe declared service ports plus the usual inference
/// set, then label each open port via `/v1/models` (or `/health`).
/// Ghosts surface as `declared: false` rows — observed but unregistered.
fn probe_services(registry: &serde_json::Value) -> Vec<cynic_fleet::FleetServiceRow> {
    const CANDIDATE_PORTS: [u16; 7] = [8080, 8081, 8082, 8085, 8087, 8088, 11434];
    let mut rows = Vec::new();
    let nodes = registry
        .get("nodes")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default();
    for node in &nodes {
        let node_id = node
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let mut hosts: Vec<String> = Vec::new();
        if let Some(network) = node.get("network").and_then(|n| n.as_object()) {
            for key in ["service", "lan", "admin"] {
                if let Some(ip) = network.get(key).and_then(|v| v.as_str()) {
                    if !ip.is_empty() && !hosts.contains(&ip.to_string()) {
                        hosts.push(ip.to_string());
                    }
                }
            }
        }
        let mut declared_ports: Vec<u16> = Vec::new();
        if let Some(services) = node.get("services").and_then(|s| s.as_array()) {
            for service in services {
                if let Some(port) = service.get("port").and_then(|p| p.as_u64()) {
                    let port = port as u16;
                    if !declared_ports.contains(&port) {
                        declared_ports.push(port);
                    }
                }
            }
        }
        for host in &hosts {
            let mut ports: Vec<u16> = declared_ports.clone();
            for port in CANDIDATE_PORTS {
                if !ports.contains(&port) {
                    ports.push(port);
                }
            }
            for port in ports {
                if let Some((label, model)) = probe_listener(host, port) {
                    rows.push(cynic_fleet::FleetServiceRow {
                        node_id: node_id.clone(),
                        host: host.clone(),
                        port,
                        label,
                        model,
                        declared: declared_ports.contains(&port),
                    });
                }
            }
        }
    }
    rows
}

/// TCP connect (600 ms) then minimal HTTP GET. Returns the label and the
/// served model name, or nothing when the port is closed or silent.
fn probe_listener(host: &str, port: u16) -> Option<(String, String)> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    let addr: SocketAddr = format!("{host}:{port}").to_socket_addrs().ok()?.next()?;
    let mut stream =
        TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(600)).ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(1200)))
        .ok()?;
    let request = format!("GET /v1/models HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut body = String::new();
    stream.read_to_string(&mut body).ok()?;
    let payload = body.split("\r\n\r\n").nth(1).unwrap_or("");
    let model = payload
        .split("\"model\"")
        .nth(1)
        .and_then(|rest| rest.split('"').nth(1))
        .unwrap_or("")
        .to_string();
    if model.is_empty() && !payload.contains("\"models\"") {
        return probe_health(host, port);
    }
    let short = model.rsplit('/').next().unwrap_or(&model).to_string();
    let label = if short.to_lowercase().contains("embed") {
        "embedding".to_string()
    } else {
        "chat".to_string()
    };
    Some((label, short))
}

/// Fallback label for listeners without an OpenAI model list.
fn probe_health(host: &str, port: u16) -> Option<(String, String)> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    let addr: SocketAddr = format!("{host}:{port}").to_socket_addrs().ok()?.next()?;
    let mut stream =
        TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(600)).ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(1200)))
        .ok()?;
    let request = format!("GET /health HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut body = String::new();
    stream.read_to_string(&mut body).ok()?;
    if body.starts_with("HTTP/1.") && body.contains("200") {
        Some(("http".to_string(), String::new()))
    } else {
        Some(("tcp".to_string(), String::new()))
    }
}

pub fn execute_fleet(command: NodeFleetCommand) -> Result<ExitCode, String> {
    match command {
        NodeFleetCommand::Report(args) => {
            let report = load_report(args.registry, args.log)?;
            let html = cynic_fleet::render_html(&report);
            std::fs::write(&args.out, &html).map_err(|e| format!("write {}: {e}", args.out))?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_default()
                );
            } else {
                println!("wrote {}", args.out);
            }
            Ok(ExitCode::SUCCESS)
        }
        NodeFleetCommand::Live { registry, log } => Ok(cynic_tui::run(&move || {
            let report = load_report(registry.clone(), log.clone())?;
            Ok(fleet_dashboard(&report))
        })),
    }
}

/// Build-runners panel composed from the same registry projection as
/// `cynic build runners`. Read-only; a resolution failure renders as one
/// line instead of breaking the dashboard.
fn runners_panel(registry_path: &str) -> cynic_tui::Panel {
    let rows = match cynic_host::registry::resolve::list_runner_nodes(Some(registry_path)) {
        Ok(runners) => runners
            .iter()
            .map(|runner| {
                vec![
                    if runner.default {
                        format!("{} [default]", runner.node_id)
                    } else {
                        runner.node_id.clone()
                    },
                    runner.role.clone(),
                    runner.ssh_target.clone(),
                ]
            })
            .collect(),
        Err(error) => vec![vec!["unresolvable".to_string(), error]],
    };
    cynic_tui::Panel::Table(cynic_tui::TablePanel {
        title: "build runners".to_string(),
        headers: ["id", "role", "ssh"]
            .iter()
            .map(|h| h.to_string())
            .collect(),
        rows,
    })
}

/// Map the fleet projection onto the generic dashboard model. The only
/// fleet-specific TUI code in the workspace; everything else is
/// the shared `cynic-tui` kit.
fn fleet_dashboard(report: &cynic_fleet::FleetReport) -> cynic_tui::Dashboard {
    let mut dashboard = cynic_tui::Dashboard::new(format!(
        "CYNIC fleet — {} node(s) · {} open debt(s) · {}",
        report.nodes.len(),
        report.debts.len(),
        report.registry_path
    ));
    dashboard.push(cynic_tui::Panel::Table(cynic_tui::TablePanel {
        title: "nodes".to_string(),
        headers: ["id", "role", "status", "cert", "authority"]
            .iter()
            .map(|h| h.to_string())
            .collect(),
        rows: report
            .nodes
            .iter()
            .map(|n| {
                vec![
                    n.id.clone(),
                    n.role.clone(),
                    n.status.clone(),
                    cynic_fleet::short_hash(&n.certificate_id),
                    n.authority_id.clone(),
                ]
            })
            .collect(),
    }));
    dashboard.push(runners_panel(&report.registry_path));
    dashboard.push(cynic_tui::Panel::Table(cynic_tui::TablePanel {
        title: "observed services".to_string(),
        headers: ["node", "listener", "label", "model", "registry"]
            .iter()
            .map(|h| h.to_string())
            .collect(),
        rows: report
            .services
            .iter()
            .map(|s| {
                vec![
                    s.node_id.clone(),
                    format!("{}:{}", s.host, s.port),
                    s.label.clone(),
                    s.model.clone(),
                    if s.declared {
                        "declared".to_string()
                    } else {
                        "GHOST".to_string()
                    },
                ]
            })
            .collect(),
    }));
    dashboard.push(cynic_tui::Panel::Text(cynic_tui::TextPanel {
        title: "open debts".to_string(),
        lines: report
            .debts
            .iter()
            .map(|d| {
                format!(
                    "[{}] {}",
                    d.agent,
                    d.body.chars().take(120).collect::<String>()
                )
            })
            .collect(),
    }));
    dashboard
}
