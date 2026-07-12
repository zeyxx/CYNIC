use std::fs;
use std::path::Path;

#[test]
fn host_exec_binary_stays_a_thin_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bin = fs::read_to_string(root.join("src/bin/cynic-host-exec.rs"))
        .expect("read cynic-host-exec.rs");

    assert!(
        bin.contains("cynic_host::host_exec_cli::execute(args)"),
        "cynic-host-exec must delegate execution to cynic_host::host_exec_cli"
    );
    assert!(
        !bin.contains("LocalHostObservationAdapter")
            && !bin.contains("LocalHostCommandRunner")
            && !bin.contains("resolve_host_profile"),
        "host-exec orchestration belongs in cynic-host::host_exec_cli, not the bin adapter"
    );
}

#[test]
fn host_exec_cli_surface_stays_decomposed_by_responsibility() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let surface =
        fs::read_to_string(root.join("src/host_exec_cli.rs")).expect("read host_exec_cli.rs");

    for module in [
        "args",
        "bootstrap",
        "bridge_commands",
        "io",
        "network_baseline",
        "profile_selection",
        "validation",
    ] {
        assert!(
            root.join(format!("src/host_exec_cli/{module}.rs"))
                .is_file(),
            "host_exec_cli::{module} must exist as a bounded responsibility module"
        );
        assert!(
            surface.contains(&format!("mod {module};")),
            "host_exec_cli.rs must declare module {module}"
        );
    }

    assert!(
        !surface.contains("LocalHostObservationAdapter")
            && !surface.contains("LocalHostCommandRunner")
            && !surface.contains("observe_host_with_resolution"),
        "host_exec_cli.rs must stay a dispatcher; adapters/usecases belong in submodules"
    );
}

#[test]
fn host_usecases_surface_stays_a_thin_orchestrator() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let surface = fs::read_to_string(root.join("src/usecases/mod.rs")).expect("read usecases");
    let reports =
        fs::read_to_string(root.join("src/usecases/reports.rs")).expect("read usecases/reports.rs");
    let status =
        fs::read_to_string(root.join("src/usecases/status.rs")).expect("read usecases/status.rs");
    let execution = fs::read_to_string(root.join("src/usecases/execution.rs"))
        .expect("read usecases/execution.rs");

    assert!(
        surface.lines().count() <= 220,
        "usecases/mod.rs must stay a thin orchestrator; current line count is {}",
        surface.lines().count()
    );
    for module in ["reports", "status", "execution"] {
        assert!(
            surface.contains(&format!("mod {module};")),
            "usecases/mod.rs must declare bounded module {module}"
        );
        assert!(
            root.join(format!("src/usecases/{module}.rs")).is_file(),
            "usecases::{module} must exist as a bounded responsibility module"
        );
    }
    assert!(
        surface.contains("#[cfg(test)]\nmod tests;"),
        "usecases tests must live outside the production orchestrator body"
    );
    for forbidden in [
        "pub struct HostObservationReports",
        "pub struct HostApplyExecutionReport",
        "fn extend_commands",
        "fn executed_result",
        "fn blocked_domain_statuses",
    ] {
        assert!(
            !surface.contains(forbidden),
            "usecases/mod.rs must not own extracted host usecase detail: {forbidden}"
        );
    }
    assert!(
        reports.contains("pub struct HostObservationReports")
            && reports.contains("pub struct HostApplyExecutionReport"),
        "usecases reports DTOs must live in reports.rs"
    );
    assert!(
        status.contains("pub(super) fn blocked_domain_statuses")
            && status.contains("pub(super) fn next_actions"),
        "host usecase status helpers must live in status.rs"
    );
    assert!(
        execution.contains("pub fn execute_host_apply") && execution.contains("fn extend_commands"),
        "host apply execution must live in execution.rs"
    );
}

#[test]
fn service_domain_uses_explicit_command_context_for_portability() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let services =
        fs::read_to_string(root.join("src/domain/services.rs")).expect("read domain/services.rs");
    let production_services = services.split("#[cfg(test)]").next().unwrap_or(&services);
    let local_runner = fs::read_to_string(root.join("src/adapters/local_command_runner.rs"))
        .expect("read adapters/local_command_runner.rs");

    assert!(
        production_services.contains("pub struct ServiceCommandContext")
            && production_services.contains("pub fn bundle_with_context")
            && production_services.contains("pub fn activation_plan_with_context")
            && production_services.contains("pub fn apply_report_with_context"),
        "service domain must expose explicit command-context entrypoints"
    );
    assert!(
        production_services.contains("pub(crate) fn openrc_service_name"),
        "OpenRC service name normalization must be shared from the service domain"
    );
    assert!(
        local_runner.contains("use crate::domain::services::openrc_service_name;")
            && !local_runner.contains("fn openrc_service_name"),
        "local command runner must reuse the shared OpenRC service name normalizer"
    );

    let production_without_context_factory = production_services
        .replace("pub fn from_current_runtime() -> Self", "")
        .replace("let layout = current_runtime_layout();", "");
    assert!(
        !production_without_context_factory.contains("current_runtime_layout()"),
        "service command construction must consume ServiceCommandContext instead of reading global runtime layout"
    );
}

#[test]
fn package_domain_uses_explicit_command_context_for_portability() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let packages =
        fs::read_to_string(root.join("src/domain/packages.rs")).expect("read domain/packages.rs");
    let production_packages = packages.split("#[cfg(test)]").next().unwrap_or(&packages);

    assert!(
        production_packages.contains("pub struct PackageCommandContext")
            && production_packages.contains("pub fn bundle_with_context")
            && production_packages.contains("pub fn activation_plan_with_context")
            && production_packages.contains("pub fn apply_report_with_context"),
        "package domain must expose explicit command-context entrypoints"
    );
    assert!(
        production_packages.contains("fn package_query_argv")
            && production_packages.contains("fn package_install_argv"),
        "package command argv builders must be pure helpers over PackageCommandContext"
    );

    let production_without_context_factory = production_packages
        .replace("pub fn from_current_runtime() -> Self", "")
        .replace("let layout = current_runtime_layout();", "");
    assert!(
        !production_without_context_factory.contains("current_runtime_layout()"),
        "package command construction must consume PackageCommandContext instead of reading global runtime layout"
    );
    for forbidden in [
        ".query_argv(pkg, &runtime_layout)",
        ".install_argv(&self.bootstrap_packages, &runtime_layout)",
    ] {
        assert!(
            !production_without_context_factory.contains(forbidden),
            "package domain must not hardwire runtime layout in command construction: {forbidden}"
        );
    }
}

#[test]
fn privilege_domain_uses_explicit_command_context_for_portability() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let privilege =
        fs::read_to_string(root.join("src/domain/privilege.rs")).expect("read domain/privilege.rs");
    let production_privilege = privilege.split("#[cfg(test)]").next().unwrap_or(&privilege);

    assert!(
        production_privilege.contains("pub struct PrivilegeCommandContext")
            && production_privilege.contains("pub fn activation_plan_with_context")
            && production_privilege.contains("pub fn apply_report_with_context"),
        "privilege domain must expose explicit command-context entrypoints"
    );
    assert!(
        production_privilege.contains("route_bin")
            && production_privilege.contains("service_manager_bin")
            && production_privilege.contains("denied_shell_bin"),
        "privilege denial checks must be generated from injectable command context"
    );

    let production_without_context_factory = production_privilege
        .replace("pub fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "")
        .replace("let capabilities = runtime_layout.capabilities();", "");
    assert!(
        !production_without_context_factory.contains("current_runtime_layout()"),
        "privilege command construction must consume PrivilegeCommandContext instead of reading global runtime layout"
    );
    for forbidden in [
        "capabilities.route_tool.command.clone()",
        "capabilities.service_manager.command.clone()",
    ] {
        assert!(
            !production_without_context_factory.contains(forbidden),
            "privilege domain must not hardwire denial command construction: {forbidden}"
        );
    }
}

#[test]
fn principal_domain_uses_explicit_command_context_for_portability() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let principal =
        fs::read_to_string(root.join("src/domain/principal.rs")).expect("read domain/principal.rs");
    let production_principal = principal.split("#[cfg(test)]").next().unwrap_or(&principal);

    assert!(
        production_principal.contains("pub struct PrincipalCommandContext")
            && production_principal.contains("pub fn activation_plan_with_context")
            && production_principal.contains("pub fn apply_report_with_context"),
        "principal domain must expose explicit command-context entrypoints"
    );
    assert!(
        production_principal.contains("getent_bin"),
        "principal identity checks must be generated from injectable command context"
    );

    let production_without_context_factory = production_principal
        .replace("pub fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "")
        .replace("getent_bin: runtime_layout.getent_bin(),", "");
    assert!(
        !production_without_context_factory.contains("current_runtime_layout()"),
        "principal command construction must consume PrincipalCommandContext instead of reading global runtime layout"
    );
    for forbidden in [
        "runtime_layout.getent_bin()",
        "format!(\"getent passwd",
        "format!(\"getent group",
    ] {
        assert!(
            !production_without_context_factory.contains(forbidden),
            "principal domain must not hardwire identity command construction: {forbidden}"
        );
    }
}
