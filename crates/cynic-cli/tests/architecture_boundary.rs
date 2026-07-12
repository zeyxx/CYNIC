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
            && root.join("src/runtime_surface/validation.rs").exists(),
        "runtime surface must remain split by paths, suite, host_checks, and validation responsibilities"
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
        paths_surface.contains("RuntimePathsView") && paths_surface.contains("RuntimeMode"),
        "runtime paths rendering must live in runtime_surface/paths.rs"
    );
    assert!(
        suite_surface.contains("build_runtime_execution_suite")
            && suite_surface.contains("build_runtime_execution_plan"),
        "runtime suite and plan command adapters must live in runtime_surface/suite.rs"
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
    assert!(
        network_surface.contains("use cynic_node::NetworkCommands;"),
        "network command model must be owned by cynic-node, not cynic-cli"
    );
    assert!(
        network_surface.contains("cynic_node::execute_network_command"),
        "network execution must delegate to cynic-node"
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
