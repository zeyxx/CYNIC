import json
import os
from urllib import error, request

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers
from organs.contracts.src.surface_contracts import json_loads_or_default
from organs.contracts.src.surface_contracts import json_object_or_raise

from ....forge.src.core.lifecycle_state import build_workflow_state
from ....forge.src.core.ports import IKernelClient
from ....forge.src.core.lifecycle_readiness import (
    activation_evidence_gaps as _readiness_activation_evidence_gaps,
    build_activation_evidence_gap_details as _readiness_build_activation_evidence_gap_details,
    fleet_nodes_ready as _readiness_fleet_nodes_ready,
    inspect_fleet as _readiness_inspect_fleet,
    lifecycle_blockers as _readiness_lifecycle_blockers,
)
from .kernel_adapter_policy import (
    build_lifecycle_task_payload,
    load_registry,
    normalize_kernel_base_url,
    validate_lifecycle_task_payload,
)


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
        if provider != "headscale":
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
            raw = kernel.get("network", {}).get("headscale", "")
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
            headers = kernel_auth_headers(self.api_key, content_type=False)
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
            health_payload = json_loads_or_default(raw, None)
            if not isinstance(health_payload, dict):
                self.kernel_probe.update({"status_error": "health_payload_invalid_json"})
        except error.HTTPError as exc:
            status = exc.code
            try:
                raw = exc.read().decode("utf-8", errors="replace")
            except Exception:
                raw = ""
            health_payload = json_loads_or_default(raw, None)
            if not isinstance(health_payload, dict):
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
            headers=kernel_auth_headers(self.api_key),
        )
        try:
            with request.urlopen(req, timeout=10) as resp:
                text = resp.read().decode("utf-8", errors="replace")
                return json_object_or_raise(text, RuntimeError("agent task dispatch response is not valid JSON"))
        except error.HTTPError as exc:
            text = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"agent task dispatch failed: HTTP {exc.code} {text[:200]}")
        except OSError as exc:
            raise RuntimeError(f"agent task dispatch failed: {exc}")

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
