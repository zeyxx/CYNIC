import json
import logging
import os
from pathlib import Path

from organs.contracts.src.adapters.kernel_http import normalize_kernel_base_url as _normalize_kernel_base_url
from organs.contracts.src.surface_contracts import read_json_file_or_default
from ....forge.src.core.lifecycle_state import validate_workflow_layers
from ....forge.src.core.lifecycle_workflow import (
    activate_lifecycle_registry_node as _workflow_activate_lifecycle_registry_node,
    apply_lifecycle_activation_evidence_patch as _workflow_apply_lifecycle_activation_evidence_patch,
    build_lifecycle_activation_plan as _workflow_build_lifecycle_activation_plan,
    build_lifecycle_activation_requests as _workflow_build_lifecycle_activation_requests,
    build_lifecycle_profile_payloads as _workflow_build_lifecycle_profile_payloads,
    build_lifecycle_runtime_paths as _workflow_build_lifecycle_runtime_paths,
    build_lifecycle_service_evidence_patch as _workflow_build_lifecycle_service_evidence_patch,
    build_lifecycle_task_payload as _workflow_build_lifecycle_task_payload,
    dispatch_lifecycle_activation_requests as _workflow_dispatch_lifecycle_activation_requests,
    lifecycle_profile_intents as _workflow_lifecycle_profile_intents,
    persist_lifecycle_activation_evidence_patch as _workflow_persist_lifecycle_activation_evidence_patch,
    persist_lifecycle_registry_node_activation as _workflow_persist_lifecycle_registry_node_activation,
    validate_lifecycle_activation_request_payload as _workflow_validate_lifecycle_activation_request_payload,
    validate_lifecycle_task_payload as _workflow_validate_lifecycle_task_payload,
    verify_lifecycle_runtime_paths as _workflow_verify_lifecycle_runtime_paths,
)

logger = logging.getLogger(__name__)


def _default_registry_path() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve() / "infra" / "registry.json"

    start = Path(__file__).resolve()
    for candidate in [start, *start.parents]:
        if (candidate / "infra").is_dir() and (candidate / "organs").is_dir() and (candidate / ".git").exists():
            return candidate / "infra" / "registry.json"

    return Path(__file__).resolve().parent / "infra" / "registry.json"


def load_registry(path: str | Path | None = None) -> dict:
    registry_path = Path(path) if path else _default_registry_path()
    return read_json_file_or_default(registry_path, {})


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

    activation = lifecycle.get("activation_requirements", {})
    if set(activation) != {build_role, deploy_role}:
        errors.append("activation_requirements must cover build/deploy roles exactly")
    for role, requirement in activation.items():
        if not isinstance(requirement.get("network"), list) or not requirement["network"]:
            errors.append(f"activation_requirements.{role}.network missing")
        if not isinstance(requirement.get("services"), list) or not requirement["services"]:
            errors.append(f"activation_requirements.{role}.services missing")
        filesystem = requirement.get("filesystem", [])
        if not isinstance(filesystem, list) or "repo_root" not in filesystem:
            errors.append(f"activation_requirements.{role}.filesystem must include repo_root")
            filesystem = []
        for path_name in filesystem:
            if path_name not in runtime_paths:
                errors.append(
                    f"activation_requirements.{role}.filesystem references unknown runtime path {path_name}"
                )

    fleet = lifecycle.get("fleet_requirements", {})
    if fleet.get("provider") not in ["headscale", "organ-fleet"]:
        errors.append("fleet_requirements.provider must be organ-fleet or headscale")
    if fleet.get("required") is not True:
        errors.append("fleet_requirements.required must be true")
    # require_online might be false for bootstrap override
    if not fleet.get("allow_bootstrap_override") and fleet.get("require_online") is not True:
        errors.append("fleet_requirements.require_online must be true unless allow_bootstrap_override is true")

    probe = lifecycle.get("kernel_bus_probe", {})
    if not str(probe.get("path", "")).startswith("/"):
        errors.append("kernel_bus_probe.path must be absolute")
    if not isinstance(probe.get("accepted_status"), list) or not probe["accepted_status"]:
        errors.append("kernel_bus_probe.accepted_status missing")
    if int(probe.get("timeout_seconds", 0)) <= 0:
        errors.append("kernel_bus_probe.timeout_seconds must be positive")

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
        "workflow": workflow,
    }


def lifecycle_profile_intents(
    profile: str,
    registry: dict | None = None,
) -> list[str]:
    return _workflow_lifecycle_profile_intents(profile, registry)


def build_lifecycle_task_payload(
    intent: str,
    domain: str,
    agent_id: str,
    registry: dict | None = None,
) -> dict:
    return _workflow_build_lifecycle_task_payload(
        intent=intent,
        domain=domain,
        agent_id=agent_id,
        registry=registry,
    )


def validate_lifecycle_task_payload(
    payload: dict,
    registry: dict | None = None,
) -> dict:
    return _workflow_validate_lifecycle_task_payload(payload, registry)


def build_lifecycle_profile_payloads(
    profile: str,
    domain: str,
    agent_id: str,
    registry: dict | None = None,
) -> dict:
    return _workflow_build_lifecycle_profile_payloads(
        profile=profile,
        domain=domain,
        agent_id=agent_id,
        registry=registry,
    )


def build_lifecycle_activation_plan(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    return _workflow_build_lifecycle_activation_plan(
        registry=registry,
        inspect_fleet=inspect_fleet,
    )


def build_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    return _workflow_build_lifecycle_activation_requests(
        registry=registry,
        inspect_fleet=inspect_fleet,
    )


def validate_lifecycle_activation_request_payload(
    payload: dict,
    registry: dict | None = None,
) -> dict:
    return _workflow_validate_lifecycle_activation_request_payload(payload, registry)


def dispatch_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    kernel_client_factory=None,
) -> dict:
    return _workflow_dispatch_lifecycle_activation_requests(
        registry=registry,
        inspect_fleet=inspect_fleet,
        kernel_client_factory=kernel_client_factory,
    )


def build_lifecycle_runtime_paths(registry: dict | None = None) -> dict:
    return _workflow_build_lifecycle_runtime_paths(registry)


def verify_lifecycle_runtime_paths(
    node_name: str,
    registry: dict | None = None,
    home: str | None = None,
) -> dict:
    return _workflow_verify_lifecycle_runtime_paths(node_name, registry, home)


def build_lifecycle_service_evidence_patch(
    node_name: str,
    registry: dict | None = None,
    service_states: dict | None = None,
) -> dict:
    return _workflow_build_lifecycle_service_evidence_patch(
        node_name,
        registry=registry,
        service_states=service_states,
    )


def apply_lifecycle_activation_evidence_patch(
    patch: dict,
    registry: dict | None = None,
) -> dict:
    return _workflow_apply_lifecycle_activation_evidence_patch(patch, registry)


def persist_lifecycle_activation_evidence_patch(
    patch: dict,
    registry_path: str | Path | None = None,
) -> dict:
    return _workflow_persist_lifecycle_activation_evidence_patch(
        patch,
        registry_path=registry_path,
    )


def activate_lifecycle_registry_node(
    node_name: str,
    registry: dict | None = None,
) -> dict:
    return _workflow_activate_lifecycle_registry_node(node_name, registry)


def persist_lifecycle_registry_node_activation(
    node_name: str,
    registry_path: str | Path | None = None,
) -> dict:
    return _workflow_persist_lifecycle_registry_node_activation(
        node_name,
        registry_path=registry_path,
    )


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


def normalize_kernel_base_url(raw: str) -> str:
    return _normalize_kernel_base_url(raw, reject_loopback=False)
