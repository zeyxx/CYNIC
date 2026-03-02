"""
P10 ProbesService — Business logic layer for probe management.

Decouples API and CLI from SelfProber implementation.
Handles status filtering, async operations, error handling.
"""

from typing import Optional, Any
import logging

from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
from cynic.kernel.core.event_bus import EventBus

logger = logging.getLogger("cynic.probes_service")


class ProbesService:
    """
    Business logic for SelfProber probe management.

    Provides interface:
    - list_probes(status) — filter by status
    - get_probe(probe_id) — single probe details
    - apply_probe(probe_id) — async apply + execute if LOW_RISK
    - dismiss_probe(probe_id) — mark dismissed
    - get_stats() — aggregate counts
    """

    def __init__(self, prober: SelfProber, bus: Optional[EventBus] = None):
        """
        Initialize service.

        Args:
            prober: SelfProber instance (source of truth)
            bus: EventBus for future event emission
        """
        self._prober = prober
        self._bus = bus

    # ─── Read Operations (Sync) ──────────────────────────────────────────

    def list_probes(self, status: str = "PENDING") -> list[dict[str, Any]]:
        """
        List proposals filtered by status.

        Args:
            status: PENDING | APPLIED | DISMISSED | ALL

        Returns:
            List of proposal dicts

        Raises:
            ValueError: Invalid status value
        """
        if status not in ("PENDING", "APPLIED", "DISMISSED", "ALL"):
            raise ValueError(f"Invalid status: {status}")

        if status == "ALL":
            proposals = self._prober.all_proposals()
        elif status == "PENDING":
            proposals = self._prober.pending()
        else:
            proposals = [p for p in self._prober.all_proposals() if p.status == status]

        return [p.to_dict() for p in proposals]

    def get_probe(self, probe_id: str) -> Optional[dict[str, Any]]:
        """
        Get single probe by ID.

        Args:
            probe_id: Probe identifier

        Returns:
            Proposal dict or None if not found
        """
        proposal = self._prober.get(probe_id)
        return proposal.to_dict() if proposal else None

    def get_stats(self) -> dict[str, Any]:
        """
        Get aggregate statistics.

        Returns:
            Dict with proposed_total, queue_size, pending, applied, dismissed
        """
        return self._prober.stats()

    # ─── Write Operations (Async) ────────────────────────────────────────

    async def apply_probe(self, probe_id: str) -> dict[str, Any]:
        """
        Apply a proposal.

        - Marks status = "APPLIED"
        - If executor available and LOW_RISK, executes immediately
        - Emits PROPOSAL_EXECUTED or PROPOSAL_FAILED event

        Args:
            probe_id: Probe identifier

        Returns:
            Result dict with status, probe_id, message

        Raises:
            ValueError: Probe not found
        """
        proposal = await self._prober.apply_async(probe_id)

        if proposal is None:
            raise ValueError(f"Probe {probe_id} not found")

        return {
            "status": "success",
            "probe_id": proposal.probe_id,
            "applied_status": proposal.status,
            "dimension": proposal.dimension,
            "message": f"Proposal {probe_id} applied"
        }

    def dismiss_probe(self, probe_id: str) -> dict[str, Any]:
        """
        Dismiss a proposal (mark as DISMISSED).

        Args:
            probe_id: Probe identifier

        Returns:
            Result dict with status, probe_id, message

        Raises:
            ValueError: Probe not found
        """
        proposal = self._prober.dismiss(probe_id)

        if proposal is None:
            raise ValueError(f"Probe {probe_id} not found")

        return {
            "status": "success",
            "probe_id": proposal.probe_id,
            "dismissed_status": proposal.status,
            "message": f"Proposal {probe_id} dismissed"
        }
