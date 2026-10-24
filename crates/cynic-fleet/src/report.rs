//! Pure fleet projection: assemble rows from adapter-supplied entries and
//! render the single-file HTML dashboard. No filesystem, no clock beyond
//! `generated_unix_s` passed in by the caller.

use crate::types::{FleetLogRow, FleetNodeRow, FleetReport, FleetServiceRow, LogEntry, NodeEntry};

pub const FLEET_REPORT_SCHEMA: &str = "cynic.fleet.report.v1";

/// Build the projection. Debts keep chronological order; infos too (the
/// surfaces pick how many recent infos to show).
pub fn assemble(
    registry_path: &str,
    generated_unix_s: u64,
    nodes: Vec<NodeEntry>,
    services: Vec<FleetServiceRow>,
    entries: Vec<LogEntry>,
) -> FleetReport {
    let rows: Vec<FleetNodeRow> = nodes
        .into_iter()
        .map(|n| FleetNodeRow {
            id: n.id,
            role: n.role,
            status: n.status,
            certificate_id: n.certificate_id,
            authority_id: n.authority_id,
        })
        .collect();
    let mut debts: Vec<FleetLogRow> = Vec::new();
    let mut infos: Vec<FleetLogRow> = Vec::new();
    for entry in entries {
        let row = FleetLogRow {
            ts_unix_s: entry.ts_unix_s,
            agent: entry.agent,
            kind: entry.kind.clone(),
            body: entry.body,
        };
        if entry.kind == "debt" {
            debts.push(row);
        } else {
            infos.push(row);
        }
    }
    debts.sort_by_key(|r| r.ts_unix_s);
    infos.sort_by_key(|r| r.ts_unix_s);
    FleetReport {
        schema: FLEET_REPORT_SCHEMA.to_string(),
        generated_unix_s,
        registry_path: registry_path.to_string(),
        nodes: rows,
        services,
        debts,
        infos,
    }
}

impl cynic_protocol::DataReport for FleetReport {
    fn schema_id(&self) -> &'static str {
        FLEET_REPORT_SCHEMA
    }

    fn to_export_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}

/// Short cert hash for dense surfaces (TUI table, HTML code cell).
pub fn short_hash(raw: &str) -> String {
    let hex: String = raw
        .chars()
        .skip_while(|c| *c != ':')
        .skip(1)
        .take(8)
        .collect();
    if hex.is_empty() {
        raw.chars().take(12).collect()
    } else {
        hex
    }
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render the standalone HTML dashboard. No JS, no network, inline CSS.
fn node_rows(nodes: &[FleetNodeRow]) -> String {
    let mut rows = String::new();
    for node in nodes {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>\n",
            escape_html(&node.id),
            escape_html(&node.role),
            escape_html(&node.status),
            escape_html(&short_hash(&node.certificate_id)),
            escape_html(&node.authority_id),
        ));
    }
    rows
}

fn service_rows(services: &[FleetServiceRow]) -> String {
    let mut rows = String::new();
    for service in services {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}:{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape_html(&service.node_id),
            escape_html(&service.host),
            service.port,
            escape_html(&service.label),
            escape_html(&service.model),
            if service.declared {
                "declared"
            } else {
                "GHOST"
            },
        ));
    }
    rows
}

fn log_rows(entries: &[FleetLogRow]) -> String {
    let mut rows = String::new();
    for entry in entries {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            entry.ts_unix_s,
            escape_html(&entry.agent),
            escape_html(&entry.body),
        ));
    }
    rows
}

pub fn render_html(report: &FleetReport) -> String {
    let node_rows = node_rows(&report.nodes);
    let service_rows = service_rows(&report.services);
    let debt_rows = log_rows(&report.debts);
    let info_rows: String = {
        log_rows(
            &report
                .infos
                .iter()
                .rev()
                .take(10)
                .cloned()
                .collect::<Vec<_>>(),
        )
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>CYNIC fleet dashboard</title>\n<style>\nbody{{font-family:monospace;max-width:72rem;margin:2rem auto;padding:0 1rem;background:#0d1117;color:#c9d1d9}}\ntable{{border-collapse:collapse;width:100%;margin-bottom:2rem}}\ntd,th{{border:1px solid #30363d;padding:.4rem .6rem;text-align:left}}\nth{{background:#161b22}}\ncode{{color:#7ee787}}\nh1{{color:#58a6ff}}\n</style>\n</head>\n<body>\n<h1>CYNIC fleet dashboard</h1>\n<p>generated {} from {}</p>\n<h2>Nodes ({})</h2>\n<table><tr><th>id</th><th>role</th><th>status</th><th>cert</th><th>authority</th></tr>\n{}</table>\n<h2>Observed services ({})</h2>\n<table><tr><th>node</th><th>listener</th><th>label</th><th>model</th><th>registry</th></tr>\n{}</table>\n<h2>Open debts ({})</h2>\n<table><tr><th>ts</th><th>agent</th><th>body</th></tr>\n{}</table>\n<h2>Latest activity</h2>\n<table><tr><th>ts</th><th>agent</th><th>body</th></tr>\n{}</table>\n</body>\n</html>\n",
        report.generated_unix_s,
        escape_html(&report.registry_path),
        report.nodes.len(),
        node_rows,
        report.services.len(),
        service_rows,
        report.debts.len(),
        debt_rows,
        info_rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LogEntry, NodeEntry};

    fn sample() -> FleetReport {
        assemble(
            "reg.json",
            1_000_000,
            vec![NodeEntry {
                id: "n1".into(),
                role: "agent_host".into(),
                status: "observed".into(),
                certificate_id: "sha256:abcdef12".into(),
                authority_id: "ca".into(),
            }],
            vec![FleetServiceRow {
                node_id: "n1".into(),
                host: "10.0.0.9".into(),
                port: 8080,
                label: "embedding".into(),
                model: "m.gguf".into(),
                declared: false,
            }],
            vec![
                LogEntry {
                    ts_unix_s: 3,
                    agent: "a".into(),
                    kind: "debt".into(),
                    body: "second".into(),
                },
                LogEntry {
                    ts_unix_s: 1,
                    agent: "a".into(),
                    kind: "debt".into(),
                    body: "first".into(),
                },
                LogEntry {
                    ts_unix_s: 2,
                    agent: "b".into(),
                    kind: "info".into(),
                    body: "<note>&".into(),
                },
            ],
        )
    }

    #[test]
    fn debts_sort_chronologically_and_infos_split() {
        let report = sample();
        assert_eq!(report.nodes.len(), 1);
        assert_eq!(report.debts.len(), 2);
        assert_eq!(report.debts[0].body, "first");
        assert_eq!(report.infos.len(), 1);
    }

    #[test]
    fn html_escapes_and_shows_short_cert() {
        let html = render_html(&sample());
        assert!(html.contains("&lt;note&gt;&amp;"));
        assert!(html.contains("abcdef12"));
    }

    #[test]
    fn short_hash_without_colon_falls_back() {
        assert_eq!(short_hash("plain"), "plain");
        assert_eq!(short_hash("sha256:abcdef1299"), "abcdef12");
    }

    #[test]
    fn test_fleet_report_data_report_contract() {
        use cynic_protocol::DataReport;
        let report = sample();
        assert_eq!(report.schema_id(), FLEET_REPORT_SCHEMA);
        let json = report.to_export_json().expect("export json");
        assert!(json.contains("cynic.fleet.report.v1"));
        let parsed: FleetReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema, FLEET_REPORT_SCHEMA);
        assert_eq!(parsed.nodes.len(), 1);
    }
}
