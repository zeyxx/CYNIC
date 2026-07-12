use std::path::Path;

use crate::domain::network_mesh_apply::{MeshBundleManifest, MeshPlanFile};
use crate::network_command_surface::{NetworkCommandSurface, argv_command, expectation};

#[derive(Debug, Clone)]
pub(super) struct MeshApplyRuntimeContext {
    pub(super) mesh_interface: String,
    pub(super) ip_bin: String,
    pub(super) wg_bin: String,
    pub(super) getent_bin: String,
}

pub(super) fn preflight(
    manifest: &MeshBundleManifest,
    plan: &MeshPlanFile,
) -> Vec<NetworkCommandSurface> {
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

pub(super) fn firewall_preflight(plan: &MeshPlanFile) -> Vec<NetworkCommandSurface> {
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

pub(super) fn tunnel_install(
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

pub(super) fn route_preflight(plan: &MeshPlanFile) -> Vec<NetworkCommandSurface> {
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

pub(super) fn route_install(
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

pub(super) fn route_observation(
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

pub(super) fn dns_install(bundle_dir: &Path) -> Vec<NetworkCommandSurface> {
    vec![expectation(
        "install_mesh_dns_records",
        format!(
            "install DNS records from {}/dns-records.json into selected DNS provider",
            bundle_dir.display()
        ),
    )]
}

pub(super) fn post_apply_observation(
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

pub(super) fn rollback_conditions() -> Vec<&'static str> {
    vec![
        "provider interface fails to activate",
        "declared peer reachability is below quorum",
        "DNS convergence fails",
        "firewall policy blocks control-plane recovery",
        "post-apply observation cannot run",
    ]
}

pub(super) fn rollback_plan(
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
