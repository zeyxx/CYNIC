"""Registry and lifecycle policy access for organ-forge."""

from __future__ import annotations

import json
from pathlib import Path

from organs.contracts.src.surface_contracts import read_json_file_or_default
from ..core.project_root import resolve_project_root_from_script

from ..core.lifecycle_state import validate_workflow_layers
from ..core.lifecycle_transition_debt import build_lifecycle_transition_debt_report
from ..core.lifecycle_runtime_artifacts import build_runtime_artifact_manifest


def _default_registry_path() -> Path:
    return resolve_project_root_from_script(__file__) / "infra" / "registry.json"


def load_registry(path: str | Path | None = None) -> dict:
    registry_path = Path(path) if path else _default_registry_path()
    return read_json_file_or_default(registry_path, {})


def required_lifecycle(registry: dict, name: str) -> dict:
    lifecycle = registry.get("lifecycle", {}).get(name)
    if not lifecycle:
        raise RuntimeError(f"registry missing lifecycle.{name}")
    return lifecycle


def required_node(
    registry: dict,
    role: str,
    deploy_target_flag: str | None = None,
) -> dict:
    for node in registry.get("nodes", []):
        if node.get("role") != role:
            continue
        if deploy_target_flag and node.get(deploy_target_flag) is not True:
            continue
        return node
    suffix = f" with {deploy_target_flag}=true" if deploy_target_flag else ""
    raise RuntimeError(f"registry missing node role={role}{suffix}")


def validate_lifecycle_policy(registry: dict | None = None) -> dict:
    registry = registry or load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")

    nodes = registry.get("nodes", [])
    organs = registry.get("organs", [])
    build_role = lifecycle["build_runner_role"]
    deploy_role = lifecycle["deploy_target_role"]
    deploy_flag = lifecycle.get("deploy_target_flag", "deploy_target")

    build_node = next((node for node in nodes if node.get("role") == build_role), None)
    deploy_node = next(
        (
            node
            for node in nodes
            if node.get("role") == deploy_role and node.get(deploy_flag) is True
        ),
        None,
    )
    organ = next(
        (item for item in organs if item.get("name") == lifecycle["orchestrator_organ"]),
        None,
    )

    errors = []
    if not build_node:
        errors.append("lifecycle build runner role does not resolve")
    if not deploy_node:
        errors.append("lifecycle deploy target role does not resolve")
    if not organ:
        errors.append("lifecycle orchestrator organ does not resolve")
    if not lifecycle.get("task_kind") or not lifecycle.get("task_domain"):
        errors.append("lifecycle task routing incomplete")
    if not isinstance(lifecycle.get("repo_targets"), list) or not lifecycle["repo_targets"]:
        errors.append("lifecycle repo_targets missing")

    host_native_roles = {"cortex_host", "inference_node", "hypervisor"}
    executable_host_transport_kinds = {"service_agent_surface", "ssh_sudo_helper"}
    for node in nodes:
        role = node.get("role")
        if role not in host_native_roles:
            continue
        transport = node.get("host_exec_transport")
        if not isinstance(transport, dict):
            errors.append(f"nodes.{node.get('name', '<unknown>')}.host_exec_transport missing")
            continue
        if transport.get("schema") != "cynic.host_exec_transport.v1":
            errors.append(
                f"nodes.{node.get('name', '<unknown>')}.host_exec_transport.schema must be cynic.host_exec_transport.v1"
            )
        if not transport.get("kind"):
            errors.append(f"nodes.{node.get('name', '<unknown>')}.host_exec_transport.kind missing")
        if not transport.get("status"):
            errors.append(f"nodes.{node.get('name', '<unknown>')}.host_exec_transport.status missing")
        if transport.get("principal") != "cynic":
            errors.append(
                f"nodes.{node.get('name', '<unknown>')}.host_exec_transport.principal must be cynic"
            )
        if not transport.get("required_bridge"):
            errors.append(
                f"nodes.{node.get('name', '<unknown>')}.host_exec_transport.required_bridge missing"
            )
        if not transport.get("reason"):
            errors.append(
                f"nodes.{node.get('name', '<unknown>')}.host_exec_transport.reason missing"
            )
        if transport.get("kind") in executable_host_transport_kinds and not (
            transport.get("transport_command_env") or transport.get("transport_command_default")
        ):
            errors.append(
                f"nodes.{node.get('name', '<unknown>')}.host_exec_transport executable kinds require transport_command_env or transport_command_default"
            )

    intents = lifecycle.get("intents", {})
    if not isinstance(intents, dict) or not intents:
        errors.append("lifecycle intents missing")
    for intent_name, intent_policy in intents.items():
        if not isinstance(intent_policy, dict):
            errors.append(f"intents.{intent_name} must be an object")
            continue
        if not intent_policy.get("objective"):
            errors.append(f"intents.{intent_name}.objective missing")
        if not isinstance(intent_policy.get("deploy"), bool):
            errors.append(f"intents.{intent_name}.deploy must be boolean")

    profiles = lifecycle.get("request_profiles", {})
    if not isinstance(profiles, dict) or not profiles:
        errors.append("lifecycle request_profiles missing")
    for profile_name, profile_intents in profiles.items():
        if not isinstance(profile_intents, list) or not profile_intents:
            errors.append(f"request_profiles.{profile_name} must be a non-empty intent list")
            continue
        for intent in profile_intents:
            if intent not in intents:
                errors.append(f"request_profiles.{profile_name} unknown intent {intent}")

    workflow = lifecycle.get("workflow", {})
    errors.extend(
        validate_workflow_layers(
            workflow,
            lifecycle["orchestrator_organ"],
        )
    )
    stages = workflow.get("canonical_stages", [])
    if not isinstance(stages, list) or len(stages) < 3:
        errors.append("lifecycle workflow.canonical_stages must define the canonical workflow")
    else:
        stage_names = [stage.get("name") for stage in stages if isinstance(stage, dict)]
        for required_stage in [
            "local_validation",
            "activation_request_generation",
            "activation_request_dispatch",
            "gate_2_request",
            "mcp_reload_request",
            "deploy_request",
        ]:
            if required_stage not in stage_names:
                errors.append(f"lifecycle workflow missing canonical stage {required_stage}")
        for stage in stages:
            if not isinstance(stage, dict):
                errors.append("lifecycle workflow canonical stages must be objects")
                continue
            if not stage.get("name"):
                errors.append("lifecycle workflow canonical stage name missing")
            if not stage.get("scope"):
                errors.append(f"lifecycle workflow stage {stage.get('name', '<unknown>')} scope missing")
            if not stage.get("make_target"):
                errors.append(f"lifecycle workflow stage {stage.get('name', '<unknown>')} make_target missing")
    planned_policy = workflow.get("planned_policy", {})
    if planned_policy.get("blocks_workflow_critical_edits") is not True:
        errors.append("lifecycle workflow planned_policy.blocks_workflow_critical_edits must be true")
    critical_targets = planned_policy.get("critical_targets", [])
    if not isinstance(critical_targets, list) or not critical_targets:
        errors.append("lifecycle workflow planned_policy.critical_targets missing")
    else:
        for target in [
            "infra/registry.json",
            "Makefile",
            "crates/cynic-kernel",
            "crates/cynic-mcp",
            "organs/forge",
            "organs/forge/src/adapters/kernel_adapter.py",
            "organs/forge/src/core/lifecycle_workflow.py",
            "organs/forge/src/core/lifecycle_payloads.py",
            "organs/forge/src/core/lifecycle_runtime.py",
            "organs/forge/src/core/lifecycle_evidence.py",
            "organs/forge/src/core/lifecycle_activation.py",
            "organs/forge/src/core/lifecycle_readiness.py",
        ]:
            if target not in critical_targets:
                errors.append(f"lifecycle workflow planned_policy missing critical target {target}")

    runtime_paths = lifecycle.get("runtime_paths", {})
    runtime_path_proofs = lifecycle.get("runtime_path_proofs", {})
    build_policy = lifecycle.get("build_policy", {})
    bootstrap_policy = build_policy.get("bootstrap", {})

    if build_policy.get("owner") != lifecycle["orchestrator_organ"]:
        errors.append("build_policy.owner must match lifecycle orchestrator organ")
    if build_policy.get("rust_release_target") != "x86_64-unknown-linux-musl":
        errors.append("build_policy.rust_release_target must be x86_64-unknown-linux-musl")
    if not build_policy.get("rust_toolchain"):
        errors.append("build_policy.rust_toolchain missing")
    artifact_targets = build_policy.get("artifact_targets", [])
    if not isinstance(artifact_targets, list) or "crates/cynic-node" not in artifact_targets:
        errors.append("build_policy.artifact_targets must include crates/cynic-node")
    if bootstrap_policy.get("preferred_transport") != "fleet_node_exec":
        errors.append("build_policy.bootstrap.preferred_transport must be fleet_node_exec")
    if bootstrap_policy.get("transport_command_env") != "CYNIC_FLEET_NODE_EXEC_COMMAND":
        errors.append(
            "build_policy.bootstrap.transport_command_env must be CYNIC_FLEET_NODE_EXEC_COMMAND"
        )
    if (
        bootstrap_policy.get("transport_command_default")
        != "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py"
    ):
        errors.append(
            "build_policy.bootstrap.transport_command_default must be the organ-forge fleet_node_exec wrapper"
        )
    if bootstrap_policy.get("principal") != "cynic":
        errors.append("build_policy.bootstrap.principal must be cynic")
    if bootstrap_policy.get("repo_materialization") not in {"rsync", "git_fetch"}:
        errors.append("build_policy.bootstrap.repo_materialization must be rsync or git_fetch")
    if bootstrap_policy.get("repo_materialization") == "rsync":
        source_path_env = bootstrap_policy.get("source_path_env", "CYNIC_PROJECT_ROOT")
        if not isinstance(source_path_env, str) or not source_path_env:
            errors.append("build_policy.bootstrap.source_path_env must be a non-empty string for rsync")
    if bootstrap_policy.get("repo_materialization") == "git_fetch":
        repo_source = bootstrap_policy.get("repo_source", {})
        if not isinstance(repo_source, dict):
            errors.append("build_policy.bootstrap.repo_source must be an object for git_fetch")
        else:
            if not repo_source.get("remote"):
                errors.append("build_policy.bootstrap.repo_source.remote missing for git_fetch")
            if not repo_source.get("ref"):
                errors.append("build_policy.bootstrap.repo_source.ref missing for git_fetch")
            ref_env = repo_source.get("ref_env")
            if ref_env is not None and (not isinstance(ref_env, str) or not ref_env):
                errors.append(
                    "build_policy.bootstrap.repo_source.ref_env must be a non-empty string when present"
                )
            if repo_source.get("host_role") != "forge_primary":
                errors.append("build_policy.bootstrap.repo_source.host_role must be forge_primary for git_fetch")
            fallback_remote_env = repo_source.get("fallback_remote_env")
            if fallback_remote_env is not None and (
                not isinstance(fallback_remote_env, str) or not fallback_remote_env
            ):
                errors.append(
                    "build_policy.bootstrap.repo_source.fallback_remote_env must be a non-empty string when present"
                )
    host_packages = bootstrap_policy.get("host_packages", [])
    if "musl-tools" not in host_packages:
        errors.append("build_policy.bootstrap.host_packages must include musl-tools")
    for host_package in ["pkg-config", "libxi-dev"]:
        if host_package not in host_packages:
            errors.append(f"build_policy.bootstrap.host_packages must include {host_package}")
    if not bootstrap_policy.get("watchdog_config_path"):
        errors.append("build_policy.bootstrap.watchdog_config_path missing")

    activation = lifecycle.get("activation_requirements", {})
    if not {build_role, deploy_role}.issubset(set(activation)):
        errors.append("activation_requirements must cover build/deploy roles")
    for role, requirement in activation.items():
        if not isinstance(requirement.get("network"), list) or not requirement["network"]:
            errors.append(f"activation_requirements.{role}.network missing")
        if not isinstance(requirement.get("services"), list) or not requirement["services"]:
            errors.append(f"activation_requirements.{role}.services missing")
        filesystem = requirement.get("filesystem", [])
        if not isinstance(filesystem, list):
            errors.append(f"activation_requirements.{role}.filesystem must be a list")
            filesystem = []
        if role in {build_role, deploy_role} and "repo_root" not in filesystem:
            errors.append(f"activation_requirements.{role}.filesystem must include repo_root")
        for path_name in filesystem:
            if path_name not in runtime_paths:
                errors.append(
                    f"activation_requirements.{role}.filesystem references unknown runtime path {path_name}"
                )

    fleet = lifecycle.get("fleet_requirements", {})
    if fleet.get("provider") not in {"synapse", "organ-fleet"}:
        errors.append("fleet_requirements.provider must be synapse or organ-fleet")
    if fleet.get("required") is not True:
        errors.append("fleet_requirements.required must be true")
    if not isinstance(fleet.get("require_online"), bool):
        errors.append("fleet_requirements.require_online must be boolean")

    probe = lifecycle.get("kernel_bus_probe", {})
    if not str(probe.get("path", "")).startswith("/"):
        errors.append("kernel_bus_probe.path must be absolute")
    if not isinstance(probe.get("accepted_status"), list) or not probe["accepted_status"]:
        errors.append("kernel_bus_probe.accepted_status missing")
    if int(probe.get("timeout_seconds", 0)) <= 0:
        errors.append("kernel_bus_probe.timeout_seconds must be positive")

    mcp_controller = lifecycle.get("mcp_freshness_controller", {})
    if mcp_controller.get("schema") != "cynic.mcp_freshness_controller.v1":
        errors.append("mcp_freshness_controller.schema must be cynic.mcp_freshness_controller.v1")
    if mcp_controller.get("owner") != lifecycle["orchestrator_organ"]:
        errors.append("mcp_freshness_controller.owner must match lifecycle orchestrator organ")
    if mcp_controller.get("consumer") != "organ-forge-hermes-agent":
        errors.append("mcp_freshness_controller.consumer must be organ-forge-hermes-agent")
    if mcp_controller.get("dispatch_profile") != "mcp-reload":
        errors.append("mcp_freshness_controller.dispatch_profile must be mcp-reload")
    if mcp_controller.get("dispatch_tool") != "cynic_lifecycle_dispatch":
        errors.append("mcp_freshness_controller.dispatch_tool must be cynic_lifecycle_dispatch")
    stale_statuses = mcp_controller.get("stale_statuses", [])
    for status_name in ["stale", "missing", "failed", "timeout"]:
        if status_name not in stale_statuses:
            errors.append(f"mcp_freshness_controller.stale_statuses missing {status_name}")
    trigger_sources = mcp_controller.get("trigger_sources", [])
    for trigger in ["crates/cynic-mcp/src", "infra/agent-exposure.json", "organs/exposure"]:
        if trigger not in trigger_sources:
            errors.append(f"mcp_freshness_controller.trigger_sources missing {trigger}")
    if int(mcp_controller.get("debounce_seconds", 0)) <= 0:
        errors.append("mcp_freshness_controller.debounce_seconds must be positive")
    if "CYNIC_MCP_CHECK_ONLY=1" not in str(mcp_controller.get("check_command", "")):
        errors.append("mcp_freshness_controller.check_command must use CYNIC_MCP_CHECK_ONLY=1")

    try:
        artifact_manifest = build_runtime_artifact_manifest(
            registry,
            now="registry-validation",
        )
    except RuntimeError as exc:
        artifact_manifest = None
        errors.append(str(exc))
    if artifact_manifest is not None:
        artifact_names = {artifact["name"] for artifact in artifact_manifest["artifacts"]}
        for required_artifact in [
            "cynic-mcp",
            "cynic-cognitive-sensor",
            "cynic-node",
            "organ-forge",
            "agent-exposure-projections",
            "lifecycle-systemd-units",
        ]:
            if required_artifact not in artifact_names:
                errors.append(f"runtime_artifacts missing {required_artifact}")
        mcp_artifact = next(
            (
                artifact
                for artifact in artifact_manifest["artifacts"]
                if artifact["name"] == "cynic-mcp"
            ),
            None,
        )
        if mcp_artifact:
            for trigger in mcp_controller.get("trigger_sources", []):
                if trigger not in mcp_artifact["source_targets"]:
                    errors.append(
                        f"runtime_artifacts.cynic-mcp.source_targets missing MCP trigger {trigger}"
                    )
            if "mcp-reload" not in mcp_artifact.get("request_profiles", []):
                errors.append("runtime_artifacts.cynic-mcp.request_profiles must include mcp-reload")

    repo_root = runtime_paths.get("repo_root")
    if not isinstance(repo_root, str) or not repo_root:
        errors.append("runtime_paths.repo_root missing")
    elif "/Bureau/" in repo_root or repo_root.startswith("/" + "home/"):
        errors.append("runtime_paths.repo_root must not point at an operator checkout")
    repo_root_proof = runtime_path_proofs.get("repo_root", {})
    if repo_root_proof.get("kind") != "git_worktree":
        errors.append("runtime_path_proofs.repo_root.kind must be git_worktree")
    if repo_root_proof.get("marker") != ".git":
        errors.append("runtime_path_proofs.repo_root.marker must be .git")
    if not isinstance(repo_root_proof.get("sentinels"), list) or not repo_root_proof["sentinels"]:
        errors.append("runtime_path_proofs.repo_root.sentinels missing")
    evidence_contract = lifecycle.get("activation_evidence_contract", {})
    if evidence_contract.get("patch_field") != "activation_evidence_patch":
        errors.append("activation_evidence_contract.patch_field must be activation_evidence_patch")
    if evidence_contract.get("node_field") != "node":
        errors.append("activation_evidence_contract.node_field must be node")
    if evidence_contract.get("runtime_paths_field") != "activation_evidence.runtime_paths":
        errors.append(
            "activation_evidence_contract.runtime_paths_field must be activation_evidence.runtime_paths"
        )
    if evidence_contract.get("services_field") != "activation_evidence.services":
        errors.append("activation_evidence_contract.services_field must be activation_evidence.services")
    verified_requires = evidence_contract.get("verified_requires", [])
    if sorted(verified_requires) != ["marker_exists", "sentinels_verified"]:
        errors.append(
            "activation_evidence_contract.verified_requires must include marker_exists and sentinels_verified"
        )
    service_active_requires = evidence_contract.get("service_active_requires", [])
    if sorted(service_active_requires) != ["active", "declared"]:
        errors.append(
            "activation_evidence_contract.service_active_requires must include declared and active"
        )
    if evidence_contract.get("ingestion_mode") != "validated_merge":
        errors.append("activation_evidence_contract.ingestion_mode must be validated_merge")

    if errors:
        raise RuntimeError("; ".join(errors))
    return {
        "status": "ok",
        "lifecycle": "kernel_release",
        "build_runner_role": build_role,
        "deploy_target_role": deploy_role,
        "orchestrator_organ": lifecycle["orchestrator_organ"],
        "fleet_provider": fleet["provider"],
        "repo_root": repo_root,
        "request_profiles": sorted(profiles),
        "runtime_artifacts": artifact_manifest,
        "workflow": workflow,
    }


def lifecycle_health_status_result(
    status_code: int,
    registry: dict | None = None,
) -> dict:
    registry = registry or load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")
    probe = lifecycle.get("kernel_bus_probe", {})
    accepted = probe.get("accepted_status", [200])
    if not isinstance(accepted, list) or not accepted:
        raise RuntimeError("kernel_bus_probe.accepted_status missing")
    return {
        "status": "ok" if status_code in accepted else "rejected",
        "http_status": status_code,
        "accepted_status": accepted,
        "accepted": status_code in accepted,
    }


def lifecycle_transition_contract(registry: dict | None = None) -> dict:
    registry = registry or load_registry()
    lifecycle = required_lifecycle(registry, "kernel_release")
    workflow = lifecycle.get("workflow", {})
    errors = validate_workflow_layers(workflow, lifecycle["orchestrator_organ"])
    return {
        "schema": "cynic.lifecycle_transition_contract_report.v1",
        "status": "ok" if not errors else "error",
        "lifecycle": "kernel_release",
        "orchestrator_organ": lifecycle["orchestrator_organ"],
        "state_layers": workflow.get("state_layers", {}),
        "transition_contract": workflow.get("transition_contract", {}),
        "errors": errors,
    }


def lifecycle_transition_debt(
    registry: dict | None = None,
    activation_report: dict | None = None,
    applied_evidence: dict | None = None,
    verified_evidence: dict | None = None,
) -> dict:
    return build_lifecycle_transition_debt_report(
        lifecycle_transition_contract(registry),
        activation_report=activation_report,
        applied_evidence=applied_evidence,
        verified_evidence=verified_evidence,
    )


class RegistryAdapter:
    """Small facade for registry-backed lifecycle lookups."""

    def __init__(self, path: str | Path | None = None):
        self.path = Path(path) if path else None

    def load(self) -> dict:
        return load_registry(self.path)

    def required_lifecycle(self, registry: dict, name: str) -> dict:
        return required_lifecycle(registry, name)

    def required_node(
        self,
        registry: dict,
        role: str,
        deploy_target_flag: str | None = None,
    ) -> dict:
        return required_node(registry, role, deploy_target_flag)

    def validate_lifecycle_policy(self, registry: dict | None = None) -> dict:
        return validate_lifecycle_policy(registry or self.load())

    def lifecycle_health_status_result(
        self,
        status_code: int,
        registry: dict | None = None,
    ) -> dict:
        return lifecycle_health_status_result(status_code, registry or self.load())

    def lifecycle_transition_contract(self, registry: dict | None = None) -> dict:
        return lifecycle_transition_contract(registry or self.load())

    def lifecycle_transition_debt(
        self,
        registry: dict | None = None,
        activation_report: dict | None = None,
        applied_evidence: dict | None = None,
        verified_evidence: dict | None = None,
    ) -> dict:
        return lifecycle_transition_debt(
            registry or self.load(),
            activation_report=activation_report,
            applied_evidence=applied_evidence,
            verified_evidence=verified_evidence,
        )
