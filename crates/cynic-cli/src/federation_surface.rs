use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use clap::Parser;
use clap::Subcommand;
use cynic_node_protocol::federation::FederationReconciliation;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

/// Federation CLI command — subcommands for federation operations.
#[derive(Debug, Subcommand)]
pub(crate) enum FederationCommand {
    /// Show the current federation reconciliation status.
    Status(FederationStatusCommand),
    /// Network enrollment operations (read-only probes for now).
    #[command(subcommand)]
    Enroll(EnrollCommand),
}

/// Enrollment subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum EnrollCommand {
    /// Run the per-plane enrollment probes and qualify stub/candidate/observed.
    Status(EnrollStatusCommand),
}

/// CLI arguments for `cynic federation status`.
#[derive(Debug, Parser)]
pub(crate) struct FederationStatusCommand {
    /// Output as JSON (machine-readable).
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// CLI arguments for `cynic federation enroll status`.
#[derive(Debug, Parser)]
pub(crate) struct EnrollStatusCommand {
    /// Output as JSON (machine-readable).
    #[arg(long, default_value_t = false)]
    pub json: bool,
    /// Registry path (infra/registry.json).
    #[arg(long, default_value = "infra/registry.json")]
    pub registry: Option<String>,
    /// Target node id in the registry (defaults to local-kernel or first node).
    #[arg(long)]
    pub node: Option<String>,
    /// WireGuard interface to probe.
    #[arg(long, default_value = "wg0")]
    pub interface: String,
    /// Egress probe URL.
    #[arg(long, default_value = "https://github.com")]
    pub egress_url: String,
    /// Skip the egress probe.
    #[arg(long, default_value_t = false)]
    pub no_egress: bool,
    /// Skip the SSH hostname probe on the admin plane.
    #[arg(long, default_value_t = false)]
    pub no_ssh: bool,
}

pub(crate) fn execute_federation(command: FederationCommand) -> ExitCode {
    match command {
        FederationCommand::Status(cmd) => {
            let registry_path = resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let runtime = cynic_host::federation_runtime::FederationRuntime::live(
                registry_path.to_string_lossy().to_string(),
                "local-kernel",
            );
            let reconciliation = runtime.reconcile();
            match cmd.execute(&reconciliation) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("federation error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        FederationCommand::Enroll(EnrollCommand::Status(cmd)) => execute_enroll_status(cmd),
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

// ---- enrollment status (read-only) ----

pub const ENROLL_STATUS_SCHEMA: &str = "cynic.federation.enroll_status.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaneProbe {
    pub plane: String,
    pub target: Option<String>,
    pub applicable: bool,
    pub green: bool,
    pub detail: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EnrollStatusReport {
    pub schema: String,
    pub status: String,
    pub mutates: bool,
    pub node_id: String,
    pub qualified_status: String,
    pub planes: Vec<PlaneProbe>,
    pub residual_unknowns: Vec<String>,
    pub observed_at_unix_ms: u64,
}

pub fn classify_qualified_status(
    admin: Option<bool>,
    service: Option<bool>,
    mesh: Option<bool>,
    egress: Option<bool>,
    current_status: &str,
) -> String {
    let required: Vec<bool> = [admin, service, mesh, egress]
        .into_iter()
        .flatten()
        .collect();
    // No applicable probe produced evidence: always stub. A registry claim of
    // "observed" without live evidence is exactly how stranded nodes hide.
    if required.is_empty() {
        return "stub".to_string();
    }
    if required.iter().all(|b| *b) {
        if current_status == "observed" {
            "observed".to_string()
        } else {
            "candidate".to_string()
        }
    } else {
        "stub".to_string()
    }
}

#[derive(Deserialize)]
struct EnrollRegistry {
    nodes: Vec<EnrollNode>,
}

#[derive(Deserialize)]
struct EnrollNode {
    id: String,
    status: Option<String>,
    network: Option<EnrollNetwork>,
    transports: Option<EnrollTransports>,
}

#[derive(Deserialize)]
struct EnrollNetwork {
    admin: Option<String>,
    service: Option<String>,
}

#[derive(Deserialize)]
struct EnrollTransports {
    ssh: Option<EnrollSsh>,
}

#[derive(Deserialize)]
struct EnrollSsh {
    target: Option<String>,
    principal: Option<String>,
}

fn read_registry(path: &str) -> Result<EnrollRegistry, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn probe_ping(addr: &str) -> (bool, Option<String>) {
    match Command::new("ping").args(["-c", "2", "-W", "2", addr]).output() {
        Ok(o) if o.status.success() => (true, None),
        Ok(o) => {
            let msg = String::from_utf8_lossy(&o.stderr).trim().to_string();
            (false, if msg.is_empty() { Some("ping failed".to_string()) } else { Some(msg) })
        }
        Err(e) => (false, Some(e.to_string())),
    }
}

fn probe_ssh(principal: &str, host: &str) -> (bool, Option<String>) {
    let target = format!("{}@{}", principal, host);
    match Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5", &target, "hostname"])
        .output()
    {
        Ok(o) if o.status.success() => (true, None),
        Ok(o) => {
            let msg = String::from_utf8_lossy(&o.stderr).trim().to_string();
            (false, if msg.is_empty() { Some("ssh failed".to_string()) } else { Some(msg) })
        }
        Err(e) => (false, Some(e.to_string())),
    }
}

fn probe_service_http(addr: &str) -> (bool, Option<String>) {
    let url = format!("http://{}:3000/api/v1/version", addr);
    match Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "--max-time", "5", &url])
        .output()
    {
        Ok(o) if o.status.success() => {
            let code = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (code == "200", Some(format!("http_code={}", code)))
        }
        Ok(o) => {
            let msg = String::from_utf8_lossy(&o.stderr).trim().to_string();
            (false, if msg.is_empty() { Some("curl failed".to_string()) } else { Some(msg) })
        }
        Err(e) => (false, Some(e.to_string())),
    }
}

fn probe_mesh(interface: &str) -> (bool, Option<String>) {
    match Command::new("wg").args(["show", interface, "latest-handshakes"]).output() {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mut recent = 0usize;
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(ts) = parts[1].parse::<u64>() {
                        if ts > 0 && now.saturating_sub(ts) < 180 {
                            recent += 1;
                        }
                    }
                }
            }
            (recent > 0, Some(format!("recent_handshakes={}", recent)))
        }
        Ok(_) => (false, Some("wg command failed".to_string())),
        Err(e) => (false, Some(format!("wg unavailable: {}", e))),
    }
}

fn probe_egress(url: &str) -> (bool, Option<String>) {
    match Command::new("curl").args(["-sI", "--max-time", "10", url]).output() {
        Ok(o) if o.status.success() => (true, None),
        Ok(o) => {
            let msg = String::from_utf8_lossy(&o.stderr).trim().to_string();
            (false, if msg.is_empty() { Some("egress failed".to_string()) } else { Some(msg) })
        }
        Err(e) => (false, Some(e.to_string())),
    }
}

pub fn build_enroll_status(cmd: &EnrollStatusCommand, registry_path: &str) -> EnrollStatusReport {
    let now_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let mut residual_unknowns: Vec<String> = Vec::new();

    let registry = match read_registry(registry_path) {
        Ok(r) => r,
        Err(e) => {
            return EnrollStatusReport {
                schema: ENROLL_STATUS_SCHEMA.to_string(),
                status: "blocked".to_string(),
                mutates: false,
                node_id: cmd.node.clone().unwrap_or_default(),
                qualified_status: "stub".to_string(),
                planes: vec![],
                residual_unknowns: vec![format!("registry read failed: {} ({})", e, registry_path)],
                observed_at_unix_ms: now_unix_ms,
            };
        }
    };

    let node_id = cmd
        .node
        .clone()
        .or_else(|| {
            if registry.nodes.iter().any(|n| n.id == "local-kernel") {
                Some("local-kernel".to_string())
            } else {
                registry.nodes.first().map(|n| n.id.clone())
            }
        })
        .unwrap_or_default();

    let node = registry.nodes.iter().find(|n| n.id == node_id);

    let current_status = node.and_then(|n| n.status.clone()).unwrap_or_default();

    let (admin_addr, service_addr, ssh_principal) = match node {
        Some(n) => {
            let admin = n.network.as_ref().and_then(|x| x.admin.clone());
            let service = n.network.as_ref().and_then(|x| x.service.clone());
            let principal = n
                .transports
                .as_ref()
                .and_then(|t| t.ssh.as_ref())
                .and_then(|s| s.principal.clone())
                .or_else(|| {
                    n.transports
                        .as_ref()
                        .and_then(|t| t.ssh.as_ref())
                        .and_then(|s| s.target.clone())
                });
            (admin, service, principal)
        }
        None => {
            residual_unknowns.push(format!("node not found in registry: {}", node_id));
            (None, None, None)
        }
    };

    let (admin_applicable, admin_green, admin_detail, admin_err) = if let Some(addr) = &admin_addr {
        let (ping_ok, ping_err) = probe_ping(addr);
        let mut green = ping_ok;
        let mut detail = Some(format!("ping={}", if ping_ok { "green" } else { "red" }));
        let mut err = ping_err;
        if !cmd.no_ssh {
            if let Some(principal) = &ssh_principal {
                let (ssh_ok, ssh_err) = probe_ssh(principal, addr);
                green = green && ssh_ok;
                detail = Some(format!(
                    "{} ssh={}",
                    detail.unwrap_or_default(),
                    if ssh_ok { "green" } else { "red" }
                ));
                if !ssh_ok {
                    err = ssh_err.or(err);
                }
            }
        }
        (true, green, detail, err)
    } else {
        (false, true, Some("no admin address declared".to_string()), None)
    };

    let (service_applicable, service_green, service_detail, service_err) =
        if let Some(addr) = &service_addr {
            let (ping_ok, ping_err) = probe_ping(addr);
            let (_http_ok, _http_detail) = probe_service_http(addr);
            let green = ping_ok;
            let detail = Some(format!(
                "ping={} http={}",
                if ping_ok { "green" } else { "red" },
                if _http_ok { "green" } else { "red" }
            ));
            let err = if !ping_ok { ping_err } else { None };
            (true, green, detail, err)
        } else {
            (false, true, Some("no service address declared".to_string()), None)
        };

    let (mesh_applicable, mesh_green, mesh_detail, mesh_err) = {
        let (green, detail) = probe_mesh(&cmd.interface);
        let unavailable = detail
            .as_ref()
            .map(|d| d.contains("unavailable") || d.contains("failed"))
            .unwrap_or(false);
        if unavailable {
            residual_unknowns.push(format!(
                "mesh probe unavailable: {}",
                detail.clone().unwrap_or_default()
            ));
            (false, true, detail, None)
        } else {
            (true, green, detail, if green { None } else { Some("no recent handshake".to_string()) })
        }
    };

    let (egress_applicable, egress_green, egress_detail, egress_err) = if cmd.no_egress {
        (false, true, Some("skipped (--no-egress)".to_string()), None)
    } else {
        let (green, err) = probe_egress(&cmd.egress_url);
        (true, green, Some(if green { "green".to_string() } else { "red".to_string() }), err)
    };

    let planes = vec![
        PlaneProbe {
            plane: "admin".to_string(),
            target: admin_addr.clone(),
            applicable: admin_applicable,
            green: admin_green,
            detail: admin_detail,
            error: admin_err,
        },
        PlaneProbe {
            plane: "service".to_string(),
            target: service_addr.clone(),
            applicable: service_applicable,
            green: service_green,
            detail: service_detail,
            error: service_err,
        },
        PlaneProbe {
            plane: "mesh".to_string(),
            target: Some(cmd.interface.clone()),
            applicable: mesh_applicable,
            green: mesh_green,
            detail: mesh_detail,
            error: mesh_err,
        },
        PlaneProbe {
            plane: "egress".to_string(),
            target: Some(cmd.egress_url.clone()),
            applicable: egress_applicable,
            green: egress_green,
            detail: egress_detail,
            error: egress_err,
        },
    ];

    let admin_outcome = if admin_applicable { Some(admin_green) } else { None };
    let service_outcome = if service_applicable { Some(service_green) } else { None };
    let mesh_outcome = if mesh_applicable { Some(mesh_green) } else { None };
    let egress_outcome = if egress_applicable { Some(egress_green) } else { None };

    let qualified_status = classify_qualified_status(
        admin_outcome,
        service_outcome,
        mesh_outcome,
        egress_outcome,
        &current_status,
    );

    let all_green = planes
        .iter()
        .filter(|p| p.applicable)
        .all(|p| p.green);
    let status = if all_green { "ok" } else { "degraded" };

    EnrollStatusReport {
        schema: ENROLL_STATUS_SCHEMA.to_string(),
        status: status.to_string(),
        mutates: false,
        node_id,
        qualified_status,
        planes,
        residual_unknowns,
        observed_at_unix_ms: now_unix_ms,
    }
}

pub fn execute_enroll_status(cmd: EnrollStatusCommand) -> ExitCode {
    let registry_path = cmd.registry.clone().unwrap_or_else(|| "infra/registry.json".to_string());
    let report = build_enroll_status(&cmd, &registry_path);
    if cmd.json {
        crate::cli_json::emit_pretty_json(&report)
    } else {
        render_enroll_status_text(&report);
        ExitCode::SUCCESS
    }
}

fn render_enroll_status_text(report: &EnrollStatusReport) {
    println!("Enrollment Status");
    println!("==================");
    println!("Node:             {}", report.node_id);
    println!("Qualified status: {}", report.qualified_status);
    println!("Status:           {}", report.status);
    println!();
    println!("Planes:");
    for p in &report.planes {
        let state = if !p.applicable {
            "n/a"
        } else if p.green {
            "GREEN"
        } else {
            "RED"
        };
        println!(
            "  {:8} {}  target={}  {}",
            p.plane,
            state,
            p.target.clone().unwrap_or_else(|| "-".to_string()),
            p.detail.clone().unwrap_or_default()
        );
        if let Some(e) = &p.error {
            println!("           error: {}", e);
        }
    }
    if !report.residual_unknowns.is_empty() {
        println!();
        println!("Unknowns:");
        for u in &report.residual_unknowns {
            println!("  - {}", u);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_node_protocol::local_autonomy::{
        AutonomyDecision, AutonomyMode, RetainedCapabilities,
    };

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
            node_control_plan: cynic_node_protocol::node_control::NodeControlPlan {
                schema: "cynic.node_control_plan.v2".to_string(),
                status: "converged".to_string(),
                node_id: "node-a".to_string(),
                role: "build_runner".to_string(),
                desired_generation: 1,
                observed_generation: Some(1),
                connection:
                    cynic_node_protocol::node_control::NodeControlConnectionState::Connected,
                decision: cynic_node_protocol::node_control::NodeControlDecision::Converged,
                execution: None,
                evidence_refs: vec![],
                unknowns: vec![],
            },
            autonomy: autonomy_decision(AutonomyMode::Coordinated),
        };
        let cmd = FederationStatusCommand { json: false };
        cmd.execute(&reconciliation).unwrap();
    }

    #[test]
    fn json_renders_all_status_variants() {
        let cases = vec![
            FederationReconciliation::Coordinated {
                node_control_plan: cynic_node_protocol::node_control::NodeControlPlan {
                    schema: "cynic.node_control_plan.v2".to_string(),
                    status: "converged".to_string(),
                    node_id: "node-a".to_string(),
                    role: "build_runner".to_string(),
                    desired_generation: 1,
                    observed_generation: Some(1),
                    connection:
                        cynic_node_protocol::node_control::NodeControlConnectionState::Connected,
                    decision: cynic_node_protocol::node_control::NodeControlDecision::Converged,
                    execution: None,
                    evidence_refs: vec![],
                    unknowns: vec![],
                },
                autonomy: autonomy_decision(AutonomyMode::Coordinated),
            },
            FederationReconciliation::Degraded {
                autonomy: autonomy_decision(AutonomyMode::Degraded),
                retained_capabilities: vec!["identity".to_string(), "health".to_string()],
            },
        ];

        for reconciliation in cases {
            let cmd = FederationStatusCommand { json: true };
            cmd.execute(&reconciliation).unwrap();
        }
    }
}


#[cfg(test)]
mod enroll_status_tests {
    use super::classify_qualified_status;

    #[test]
    fn all_green_without_observed_becomes_candidate() {
        assert_eq!(classify_qualified_status(Some(true), Some(true), Some(true), Some(true), "stub"), "candidate");
    }

    #[test]
    fn all_green_with_observed_stays_observed() {
        assert_eq!(classify_qualified_status(Some(true), Some(true), Some(true), Some(true), "observed"), "observed");
    }

    #[test]
    fn any_red_becomes_stub() {
        assert_eq!(classify_qualified_status(Some(true), Some(false), Some(true), Some(true), "candidate"), "stub");
        assert_eq!(classify_qualified_status(None, None, None, Some(false), "observed"), "stub");
    }

    #[test]
    fn no_applicable_planes_always_stub() {
        assert_eq!(classify_qualified_status(None, None, None, None, "observed"), "stub");
    }

    #[test]
    fn no_applicable_planes_non_observed_becomes_stub() {
        assert_eq!(classify_qualified_status(None, None, None, None, "candidate"), "stub");
    }
}
