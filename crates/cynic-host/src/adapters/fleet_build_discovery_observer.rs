//! Fleet build-discovery observer.

use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use cynic_build::build_discovery::BuildDiscoveryReport;
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
pub const FLEET_BUILD_DISCOVERY_SCHEMA: &str = "cynic.fleet_build_discovery.v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FleetBuildDiscoveryNode {
    Observed {
        node_id: String,
        role: String,
        ip: String,
        report: BuildDiscoveryReport,
    },
    Failed {
        node_id: String,
        role: String,
        ip: String,
        error: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBuildDiscoveryReport {
    pub schema: String,
    pub corpus_unix_s: u64,
    pub total_nodes: usize,
    pub observed_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<FleetBuildDiscoveryNode>,
    pub residual_unknowns: Vec<String>,
    pub budget: Option<cynic_protocol::task_pipeline::EnergyBudget>,
    pub poll_cost_ms: Option<u64>,
}
fn poll_budget_ms() -> u64 {
    std::env::var("CYNIC_POLL_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000)
}
pub struct FleetBuildDiscoveryObserver<R: ArgvRunner> {
    registry_path: String,
    ssh_target_template: String,
    runner: Arc<R>,
}
impl FleetBuildDiscoveryObserver<LocalArgvRunner> {
    pub fn new(p: impl Into<String>) -> Self {
        Self::with_runner(p, LocalArgvRunner)
    }
}
impl<R: ArgvRunner> FleetBuildDiscoveryObserver<R> {
    pub fn with_runner(p: impl Into<String>, runner: R) -> Self {
        Self {
            registry_path: p.into(),
            ssh_target_template: "cynic@{ip}".into(),
            runner: Arc::new(runner),
        }
    }
    pub fn with_ssh_target(mut self, t: impl Into<String>) -> Self {
        self.ssh_target_template = t.into();
        self
    }
    fn ssh_target_for(&self, ip: &str) -> String {
        self.ssh_target_template.replace("{ip}", ip)
    }
    pub fn observe(&self) -> FleetBuildDiscoveryReport {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let start = std::time::Instant::now();
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes = Vec::with_capacity(discovery.nodes.len());
        let (mut obs, mut fail) = (0, 0);
        for d in &discovery.nodes {
            let is_local = hostname == d.node_id || hostname == d.ip;
            let node = if is_local {
                match observe_local_discovery() {
                    Ok(r) => {
                        obs += 1;
                        FleetBuildDiscoveryNode::Observed {
                            node_id: d.node_id.clone(),
                            role: d.role.clone(),
                            ip: d.ip.clone(),
                            report: r,
                        }
                    }
                    Err(e) => {
                        fail += 1;
                        FleetBuildDiscoveryNode::Failed {
                            node_id: d.node_id.clone(),
                            role: d.role.clone(),
                            ip: d.ip.clone(),
                            error: e,
                        }
                    }
                }
            } else {
                let tgt = self.ssh_target_for(&d.ip);
                match observe_remote_discovery(self.runner.clone(), &tgt) {
                    Ok(r) => {
                        obs += 1;
                        FleetBuildDiscoveryNode::Observed {
                            node_id: d.node_id.clone(),
                            role: d.role.clone(),
                            ip: d.ip.clone(),
                            report: r,
                        }
                    }
                    Err(e) => {
                        fail += 1;
                        FleetBuildDiscoveryNode::Failed {
                            node_id: d.node_id.clone(),
                            role: d.role.clone(),
                            ip: d.ip.clone(),
                            error: e,
                        }
                    }
                }
            };
            nodes.push(node);
        }
        let cost = start.elapsed().as_millis() as u64;
        let mut ru = discovery.residual_unknowns;
        if cost > poll_budget_ms() {
            ru.push("poll_cost_exceeded_budget".into());
        }
        FleetBuildDiscoveryReport {
            schema: FLEET_BUILD_DISCOVERY_SCHEMA.into(),
            corpus_unix_s: now,
            total_nodes: discovery.nodes.len(),
            observed_nodes: obs,
            failed_nodes: fail,
            nodes,
            residual_unknowns: ru,
            budget: Some(cynic_protocol::task_pipeline::EnergyBudget {
                wall_time_ms: 5000,
                inference_tokens: None,
                cpu_ms: Some(5000),
                ram_bytes_s: None,
            }),
            poll_cost_ms: Some(cost),
        }
    }
}
fn observe_local_discovery() -> Result<BuildDiscoveryReport, String> {
    use crate::domain::build_discovery::probe::parse_cargo_metadata;
    use crate::process_observation::capture_command;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let json = capture_command("cargo", &["metadata", "--format-version=1", "--no-deps"])
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout)
        .ok_or("cargo_metadata_spawn_failed".to_string())?;
    let tc = capture_command("rustc", &["--version"])
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout.trim().to_string())
        .unwrap_or_else(|| "stable".into());
    let host = capture_command("rustc", &["-vV"])
        .ok()
        .filter(|c| c.success)
        .map(|c| {
            c.stdout
                .lines()
                .find(|l| l.starts_with("host:"))
                .map(|l| l["host:".len()..].trim().to_string())
                .unwrap_or_else(|| "x86_64-unknown-linux-gnu".into())
        })
        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".into());
    parse_cargo_metadata(&json, now, tc, host)
}
fn observe_remote_discovery<R: ArgvRunner>(
    runner: Arc<R>,
    target: &str,
) -> Result<BuildDiscoveryReport, String> {
    use crate::domain::build_discovery::probe::parse_cargo_metadata;
    let run = |cmd: &[&str]| {
        let mut a = vec![
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=yes",
            target,
        ];
        a.extend_from_slice(cmd);
        let o = runner.run(&a);
        if o.success() {
            Ok(o.stdout)
        } else {
            Err(o.spawn_error.unwrap_or(o.stderr))
        }
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let json = run(&["cargo", "metadata", "--format-version=1", "--no-deps"])?;
    let tc = run(&["rustc", "--version"])
        .unwrap_or_else(|_| "stable".into())
        .trim()
        .to_string();
    let vv = run(&["rustc", "-vV"]).unwrap_or_default();
    let host = vv
        .lines()
        .find(|l| l.starts_with("host:"))
        .map(|l| l["host:".len()..].trim().to_string())
        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".into());
    parse_cargo_metadata(&json, now, tc, host)
}
