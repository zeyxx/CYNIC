
import json
import logging
import os
import socket
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse

from organs.contracts.src.adapters.kernel_http import (
    kernel_auth_headers,
    normalize_kernel_base_url as _normalize_kernel_base_url,
)
from organs.contracts.src.surface_contracts import json_object_or_raise

from ..core.ports import IKernelClient
from .fleet_evidence_adapter import FleetEvidenceAdapter
from .kernel_bus_probe_adapter import KernelBusProbeAdapter
from .kernel_task_client import KernelTaskClient
from .lifecycle_diagnostic_service import LifecycleDiagnosticService
from .registry_adapter import (
    RegistryAdapter,
    lifecycle_health_status_result,
    load_registry,
    validate_lifecycle_policy,
)
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
)

logger = logging.getLogger(__name__)


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


def normalize_kernel_base_url(raw: str) -> str:
    return _normalize_kernel_base_url(raw, reject_loopback=False)


class KernelRestClient(IKernelClient):
    def __init__(
        self,
        uri: str = None,
        require_api_key: bool = True,
        strict_lifecycle: bool = True,
    ):
        self.registry_adapter = RegistryAdapter()
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
        self.kernel_tasks = KernelTaskClient(self.base_url, self.api_key)
        self.fleet_evidence = FleetEvidenceAdapter()
        self.kernel_bus_probe = KernelBusProbeAdapter(self.fleet_evidence)
        self.lifecycle_diagnostics = LifecycleDiagnosticService()
        if strict_lifecycle:
            self._probe_kernel_bus()

    def _load_registry(self) -> dict:
        return self._registry_adapter().load()

    def _registry_adapter(self) -> RegistryAdapter:
        adapter = getattr(self, "registry_adapter", None)
        if adapter is None:
            adapter = RegistryAdapter()
            self.registry_adapter = adapter
        return adapter

    def _required_lifecycle(self, name: str) -> dict:
        return self._registry_adapter().required_lifecycle(self.registry, name)

    def _required_node(self, role: str, deploy_target_flag: str | None = None) -> dict:
        return self._registry_adapter().required_node(
            self.registry,
            role,
            deploy_target_flag,
        )

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
        if provider == "organ-fleet":
            self._probe_fleet_inventory_report()
            if not self.fleet_inventory_probe.get("status") == "ok":
                reason = self.fleet_inventory_probe.get("error") or self.fleet_inventory_probe.get("status", "not_run")
                raise RuntimeError(f"registry lifecycle fleet inventory failed ({reason})")
            return
        if provider != "synapse":
            raise RuntimeError(f"unsupported lifecycle fleet provider: {provider}")
        self.inspect_fleet()
        if not self.fleet_probe.get("status") == "ok":
            reason = self.fleet_probe.get("error") or self.fleet_probe.get("status", "not_run")
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
            extra_role_nodes=self._activation_role_nodes(),
        )

    def _activation_evidence_gap_details(self) -> list[dict]:
        return _readiness_build_activation_evidence_gap_details(
            self.lifecycle,
            self.build_node,
            self.deploy_node,
            env=os.environ,
            extra_role_nodes=self._activation_role_nodes(),
        )

    def _activation_role_nodes(self) -> dict:
        roles = set(self.lifecycle.get("activation_requirements", {}))
        role_nodes = {
            self.lifecycle["build_runner_role"]: self.build_node,
            self.lifecycle["deploy_target_role"]: self.deploy_node,
        }
        for role in roles.difference(role_nodes):
            registry = getattr(self, "registry", {})
            node = next(
                (item for item in registry.get("nodes", []) if item.get("role") == role),
                None,
            )
            if node:
                role_nodes[role] = node
        return role_nodes

    @staticmethod
    def _extract_fleet_inventory_from_health(payload: dict) -> dict:
        return FleetEvidenceAdapter.extract_inventory_from_health(payload)

    @staticmethod
    def _extract_fleet_inventory_from_fleet_report(report: dict) -> dict:
        return FleetEvidenceAdapter.extract_inventory_from_fleet_report(report)

    def _probe_fleet_inventory_report(self) -> None:
        self.fleet_inventory_probe = (
            self._fleet_evidence_adapter().probe_fleet_inventory_report(env=os.environ)
        )

    @staticmethod
    def _kernel_address_from_node(node: dict) -> str:
        if not isinstance(node, dict):
            return ""
        network = (
            node.get("network", {}).get("synapse")
            or node.get("network", {}).get("local")
            or ""
        )
        if network.startswith("<") and network.endswith(">"):
            env_name = network[1:-1]
            network = os.environ.get(env_name, "")
        if not network:
            return ""
        port = 3030
        for service in node.get("services", []):
            if not isinstance(service, dict):
                continue
            if service.get("name") != "cynic-kernel":
                continue
            service_port = service.get("port")
            if isinstance(service_port, int) and service_port > 0:
                port = service_port
                break
            if isinstance(service_port, str) and service_port.isdigit():
                port = int(service_port)
                break
        return f"{network}:{port}"

    @staticmethod
    def _kernel_endpoint_reachable(base_url: str, timeout_seconds: float = 1.0) -> bool:
        parsed = urlparse(base_url if "://" in base_url else f"http://{base_url}")
        host = parsed.hostname
        if not host:
            return False
        port = parsed.port or 3030
        try:
            with socket.create_connection((host, port), timeout=timeout_seconds):
                return True
        except OSError:
            return False

    def _kernel_addr(self, explicit: str | None) -> str:
        raw = explicit or os.environ.get("CYNIC_REST_ADDR", "")
        self.kernel_endpoint_source = "explicit" if explicit else "env:CYNIC_REST_ADDR"
        if not raw:
            kernel = self._required_node(
                self.lifecycle["deploy_target_role"],
                deploy_target_flag=self.lifecycle.get("deploy_target_flag"),
            )
            raw = kernel.get("network", {}).get("synapse", "")
            self.kernel_endpoint_source = "registry:kernel_primary.network.synapse"
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
            override_enabled = bool(override and os.environ.get(override) == "1")
            if self.strict_lifecycle and not override_enabled:
                raise RuntimeError(
                    "local kernel bus is not canonical; point CYNIC_REST_ADDR to registry kernel_primary"
                )
            normalized = normalize_kernel_base_url(raw)
            if self._kernel_endpoint_reachable(normalized):
                return normalized
            fallback = self._kernel_address_from_node(self.deploy_node)
            if fallback:
                raw = fallback
                self.kernel_endpoint_source = "registry:kernel_primary.network.synapse"
                self.kernel_bus_canonical = True
            else:
                return normalized
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

    def _fleet_probe_value(self) -> dict:
        return self._lifecycle_diagnostic_service().fleet_probe_value(
            getattr(self, "fleet_probe", None)
        )

    @staticmethod
    def _probe_dict_value(probe: dict | None) -> dict:
        return LifecycleDiagnosticService.probe_dict_value(probe)

    def lifecycle_context(self) -> dict:
        activation_gap_details = self._activation_evidence_gap_details()
        activation_gaps = self._activation_evidence_gaps()
        fleet_inventory_probe = self._probe_dict_value(
            getattr(self, "fleet_inventory_probe", None)
        )
        if fleet_inventory_probe.get("status") == "not_run":
            try:
                self._probe_kernel_bus()
            except Exception:
                pass
            fleet_inventory_probe = self._probe_dict_value(
                getattr(self, "fleet_inventory_probe", None)
            )
        if fleet_inventory_probe.get("status") == "not_run":
            try:
                self._probe_fleet_inventory_report()
            except Exception as exc:
                self.fleet_inventory_probe = {
                    "enabled": True,
                    "status": "error",
                    "error": type(exc).__name__,
                }
            fleet_inventory_probe = self._probe_dict_value(
                getattr(self, "fleet_inventory_probe", {"enabled": False, "status": "error"})
            )

        self.fleet_inventory_probe = fleet_inventory_probe
        self.fleet_probe = self._fleet_probe_value()
        return self._lifecycle_diagnostic_service().build_context(
            lifecycle=self.lifecycle,
            build_node=self.build_node,
            deploy_node=self.deploy_node,
            build_target=self.build_target,
            deploy_target=self.deploy_target,
            kernel_endpoint_source=self.kernel_endpoint_source,
            kernel_endpoint=self._redact_url(self.base_url),
            kernel_bus_canonical=self.kernel_bus_canonical,
            kernel_probe=self.kernel_probe,
            fleet_probe=self.fleet_probe,
            fleet_inventory_probe=self.fleet_inventory_probe,
            activation_gap_details=activation_gap_details,
            activation_gaps=activation_gaps,
            activation_role_nodes=list(self._activation_role_nodes().values()),
        )

    def _lifecycle_blockers(
        self,
        activation_gap_details: list[dict],
        required_status: str | None,
        status_nodes: list[dict] | None = None,
    ) -> list[dict]:
        return self._lifecycle_diagnostic_service().lifecycle_blockers(
            self.lifecycle,
            self.build_node,
            self.deploy_node,
            activation_gap_details=activation_gap_details,
            required_status=required_status,
            fleet_probe=getattr(self, "fleet_probe", None),
            fleet_inventory_probe=getattr(self, "fleet_inventory_probe", None),
            status_nodes=status_nodes,
        )

    def _fleet_nodes_ready(self) -> bool:
        return self._lifecycle_diagnostic_service().fleet_nodes_ready(
            self.lifecycle,
            getattr(self, "fleet_probe", None),
            getattr(self, "fleet_inventory_probe", None),
        )

    def inspect_fleet(self) -> None:
        self.fleet_probe = self._fleet_evidence_adapter().inspect_fleet(
            self.build_target,
            self.deploy_target,
        )

    def _fleet_evidence_adapter(self) -> FleetEvidenceAdapter:
        adapter = getattr(self, "fleet_evidence", None)
        if adapter is None:
            adapter = FleetEvidenceAdapter()
            self.fleet_evidence = adapter
        return adapter

    def _lifecycle_diagnostic_service(self) -> LifecycleDiagnosticService:
        service = getattr(self, "lifecycle_diagnostics", None)
        if service is None:
            service = LifecycleDiagnosticService()
            self.lifecycle_diagnostics = service
        return service

    def _probe_kernel_bus(self) -> None:
        result = self._kernel_bus_probe_adapter().probe(
            base_url=self.base_url,
            api_key=self.api_key,
            probe_config=self.lifecycle.get("kernel_bus_probe") or {},
        )
        if result["kernel_probe"] is None:
            return
        self.kernel_probe = result["kernel_probe"]
        self.fleet_inventory_probe = result["fleet_inventory_probe"]

    def _kernel_bus_probe_adapter(self) -> KernelBusProbeAdapter:
        adapter = getattr(self, "kernel_bus_probe", None)
        if adapter is None:
            adapter = KernelBusProbeAdapter(self._fleet_evidence_adapter())
            self.kernel_bus_probe = adapter
        return adapter

    def _post_agent_task(self, payload: dict) -> dict:
        return self._kernel_task_client().post_agent_task(payload)

    def _list_agent_tasks(
        self, *, kind: str, domain: str, completed: bool = False, limit: int = 50
    ) -> dict:
        return self._kernel_task_client().list_agent_tasks(
            kind=kind,
            domain=domain,
            completed=completed,
            limit=limit,
        )

    def _assert_agent_task_visible(self, task_id: str, *, kind: str, domain: str) -> None:
        for completed in (False, True):
            payload = self._list_agent_tasks(kind=kind, domain=domain, completed=completed)
            tasks = payload.get("tasks") if isinstance(payload, dict) else []
            if any(isinstance(task, dict) and task.get("id") == task_id for task in tasks or []):
                return
        raise RuntimeError(
            f"agent task read-back incomplete: {task_id} not visible in pending or completed mempool"
        )

    def _kernel_task_client(self) -> KernelTaskClient:
        client = getattr(self, "kernel_tasks", None)
        if client is None:
            client = KernelTaskClient(self.base_url, self.api_key)
            self.kernel_tasks = client
        return client

    def _content_for_intent(self, intent: str, patch: str) -> dict:
        payload = build_lifecycle_task_payload(
            intent=intent,
            domain=self.lifecycle["task_domain"],
            agent_id="organ-forge-cli",
            registry=self.registry,
        )
        content = json_object_or_raise(payload["content"], RuntimeError("payload.content is not valid JSON"))
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
        task_id = response.get("task_id")
        if not task_id:
            raise RuntimeError(f"agent task dispatch response missing task_id: {response}")
        self._assert_agent_task_visible(
            task_id,
            kind=payload["kind"],
            domain=payload["domain"],
        )
        yield json.dumps(response, separators=(",", ":")) + "\n"

    async def listen_events(self):
        import httpx
        url = f"{self.base_url}/events"
        headers = kernel_auth_headers(self.api_key, content_type=False)
            
        async with httpx.AsyncClient(timeout=None) as client:
            async with client.stream("GET", url, headers=headers) as response:
                response.raise_for_status()
                async for line in response.aiter_lines():
                    if line.startswith("data: "):
                        data = line[6:]
                        if data:
                            yield data
