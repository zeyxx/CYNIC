use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator::CommandExecution;
use crate::network_actuator::execution::run_command;
use crate::network_command_surface::{NetworkCommandSurface, argv_command, expectation};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};

const MANIFEST_FILENAME: &str = "manifest.json";
const PLAN_FILENAME: &str = "mesh-plan.json";

#[derive(Debug, Clone, Deserialize)]
struct MeshBundleManifest {
    schema: String,
    status: String,
    provider: String,
    bundle_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MeshPlanFile {
    provider: String,
    cidr: Option<String>,
    desired_peer_count: usize,
    desired_peers: Vec<MeshPlanPeerFile>,
    routes: Vec<MeshPlanRouteFile>,
    dns: Vec<MeshPlanDnsRecordFile>,
    acl: Vec<MeshPlanAclRuleFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct MeshPlanPeerFile {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MeshPlanRouteFile {
    destination: String,
    via_peer: String,
    provider: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MeshPlanDnsRecordFile {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MeshPlanAclRuleFile {}

#[derive(Debug, Clone)]
struct MeshApplyRuntimeContext {
    mesh_interface: String,
    ip_bin: String,
    wg_bin: String,
    getent_bin: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkMeshApplyPlanReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) provider: String,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_complete: bool,
    pub(crate) desired_peer_count: usize,
    pub(crate) preflight: Vec<NetworkCommandSurface>,
    pub(crate) firewall_preflight: Vec<NetworkCommandSurface>,
    pub(crate) tunnel_install: Vec<NetworkCommandSurface>,
    pub(crate) route_preflight: Vec<NetworkCommandSurface>,
    pub(crate) route_install: Vec<NetworkCommandSurface>,
    pub(crate) route_observation: Vec<NetworkCommandSurface>,
    pub(crate) dns_install: Vec<NetworkCommandSurface>,
    pub(crate) post_apply_observation: Vec<NetworkCommandSurface>,
    pub(crate) rollback_conditions: Vec<&'static str>,
    pub(crate) rollback_plan: Vec<NetworkCommandSurface>,
    pub(crate) actuator_policy: MeshActuatorPolicyProjection,
    pub(crate) apply_requires: Vec<&'static str>,
    pub(crate) can_apply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MeshActuatorPolicyProjection {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) source: &'static str,
    pub(crate) allowed_profiles: Vec<MeshActuatorProfile>,
    pub(crate) forbidden_tokens: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MeshActuatorProfile {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) command_count: usize,
    pub(crate) command_prefixes: Vec<NetworkCommandSurface>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkMeshApplyProfileReport {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) profile: String,
    pub(crate) apply: bool,
    pub(crate) closed_environment: bool,
    pub(crate) bundle_dir: String,
    pub(crate) commands: Vec<NetworkCommandSurface>,
    pub(crate) executions: Vec<CommandExecution>,
    pub(crate) violations: Vec<NetworkDiagnostic>,
}

pub(crate) fn build_network_mesh_apply_plan(
    bundle_dir: &str,
) -> Result<NetworkMeshApplyPlanReport, String> {
    let bundle_dir = Path::new(bundle_dir);
    let manifest: MeshBundleManifest = read_json(bundle_dir.join(MANIFEST_FILENAME))?;
    let plan: MeshPlanFile = read_json(bundle_dir.join(PLAN_FILENAME))?;
    let runtime_layout = current_runtime_layout();
    let capabilities = runtime_layout.capabilities();
    let context = MeshApplyRuntimeContext {
        mesh_interface: capabilities.wireguard_tool.mesh_interface.clone(),
        ip_bin: capabilities.route_tool.command.clone(),
        wg_bin: capabilities.wireguard_tool.command.clone(),
        getent_bin: capabilities.name_service.resolver_command.clone(),
    };
    let bundle_complete = manifest
        .bundle_files
        .iter()
        .all(|file| bundle_dir.join(file).is_file());

    let preflight = preflight(&manifest, &plan);
    let firewall_preflight = firewall_preflight(&plan);
    let tunnel_install = tunnel_install(bundle_dir, &manifest, &plan);
    let route_preflight = route_preflight(&plan);
    let route_install = route_install(&plan, &context);
    let route_observation = route_observation(&plan, &context);
    let dns_install = dns_install(bundle_dir);
    let post_apply_observation = post_apply_observation(&plan, &context);
    let rollback_plan = rollback_plan(&plan, &context);
    let actuator_policy = mesh_actuator_policy(
        &firewall_preflight,
        &tunnel_install,
        &route_preflight,
        &route_install,
        &route_observation,
        &dns_install,
        &post_apply_observation,
        &rollback_plan,
    );

    Ok(NetworkMeshApplyPlanReport {
        schema: "cynic.network_mesh_apply_plan.v1",
        status: if bundle_complete {
            "planned"
        } else {
            "blocked"
        },
        mutates: false,
        provider: manifest.provider.clone(),
        bundle_dir: bundle_dir.to_string_lossy().to_string(),
        bundle_complete,
        desired_peer_count: plan.desired_peer_count,
        preflight,
        firewall_preflight,
        tunnel_install,
        route_preflight,
        route_install,
        route_observation,
        dns_install,
        post_apply_observation,
        rollback_conditions: rollback_conditions(),
        rollback_plan,
        actuator_policy,
        apply_requires: vec![
            "operator_confirms_mutation",
            "private_keys_materialized_outside_git",
            "bounded_network_actuator_installed",
            "firewall_policy_reviewed",
            "rollback_plan_available",
        ],
        can_apply: false,
    })
}

pub(crate) fn apply_network_mesh_profile(
    bundle_dir: &str,
    profile: &str,
    apply: bool,
    closed_environment: bool,
) -> Result<NetworkMeshApplyProfileReport, String> {
    let plan = build_network_mesh_apply_plan(bundle_dir)?;
    let selected_profile = plan
        .actuator_policy
        .allowed_profiles
        .iter()
        .find(|item| item.name == profile);
    let mut violations =
        validate_mesh_profile_request(&plan, selected_profile, profile, apply, closed_environment);
    let commands = selected_profile
        .map(|item| item.command_prefixes.clone())
        .unwrap_or_default();

    if violations.is_empty() {
        violations.extend(non_executable_violations(&commands));
    }

    if !violations.is_empty() || commands.is_empty() {
        return Ok(NetworkMeshApplyProfileReport {
            schema: "cynic.network_mesh_apply_profile.v1",
            generated_at: Utc::now().to_rfc3339(),
            status: "refused",
            profile: profile.to_string(),
            apply,
            closed_environment,
            bundle_dir: bundle_dir.to_string(),
            commands,
            executions: Vec::new(),
            violations,
        });
    }

    let executions = commands.iter().map(run_command).collect::<Vec<_>>();
    let success = executions
        .iter()
        .all(|execution| execution.exit_code == Some(0));

    Ok(NetworkMeshApplyProfileReport {
        schema: "cynic.network_mesh_apply_profile.v1",
        generated_at: Utc::now().to_rfc3339(),
        status: if success { "completed" } else { "failed" },
        profile: profile.to_string(),
        apply,
        closed_environment,
        bundle_dir: bundle_dir.to_string(),
        commands,
        executions,
        violations: Vec::new(),
    })
}

fn validate_mesh_profile_request(
    plan: &NetworkMeshApplyPlanReport,
    selected_profile: Option<&MeshActuatorProfile>,
    profile: &str,
    apply: bool,
    closed_environment: bool,
) -> Vec<NetworkDiagnostic> {
    let mut violations = Vec::new();
    if profile.trim().is_empty() {
        violations.push(diagnostic(
            "missing_profile",
            "request_validation",
            "profile is required",
        ));
    }
    if !apply {
        violations.push(diagnostic(
            "apply_flag_required",
            "request_validation",
            "apply flag is required for mesh actuator execution",
        ));
    }
    if !plan.bundle_complete {
        violations.push(diagnostic(
            "bundle_incomplete",
            "request_validation",
            "mesh bundle is incomplete",
        ));
    }
    if selected_profile.is_none() {
        violations.push(diagnostic(
            "profile_not_allowed_for_mesh_bundle",
            "request_validation",
            "profile is not present in actuator_policy.allowed_profiles",
        ));
    }
    if is_mutating_mesh_profile(profile) && !closed_environment {
        violations.push(diagnostic(
            "closed_environment_required",
            "request_validation",
            "mutating mesh profiles require an explicitly declared closed environment",
        ));
    }
    violations
}

fn is_mutating_mesh_profile(profile: &str) -> bool {
    matches!(
        profile,
        "mesh-route-install" | "mesh-rollback" | "mesh-tunnel-install" | "mesh-dns-install"
    )
}

fn non_executable_violations(commands: &[NetworkCommandSurface]) -> Vec<NetworkDiagnostic> {
    commands
        .iter()
        .filter(|command| {
            !matches!(
                command.invocation,
                crate::network_command_surface::NetworkCommandInvocation::Argv { .. }
            )
        })
        .map(|command| {
            let _role = &command.role;
            diagnostic(
                "profile_contains_non_executable_surface",
                "request_validation",
                "profile contains non-argv command; mesh apply-profile only executes argv surfaces",
            )
        })
        .collect()
}

fn mesh_actuator_policy(
    firewall_preflight: &[NetworkCommandSurface],
    tunnel_install: &[NetworkCommandSurface],
    route_preflight: &[NetworkCommandSurface],
    route_install: &[NetworkCommandSurface],
    route_observation: &[NetworkCommandSurface],
    dns_install: &[NetworkCommandSurface],
    post_apply_observation: &[NetworkCommandSurface],
    rollback_plan: &[NetworkCommandSurface],
) -> MeshActuatorPolicyProjection {
    MeshActuatorPolicyProjection {
        schema: "cynic.network_mesh_actuator_policy_projection.v1",
        status: "planned",
        mutates: false,
        source: "cynic.network_mesh_apply_plan.v1",
        allowed_profiles: vec![
            profile(
                "mesh-firewall-preflight",
                "Prepare or validate firewall transaction before tunnel activation.",
                firewall_preflight,
            ),
            profile(
                "mesh-tunnel-install",
                "Install provider tunnel configuration after secret material is supplied.",
                tunnel_install,
            ),
            profile(
                "mesh-route-preflight",
                "Validate declared mesh route bindings before route mutation.",
                route_preflight,
            ),
            profile(
                "mesh-route-install",
                "Install mesh routes after tunnel activation.",
                route_install,
            ),
            profile(
                "mesh-route-observe",
                "Observe mesh route state after route mutation.",
                route_observation,
            ),
            profile("mesh-dns-install", "Install mesh DNS records.", dns_install),
            profile(
                "mesh-observe",
                "Observe mesh convergence after apply.",
                post_apply_observation,
            ),
            profile(
                "mesh-rollback",
                "Rollback routes, DNS, firewall, and tunnel configuration.",
                rollback_plan,
            ),
        ],
        forbidden_tokens: vec![
            "sh", "bash", "-c", "curl", "wget", "nc", "python", "python3", "perl", "ruby", ">",
            ">>", "|", "&&", ";",
        ],
    }
}

fn profile(
    name: &'static str,
    description: &'static str,
    commands: &[NetworkCommandSurface],
) -> MeshActuatorProfile {
    MeshActuatorProfile {
        name,
        description,
        command_count: commands.len(),
        command_prefixes: commands.to_vec(),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T, String> {
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn preflight(manifest: &MeshBundleManifest, plan: &MeshPlanFile) -> Vec<NetworkCommandSurface> {
    vec![
        expectation(
            "bundle_schema",
            format!("manifest schema is {}", manifest.schema),
        ),
        expectation(
            "bundle_status",
            format!("manifest status is {}", manifest.status),
        ),
        expectation(
            "peer_count",
            format!(
                "mesh plan contains {} desired peers",
                plan.desired_peer_count
            ),
        ),
        expectation(
            "provider_secret_material",
            format!(
                "all {} private keys are supplied outside the repository before apply",
                plan.provider
            ),
        ),
    ]
}

fn firewall_preflight(plan: &MeshPlanFile) -> Vec<NetworkCommandSurface> {
    vec![
        expectation(
            "firewall_transaction",
            "install firewall policy atomically before opening mesh routes",
        ),
        expectation(
            "deny_untunneled_mesh",
            format!(
                "deny traffic to mesh cidr {} outside selected provider interface",
                plan.cidr.as_deref().unwrap_or("<mesh-cidr>")
            ),
        ),
        expectation(
            "allow_declared_services",
            format!("allow {} declared mesh service ACL entries", plan.acl.len()),
        ),
    ]
}

fn tunnel_install(
    bundle_dir: &Path,
    manifest: &MeshBundleManifest,
    plan: &MeshPlanFile,
) -> Vec<NetworkCommandSurface> {
    match manifest.provider.as_str() {
        "wireguard" => plan
            .desired_peers
            .iter()
            .map(|peer| {
                expectation(
                    "install_wireguard_peer_template",
                    format!(
                        "materialize {}/wireguard/{}.conf.tpl with private key into OS-specific WireGuard config store",
                        bundle_dir.display(),
                        peer.id
                    ),
                )
            })
            .collect(),
        provider => vec![expectation(
            "install_provider_bundle",
            format!("install provider-specific bundle for {provider}"),
        )],
    }
}

fn route_preflight(plan: &MeshPlanFile) -> Vec<NetworkCommandSurface> {
    plan.routes
        .iter()
        .map(|route| {
            expectation(
                "route_binding",
                format!(
                    "route {} is declared via peer {} using provider {}",
                    route.destination, route.via_peer, route.provider
                ),
            )
        })
        .collect()
}

fn route_install(
    plan: &MeshPlanFile,
    context: &MeshApplyRuntimeContext,
) -> Vec<NetworkCommandSurface> {
    plan.routes
        .iter()
        .map(|route| {
            argv_command(
                "install_mesh_route",
                vec![
                    context.ip_bin.clone(),
                    "route".to_string(),
                    "replace".to_string(),
                    route.destination.clone(),
                    "dev".to_string(),
                    context.mesh_interface.clone(),
                ],
            )
        })
        .collect()
}

fn route_observation(
    plan: &MeshPlanFile,
    context: &MeshApplyRuntimeContext,
) -> Vec<NetworkCommandSurface> {
    plan.routes
        .iter()
        .map(|route| {
            argv_command(
                "inspect_mesh_route",
                vec![
                    context.ip_bin.clone(),
                    "route".to_string(),
                    "show".to_string(),
                    route.destination.clone(),
                ],
            )
        })
        .collect()
}

fn dns_install(bundle_dir: &Path) -> Vec<NetworkCommandSurface> {
    vec![expectation(
        "install_mesh_dns_records",
        format!(
            "install DNS records from {}/dns-records.json into selected DNS provider",
            bundle_dir.display()
        ),
    )]
}

fn post_apply_observation(
    plan: &MeshPlanFile,
    context: &MeshApplyRuntimeContext,
) -> Vec<NetworkCommandSurface> {
    let mut commands = vec![
        argv_command(
            "inspect_wireguard",
            vec![context.wg_bin.clone(), "show".to_string()],
        ),
        argv_command(
            "inspect_mesh_routes",
            vec![
                context.ip_bin.clone(),
                "route".to_string(),
                "show".to_string(),
            ],
        ),
    ];
    if let Some(record) = plan.dns.first() {
        commands.push(argv_command(
            "inspect_mesh_dns",
            vec![
                context.getent_bin.clone(),
                "hosts".to_string(),
                record.name.clone(),
            ],
        ));
    }
    commands
}

fn rollback_conditions() -> Vec<&'static str> {
    vec![
        "provider interface fails to activate",
        "declared peer reachability is below quorum",
        "DNS convergence fails",
        "firewall policy blocks control-plane recovery",
        "post-apply observation cannot run",
    ]
}

fn rollback_plan(
    plan: &MeshPlanFile,
    context: &MeshApplyRuntimeContext,
) -> Vec<NetworkCommandSurface> {
    let mut commands = plan
        .routes
        .iter()
        .map(|route| {
            argv_command(
                "remove_mesh_route",
                vec![
                    context.ip_bin.clone(),
                    "route".to_string(),
                    "del".to_string(),
                    route.destination.clone(),
                ],
            )
        })
        .collect::<Vec<_>>();
    commands.extend([
        expectation(
            "restore_previous_dns",
            "restore DNS provider state captured during preflight",
        ),
        expectation(
            "restore_previous_firewall",
            "restore previous firewall transaction snapshot",
        ),
        expectation(
            "deactivate_new_tunnel",
            "deactivate newly installed provider tunnel config",
        ),
    ]);
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_command_surface::NetworkCommandInvocation;
    use crate::network_mesh_bundle::build_network_mesh_bundle;

    #[test]
    fn apply_plan_reads_bundle_without_mutating() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        build_network_mesh_bundle(&registry_path, None, out.path().to_str().expect("path"))
            .expect("bundle");
        let plan =
            build_network_mesh_apply_plan(out.path().to_str().expect("path")).expect("apply plan");
        assert_eq!(plan.schema, "cynic.network_mesh_apply_plan.v1");
        assert!(plan.bundle_complete);
        assert!(!plan.mutates);
        assert!(!plan.can_apply);
        assert!(!plan.rollback_plan.is_empty());
        assert_eq!(plan.actuator_policy.allowed_profiles.len(), 8);
        assert!(
            plan.actuator_policy
                .allowed_profiles
                .iter()
                .any(|profile| profile.name == "mesh-rollback")
        );
    }

    #[test]
    fn projected_actuator_policy_contains_no_shell_commands() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        build_network_mesh_bundle(&registry_path, None, out.path().to_str().expect("path"))
            .expect("bundle");
        let plan =
            build_network_mesh_apply_plan(out.path().to_str().expect("path")).expect("apply plan");

        for profile in &plan.actuator_policy.allowed_profiles {
            for command in &profile.command_prefixes {
                assert!(
                    !matches!(command.invocation, NetworkCommandInvocation::Shell { .. }),
                    "profile {} contains shell command {}",
                    profile.name,
                    command.role
                );
            }
        }
    }

    #[test]
    fn route_install_profile_is_argv_only_but_requires_closed_environment() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        build_network_mesh_bundle(&registry_path, None, out.path().to_str().expect("path"))
            .expect("bundle");
        let plan =
            build_network_mesh_apply_plan(out.path().to_str().expect("path")).expect("apply plan");
        let route_install = plan
            .actuator_policy
            .allowed_profiles
            .iter()
            .find(|profile| profile.name == "mesh-route-install")
            .expect("route install profile");
        assert!(route_install.command_prefixes.iter().all(|command| {
            matches!(command.invocation, NetworkCommandInvocation::Argv { .. })
        }));

        let report = apply_network_mesh_profile(
            out.path().to_str().expect("path"),
            "mesh-route-install",
            true,
            false,
        )
        .expect("apply profile");

        assert_eq!(report.status, "refused");
        assert!(report.executions.is_empty());
        assert!(
            report
                .violations
                .iter()
                .any(|item| item.code == "closed_environment_required")
        );
    }
}
