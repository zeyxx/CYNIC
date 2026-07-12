use std::path::Path;

use super::command_plan::{
    MeshApplyRuntimeContext, firewall_preflight, post_apply_observation, rollback_conditions,
    rollback_plan, route_install,
};
use crate::domain::network_mesh_apply::{
    MeshPlanAclFile, MeshPlanDnsFile, MeshPlanFile, MeshPlanRouteFile,
};
use crate::network_command_surface::NetworkCommandInvocation;

fn sample_plan(with_dns: bool) -> MeshPlanFile {
    MeshPlanFile {
        provider: "wireguard".to_string(),
        cidr: Some("10.30.0.0/24".to_string()),
        desired_peer_count: 2,
        desired_peers: Vec::new(),
        routes: vec![MeshPlanRouteFile {
            destination: "10.30.0.10/32".to_string(),
            via_peer: "node-a".to_string(),
            provider: "wireguard".to_string(),
        }],
        dns: if with_dns {
            vec![MeshPlanDnsFile {
                name: "node-a.labmesh.example".to_string(),
                address: "10.30.0.10".to_string(),
            }]
        } else {
            Vec::new()
        },
        acl: vec![MeshPlanAclFile {
            id: "allow:ssh".to_string(),
            source: "mesh".to_string(),
            destination: "node-a".to_string(),
            port: Some(22),
            protocol: "tcp".to_string(),
            action: "allow".to_string(),
        }],
    }
}

fn sample_context() -> MeshApplyRuntimeContext {
    MeshApplyRuntimeContext {
        mesh_interface: "cynic-mesh0".to_string(),
        ip_bin: "/portable/bin/ip".to_string(),
        wg_bin: "/portable/bin/wg".to_string(),
        getent_bin: "/portable/bin/getent".to_string(),
    }
}

#[kani::proof]
fn proof_route_install_uses_injected_ip_bin_and_mesh_interface() {
    let commands = route_install(&sample_plan(false), &sample_context());
    assert_eq!(commands.len(), 1);
    match &commands[0].invocation {
        NetworkCommandInvocation::Argv { argv } => {
            assert_eq!(
                argv,
                &vec![
                    "/portable/bin/ip".to_string(),
                    "route".to_string(),
                    "replace".to_string(),
                    "10.30.0.10/32".to_string(),
                    "dev".to_string(),
                    "cynic-mesh0".to_string(),
                ]
            );
        }
        other => panic!("expected argv surface, got {other:?}"),
    }
}

#[kani::proof]
fn proof_post_apply_observation_adds_dns_probe_only_when_dns_exists() {
    let with_dns = post_apply_observation(&sample_plan(true), &sample_context());
    let without_dns = post_apply_observation(&sample_plan(false), &sample_context());

    assert_eq!(with_dns.len(), 3);
    assert_eq!(without_dns.len(), 2);
}

#[kani::proof]
fn proof_firewall_preflight_reports_acl_cardinality_and_cidr() {
    let checks = firewall_preflight(&sample_plan(false));

    assert_eq!(checks.len(), 3);
    assert!(matches!(
        &checks[1].invocation,
        NetworkCommandInvocation::Expectation { detail }
        if detail.contains("10.30.0.0/24")
    ));
    assert!(matches!(
        &checks[2].invocation,
        NetworkCommandInvocation::Expectation { detail }
        if detail.contains("1 declared mesh service ACL entries")
    ));
}

#[kani::proof]
fn proof_rollback_plan_includes_route_removal_and_tail_expectations() {
    let commands = rollback_plan(&sample_plan(false), &sample_context());

    assert_eq!(commands.len(), 4);
    assert_eq!(rollback_conditions().len(), 5);
    match &commands[0].invocation {
        NetworkCommandInvocation::Argv { argv } => {
            assert_eq!(argv[0], "/portable/bin/ip");
            assert_eq!(argv[1], "route");
            assert_eq!(argv[2], "del");
        }
        other => panic!("expected argv surface, got {other:?}"),
    }
    assert!(matches!(
        &commands[3].invocation,
        NetworkCommandInvocation::Expectation { detail }
        if detail.contains("deactivate newly installed provider tunnel config")
    ));
}

#[kani::proof]
fn proof_dns_install_is_bundle_relative() {
    let commands = super::command_plan::dns_install(Path::new("/virtual/bundle"));

    assert_eq!(commands.len(), 1);
    assert!(matches!(
        &commands[0].invocation,
        NetworkCommandInvocation::Expectation { detail }
        if detail.contains("/virtual/bundle/dns-records.json")
    ));
}
