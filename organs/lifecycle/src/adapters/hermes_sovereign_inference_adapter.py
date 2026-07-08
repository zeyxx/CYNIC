"""Sovereign inference adapter backed by Hermes-Agent tasks."""

from __future__ import annotations

import json
from typing import Any, Dict

from organs.contracts.src.hermes_dispatch import HermesDispatcher
from organs.lifecycle.src.core.dependency_convergence import (
    build_dependency_convergence_hermes_task,
)
from organs.lifecycle.src.core.ports import SovereignInferencePort


class HermesAgentSovereignInferenceAdapter(SovereignInferencePort):
    """Dispatch bounded inference work to Hermes-Agent through the Kernel mempool."""

    def __init__(self, dispatcher: HermesDispatcher | None = None):
        self.dispatcher = dispatcher or HermesDispatcher("organ-lifecycle")

    def request_dependency_convergence(self, convergence_pack: Dict[str, Any]) -> Dict[str, Any]:
        proposal = convergence_pack.get("proposal", {})
        if proposal.get("requires_sovereign_inference") is not True:
            return {
                "status": "skipped",
                "adapter": "hermes-agent",
                "reason": "deterministic analysis found no cycle requiring sovereign inference",
                "kind": "hermes",
                "domain": "organ-lifecycle",
                "consumer": "organ-lifecycle-hermes-agent",
                "targets": [],
                "coord_targets": [],
            }
        task = build_dependency_convergence_hermes_task(convergence_pack)
        payload = {
            **task,
            "content": json.dumps(task["content"], separators=(",", ":")),
        }
        ok = self.dispatcher.dispatch_task(payload)
        return {
            "status": "queued" if ok else "blocked",
            "adapter": "hermes-agent",
            "kind": payload["kind"],
            "domain": payload["domain"],
            "consumer": payload["consumer"],
            "targets": task["content"]["targets"],
            "coord_targets": task["content"]["coord_targets"],
        }
