"""
CYNIC AccountAgent — Step 6 (ACCOUNT): Cost Ledger + Budget Enforcement

Subscribes to JUDGMENT_CREATED. Tracks cumulative costs across:
  - Session total
  - Per-reality (CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS)
  - Per-dog (SAGE, GUARDIAN, ANALYST, ...)

Budget enforcement (φ-derived thresholds):
  BUDGET_WARNING   emitted at PHI_INV_2 = 38.2% of budget remaining
  BUDGET_EXHAUSTED emitted at 0% remaining

EScore integration (RUN dimension):
  Each Dog's RUN E-Score is updated per judgment.
  Free Ollama inference with high Q → RUN = 100.
  Expensive inference with low Q → RUN → 0.
  Formula: cost_per_q = cost_usd / max(q, 0.1)
           run = 100 × (1 - cost_per_q / CAP)  clamped [0, 100]

The AccountAgent closes step 6 of the 7-step cycle:
  PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
"""
from __future__ import annotations

import logging
import time
from typing import Any, Dict, Optional

from cynic.core.event_bus import CoreEvent, Event, EventBus, get_core_bus
from cynic.core.phi import PHI_INV_2, MAX_Q_SCORE

logger = logging.getLogger("cynic.judge.account")

# φ-derived warning threshold: warn when 38.2% of budget remains
_WARNING_RATIO: float = PHI_INV_2            # 0.382

# Default session budget (10 USD — covers ~100 Ollama-heavy MACRO cycles)
_DEFAULT_SESSION_BUDGET_USD: float = 10.0

# RUN score calibration: cost_per_q ($/Q-point) → RUN in [0, 100]
# cost=0 (free Ollama) → 100. cap=$0.02/Q-pt → 0.
_COST_EFFICIENCY_CAP: float = 0.02          # $/Q-point at which RUN → 0


class AccountAgent:
    """
    Step 6 (ACCOUNT): Cost ledger, budget enforcement, EScore-RUN update.

    Subscribes to JUDGMENT_CREATED. Fire-and-forget — never blocks pipeline.
    All state is in-memory (starts fresh each session).
    """

    def __init__(
        self,
        session_budget_usd: float = _DEFAULT_SESSION_BUDGET_USD,
        escore_tracker: Optional[Any] = None,
    ) -> None:
        self._session_budget_usd = session_budget_usd
        self._escore_tracker = escore_tracker   # Optional[EScoreTracker]
        self._total_cost_usd: float = 0.0
        self._cost_by_reality: Dict[str, float] = {}
        self._cost_by_dog: Dict[str, float] = {}
        self._judgment_count: int = 0
        self._warning_emitted: bool = False
        self._exhausted_emitted: bool = False
        self._started_at: float = time.time()
        self._handler = self._on_judgment

    # ── Lifecycle ────────────────────────────────────────────────────────

    def start(self, bus: EventBus) -> None:
        """Subscribe to JUDGMENT_CREATED. Wire before first judgment."""
        bus.on(CoreEvent.JUDGMENT_CREATED, self._handler)
        logger.info(
            "AccountAgent started — budget=$%.2f, EScore=%s",
            self._session_budget_usd,
            "wired" if self._escore_tracker else "none",
        )

    def stop(self, bus: EventBus) -> None:
        """Unsubscribe from JUDGMENT_CREATED."""
        bus.off(CoreEvent.JUDGMENT_CREATED, self._handler)
        logger.info("AccountAgent stopped — total cost=$%.4f", self._total_cost_usd)

    def set_escore_tracker(self, tracker: Any) -> None:
        """Inject EScoreTracker (dependency injection, avoids circular imports)."""
        self._escore_tracker = tracker

    # ── Handler ──────────────────────────────────────────────────────────

    async def _on_judgment(self, event: Event) -> None:
        try:
            p = event.payload or {}
            cost = float(p.get("cost_usd", 0.0))
            q_score = float(p.get("q_score", 0.0))
            reality = str(p.get("reality", "UNKNOWN"))
            dog_votes: dict = p.get("dog_votes") or {}

            # Accumulate totals
            self._total_cost_usd += cost
            self._judgment_count += 1
            self._cost_by_reality[reality] = (
                self._cost_by_reality.get(reality, 0.0) + cost
            )

            # Per-dog cost split + EScore-RUN update
            if dog_votes:
                per_dog_cost = cost / len(dog_votes)
                for dog_id, vote_score in dog_votes.items():
                    self._cost_by_dog[dog_id] = (
                        self._cost_by_dog.get(dog_id, 0.0) + per_dog_cost
                    )
                    if self._escore_tracker is not None:
                        run_score = self._compute_run_score(
                            q_score=float(vote_score),
                            cost_usd=per_dog_cost,
                        )
                        try:
                            self._escore_tracker.update(
                                f"agent:{dog_id}", "RUN", run_score, reality=reality
                            )
                        except Exception:
                            pass  # Bad reality string etc. — never block

            # Budget enforcement — emits BUDGET_WARNING / BUDGET_EXHAUSTED
            await self._check_budget()

        except Exception as exc:
            logger.debug("AccountAgent._on_judgment (non-fatal): %s", exc)

    # ── Budget enforcement ────────────────────────────────────────────────

    async def _check_budget(self) -> None:
        """Emit CoreEvent thresholds when budget crosses φ-derived limits."""
        remaining = self._session_budget_usd - self._total_cost_usd
        ratio_remaining = remaining / max(self._session_budget_usd, 1e-9)

        if not self._warning_emitted and ratio_remaining <= _WARNING_RATIO:
            self._warning_emitted = True
            await get_core_bus().emit(Event(
                type=CoreEvent.BUDGET_WARNING,
                payload={
                    "remaining_usd": round(remaining, 4),
                    "total_spent_usd": round(self._total_cost_usd, 4),
                    "ratio_remaining": round(ratio_remaining, 3),
                    "session_budget_usd": self._session_budget_usd,
                },
                source="account_agent",
            ))
            logger.warning(
                "BUDGET_WARNING: $%.4f of $%.2f remaining (%.1f%%)",
                remaining, self._session_budget_usd, ratio_remaining * 100,
            )

        if not self._exhausted_emitted and remaining <= 0.0:
            self._exhausted_emitted = True
            await get_core_bus().emit(Event(
                type=CoreEvent.BUDGET_EXHAUSTED,
                payload={
                    "total_spent_usd": round(self._total_cost_usd, 4),
                    "session_budget_usd": self._session_budget_usd,
                    "overspend_usd": round(-remaining, 4),
                },
                source="account_agent",
            ))
            logger.error(
                "BUDGET_EXHAUSTED: spent $%.4f of $%.2f budget",
                self._total_cost_usd, self._session_budget_usd,
            )

    # ── Cost efficiency ───────────────────────────────────────────────────

    @staticmethod
    def _compute_run_score(q_score: float, cost_usd: float) -> float:
        """
        Map (Q-score, cost) → RUN E-Score dimension [0, 100].

        Free inference (Ollama, cost=0) → 100 regardless of Q.
        High cost + low Q → approaches 0.

        cost_per_q = cost_usd / max(q_score, 0.1)
        run = 100 × (1 - cost_per_q / _COST_EFFICIENCY_CAP) clamped [0, 100]
        """
        if cost_usd <= 0.0:
            return 100.0   # Free (Ollama) → perfect efficiency
        cost_per_q = cost_usd / max(q_score, 0.1)
        run_score = 100.0 * (1.0 - cost_per_q / _COST_EFFICIENCY_CAP)
        return max(0.0, min(100.0, run_score))

    # ── Properties + Stats ────────────────────────────────────────────────

    @property
    def total_cost_usd(self) -> float:
        return self._total_cost_usd

    @property
    def budget_remaining_usd(self) -> float:
        return self._session_budget_usd - self._total_cost_usd

    def stats(self) -> Dict[str, Any]:
        remaining = self._session_budget_usd - self._total_cost_usd
        ratio_remaining = remaining / max(self._session_budget_usd, 1e-9)
        return {
            "total_cost_usd": round(self._total_cost_usd, 4),
            "session_budget_usd": self._session_budget_usd,
            "budget_remaining_usd": round(remaining, 4),
            "budget_ratio_remaining": round(ratio_remaining, 3),
            "judgment_count": self._judgment_count,
            "cost_by_reality": {
                k: round(v, 4) for k, v in sorted(self._cost_by_reality.items())
            },
            "cost_by_dog": {
                k: round(v, 4) for k, v in sorted(self._cost_by_dog.items())
            },
            "warning_emitted": self._warning_emitted,
            "exhausted_emitted": self._exhausted_emitted,
            "uptime_s": round(time.time() - self._started_at, 1),
        }
