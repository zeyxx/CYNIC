use crate::network_command_surface::{NetworkCommandSurface, argv_command};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};

use super::context::NetworkActuatorBundleContext;

pub(super) fn preflight_commands(bundle_dir_str: &str) -> Vec<NetworkCommandSurface> {
    vec![
        cargo_run_network_command(
            "bundle",
            vec![
                "actuator-bundle".to_string(),
                "--output-dir".to_string(),
                bundle_dir_str.to_string(),
            ],
        ),
        status_command(),
        evidence_command(),
    ]
}

pub(super) fn postflight_commands(
    _context: &NetworkActuatorBundleContext,
) -> Vec<NetworkCommandSurface> {
    // No actuation proof here: proving `network-actuate` runs belongs to the
    // recovery flow on the target node, not to the bundle install plan.
    vec![status_command(), evidence_command()]
}

pub(super) fn denial_checks(context: &NetworkActuatorBundleContext) -> Vec<NetworkCommandSurface> {
    vec![
        deny_route_command(context),
        deny_wireguard_command(context),
        deny_shell_command(context),
    ]
}

pub(super) fn deny_route_command(context: &NetworkActuatorBundleContext) -> NetworkCommandSurface {
    argv_command(
        "deny_route",
        vec![
            context.sudo_bin.clone(),
            "-n".to_string(),
            context.route_bin.clone(),
            "route".to_string(),
            "show".to_string(),
        ],
    )
}

pub(super) fn success_criteria() -> Vec<NetworkDiagnostic> {
    vec![
        activation_status_active_criterion(),
        network_actuator_status_active_criterion(),
        direct_commands_denied_criterion(),
    ]
}

fn deny_wireguard_command(context: &NetworkActuatorBundleContext) -> NetworkCommandSurface {
    argv_command(
        "deny_wireguard",
        vec![
            context.sudo_bin.clone(),
            "-n".to_string(),
            context.wireguard_bin.clone(),
            "show".to_string(),
        ],
    )
}

fn deny_shell_command(context: &NetworkActuatorBundleContext) -> NetworkCommandSurface {
    argv_command(
        "deny_shell",
        vec![
            context.sudo_bin.clone(),
            "-n".to_string(),
            context.denied_shell_bin.clone(),
            "-c".to_string(),
            "true".to_string(),
        ],
    )
}

fn activation_status_active_criterion() -> NetworkDiagnostic {
    diagnostic(
        "activation_status_active",
        "success_criteria",
        "activation-status reports status=active",
    )
}

fn network_actuator_status_active_criterion() -> NetworkDiagnostic {
    diagnostic(
        "network_actuator_status_active",
        "success_criteria",
        "cynic-node network actuator-status reports status=active",
    )
}

fn direct_commands_denied_criterion() -> NetworkDiagnostic {
    diagnostic(
        "direct_commands_denied",
        "success_criteria",
        "direct ip/wg/shell commands remain denied for cynic",
    )
}

fn status_command() -> NetworkCommandSurface {
    cargo_run_network_command(
        "status",
        vec!["actuator-status".to_string(), "--json".to_string()],
    )
}

fn evidence_command() -> NetworkCommandSurface {
    cargo_run_network_command(
        "evidence",
        vec!["evidence".to_string(), "--json".to_string()],
    )
}

fn cargo_run_network_command(
    role: &'static str,
    network_args: Vec<String>,
) -> NetworkCommandSurface {
    let mut argv = vec![
        "cargo".to_string(),
        "run".to_string(),
        "-q".to_string(),
        "-p".to_string(),
        "cynic-node".to_string(),
        "--".to_string(),
        "network".to_string(),
    ];
    argv.extend(network_args);
    argv_command(role, argv)
}
