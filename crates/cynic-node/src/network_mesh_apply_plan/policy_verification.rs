use super::policy::mesh_actuator_policy;
use crate::network_command_surface::{NetworkCommandInvocation, argv_command, expectation};

#[kani::proof]
fn proof_mesh_policy_exposes_stable_profile_order_and_count() {
    let routes = vec![argv_command(
        "install_mesh_route",
        vec!["/portable/bin/ip".to_string(), "route".to_string()],
    )];
    let policy = mesh_actuator_policy(
        &[expectation("firewall", "ok")],
        &[expectation("tunnel", "ok")],
        &[expectation("route-preflight", "ok")],
        &routes,
        &[argv_command(
            "inspect_mesh_route",
            vec!["/portable/bin/ip".to_string(), "route".to_string()],
        )],
        &[expectation("dns", "ok")],
        &[argv_command(
            "inspect_wireguard",
            vec!["/portable/bin/wg".to_string(), "show".to_string()],
        )],
        &[expectation("rollback", "ok")],
    );

    assert_eq!(policy.allowed_profiles.len(), 8);
    assert_eq!(policy.allowed_profiles[0].name, "mesh-firewall-preflight");
    assert_eq!(policy.allowed_profiles[3].name, "mesh-route-install");
    assert_eq!(policy.allowed_profiles[7].name, "mesh-rollback");
    assert_eq!(policy.allowed_profiles[3].command_count, 1);
}

#[kani::proof]
fn proof_mesh_policy_forbidden_tokens_keep_shell_escape_vectors_blocked() {
    let policy = mesh_actuator_policy(&[], &[], &[], &[], &[], &[], &[], &[]);

    assert!(policy.forbidden_tokens.contains(&"sh"));
    assert!(policy.forbidden_tokens.contains(&"bash"));
    assert!(policy.forbidden_tokens.contains(&"-c"));
    assert!(policy.forbidden_tokens.contains(&"|"));
    assert!(policy.forbidden_tokens.contains(&";"));
}

#[kani::proof]
fn proof_mesh_policy_preserves_argv_surfaces_verbatim() {
    let route_install = vec![argv_command(
        "install_mesh_route",
        vec![
            "/portable/bin/ip".to_string(),
            "route".to_string(),
            "replace".to_string(),
        ],
    )];
    let policy = mesh_actuator_policy(&[], &[], &[], &route_install, &[], &[], &[], &[]);
    let profile = &policy.allowed_profiles[3];

    assert_eq!(profile.name, "mesh-route-install");
    match &profile.command_prefixes[0].invocation {
        NetworkCommandInvocation::Argv { argv } => {
            assert_eq!(argv[0], "/portable/bin/ip");
            assert_eq!(argv[1], "route");
            assert_eq!(argv[2], "replace");
        }
        other => panic!("expected argv surface, got {other:?}"),
    }
}
