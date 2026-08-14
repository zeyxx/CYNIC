#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn cli_delegates_lab_runtime_to_cynic_host() {
    let root = crate_root();
    assert!(
        !root.join("src/lab_plan.rs").exists(),
        "lab planning/runtime logic belongs in cynic-host::lab, not cynic-cli"
    );

    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let lab_surface =
        fs::read_to_string(root.join("src/lab_surface.rs")).expect("read lab_surface.rs");
    assert!(
        lab_surface.contains("use cynic_host::lab as lab_plan;"),
        "cynic-cli must delegate lab surfaces to cynic_host::lab"
    );
    assert!(
        main_rs.contains("mod lab_surface;") && !main_rs.contains("enum LabCommand"),
        "lab command handling must live in lab_surface, not main.rs"
    );
    assert!(
        !main_rs.contains("mod lab_plan;"),
        "cynic-cli must not own a local lab_plan module"
    );
    assert!(
        main_rs.contains("mod crate_audit;"),
        "crate audit support must live outside main.rs"
    );
}

#[test]
fn cli_exposes_framework_bootstrap_as_canonical_surface() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let framework_surface = fs::read_to_string(root.join("src/bootstrap_surface/framework.rs"))
        .expect("read bootstrap_surface/framework.rs");
    let command_manifest =
        fs::read_to_string(root.join("src/command_manifest.rs")).expect("read command_manifest.rs");
    assert!(
        framework_surface.contains("use cynic_host::framework_bootstrap;"),
        "framework bootstrap must be owned by cynic-host, not the CLI"
    );
    assert!(
        !main_rs.contains("use cynic_host::framework_bootstrap;"),
        "main.rs must not own framework bootstrap wiring"
    );
    assert!(
        command_manifest.contains("cynic bootstrap framework bundle --json --output-dir <dir>"),
        "CLI manifest must expose the canonical framework bootstrap surface"
    );
    assert!(
        command_manifest
            .contains("cynic bootstrap framework install-cli --json [--apply] [--source <path>] [--target <path>]"),
        "CLI manifest must expose the standalone CLI install surface"
    );
    assert!(
        command_manifest.contains("cynic dev crate-audit --json"),
        "CLI manifest must expose the crate audit development surface"
    );
    assert!(
        command_manifest.contains("cynic dev kernel-audit --json"),
        "CLI manifest must expose the kernel audit development surface"
    );
    assert!(
        command_manifest.contains("cynic dev ownership-audit --json"),
        "CLI manifest must expose the ownership audit development surface"
    );
    assert!(
        command_manifest.contains("cynic dev surface-matrix --json [--output <path>] [--persist]"),
        "CLI manifest must expose the surface matrix development report"
    );
    assert!(
        command_manifest
            .contains("cynic dev check --json [--architecture-only] [--target-root <path>]"),
        "CLI manifest must expose the dev check proof surface"
    );
}

#[test]
fn cli_exposes_runtime_harness_without_owning_the_model() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let host_checks = fs::read_to_string(root.join("src/runtime_surface/host_checks.rs"))
        .expect("read runtime_surface/host_checks.rs");
    assert!(
        host_checks.contains("cynic_host::runtime_harness::runtime_harness_catalog"),
        "runtime harness catalog must be owned by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::Harness"),
        "CLI manifest must expose the canonical runtime harness surface"
    );
}

#[test]
fn cli_delegates_host_conformance_to_cynic_host() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let host_checks = fs::read_to_string(root.join("src/runtime_surface/host_checks.rs"))
        .expect("read runtime_surface/host_checks.rs");
    assert!(
        host_checks.contains("cynic_host::conformance::observe_host_conformance"),
        "host conformance must be owned by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::Conformance"),
        "CLI manifest must expose the canonical runtime conformance surface"
    );
}

#[test]
fn cli_delegates_runtime_metrics_to_cynic_host() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let host_checks = fs::read_to_string(root.join("src/runtime_surface/host_checks.rs"))
        .expect("read runtime_surface/host_checks.rs");
    assert!(
        host_checks
            .contains("cynic_host::runtime_metrics::host_runtime_metrics_with_external_proof"),
        "runtime metrics must be computed by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::Metrics"),
        "CLI manifest must expose the canonical runtime metrics surface"
    );
}

#[test]
fn cli_delegates_runtime_network_preflight_to_cynic_host() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let network_preflight =
        fs::read_to_string(root.join("src/runtime_surface/network_preflight.rs"))
            .expect("read runtime_surface/network_preflight.rs");
    assert!(
        network_preflight
            .contains("cynic_host::runtime_network_preflight::observe_runtime_network_preflight"),
        "runtime network preflight must be owned by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::NetworkPreflight"),
        "CLI manifest must expose the canonical runtime network preflight surface"
    );
}

#[test]
fn cli_delegates_runtime_execution_context_to_cynic_host() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let execution_context =
        fs::read_to_string(root.join("src/runtime_surface/execution_context.rs"))
            .expect("read runtime_surface/execution_context.rs");
    assert!(
        execution_context
            .contains("cynic_host::runtime_execution_context::observe_runtime_execution_context"),
        "runtime execution context must be owned by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::ExecutionContext"),
        "CLI manifest must expose the canonical runtime execution context surface"
    );
}

#[test]
fn cli_delegates_runtime_resolver_plan_to_cynic_host() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let resolver_plan = fs::read_to_string(root.join("src/runtime_surface/resolver_plan.rs"))
        .expect("read runtime_surface/resolver_plan.rs");
    assert!(
        resolver_plan.contains("cynic_host::runtime_resolver_plan::build_runtime_resolver_plan"),
        "runtime resolver planning must be owned by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::ResolverPlan"),
        "CLI manifest must expose the canonical runtime resolver planning surface"
    );
}

#[test]
fn cli_delegates_runtime_agent_matrix_to_cynic_host() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let agent_matrix = fs::read_to_string(root.join("src/runtime_surface/agent_matrix.rs"))
        .expect("read runtime_surface/agent_matrix.rs");
    let command_manifest =
        fs::read_to_string(root.join("src/command_manifest.rs")).expect("read command_manifest.rs");
    let surface_registry = fs::read_to_string(root.join("../../infra/surface-registry.json"))
        .expect("read infra/surface-registry.json");

    assert!(
        agent_matrix.contains("cynic_host::agent_matrix::build_agent_matrix_report"),
        "runtime agent matrix must be owned by cynic-host, not cynic-cli"
    );
    assert!(
        runtime_surface.contains("RuntimeCommand::AgentMatrix"),
        "CLI runtime surface must expose the agent-matrix command"
    );
    assert!(
        command_manifest.contains("cynic runtime agent-matrix --json"),
        "CLI manifest must expose the canonical runtime agent-matrix surface"
    );
    assert!(
        surface_registry.contains("\"id\":\"runtime_agent_matrix\"")
            || surface_registry.contains("\"id\": \"runtime_agent_matrix\""),
        "surface registry must declare the runtime agent-matrix canonical surface"
    );
}

#[test]
fn deps_surface_is_split_into_thin_cli_modules() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let deps_surface =
        fs::read_to_string(root.join("src/deps_surface.rs")).expect("read deps_surface.rs");
    let deps_model = fs::read_to_string(root.join("src/deps_surface_model.rs"))
        .expect("read deps_surface_model.rs");
    let deps_specs = fs::read_to_string(root.join("src/deps_surface_specs.rs"))
        .expect("read deps_surface_specs.rs");
    let deps_probe = fs::read_to_string(root.join("src/deps_surface_probe.rs"))
        .expect("read deps_surface_probe.rs");
    let deps_render = fs::read_to_string(root.join("src/deps_surface_render.rs"))
        .expect("read deps_surface_render.rs");
    let deps_plan = fs::read_to_string(root.join("src/deps_surface_plan.rs"))
        .expect("read deps_surface_plan.rs");
    let deps_pins = fs::read_to_string(root.join("src/deps_surface_pins.rs"))
        .expect("read deps_surface_pins.rs");
    let deps_report = fs::read_to_string(root.join("src/deps_surface_report.rs"))
        .expect("read deps_surface_report.rs");
    let deps_upgrade = fs::read_to_string(root.join("src/deps_surface_upgrade.rs"))
        .expect("read deps_surface_upgrade.rs");

    assert!(
        main_rs.contains("mod deps_surface_model;")
            && main_rs.contains("mod deps_surface_specs;")
            && main_rs.contains("mod deps_surface_plan;")
            && main_rs.contains("mod deps_surface_pins;")
            && main_rs.contains("mod deps_surface_probe;")
            && main_rs.contains("mod deps_surface_report;")
            && main_rs.contains("mod deps_surface_render;")
            && main_rs.contains("mod deps_surface_upgrade;"),
        "deps surface must be split into explicit model/specs/plan/pins/probe/report/render/upgrade modules"
    );
    assert!(
        deps_surface.contains("use crate::deps_surface_plan::build_plan;")
            && deps_surface.contains("use crate::deps_surface_pins::load_pins;")
            && deps_surface.contains("use crate::deps_surface_report::build_deps_report;")
            && deps_surface.contains("use crate::deps_surface_upgrade::execute_deps_upgrade as execute_deps_upgrade_impl;")
            && deps_surface.contains("use crate::deps_surface_render::{print_deps_human, print_plan_human};"),
        "deps_surface.rs must delegate planning, pins loading, reporting, rendering, and upgrade to dedicated modules"
    );
    assert!(
        deps_surface.lines().count() < 800,
        "deps_surface.rs must remain thin enough to avoid regressing into a monolith"
    );
    assert!(
        deps_model.contains("pub(crate) struct DepsReport")
            && deps_specs.contains("pub(crate) fn specs()")
            && deps_plan.contains("pub(crate) fn build_plan")
            && deps_pins.contains("pub(crate) fn load_pins")
            && deps_probe.contains("pub(crate) fn probe_and_evaluate")
            && deps_report.contains("pub(crate) fn build_deps_report")
            && deps_upgrade.contains("pub(crate) fn execute_deps_upgrade")
            && deps_render.contains("pub(crate) fn print_deps_human"),
        "deps surface split must preserve clear ownership: model/specs/plan/pins/probe/report/render/upgrade"
    );
}

#[test]
fn dev_check_declares_isolated_cargo_target_policy() {
    let root = crate_root();
    let check =
        fs::read_to_string(root.join("src/dev_surface/check.rs")).expect("read dev check surface");

    assert!(
        check.contains("CargoTargetPolicy")
            && check.contains("per_step_target_dir")
            && check.contains("shared_worktree_target_allowed: false")
            && check.contains("make_check")
            && check.contains("CANONICAL_GATE")
            && check.contains("ARCHITECTURE_BOUNDARY_GATE")
            && check.contains("architecture_only")
            && check.contains("architecture_boundary_ok"),
        "dev check must expose a machine-readable isolated Cargo target policy and delegate to make check"
    );
    assert!(
        check.contains("invalid symbol index")
            && check.contains("internal compiler error: Segmentation fault")
            && check.contains("compiler_or_linker_artifact_corruption"),
        "dev check must classify linker artifact corruption explicitly"
    );
    assert!(
        check.contains("CARGO_BUILD_JOBS")
            && check.contains("CMAKE_BUILD_PARALLEL_LEVEL")
            && check.contains("\"1\".to_string()"),
        "dev check must serialize Rust/native build jobs for deterministic validation"
    );
}

#[test]
fn makefile_uses_isolated_cargo_target_dir_for_automated_surfaces() {
    let root = crate_root();
    let makefile = fs::read_to_string(root.join("../../Makefile")).expect("read Makefile");

    assert!(
        makefile.contains("CYNIC_CARGO_TARGET_ROOT ?= /tmp/cynic-cargo-targets")
            && makefile.contains("CYNIC_CARGO_BUILD_JOBS ?= 1")
            && makefile.contains(
                "CARGO_TARGET_DIR := $(if $(strip $(CARGO_TARGET_DIR)),$(CARGO_TARGET_DIR),$(CYNIC_CARGO_TARGET_ROOT)/$(CYNIC_CARGO_TARGET_PROFILE))"
            )
            && makefile.contains("CARGO_BUILD_JOBS=$(CYNIC_CARGO_BUILD_JOBS)")
            && makefile.contains("CMAKE_BUILD_PARALLEL_LEVEL=$(CYNIC_CARGO_BUILD_JOBS)")
            && makefile.contains("CARGO_TARGET_DIR=$(CARGO_TARGET_DIR)"),
        "Makefile automation must use isolated targets and serialized native build jobs"
    );
    assert!(
        makefile.contains("$(CARGO_TARGET_DIR)/debug/cynic")
            && !makefile.contains("sudo target/debug/cynic bootstrap framework install-cli"),
        "install-cli must install the binary from the declared isolated Cargo target dir"
    );
}

#[test]
fn cli_runtime_surface_is_split_out_of_main() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    assert!(
        main_rs.contains("mod runtime_surface;"),
        "main.rs must delegate runtime command handling to runtime_surface"
    );
    assert!(
        !main_rs.contains("enum RuntimeCommand"),
        "runtime command enum must not live in main.rs"
    );
    assert!(
        root.join("src/runtime_surface/paths.rs").exists()
            && root.join("src/runtime_surface/suite.rs").exists()
            && root.join("src/runtime_surface/host_checks.rs").exists()
            && root
                .join("src/runtime_surface/network_preflight.rs")
                .exists()
            && root.join("src/runtime_surface/report.rs").exists()
            && root.join("src/runtime_surface/report/build.rs").exists()
            && root.join("src/runtime_surface/report/catalog.rs").exists()
            && root.join("src/runtime_surface/report/summary.rs").exists()
            && root.join("src/runtime_surface/report/types.rs").exists()
            && root.join("src/runtime_surface/validation.rs").exists(),
        "runtime surface must remain split by paths, suite, host_checks, network_preflight, report, report submodules, and validation responsibilities"
    );
    assert!(
        root.join("src/command_manifest.rs").exists(),
        "CLI command manifest must live outside main.rs"
    );
    assert!(
        main_rs.contains("mod dev_surface;") && !main_rs.contains("enum DevCommand"),
        "dev command handling must live in dev_surface, not main.rs"
    );
    assert!(
        root.join("src/dev_surface/check.rs").exists(),
        "dev check execution must live in a granular dev_surface module"
    );
    assert!(
        root.join("src/dev_surface/manifest.rs").exists()
            && root.join("src/dev_surface/audit_persist.rs").exists(),
        "dev surface must split manifest projections and audit persistence into dedicated modules"
    );
    assert!(
        main_rs.contains("mod audit_surface;") && !main_rs.contains("enum AuditCommand"),
        "audit command handling must live in audit_surface, not main.rs"
    );
    assert!(
        main_rs.contains("mod network_surface;")
            && !main_rs.contains("enum NetworkCommand")
            && !main_rs.contains("fn execute_network"),
        "network command handling must live in network_surface, not main.rs"
    );
    assert!(
        main_rs.contains("mod bootstrap_surface;")
            && !main_rs.contains("enum BootstrapCommand")
            && !main_rs.contains("fn execute_bootstrap"),
        "bootstrap command handling must live in bootstrap_surface, not main.rs"
    );
    assert!(
        root.join("src/bootstrap_surface/host.rs").exists()
            && root.join("src/bootstrap_surface/bridge.rs").exists()
            && root.join("src/bootstrap_surface/constellation.rs").exists()
            && root.join("src/bootstrap_surface/selection.rs").exists(),
        "bootstrap surface must remain split by host, bridge, constellation, and selection responsibilities"
    );
    assert!(
        main_rs.contains("mod host_exec_alias_surface;")
            && !main_rs.contains("fn execute_host_exec_alias")
            && !main_rs.contains("fn parse_profile_args"),
        "bounded host-exec alias must live outside main.rs"
    );
    assert!(
        main_rs.contains("mod fleet_surface;")
            && !main_rs.contains("enum FleetCommand")
            && !main_rs.contains("fn execute_fleet"),
        "fleet command handling must live in fleet_surface, not main.rs"
    );
    assert!(
        main_rs.contains("mod transport_surface;")
            && !main_rs.contains("enum TransportCommand")
            && !main_rs.contains("fn execute_transport"),
        "transport command handling must live in transport_surface, not main.rs"
    );
    assert!(
        main_rs.contains("mod root_dispatch;"),
        "main.rs must register a dedicated root_dispatch module"
    );
    let root_dispatch =
        fs::read_to_string(root.join("src/root_dispatch.rs")).expect("read root_dispatch.rs");
    assert!(
        root_dispatch.contains("pub(crate) fn execute_root_command(command: Command)")
            && root_dispatch.contains("fn execute_deps_command(")
            && root_dispatch.contains("fn execute_chat_command("),
        "root dispatch must own top-level command routing and argument assembly"
    );
    assert!(
        !main_rs.contains("chat_surface::ChatArgs {")
            && !main_rs.contains("deps_surface::execute_deps_upgrade")
            && main_rs.contains("Some(command) => execute_root_command(command)"),
        "main.rs must remain a thin CLI facade over root_dispatch"
    );
}

#[test]
fn lifecycle_surface_delegates_human_rendering() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let lifecycle_surface = fs::read_to_string(root.join("src/lifecycle_surface.rs"))
        .expect("read lifecycle_surface.rs");
    let lifecycle_render = fs::read_to_string(root.join("src/lifecycle_surface_render.rs"))
        .expect("read lifecycle_surface_render.rs");

    assert!(
        main_rs.contains("mod lifecycle_surface_render;"),
        "main.rs must register lifecycle_surface_render as a dedicated module"
    );
    assert!(
        lifecycle_surface.contains("use crate::lifecycle_surface_render::{")
            && lifecycle_surface.contains("print_status_human")
            && lifecycle_surface.contains("print_kernel_release_human"),
        "lifecycle_surface.rs must delegate human rendering to lifecycle_surface_render"
    );
    assert!(
        lifecycle_render.contains("pub(crate) fn print_status_human")
            && lifecycle_render.contains("pub(crate) fn print_plan_human")
            && lifecycle_render.contains("pub(crate) fn print_gate_human")
            && lifecycle_render.contains("pub(crate) fn print_promote_human")
            && lifecycle_render.contains("pub(crate) fn print_ops_human")
            && lifecycle_render.contains("pub(crate) fn print_repair_human")
            && lifecycle_render.contains("pub(crate) fn print_apply_human")
            && lifecycle_render.contains("pub(crate) fn print_kernel_release_human"),
        "lifecycle human rendering must live in the dedicated render module"
    );
}

#[test]
fn deps_and_lifecycle_use_shared_json_emitter() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let cli_json = fs::read_to_string(root.join("src/cli_json.rs")).expect("read cli_json.rs");
    let deps_surface =
        fs::read_to_string(root.join("src/deps_surface.rs")).expect("read deps_surface.rs");
    let lifecycle_surface = fs::read_to_string(root.join("src/lifecycle_surface.rs"))
        .expect("read lifecycle_surface.rs");

    assert!(
        main_rs.contains("mod cli_json;"),
        "main.rs must register the shared cli_json module"
    );
    assert!(
        main_rs.contains("mod fs_json;"),
        "main.rs must register the shared fs_json module"
    );
    assert!(
        cli_json.contains("pub(crate) fn emit_pretty_json"),
        "cli_json must own the shared pretty JSON emitter"
    );
    assert!(
        deps_surface.contains("use crate::cli_json::emit_pretty_json;")
            && lifecycle_surface.contains("use crate::cli_json::emit_pretty_json;"),
        "deps and lifecycle surfaces must share the common JSON emitter"
    );
}

#[test]
fn runtime_and_dev_surfaces_share_file_json_writer() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let fs_json = fs::read_to_string(root.join("src/fs_json.rs")).expect("read fs_json.rs");
    let runtime_report = fs::read_to_string(root.join("src/runtime_surface/report.rs"))
        .expect("read runtime report");
    let dev_persist = fs::read_to_string(root.join("src/dev_surface/audit_persist.rs"))
        .expect("read dev_surface/audit_persist.rs");
    let surface_matrix = fs::read_to_string(root.join("src/dev_surface/surface_matrix.rs"))
        .expect("read dev_surface/surface_matrix.rs");

    assert!(
        main_rs.contains("mod fs_json;"),
        "main.rs must register the shared fs_json module"
    );
    assert!(
        fs_json.contains("pub(crate) fn write_pretty_json_file"),
        "fs_json must own the shared pretty JSON file writer"
    );
    assert!(
        runtime_report.contains("use crate::fs_json::write_pretty_json_file;")
            && dev_persist.contains("use crate::fs_json::write_pretty_json_file;")
            && surface_matrix.contains("use crate::fs_json::write_pretty_json_file;"),
        "runtime report and dev audit/matrix persistence must share the common JSON file writer"
    );
}

#[test]
fn audits_share_rust_source_discovery_helpers() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let rust_source =
        fs::read_to_string(root.join("src/rust_source.rs")).expect("read rust_source.rs");
    let crate_audit =
        fs::read_to_string(root.join("src/crate_audit.rs")).expect("read crate_audit.rs");
    let kernel_audit = fs::read_to_string(root.join("src/kernel_boundary_audit.rs"))
        .expect("read kernel_boundary_audit.rs");
    let ownership_audit =
        fs::read_to_string(root.join("src/ownership_audit.rs")).expect("read ownership_audit.rs");
    let source_scan = fs::read_to_string(root.join("src/agent_exposure_audit/source_scan.rs"))
        .expect("read agent_exposure_audit/source_scan.rs");

    assert!(
        main_rs.contains("mod rust_source;"),
        "main.rs must register the shared rust_source module"
    );
    assert!(
        rust_source.contains("pub(crate) fn workspace_root")
            && rust_source.contains("pub(crate) fn relative_to_workspace")
            && rust_source.contains("pub(crate) fn relative_path")
            && rust_source.contains("pub(crate) fn rust_files_under")
            && rust_source.contains("pub(crate) fn line_count"),
        "rust_source must own shared workspace-relative path and Rust file discovery helpers"
    );
    assert!(
        crate_audit.contains("use crate::rust_source::{line_count, relative_to_workspace, rust_files_under, workspace_root};")
            && kernel_audit.contains("use crate::rust_source::{line_count, relative_to_workspace, rust_files_under, workspace_root};")
            && ownership_audit.contains("use crate::rust_source::{relative_path, rust_files_under, workspace_root};")
            && source_scan.contains("use crate::rust_source::rust_files_under;"),
        "audits and source scans must share the common rust_source helpers instead of reimplementing file discovery"
    );
}

#[test]
fn lifecycle_surface_uses_shared_exit_policy() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let cli_exit = fs::read_to_string(root.join("src/cli_exit.rs")).expect("read cli_exit.rs");
    let lifecycle_surface = fs::read_to_string(root.join("src/lifecycle_surface.rs"))
        .expect("read lifecycle_surface.rs");

    assert!(
        main_rs.contains("mod cli_exit;"),
        "main.rs must register the shared cli_exit module"
    );
    assert!(
        cli_exit.contains("pub(crate) fn success_if_status")
            && cli_exit.contains("pub(crate) fn success_if_status_in"),
        "cli_exit must own reusable status-based exit helpers"
    );
    assert!(
        lifecycle_surface
            .contains("use crate::cli_exit::{success_if_status, success_if_status_in};")
            && lifecycle_surface
                .contains("success_if_status_in(&report.status, &[\"ok\", \"planned\"], code)")
            && lifecycle_surface.contains("success_if_status(&report.status, \"ok\", code)"),
        "lifecycle_surface must use shared exit helpers instead of re-encoding local status policies"
    );
}

#[test]
fn lifecycle_surface_uses_shared_context_helpers() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let lifecycle_surface = fs::read_to_string(root.join("src/lifecycle_surface.rs"))
        .expect("read lifecycle_surface.rs");
    let lifecycle_context = fs::read_to_string(root.join("src/lifecycle_surface_context.rs"))
        .expect("read lifecycle_surface_context.rs");

    assert!(
        main_rs.contains("mod lifecycle_surface_context;"),
        "main.rs must register lifecycle_surface_context as a dedicated module"
    );
    assert!(
        lifecycle_surface.contains("use crate::lifecycle_surface_context::{runner, state_path};"),
        "lifecycle_surface.rs must delegate local runner/state context helpers"
    );
    assert!(
        lifecycle_context.contains("pub(crate) fn runner()")
            && lifecycle_context.contains("pub(crate) fn state_path("),
        "lifecycle surface context module must own runner and state path helpers"
    );
}

#[test]
fn lifecycle_surface_delegates_command_execution() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let lifecycle_surface = fs::read_to_string(root.join("src/lifecycle_surface.rs"))
        .expect("read lifecycle_surface.rs");
    let lifecycle_execute = fs::read_to_string(root.join("src/lifecycle_surface_execute.rs"))
        .expect("read lifecycle_surface_execute.rs");

    assert!(
        main_rs.contains("mod lifecycle_surface_execute;"),
        "main.rs must register lifecycle_surface_execute as a dedicated module"
    );
    assert!(
        lifecycle_surface.contains("use crate::lifecycle_surface_execute::{")
            && lifecycle_surface.contains("execute_status")
            && lifecycle_surface.contains("execute_kernel_release"),
        "lifecycle_surface.rs must delegate command execution to lifecycle_surface_execute"
    );
    assert!(
        lifecycle_execute.contains("pub(crate) fn execute_status")
            && lifecycle_execute.contains("pub(crate) fn execute_plan")
            && lifecycle_execute.contains("pub(crate) fn execute_gate")
            && lifecycle_execute.contains("pub(crate) fn execute_promote")
            && lifecycle_execute.contains("pub(crate) fn execute_report")
            && lifecycle_execute.contains("pub(crate) fn execute_repair")
            && lifecycle_execute.contains("pub(crate) fn execute_apply")
            && lifecycle_execute.contains("pub(crate) fn execute_kernel_release"),
        "lifecycle execution details must live in lifecycle_surface_execute.rs"
    );
}

#[test]
fn integrations_surface_delegates_rendering_and_shared_json() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let integrations_surface = fs::read_to_string(root.join("src/integrations_surface.rs"))
        .expect("read integrations_surface.rs");
    let integrations_readonly =
        fs::read_to_string(root.join("src/integrations_readonly_surface.rs"))
            .expect("read integrations_readonly_surface.rs");
    let integrations_render = fs::read_to_string(root.join("src/integrations_surface_render.rs"))
        .expect("read integrations_surface_render.rs");

    assert!(
        main_rs.contains("mod integrations_surface_render;"),
        "main.rs must register integrations_surface_render as a dedicated module"
    );
    assert!(
        integrations_surface.contains("use crate::cli_json::emit_pretty_json;")
            && integrations_surface.contains("use crate::integrations_surface_render::{")
            && integrations_surface.contains("print_integration_adopt_human")
            && integrations_surface.contains("print_integration_connect_human"),
        "integrations_surface.rs must use shared JSON emission and delegate human rendering"
    );
    assert!(
        integrations_readonly.contains("use crate::cli_json::emit_pretty_json;")
            && integrations_readonly.contains("use crate::integrations_surface_render::{")
            && integrations_readonly.contains("print_integrations_status_human")
            && integrations_readonly.contains("print_integrations_plan_human"),
        "integrations_readonly_surface.rs must use shared JSON emission and delegate readonly rendering"
    );
    assert!(
        integrations_render.contains("pub(crate) fn print_integrations_status_human")
            && integrations_render.contains("pub(crate) fn print_integrations_plan_human")
            && integrations_render.contains("pub(crate) fn print_integration_adopt_human")
            && integrations_render.contains("pub(crate) fn print_integration_connect_human"),
        "integration human rendering must live in integrations_surface_render.rs"
    );
}

#[test]
fn cli_runtime_surface_is_a_facade_over_granular_modules() {
    let root = crate_root();
    let runtime_surface =
        fs::read_to_string(root.join("src/runtime_surface.rs")).expect("read runtime_surface.rs");
    let paths_surface =
        fs::read_to_string(root.join("src/runtime_surface/paths.rs")).expect("read paths.rs");
    let suite_surface =
        fs::read_to_string(root.join("src/runtime_surface/suite.rs")).expect("read suite.rs");
    let report_surface =
        fs::read_to_string(root.join("src/runtime_surface/report.rs")).expect("read report.rs");
    let report_build = fs::read_to_string(root.join("src/runtime_surface/report/build.rs"))
        .expect("read report/build.rs");
    let report_catalog = fs::read_to_string(root.join("src/runtime_surface/report/catalog.rs"))
        .expect("read report/catalog.rs");
    let report_summary = fs::read_to_string(root.join("src/runtime_surface/report/summary.rs"))
        .expect("read report/summary.rs");
    let report_types = fs::read_to_string(root.join("src/runtime_surface/report/types.rs"))
        .expect("read report/types.rs");
    let inference_surface =
        fs::read_to_string(root.join("src/runtime_surface/inference_surface.rs"))
            .expect("read inference_surface.rs");
    let inference_hf = fs::read_to_string(root.join("src/runtime_surface/inference_surface/hf.rs"))
        .expect("read inference_surface/hf.rs");
    let inference_local =
        fs::read_to_string(root.join("src/runtime_surface/inference_surface/local_runtime.rs"))
            .expect("read inference_surface/local_runtime.rs");
    let inference_dispatch =
        fs::read_to_string(root.join("src/runtime_surface/inference_dispatch.rs"))
            .expect("read inference_dispatch.rs");
    let compat_aliases = fs::read_to_string(root.join("src/runtime_surface/compat_aliases.rs"))
        .expect("read compat_aliases.rs");
    let validation_surface = fs::read_to_string(root.join("src/runtime_surface/validation.rs"))
        .expect("read validation.rs");
    let validation_steps = fs::read_to_string(root.join("src/runtime_surface/validation/steps.rs"))
        .expect("read validation/steps.rs");
    let validation_predicates =
        fs::read_to_string(root.join("src/runtime_surface/validation/predicates.rs"))
            .expect("read validation/predicates.rs");
    let validation_fixtures =
        fs::read_to_string(root.join("src/runtime_surface/validation/proof_fixtures.rs"))
            .expect("read validation/proof_fixtures.rs");
    let validation_fs =
        fs::read_to_string(root.join("src/runtime_surface/validation/fs_assertions.rs"))
            .expect("read validation/fs_assertions.rs");

    assert!(
        runtime_surface.contains("mod paths;")
            && runtime_surface.contains("mod suite;")
            && runtime_surface.contains("mod host_checks;")
            && runtime_surface.contains("mod network_preflight;")
            && runtime_surface.contains("mod report;")
            && runtime_surface.contains("mod compat_aliases;")
            && runtime_surface.contains("mod inference_dispatch;")
            && runtime_surface.contains("mod validation;"),
        "runtime_surface.rs must stay a facade over granular runtime modules"
    );
    assert!(
        !runtime_surface.contains("RuntimeValidationReport")
            && !runtime_surface.contains("RuntimePathsView")
            && !runtime_surface.contains("ProcessCommand"),
        "runtime_surface.rs must not contain validation internals or path rendering details"
    );
    assert!(
        paths_surface.contains("RuntimePathsView")
            && paths_surface.contains("projection_class")
            && paths_surface.contains("resolve_runtime_paths"),
        "runtime paths rendering must live in runtime_surface/paths.rs"
    );
    assert!(
        suite_surface.contains("build_runtime_execution_suite")
            && suite_surface.contains("build_runtime_execution_plan"),
        "runtime suite and plan command adapters must live in runtime_surface/suite.rs"
    );
    assert!(
        inference_surface.contains("mod hf;")
            && inference_surface.contains("mod local_runtime;")
            && inference_surface.contains("pub(crate) use hf::")
            && inference_surface.contains("pub(crate) use local_runtime::"),
        "inference_surface.rs must stay a facade over hf and local_runtime projection modules"
    );
    assert!(
        inference_hf.contains("execute_inference_hf_download")
            && inference_hf.contains("HfDownloadArgs"),
        "HF-specific inference projection must live in runtime_surface/inference_surface/hf.rs"
    );
    assert!(
        inference_local.contains("execute_inference_start")
            && inference_local.contains("execute_inference_models_consolidate"),
        "local runtime inference projection must live in runtime_surface/inference_surface/local_runtime.rs"
    );
    assert!(
        inference_dispatch.contains("try_execute_inference")
            && inference_dispatch.contains("inference_surface::execute_inference"),
        "runtime inference command dispatch must live in runtime_surface/inference_dispatch.rs"
    );
    assert!(
        compat_aliases.contains("try_execute_compat_alias")
            && compat_aliases.contains("execute_deps_audit"),
        "runtime compatibility aliases must live in runtime_surface/compat_aliases.rs"
    );
    assert!(
        report_surface.contains("mod build;")
            && report_surface.contains("mod catalog;")
            && report_surface.contains("mod summary;")
            && report_surface.contains("mod types;")
            && report_surface.contains("build_runtime_report(DEFAULT_RUNTIME_REPORT_PATH)"),
        "runtime report facade must delegate build, catalog, summary, and types to dedicated submodules"
    );
    assert!(
        report_build.contains("pub(crate) fn build_runtime_report")
            && report_build.contains("required_reports_for_phase")
            && report_build.contains("sovereign_git_summary"),
        "runtime report assembly must live in runtime_surface/report/build.rs"
    );
    assert!(
        report_catalog.contains("pub(crate) fn required_reports_for_phase")
            && report_catalog.contains("pub(crate) fn promotion_gate_for_phase")
            && report_catalog.contains("pub(crate) fn report_surfaces"),
        "runtime report catalog tables must live in runtime_surface/report/catalog.rs"
    );
    assert!(
        report_summary.contains("pub(crate) fn inference_summary")
            && report_summary.contains("pub(crate) fn sovereign_git_summary"),
        "runtime report inference and sovereign git summaries must live in runtime_surface/report/summary.rs"
    );
    assert!(
        report_types.contains("pub(crate) struct RuntimeReport")
            && report_types.contains("pub(crate) struct RuntimeReportPhase")
            && report_types.contains("pub(crate) struct SovereignGitSummary"),
        "runtime report JSON types must live in runtime_surface/report/types.rs"
    );
    assert!(
        report_surface.lines().count() < 250,
        "runtime_surface/report.rs must remain a thin facade, not a report monolith"
    );
    assert!(
        report_surface.contains("/var/lib/cynic/reports/runtime-report.json")
            && report_surface.contains("written_path")
            && !report_surface.contains("~/.config")
            && !report_surface.contains("/home/user")
            && !report_surface.contains(".config/cynic"),
        "runtime report persistence must target installed runtime state, not user home"
    );
    assert!(
        validation_surface.contains("RuntimeValidationReport")
            && validation_surface.contains("mod steps;")
            && validation_surface.contains("mod predicates;")
            && validation_surface.contains("mod proof_fixtures;")
            && validation_surface.contains("mod fs_assertions;"),
        "runtime validation orchestrator must stay in runtime_surface/validation.rs"
    );
    assert!(
        validation_steps.contains("run_json_validation_step")
            && validation_steps.contains("RuntimeValidationStep"),
        "runtime validation command execution must live in validation/steps.rs"
    );
    assert!(
        validation_predicates.contains("suite_covers_cloud_prisms")
            && validation_predicates.contains("plan_consumes_network_runtime_proof"),
        "runtime validation JSON predicates must live in validation/predicates.rs"
    );
    assert!(
        validation_fixtures.contains("write_satisfied_host_identity_proof")
            && validation_fixtures.contains("write_satisfied_architecture_boundary_proof"),
        "runtime validation proof fixtures must live in validation/proof_fixtures.rs"
    );
    assert!(
        validation_fs.contains("validate_directory_contains")
            && validation_fs.contains("registry_dns_zone"),
        "runtime validation filesystem assertions must live in validation/fs_assertions.rs"
    );
}

#[test]
fn dev_surface_is_a_thin_facade_over_manifest_and_persistence_modules() {
    let root = crate_root();
    let dev_surface =
        fs::read_to_string(root.join("src/dev_surface.rs")).expect("read dev_surface.rs");
    let dev_manifest = fs::read_to_string(root.join("src/dev_surface/manifest.rs"))
        .expect("read dev_surface/manifest.rs");
    let dev_persist = fs::read_to_string(root.join("src/dev_surface/audit_persist.rs"))
        .expect("read dev_surface/audit_persist.rs");

    assert!(
        dev_surface.contains("mod manifest;")
            && dev_surface.contains("mod audit_persist;")
            && dev_surface.contains("execute_surfaces(json)")
            && dev_surface.contains("execute_framework(json)"),
        "dev_surface.rs must delegate protocol projections to dev_surface/manifest.rs"
    );
    assert!(
        dev_manifest.contains("pub(crate) fn execute_surfaces")
            && dev_manifest.contains("pub(crate) fn execute_outcomes")
            && dev_manifest.contains("pub(crate) fn execute_kpi_score")
            && dev_manifest.contains("pub(crate) fn execute_framework"),
        "development manifest projections must live in dev_surface/manifest.rs"
    );
    assert!(
        dev_persist.contains("pub(crate) fn maybe_persist_dev_audit_report")
            && dev_persist.contains("pub(crate) fn maybe_persist_kernel_audit_report")
            && dev_persist.contains("write_dev_audit_report"),
        "development audit persistence must live in dev_surface/audit_persist.rs"
    );
    assert!(
        dev_surface.lines().count() < 260,
        "dev_surface.rs must remain a thin facade, not a monolith"
    );
}

#[test]
fn cli_bootstrap_surface_is_granular_and_delegates_to_cynic_host() {
    let root = crate_root();
    let bootstrap_surface = fs::read_to_string(root.join("src/bootstrap_surface.rs"))
        .expect("read bootstrap_surface.rs");
    let host_surface =
        fs::read_to_string(root.join("src/bootstrap_surface/host.rs")).expect("read host.rs");
    let bridge_surface =
        fs::read_to_string(root.join("src/bootstrap_surface/bridge.rs")).expect("read bridge.rs");
    let selection_surface = fs::read_to_string(root.join("src/bootstrap_surface/selection.rs"))
        .expect("read selection.rs");

    assert!(
        bootstrap_surface.contains("mod host;")
            && bootstrap_surface.contains("mod bridge;")
            && bootstrap_surface.contains("mod constellation;")
            && bootstrap_surface.contains("pub(crate) mod selection;"),
        "bootstrap_surface.rs must stay a facade over granular modules"
    );
    assert!(
        host_surface.contains("use cynic_host::usecases::")
            && host_surface.contains("execute_host_apply"),
        "host bootstrap execution must delegate to cynic-host usecases"
    );
    assert!(
        bridge_surface.contains("use cynic_host::bridge as host_exec_bridge;"),
        "host-exec bridge behavior must be owned by cynic-host"
    );
    assert!(
        selection_surface.contains("resolve_current_host_profile")
            && selection_surface.contains("resolve_host_profile_resolution"),
        "profile selection must use registry-backed cynic-host resolution"
    );
}

#[test]
fn cli_delegates_network_runtime_to_cynic_node() {
    let root = crate_root();
    let network_surface =
        fs::read_to_string(root.join("src/network_surface.rs")).expect("read network_surface.rs");
    let network_base = fs::read_to_string(root.join("src/network_surface/base.rs"))
        .expect("read network_surface/base.rs");
    let network_dns = fs::read_to_string(root.join("src/network_surface/dns.rs"))
        .expect("read network_surface/dns.rs");
    let network_mesh = fs::read_to_string(root.join("src/network_surface/mesh.rs"))
        .expect("read network_surface/mesh.rs");
    let network_provider =
        fs::read_to_string(root.join("src/network_surface/provider_fallback.rs"))
            .expect("read network_surface/provider_fallback.rs");
    let network_actuator = fs::read_to_string(root.join("src/network_surface/actuator.rs"))
        .expect("read network_surface/actuator.rs");
    assert!(
        network_surface.contains("use cynic_node::NetworkCommands;"),
        "network command model must be owned by cynic-node, not cynic-cli"
    );
    assert!(
        network_surface.contains("mod base;")
            && network_surface.contains("mod dns;")
            && network_surface.contains("mod mesh;")
            && network_surface.contains("mod provider_fallback;")
            && network_surface.contains("mod actuator;"),
        "network surface must stay split by base, dns, mesh, provider_fallback, and actuator responsibilities"
    );
    assert!(
        network_base.contains("cynic_node::execute_network_command")
            && network_dns.contains("cynic_node::execute_network_command")
            && network_mesh.contains("cynic_node::execute_network_command")
            && network_provider.contains("cynic_node::execute_network_command")
            && network_actuator.contains("cynic_node::execute_network_command"),
        "network execution must delegate to cynic-node from dedicated network_surface modules"
    );
    assert!(
        network_mesh.contains("fn emit_mesh_status(") && network_surface.lines().count() < 320,
        "mesh status projection must live outside the network facade and network_surface.rs must remain thin"
    );
}

#[test]
fn surface_audit_is_split_into_reusable_audit_modules() {
    let root = crate_root();
    let surface_audit =
        fs::read_to_string(root.join("src/surface_audit.rs")).expect("read surface_audit.rs");
    let surface_types = fs::read_to_string(root.join("src/surface_audit/types.rs"))
        .expect("read surface_audit/types.rs");
    let surface_checks = fs::read_to_string(root.join("src/surface_audit/checks.rs"))
        .expect("read surface_audit/checks.rs");
    let surface_external =
        fs::read_to_string(root.join("src/surface_audit/external_projection.rs"))
            .expect("read surface_audit/external_projection.rs");
    let surface_paths = fs::read_to_string(root.join("src/surface_audit/paths.rs"))
        .expect("read surface_audit/paths.rs");

    assert!(
        surface_audit.contains("mod checks;")
            && surface_audit.contains("mod external_projection;")
            && surface_audit.contains("mod paths;")
            && surface_audit.contains("mod types;"),
        "surface audit must split checks, external projections, paths, and types into dedicated modules"
    );
    assert!(
        surface_audit.contains("pub(crate) use types::")
            && surface_audit.contains("audit_external_projection_files")
            && surface_audit.contains("audit_surface(surface, &ids)"),
        "surface_audit.rs must remain a thin assembly facade over reusable audit modules"
    );
    assert!(
        surface_types.contains("pub(crate) struct SurfaceAuditReport")
            && surface_types.contains("pub(crate) struct ExternalProjectionFinding"),
        "surface audit JSON types must live in surface_audit/types.rs"
    );
    assert!(
        surface_checks.contains("pub(crate) fn audit_surface")
            && surface_checks.contains("check_mutation_policy_matches_source")
            && surface_checks.contains("mutation_signals"),
        "surface audit rule evaluation must live in surface_audit/checks.rs"
    );
    assert!(
        surface_external.contains("pub(crate) fn audit_external_projection_files")
            && surface_external.contains("audit_projection_file"),
        "external projection auditing must live in surface_audit/external_projection.rs"
    );
    assert!(
        surface_paths.contains("pub(crate) fn read_adapter_source")
            && surface_paths.contains("pub(crate) fn adapter_module_path")
            && surface_paths.contains("pub(crate) fn owner_module_path"),
        "surface audit path and source discovery must live in surface_audit/paths.rs"
    );
    assert!(
        surface_audit.lines().count() < 120,
        "surface_audit.rs must remain a thin facade, not a monolith"
    );
}

#[test]
fn agent_exposure_audit_is_split_into_scan_and_check_modules() {
    let root = crate_root();
    let audit = fs::read_to_string(root.join("src/agent_exposure_audit.rs"))
        .expect("read agent_exposure_audit.rs");
    let types = fs::read_to_string(root.join("src/agent_exposure_audit/types.rs"))
        .expect("read agent_exposure_audit/types.rs");
    let json_scan = fs::read_to_string(root.join("src/agent_exposure_audit/json_scan.rs"))
        .expect("read agent_exposure_audit/json_scan.rs");
    let source_scan = fs::read_to_string(root.join("src/agent_exposure_audit/source_scan.rs"))
        .expect("read agent_exposure_audit/source_scan.rs");
    let checks = fs::read_to_string(root.join("src/agent_exposure_audit/checks.rs"))
        .expect("read agent_exposure_audit/checks.rs");

    assert!(
        audit.contains("mod checks;")
            && audit.contains("mod json_scan;")
            && audit.contains("mod source_scan;")
            && audit.contains("mod types;"),
        "agent exposure audit must split checks, json_scan, source_scan, and types into dedicated modules"
    );
    assert!(
        audit.contains("pub(crate) use types::{AgentExposureAuditReport, AgentExposureFinding};")
            && audit.contains("scan_mcp_tool_names")
            && audit.contains("check_agent_env_paths"),
        "agent_exposure_audit.rs must remain a thin assembly facade over reusable scan and check modules"
    );
    assert!(
        types.contains("pub(crate) struct AgentExposureAuditReport")
            && types.contains("pub(crate) struct AgentExposureFinding"),
        "agent exposure audit JSON types must live in agent_exposure_audit/types.rs"
    );
    assert!(
        json_scan.contains("pub(crate) fn load_json")
            && json_scan.contains("pub(crate) fn exposed_tools")
            && json_scan.contains("pub(crate) fn count_tiers"),
        "agent exposure JSON scanning must live in agent_exposure_audit/json_scan.rs"
    );
    assert!(
        source_scan.contains("pub(crate) fn scan_mcp_tool_names")
            && source_scan.contains("pub(crate) fn extract_tool_names")
            && source_scan.contains("parse_tool_name"),
        "agent exposure source scanning must live in agent_exposure_audit/source_scan.rs"
    );
    assert!(
        checks.contains("pub(crate) fn check_no_machine_local_paths")
            && checks.contains("pub(crate) fn check_docs_quarantine")
            && checks.contains("pub(crate) fn check_agent_env_paths"),
        "agent exposure path and quarantine checks must live in agent_exposure_audit/checks.rs"
    );
    assert!(
        audit.lines().count() < 220,
        "agent_exposure_audit.rs must remain a thin facade, not a monolith"
    );
}

#[test]
fn cli_host_exec_alias_is_a_bounded_runtime_surface() {
    let root = crate_root();
    let alias_surface = fs::read_to_string(root.join("src/host_exec_alias_surface.rs"))
        .expect("read host_exec_alias_surface.rs");
    assert!(
        alias_surface.contains("cynic-host-exec")
            && alias_surface.contains("bridge-status")
            && alias_surface.contains("network-baseline"),
        "host-exec alias must expose only bounded runtime profiles"
    );
    assert!(
        alias_surface.contains("use cynic_host::bridge;")
            && alias_surface.contains("observe_host_with_resolution"),
        "host-exec alias must delegate runtime behavior to cynic-host"
    );
}

#[test]
fn cli_fleet_and_transport_surfaces_are_thin_adapters() {
    let root = crate_root();
    let fleet_surface =
        fs::read_to_string(root.join("src/fleet_surface.rs")).expect("read fleet_surface.rs");
    let transport_surface = fs::read_to_string(root.join("src/transport_surface.rs"))
        .expect("read transport_surface.rs");
    assert!(
        fleet_surface.contains("fleet_report::emit_fleet_report")
            && fleet_surface.contains("resolve_runtime_paths"),
        "fleet surface must delegate reporting and runtime path resolution"
    );
    assert!(
        transport_surface.contains("use cynic_host::transport;")
            && transport_surface.contains("build_transport_matrix_report"),
        "transport surface must delegate to cynic-host transport"
    );
}

#[test]
fn commands_manifest_exposes_canonical_surface_boundaries() {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_cynic"));
    let output = Command::new(&source)
        .args(["commands", "--json"])
        .output()
        .expect("run cynic commands --json");
    assert!(output.status.success(), "commands manifest must execute");
    let manifest: Value = serde_json::from_slice(&output.stdout).expect("parse commands manifest");

    let surfaces = manifest
        .get("canonical_surfaces")
        .and_then(Value::as_array)
        .expect("canonical_surfaces array");
    let domains = manifest
        .get("domains")
        .and_then(Value::as_array)
        .expect("domains array");

    assert!(
        surfaces.len() >= domains.len(),
        "canonical surfaces must cover at least the public command domains"
    );

    let mut ids = std::collections::BTreeSet::new();
    for surface in surfaces {
        let id = surface
            .get("id")
            .and_then(Value::as_str)
            .expect("surface id");
        assert!(ids.insert(id.to_string()), "duplicate surface id {id}");
        assert!(
            surface
                .get("owner_crate")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty()),
            "surface {id} must declare owner_crate"
        );
        assert!(
            surface
                .get("domain_owner")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty()),
            "surface {id} must declare domain_owner"
        );
        assert!(
            surface
                .get("mutation_policy")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty()),
            "surface {id} must declare mutation_policy"
        );
        assert!(
            surface
                .get("anti_responsibilities")
                .and_then(Value::as_array)
                .is_some_and(|values| !values.is_empty()),
            "surface {id} must declare anti_responsibilities"
        );
        assert!(
            surface
                .get("owner_modules")
                .and_then(Value::as_array)
                .is_some_and(|values| !values.is_empty()),
            "surface {id} must declare owner_modules"
        );
        assert!(
            surface
                .get("owner_call_markers")
                .and_then(Value::as_array)
                .is_some_and(|values| !values.is_empty()),
            "surface {id} must declare owner_call_markers"
        );
        assert!(
            surface
                .get("canonical_entrypoints")
                .and_then(Value::as_array)
                .is_some_and(|values| !values.is_empty()),
            "surface {id} must declare canonical_entrypoints"
        );
    }

    for required in [
        "runtime_paths",
        "runtime_execution_suite",
        "runtime_host_checks",
        "runtime_validation",
        "host_bootstrap",
        "framework_bootstrap",
        "network_runtime",
        "portable_lab",
        "agent_development",
    ] {
        assert!(
            ids.contains(required),
            "missing canonical surface {required}"
        );
    }
}

#[test]
fn canonical_surface_audit_executes_against_real_modules() {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_cynic"));
    let output = Command::new(&source)
        .args(["dev", "surface-audit", "--json"])
        .output()
        .expect("run cynic dev surface-audit --json");
    assert!(output.status.success(), "surface audit must pass");
    let report: Value = serde_json::from_slice(&output.stdout).expect("parse surface audit");
    assert_eq!(
        report.get("schema").and_then(Value::as_str),
        Some("cynic.cli.surface_audit.v1")
    );
    assert_eq!(report.get("status").and_then(Value::as_str), Some("ok"));
    assert_eq!(report.get("blocked_count").and_then(Value::as_u64), Some(0));
    assert!(
        report
            .get("surfaces")
            .and_then(Value::as_array)
            .is_some_and(|surfaces| surfaces.iter().all(|surface| {
                surface.get("status").and_then(Value::as_str) == Some("ok")
                    && surface
                        .get("owner_module_paths")
                        .and_then(Value::as_array)
                        .is_some_and(|paths| !paths.is_empty())
                    && surface
                        .get("checks")
                        .and_then(Value::as_array)
                        .is_some_and(|checks| {
                            !checks.is_empty()
                                && checks.iter().any(|check| {
                                    check.get("name").and_then(Value::as_str)
                                        == Some("owner_calls_present")
                                })
                        })
            })),
        "all declared surfaces must have passing checks"
    );
    assert!(
        report
            .get("external_projection_findings")
            .and_then(Value::as_array)
            .is_some_and(|findings| {
                findings.iter().any(|finding| {
                    finding.get("id").and_then(Value::as_str) == Some("systemd_kernel_user_unit")
                        && finding.get("status").and_then(Value::as_str) == Some("ok")
                        && finding.get("projection_class").and_then(Value::as_str)
                            == Some("operator_projection")
                }) && findings.iter().any(|finding| {
                    finding.get("id").and_then(Value::as_str) == Some("systemd_node_user_unit")
                        && finding.get("status").and_then(Value::as_str) == Some("ok")
                        && finding.get("projection_class").and_then(Value::as_str)
                            == Some("operator_projection")
                }) && findings.iter().any(|finding| {
                    finding.get("id").and_then(Value::as_str)
                        == Some("systemd_kernel_installed_unit")
                        && finding.get("status").and_then(Value::as_str) == Some("ok")
                        && finding.get("projection_class").and_then(Value::as_str)
                            == Some("runtime_authority_adapter")
                }) && findings.iter().any(|finding| {
                    finding.get("id").and_then(Value::as_str) == Some("systemd_node_installed_unit")
                        && finding.get("status").and_then(Value::as_str) == Some("ok")
                        && finding.get("projection_class").and_then(Value::as_str)
                            == Some("runtime_authority_adapter")
                })
            }),
        "systemd user units and installed system units must expose distinct projection classes"
    );
}

#[test]
fn host_bootstrap_e2e_emits_non_mutating_runtime_contract() {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_cynic"));
    let registry = crate_root()
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("infra/registries/single-node.json");
    let output = Command::new(&source)
        .env("CYNIC_HOST_PROBE_TIMEOUT_MS", "200")
        .args([
            "bootstrap",
            "host",
            "e2e",
            "--node",
            "local-kernel",
            "--registry",
            registry.to_str().expect("registry path"),
            "--runtime-profile",
            "single-node",
            "--dry-run",
        ])
        .output()
        .expect("run cynic bootstrap host e2e");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "parse e2e json: {error}; status={:?}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    assert_eq!(
        report.get("schema").and_then(Value::as_str),
        Some("cynic.host_bootstrap.e2e.v1")
    );
    assert_eq!(report.get("mode").and_then(Value::as_str), Some("dry_run"));
    assert!(
        matches!(
            report.get("status").and_then(Value::as_str),
            Some("ok" | "blocked")
        ),
        "e2e status must be explicit"
    );
    assert_eq!(
        report
            .pointer("/apply_execution/dry_run")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        report
            .pointer("/apply_execution/commands")
            .and_then(Value::as_array)
            .is_some_and(|commands| commands.iter().all(|command| {
                command.get("executed").and_then(Value::as_bool) == Some(false)
            })),
        "host e2e must not execute commands in dry-run mode"
    );
    assert_eq!(
        report
            .pointer("/runtime_plan/schema")
            .and_then(Value::as_str),
        Some("cynic.runtime_execution_plan.v1")
    );
}

#[test]
fn cynic_binary_can_act_as_bounded_host_exec_alias() {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_cynic"));
    let temp =
        std::env::temp_dir().join(format!("cynic-host-exec-alias-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).expect("create temp dir");
    let alias = temp.join("cynic-host-exec");
    fs::copy(&source, &alias).expect("copy cynic binary as host-exec alias");

    let output = Command::new(&alias)
        .arg("bridge-status")
        .output()
        .expect("run host-exec alias");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"schema\": \"cynic.host_exec_bridge.status.v1\""));
    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn surface_registry_is_split_into_catalog_and_types() {
    let root = crate_root();
    let registry =
        fs::read_to_string(root.join("src/surface_registry.rs")).expect("read surface_registry.rs");
    let registry_types = fs::read_to_string(root.join("src/surface_registry/types.rs"))
        .expect("read surface_registry/types.rs");
    let registry_catalog = fs::read_to_string(root.join("src/surface_registry/catalog.rs"))
        .expect("read surface_registry/catalog.rs");
    let protocol_surface_registry =
        fs::read_to_string(root.join("../cynic-protocol/src/surface_registry.rs"))
            .expect("read cynic-protocol/src/surface_registry.rs");
    let infra_surface_registry = fs::read_to_string(root.join("../../infra/surface-registry.json"))
        .expect("read infra/surface-registry.json");
    let infra_surface_manifest =
        fs::read_to_string(root.join("../../infra/surface-registry/manifest.json"))
            .expect("read infra/surface-registry/manifest.json");
    let parsed_manifest = serde_json::from_str::<Value>(&infra_surface_manifest)
        .expect("surface registry manifest must be valid JSON");
    let manifest_entries = parsed_manifest
        .get("surfaces")
        .and_then(Value::as_array)
        .expect("manifest surface entries must be an array");
    let mut manifest_ids = BTreeSet::new();

    for entry in manifest_entries {
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .expect("manifest surface entry must contain an id");
        let path = entry
            .get("path")
            .and_then(Value::as_str)
            .expect("manifest surface entry must contain a path");
        let fragment_path = root.join("../../infra/surface-registry/").join(path);
        let fragment = fs::read_to_string(&fragment_path).expect("read surface registry fragment");
        let fragment_object = serde_json::from_str::<Value>(&fragment)
            .expect("surface registry fragment must be valid JSON");
        let fragment_id = fragment_object
            .get("id")
            .and_then(Value::as_str)
            .expect("surface registry fragment must contain an id");
        assert_eq!(fragment_id, id);
        assert!(fragment_path.exists());
        assert!(
            manifest_ids.insert(id.to_string()),
            "manifest id duplicates are not allowed"
        );
    }

    let parsed_assembled_registry = serde_json::from_str::<Value>(&infra_surface_registry)
        .expect("infra/surface-registry.json must be valid JSON");
    let assembled_ids: BTreeSet<String> = parsed_assembled_registry
        .get("surfaces")
        .and_then(Value::as_array)
        .expect("assembled registry must expose a surfaces array")
        .iter()
        .filter_map(|surface| {
            surface
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect();

    assert!(
        registry.contains("mod catalog;")
            && registry.contains("mod types;")
            && registry.contains("pub(crate) use catalog::canonical_cli_surfaces;")
            && registry.contains("pub(crate) use types::CanonicalCliSurface;"),
        "surface_registry.rs must remain a thin facade over catalog and types"
    );
    assert!(
        registry_types.contains("pub(crate) use cynic_protocol::CanonicalCliSurface;")
            && protocol_surface_registry.contains("pub struct CanonicalCliSurface")
            && protocol_surface_registry.contains("pub struct SurfaceRegistryDocument"),
        "surface registry shape must live in cynic-protocol and be re-exported by the CLI"
    );
    assert!(
        registry_catalog.contains("surface-registry/manifest.json")
            && registry_catalog.contains("env!(\"CARGO_MANIFEST_DIR\")")
            && registry_catalog.contains("SURFACE_REGISTRY_FRAGMENT_INDEX")
            && registry_catalog.contains("pub(crate) fn canonical_cli_surfaces()")
            && (infra_surface_registry.contains("\"schema\":\"cynic.surface_registry.v1\"")
                || infra_surface_registry.contains("\"schema\": \"cynic.surface_registry.v1\""))
            && parsed_manifest
                .get("schema")
                .and_then(Value::as_str)
                .is_some_and(|value| value == cynic_protocol::SURFACE_REGISTRY_SCHEMA)
            && registry.lines().count() < 40,
        "surface registry catalog must load canonical data from infra and the facade must remain thin"
    );

    assert!(
        manifest_entries.len() == assembled_ids.len(),
        "manifest surface count must match assembled registry surface count"
    );
    assert_eq!(
        manifest_ids, assembled_ids,
        "surface ids in manifest must match ids in assembled surface registry"
    );
}
