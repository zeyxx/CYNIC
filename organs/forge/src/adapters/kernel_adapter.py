import json
import logging
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from urllib import error, request

from ..core.ports import IKernelClient
from ..core.lifecycle_state import build_workflow_state, validate_workflow_layers
from ..core.lifecycle_workflow import (
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
from ..core.lifecycle_readiness import (
    activation_evidence_gaps as _readiness_activation_evidence_gaps,
    build_activation_evidence_gap_details as _readiness_build_activation_evidence_gap_details,
    fleet_nodes_ready as _readiness_fleet_nodes_ready,
    inspect_fleet as _readiness_inspect_fleet,
    lifecycle_blockers as _readiness_lifecycle_blockers,
)

logger = logging.getLogger(__name__)


def _default_registry_path() -> Path:
    return Path(__file__).resolve().parents[4] / "infra" / "registry.json"


def load_registry(path: str | Path | None = None) -> dict:
    registry_path = Path(path) if path else _default_registry_path()
    with registry_path.open() as f:
        return json.load(f)


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
    if fleet.get("provider") != "tailscale":
        errors.append("fleet_requirements.provider must be tailscale")
    if fleet.get("required") is not True:
        errors.append("fleet_requirements.required must be true")
    if fleet.get("require_online") is not True:
        errors.append("fleet_requirements.require_online must be true")

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
    elif "/Bureau/" in repo_root or repo_root.startswith("/home/"):
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
    if not raw:
        raise RuntimeError("CYNIC_REST_ADDR is required")
    if raw.startswith(("http://", "https://")):
        return raw.rstrip("/")
    return f"http://{raw.rstrip('/')}"


class KernelRestClient(IKernelClient):
    def __init__(
        self,
        uri: str = None,
        require_api_key: bool = True,
        strict_lifecycle: bool = True,
    ):
        self.registry = self._load_registry()
        self.lifecycle = self._required_lifecycle("kernel_release")
        self.build_node = self._required_node(self.lifecycle["build_runner_role"])
        self.deploy_node = self._required_node(
            self.lifecycle["deploy_target_role"],
            deploy_target_flag=self.lifecycle.get("deploy_target_flag"),
        )
        self.build_target = self.build_node["name"]
        self.deploy_target = self.deploy_node["name"]
        self.kernel_endpoint_source = "unknown"
        self.kernel_bus_canonical = True
        self.kernel_probe = {"enabled": False, "status": "not_run"}
        self.fleet_probe = {"enabled": False, "status": "not_run"}
        self.fleet_inventory_probe = {"enabled": False, "status": "not_run"}
        self.strict_lifecycle = strict_lifecycle
        if strict_lifecycle:
            self._require_ready_nodes()
        self.base_url = self._kernel_addr(uri)
        self.api_key = os.environ.get("CYNIC_" + "API" + "_KEY", "")
        if require_api_key and not self.api_key:
            raise RuntimeError("CYNIC_API_KEY is required for organ-forge kernel dispatch")
        if strict_lifecycle:
            self._probe_kernel_bus()

    def _load_registry(self) -> dict:
        return load_registry()

    def _required_lifecycle(self, name: str) -> dict:
        lifecycle = self.registry.get("lifecycle", {}).get(name)
        if not lifecycle:
            raise RuntimeError(f"registry missing lifecycle.{name}")
        return lifecycle

    def _required_node(self, role: str, deploy_target_flag: str | None = None) -> dict:
        for node in self.registry.get("nodes", []):
            if node.get("role") != role:
                continue
            if deploy_target_flag and node.get(deploy_target_flag) is not True:
                continue
            return node
        suffix = f" with {deploy_target_flag}=true" if deploy_target_flag else ""
        raise RuntimeError(f"registry missing node role={role}{suffix}")

    def _require_ready_nodes(self) -> None:
        required = self.lifecycle.get("required_node_status")
        if not required:
            return
        missing_evidence = self._activation_evidence_gaps()
        if missing_evidence:
            raise RuntimeError(
                "registry lifecycle activation evidence is incomplete "
                f"({'; '.join(missing_evidence)})"
            )
        self._require_fleet_presence()
        not_ready = [
            f"{node.get('name')}:{node.get('status', 'unknown')}"
            for node in (self.build_node, self.deploy_node)
            if node.get("status") != required
        ]
        if not_ready:
            raise RuntimeError(
                "registry lifecycle nodes are not ready "
                f"(required status={required}; found {', '.join(not_ready)})"
            )

    def _require_fleet_presence(self) -> None:
        requirements = self.lifecycle.get("fleet_requirements", {})
        if not requirements.get("required", False):
            return
        provider = requirements.get("provider")
        if provider != "tailscale":
            raise RuntimeError(f"unsupported lifecycle fleet provider: {provider}")
        self.inspect_fleet()
        if self.fleet_probe.get("status") != "ok":
            reason = self.fleet_probe.get("error", self.fleet_probe.get("status", "unknown"))
            raise RuntimeError(f"registry lifecycle fleet probe failed ({reason})")
        missing = self.fleet_probe.get("missing", [])
        offline = [
            name
            for name, peer in self.fleet_probe.get("matches", {}).items()
            if requirements.get("require_online", True) and not peer.get("online")
        ]
        if missing or offline:
            details = []
            if missing:
                details.append(f"missing={','.join(missing)}")
            if offline:
                details.append(f"offline={','.join(offline)}")
            raise RuntimeError(
                "registry lifecycle fleet targets are not ready "
                f"({'; '.join(details)})"
            )

    def _activation_evidence_gaps(self) -> list[str]:
        return _readiness_activation_evidence_gaps(
            self.lifecycle,
            self.build_node,
            self.deploy_node,
            env=os.environ,
        )

    def _activation_evidence_gap_details(self) -> list[dict]:
        return _readiness_build_activation_evidence_gap_details(
            self.lifecycle,
            self.build_node,
            self.deploy_node,
            env=os.environ,
        )

    @staticmethod
    def _extract_fleet_inventory_from_health(payload: dict) -> dict:
        environment = payload.get("environment")
        if not isinstance(environment, dict):
            return {"enabled": True, "status": "missing_environment"}

        probes = environment.get("probes")
        if not isinstance(probes, list):
            return {"enabled": True, "status": "missing_probes"}

        for probe in probes:
            if not isinstance(probe, dict):
                continue
            if probe.get("name") != "fleet_inventory":
                continue
            details = probe.get("details")
            if isinstance(details, dict) and details.get("kind") == "fleet_inventory":
                data = details.get("data", {})
            else:
                data = details if isinstance(details, dict) else {}
            if not isinstance(data, dict):
                return {
                    "enabled": True,
                    "status": str(probe.get("status", "unknown")),
                    "error": "fleet_inventory_data_not_object",
                }
            summary = data.get("summary", {})
            nodes = data.get("nodes", [])
            extracted = {
                "enabled": True,
                "status": str(probe.get("status", "unknown")),
                "name": probe.get("name", "fleet_inventory"),
                "summary": summary if isinstance(summary, dict) else {},
            }
            if nodes:
                extracted["nodes"] = nodes
            if probe.get("error"):
                extracted["error"] = str(probe.get("error"))
            if probe.get("duration_ms") is not None:
                extracted["duration_ms"] = probe["duration_ms"]
            return extracted
        return {"enabled": True, "status": "missing_probe"}

    def _kernel_addr(self, explicit: str | None) -> str:
        raw = explicit or os.environ.get("CYNIC_REST_ADDR", "")
        self.kernel_endpoint_source = "explicit" if explicit else "env:CYNIC_REST_ADDR"
        if not raw:
            kernel = self._required_node(
                self.lifecycle["deploy_target_role"],
                deploy_target_flag=self.lifecycle.get("deploy_target_flag"),
            )
            raw = kernel.get("network", {}).get("tailscale", "")
            self.kernel_endpoint_source = "registry:kernel_primary.network.tailscale"
        if not raw:
            raise RuntimeError("CYNIC_REST_ADDR is required")
        if raw.startswith("<") and raw.endswith(">"):
            env_name = raw[1:-1]
            raw = os.environ.get(env_name, "")
            self.kernel_endpoint_source = f"env:{env_name}"
            if not raw:
                if not self.strict_lifecycle:
                    raw = f"<{env_name}>"
                else:
                    raise RuntimeError(
                        f"{env_name} is required to resolve registry kernel_primary endpoint"
                    )
        if raw.startswith("<") and raw.endswith(">"):
            if self.strict_lifecycle:
                raise RuntimeError(f"{env_name} is required to resolve registry kernel_primary endpoint")
            return raw
        if raw.startswith(("localhost:", "127.")) or raw.startswith(
            ("http://localhost:", "http://127.")
        ):
            self.kernel_bus_canonical = False
            override = self.lifecycle.get("local_kernel_bus_override_env")
            if self.strict_lifecycle and (not override or os.environ.get(override) != "1"):
                raise RuntimeError(
                    "local kernel bus is not canonical; point CYNIC_REST_ADDR to registry kernel_primary"
                )
        return normalize_kernel_base_url(raw)

    @staticmethod
    def _redact_url(url: str) -> str:
        scheme = ""
        rest = url
        if "://" in url:
            scheme, rest = url.split("://", 1)
            scheme += "://"
        host_port = rest.split("/", 1)[0]
        host, sep, port = host_port.partition(":")
        if host in {"localhost"} or host.startswith("127."):
            redacted_host = host
        elif host.startswith("100."):
            redacted_host = "100.x.x.x"
        elif host.startswith("192.168."):
            redacted_host = "192.168.x.x"
        else:
            redacted_host = "<redacted-host>"
        return f"{scheme}{redacted_host}{sep}{port}" if sep else f"{scheme}{redacted_host}"

    def lifecycle_context(self) -> dict:
        required_status = self.lifecycle.get("required_node_status")
        activation_gap_details = self._activation_evidence_gap_details()
        activation_gaps = self._activation_evidence_gaps()
        fleet_inventory_probe = getattr(
            self,
            "fleet_inventory_probe",
            {"enabled": False, "status": "not_run"},
        )
        if (
            fleet_inventory_probe.get("status") == "not_run"
        ):
            try:
                self._probe_kernel_bus()
            except Exception:
                pass
            fleet_inventory_probe = getattr(
                self,
                "fleet_inventory_probe",
                {"enabled": False, "status": "error"},
            )

        self.fleet_inventory_probe = fleet_inventory_probe
        status_nodes_ready = (
            not any(
                node.get("status") != required_status
                for node in (self.build_node, self.deploy_node)
            )
            if required_status
            else True
        )
        fleet_nodes_ready = self._fleet_nodes_ready()
        lifecycle_blockers = self._lifecycle_blockers(
            activation_gap_details=activation_gap_details,
            required_status=required_status,
        )
        workflow_state = build_workflow_state(
            self.lifecycle,
            build_node=self.build_node,
            deploy_node=self.deploy_node,
            required_status=required_status,
            activation_gap_details=activation_gap_details,
            activation_gaps=activation_gaps,
            fleet_probe=self.fleet_probe,
            kernel_probe=self.kernel_probe,
            fleet_inventory_probe=self.fleet_inventory_probe,
            status_nodes_ready=status_nodes_ready,
            fleet_nodes_ready=fleet_nodes_ready,
            lifecycle_blockers=lifecycle_blockers,
        )
        return {
            "lifecycle": "kernel_release",
            "orchestrator_organ": self.lifecycle["orchestrator_organ"],
            "task_kind": self.lifecycle["task_kind"],
            "task_domain": self.lifecycle["task_domain"],
            "build_target": self.build_target,
            "build_target_status": self.build_node.get("status", "unknown"),
            "deploy_target": self.deploy_target,
            "deploy_target_status": self.deploy_node.get("status", "unknown"),
            "kernel_endpoint_source": self.kernel_endpoint_source,
            "kernel_endpoint": self._redact_url(self.base_url),
            "kernel_bus_canonical": self.kernel_bus_canonical,
            "kernel_probe": self.kernel_probe,
            "fleet_probe": self.fleet_probe,
            "fleet_inventory_probe": self.fleet_inventory_probe,
            "fleet_requirements": self.lifecycle.get("fleet_requirements", {}),
            "required_node_status": required_status,
            "activation_requirements": self.lifecycle.get("activation_requirements", {}),
            "request_profiles": self.lifecycle.get("request_profiles", {}),
            "intents": self.lifecycle.get("intents", {}),
            "workflow": self.lifecycle.get("workflow", {}),
            "workflow_state": workflow_state,
            "activation_evidence_gaps": activation_gaps,
            "activation_evidence_gap_details": activation_gap_details,
            "status_nodes_ready": status_nodes_ready,
            "fleet_nodes_ready": fleet_nodes_ready,
            "lifecycle_nodes_ready": (
                status_nodes_ready and not activation_gaps and fleet_nodes_ready
            ),
            "lifecycle_blockers": lifecycle_blockers,
            "requires_e2e": self.lifecycle.get("requires_e2e", True),
            "repo_targets": self.lifecycle.get("repo_targets", []),
        }

    def _lifecycle_blockers(
        self,
        activation_gap_details: list[dict],
        required_status: str | None,
    ) -> list[dict]:
        fleet_inventory_probe = getattr(
            self,
            "fleet_inventory_probe",
            {"enabled": False, "status": "not_run"},
        )
        return _readiness_lifecycle_blockers(
            self.lifecycle,
            self.build_node,
            self.deploy_node,
            activation_gap_details=activation_gap_details,
            required_status=required_status,
            fleet_probe=self.fleet_probe,
            fleet_inventory_probe=fleet_inventory_probe,
        )

    def _fleet_nodes_ready(self) -> bool:
        fleet_inventory_probe = getattr(
            self,
            "fleet_inventory_probe",
            {"enabled": False, "status": "not_run"},
        )
        return _readiness_fleet_nodes_ready(
            self.lifecycle,
            self.fleet_probe,
            fleet_inventory_probe,
        )

    def inspect_fleet(self) -> None:
        self.fleet_probe = _readiness_inspect_fleet(self.build_target, self.deploy_target)

    def _probe_kernel_bus(self) -> None:
        probe = self.lifecycle.get("kernel_bus_probe") or {}
        if not probe:
            return
        path = probe.get("path", "/health")
        timeout = int(probe.get("timeout_seconds", 10))
        accepted = set(probe.get("accepted_status", [200]))
        headers = {}
        if probe.get("requires_bearer_auth", True):
            if not self.api_key:
                raise RuntimeError("CYNIC_API_KEY is required for kernel bus probe")
            headers["Authorization"] = f"Bearer {self.api_key}"
        req = request.Request(
            f"{self.base_url}{path}",
            method="GET",
            headers=headers,
        )
        self.kernel_probe = {
            "enabled": True,
            "path": path,
            "accepted_status": sorted(accepted),
            "status": "pending",
        }
        health_payload = None
        try:
            with request.urlopen(req, timeout=timeout) as resp:
                status = resp.status
                raw = resp.read().decode("utf-8", errors="replace")
            try:
                health_payload = json.loads(raw)
            except json.JSONDecodeError:
                self.kernel_probe.update({"status_error": "health_payload_invalid_json"})
        except error.HTTPError as exc:
            status = exc.code
            try:
                raw = exc.read().decode("utf-8", errors="replace")
            except Exception:
                raw = ""
            try:
                health_payload = json.loads(raw)
            except Exception:
                self.kernel_probe.update({"status_error": "health_payload_invalid_json"})
        except OSError as exc:
            self.kernel_probe.update({"status": "error", "error": type(exc).__name__})
            raise RuntimeError(f"kernel bus probe failed: {type(exc).__name__}")
        self.kernel_probe.update({"status": "ok" if status in accepted else "rejected", "http_status": status})
        if health_payload is not None:
            self.fleet_inventory_probe = self._extract_fleet_inventory_from_health(
                health_payload
            )
        else:
            self.fleet_inventory_probe = {"enabled": True, "status": "missing_body"}
        if status not in accepted:
            raise RuntimeError(
                f"kernel bus probe rejected HTTP {status}; accepted={sorted(accepted)}"
            )

    def _post_agent_task(self, payload: dict) -> dict:
        body = json.dumps(payload).encode("utf-8")
        req = request.Request(
            f"{self.base_url}/agent-tasks",
            data=body,
            method="POST",
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json",
            },
        )
        try:
            with request.urlopen(req, timeout=10) as resp:
                text = resp.read().decode("utf-8", errors="replace")
                return json.loads(text)
        except error.HTTPError as exc:
            text = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"agent task dispatch failed: HTTP {exc.code} {text[:200]}")
        except (OSError, json.JSONDecodeError) as exc:
            raise RuntimeError(f"agent task dispatch failed: {exc}")

    def _content_for_intent(self, intent: str, patch: str) -> dict:
        payload = build_lifecycle_task_payload(
            intent=intent,
            domain=self.lifecycle["task_domain"],
            agent_id="organ-forge-cli",
            registry=self.registry,
        )
        content = json.loads(payload["content"])
        if patch:
            content["patch_sha256"] = __import__("hashlib").sha256(patch.encode("utf-8")).hexdigest()
        return content

    async def dispatch_task_and_stream(self, intent: str, patch: str):
        payload = {
            "kind": self.lifecycle["task_kind"],
            "domain": self.lifecycle["task_domain"],
            "agent_id": "organ-forge-cli",
            "content": json.dumps(self._content_for_intent(intent, patch), separators=(",", ":")),
        }
        validate_lifecycle_task_payload(payload, self.registry)
        response = self._post_agent_task(payload)
        yield json.dumps(response, separators=(",", ":")) + "\n"
