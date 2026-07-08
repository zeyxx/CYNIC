from __future__ import annotations

from datetime import datetime, timezone
from typing import Any, Dict

from .ports import ObservationPort, SupplyChainAnalysisPort


class LifecycleRuntime:
    """Application service that turns declared supply-chain truth into an operational report."""

    def __init__(
        self,
        *,
        analyzer: SupplyChainAnalysisPort,
        observation_port: ObservationPort | None = None,
        agent_id: str = "organ-lifecycle-runtime",
        domain: str = "organ-lifecycle",
    ):
        self.analyzer = analyzer
        self.observation_port = observation_port
        self.agent_id = agent_id
        self.domain = domain

    async def snapshot(self) -> Dict[str, Any]:
        return self.analyzer.analyze_supply_chain()

    async def run_once(self) -> Dict[str, Any]:
        report = await self.snapshot()
        observed_at = (
            datetime.now(timezone.utc)
            .replace(microsecond=0)
            .isoformat()
            .replace("+00:00", "Z")
        )
        signal = {
            "tool": "lifecycle_runtime",
            "domain": self.domain,
            "status": report.get("status", "unknown"),
            "agent_id": self.agent_id,
            "consumer": "organ-forge-hermes-agent",
            "action": "review_supply_chain",
            "value": report,
            "confidence": "observed",
            "timestamp": observed_at,
        }
        if self.observation_port is not None:
            await self.observation_port.observe(signal)
        return {
            "status": report.get("status", "unknown"),
            "report": report,
            "signal": signal,
        }
