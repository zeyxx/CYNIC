use super::layers::runtime_execution_layers;
use super::types::{
    RUNTIME_EXECUTION_PLAN_SCHEMA, RUNTIME_EXECUTION_SUITE_SCHEMA, RuntimeExecutionCommand,
    RuntimeExecutionLayer, RuntimeExecutionPlan, RuntimePlannedExecutionLayer,
    RuntimeRegistrySummary,
};

pub fn runtime_missing_gates(
    entry_gate: &[&'static str],
    observed_gates: &[String],
) -> Vec<&'static str> {
    entry_gate
        .iter()
        .copied()
        .filter(|gate| !observed_gates.iter().any(|observed| observed == gate))
        .collect()
}

pub fn runtime_initial_observed_gates() -> Vec<String> {
    vec!["registry_json_valid".to_string()]
}

pub fn build_runtime_execution_plan_from_gates(
    registry_path: &str,
    profile: &str,
    registry_summary: RuntimeRegistrySummary,
    observed_gates: Vec<String>,
) -> RuntimeExecutionPlan {
    let mut observed_gates = observed_gates;
    let mut planned_layers = Vec::new();
    let mut executable_commands = Vec::new();

    for layer in runtime_execution_layers(registry_path) {
        let missing_entry_gate = runtime_missing_gates(&layer.entry_gate, &observed_gates);
        let read_only_ready = missing_entry_gate.is_empty();
        let dry_run_ready = read_only_ready && layer.dry_run.is_some();
        let status = planned_layer_status(&missing_entry_gate, &layer);
        let reason = planned_layer_reason(&missing_entry_gate, &layer);
        let read_only_probe = RuntimeExecutionCommand {
            kind: "read_only_probe",
            argv: layer.read_only_probe.clone(),
            mutates: false,
            implemented: true,
        };
        let dry_run = layer.dry_run.as_ref().map(|argv| RuntimeExecutionCommand {
            kind: "dry_run",
            argv: argv.clone(),
            mutates: false,
            implemented: runtime_dry_run_is_implemented(argv),
        });
        let apply = layer.apply.as_ref().map(|argv| RuntimeExecutionCommand {
            kind: "apply",
            argv: argv.clone(),
            mutates: true,
            implemented: false,
        });

        if read_only_ready {
            executable_commands.push(read_only_probe.clone());
        }
        if dry_run_ready
            && let Some(command) = &dry_run
            && command.implemented
        {
            executable_commands.push(command.clone());
        }
        if layer.id == "l0_registry_and_paths" && read_only_ready {
            observed_gates
                .extend(["runtime_paths_resolved", "registry_consumed"].map(str::to_string));
        }

        planned_layers.push(RuntimePlannedExecutionLayer {
            id: layer.id,
            name: layer.name,
            status,
            reason,
            entry_gate: layer.entry_gate,
            missing_entry_gate,
            read_only_probe,
            dry_run,
            apply,
            exit_gate: layer.exit_gate,
            can_execute_read_only: read_only_ready,
            can_dry_run: dry_run_ready,
            can_apply: false,
        });
    }

    let status = if planned_layers.iter().any(|layer| layer.status == "blocked") {
        "planned_with_blockers"
    } else {
        "ready"
    };

    observed_gates.sort();
    observed_gates.dedup();

    RuntimeExecutionPlan {
        schema: RUNTIME_EXECUTION_PLAN_SCHEMA,
        status,
        registry_path: registry_path.to_string(),
        profile: profile.to_string(),
        suite_schema: RUNTIME_EXECUTION_SUITE_SCHEMA,
        rule: "the plan is a non-mutating projection; apply commands remain disabled until runtime proof records satisfy entry gates",
        proof_model: "current implementation derives registry/path gates at plan build and requires observed runtime evidence for host, network, storage, service, kernel, and agent gates",
        registry_summary,
        observed_gates,
        layers: planned_layers,
        executable_commands,
    }
}

fn planned_layer_status(
    missing_entry_gate: &[&'static str],
    layer: &RuntimeExecutionLayer,
) -> &'static str {
    if !missing_entry_gate.is_empty() {
        return "blocked";
    }
    if layer.apply.is_some() {
        "ready_for_probe_and_dry_run"
    } else if layer.dry_run.is_some() {
        "ready_for_probe_and_artifact"
    } else {
        "ready_for_probe"
    }
}

fn planned_layer_reason(
    missing_entry_gate: &[&'static str],
    layer: &RuntimeExecutionLayer,
) -> String {
    if missing_entry_gate.is_empty() {
        return format!(
            "entry gates satisfied; execute read-only probe before considering exit gates: {}",
            layer.exit_gate.join(", ")
        );
    }
    format!(
        "missing entry gates: {}; previous layer must produce durable runtime proof first",
        missing_entry_gate.join(", ")
    )
}

fn runtime_dry_run_is_implemented(argv: &[String]) -> bool {
    argv.first().is_none_or(|program| program != "surreal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_gates_are_derived_from_observed_runtime_evidence() {
        assert_eq!(
            runtime_missing_gates(
                &["registry_json_valid", "host_identity_observed"],
                &["registry_json_valid".to_string()]
            ),
            vec!["host_identity_observed"]
        );
    }

    #[test]
    fn runtime_planner_opens_layers_from_observed_gates() {
        let summary = RuntimeRegistrySummary {
            node_count: 1,
            roles: vec!["kernel_primary".to_string()],
            service_count: 3,
            dns_zone: Some("localhost".to_string()),
            provider_cidr: None,
        };

        let no_proof = build_runtime_execution_plan_from_gates(
            "infra/registries/single-node.json",
            "single-node",
            summary.clone(),
            runtime_initial_observed_gates(),
        );
        let l2 = no_proof
            .layers
            .iter()
            .find(|layer| layer.id == "l2_network_projection")
            .expect("l2 exists");
        assert_eq!(l2.status, "blocked");
        assert_eq!(l2.missing_entry_gate, vec!["host_identity_observed"]);

        let with_host = build_runtime_execution_plan_from_gates(
            "infra/registries/single-node.json",
            "single-node",
            summary.clone(),
            vec![
                "registry_json_valid".to_string(),
                "host_identity_observed".to_string(),
            ],
        );
        let l2 = with_host
            .layers
            .iter()
            .find(|layer| layer.id == "l2_network_projection")
            .expect("l2 exists");
        assert_eq!(l2.status, "ready_for_probe_and_dry_run");
        assert!(l2.missing_entry_gate.is_empty());

        let with_network = build_runtime_execution_plan_from_gates(
            "infra/registries/single-node.json",
            "single-node",
            summary,
            vec![
                "registry_json_valid".to_string(),
                "host_identity_observed".to_string(),
                "network_runtime_observed".to_string(),
            ],
        );
        let l3 = with_network
            .layers
            .iter()
            .find(|layer| layer.id == "l3_storage_projection")
            .expect("l3 exists");
        assert_eq!(l3.status, "ready_for_probe_and_artifact");
        assert!(l3.missing_entry_gate.is_empty());
    }
}
