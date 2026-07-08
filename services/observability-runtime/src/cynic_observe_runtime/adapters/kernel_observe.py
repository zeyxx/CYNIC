from __future__ import annotations

import importlib.util
import logging
import os
import requests
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from cynic_observe_runtime.ports.observation import ObservationPort
from cynic_observe_runtime.ports.hermes import HermesKanbanGraphPort


def _bootstrap_contracts_project_root() -> None:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    candidates = []
    if env_root:
        candidates.append(Path(env_root).expanduser().resolve())
    start = Path(__file__).resolve()
    candidates.extend(start.parents)
    for candidate in candidates:
        if (candidate / "organs" / "contracts").is_dir() and (candidate / "infra").is_dir():
            candidate_str = str(candidate)
            if candidate_str not in sys.path:
                sys.path.insert(0, candidate_str)
            return


_bootstrap_contracts_project_root()

try:
    from organs.contracts.src.adapters.hermes_kanban_graph import build_hermes_kanban_graph_event
except ModuleNotFoundError:
    _fallback_root = Path(os.environ.get("CYNIC_PROJECT_ROOT", Path(__file__).resolve().parents[5]))
    _contracts_adapter = _fallback_root / "organs" / "contracts" / "src" / "adapters" / "hermes_kanban_graph.py"
    spec = importlib.util.spec_from_file_location("cynic_contracts_hermes_kanban_graph", _contracts_adapter)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load shared Hermes kanban graph adapter from {_contracts_adapter}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    build_hermes_kanban_graph_event = module.build_hermes_kanban_graph_event


logger = logging.getLogger("kernel_observe_adapter")


class KernelObserveTransportMixin(ObservationPort, HermesKanbanGraphPort):
    kernel_addr: str

    def _headers(self) -> dict[str, str]:
        raise NotImplementedError

    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        try:
            payload = {
                "domain": domain,
                "agent_id": agent_id,
                "observed_at": datetime.now(timezone.utc).isoformat(),
                "content": content,
            }
            response = requests.post(
                f"{self.kernel_addr}/observe",
                headers=self._headers(),
                json=payload,
                timeout=3,
            )
            if response.status_code >= 400:
                logger.debug("observe rejected by kernel: HTTP %s", response.status_code)
                return False
            return True
        except requests.RequestException as e:
            logger.debug("observe unreachable (fire-and-forget): %s", e)
            return False

    def publish_hermes_kanban_event(
        self,
        *,
        task_id: str,
        task_domain: str,
        event: str,
        agent_id: str,
        hermes_kanban_task_id: Optional[str] = None,
        status: Optional[str] = None,
        payload: Optional[dict] = None,
    ) -> bool:
        return self.observe(
            domain="hermes_execution",
            content=build_hermes_kanban_graph_event(
                task_id=task_id,
                task_domain=task_domain,
                event=event,
                hermes_kanban_task_id=hermes_kanban_task_id,
                status=status,
                payload=payload,
            ),
            agent_id=agent_id,
        )
