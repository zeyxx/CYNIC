"""
CYNIC SelfProber — L4 CYNIC→CYNIC Self-improvement loop

Listens to EMERGENCE_DETECTED (from ResidualDetector). When a pattern
(SPIKE / RISING / STABLE_HIGH) is flagged, SelfProber analyzes three
internal signals and generates improvement proposals:

  1. QTABLE   — state-action pairs with Q-value < φ² (38.2%) after ≥3 visits
  2. ESCORE   — dogs with JUDGE reputation score < 38.2 (below GROWL floor)
  3. CONFIG   — pattern-specific parameter suggestions (axiom weights, thresholds)

Each analysis produces zero or more SelfProposal dataclasses.
Proposals are:
  • Persisted to ~/.cynic/self_proposals.json (rolling cap F(10)=55)
  • Emitted as SELF_IMPROVEMENT_PROPOSED on the core bus
  • Exposed via /self-probes API and CLI `probes` command

No LLM calls. Pure in-memory analysis — runs in <1ms.

Lifecycle:
  prober = SelfProber()
  prober.set_qtable(qtable)
  prober.set_residual_detector(residual_detector)
  prober.set_escore_tracker(escore_tracker)
  prober.start(get_core_bus())   # subscibe to EMERGENCE_DETECTED
"""
from __future__ import annotations


import json
import logging
import os
import random
import time
from dataclasses import dataclass, field
from typing import Any


from cynic.core.phi import PHI_INV_2, fibonacci
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import SelfImprovementProposedPayload

logger = logging.getLogger("cynic.judge.self_probe")

_CYNIC_DIR = os.path.join(os.path.expanduser("~"), ".cynic")
_PROPOSALS_PATH = os.path.join(_CYNIC_DIR, "self_proposals.json")

# Rolling cap: F(10) = 55
_MAX_PROPOSALS: int = fibonacci(10)   # 55

# QTable: value below this is "persistently low" (GROWL floor normalized)
_LOW_Q_THRESHOLD: float = PHI_INV_2   # 0.382

# Min visits for a QTable entry to be worth analyzing
_MIN_VISITS: int = fibonacci(4)       # 3


def _short_id() -> str:
    return "%08x" % random.getrandbits(32)


# ── Dataclass ────────────────────────────────────────────────────────────────

@dataclass
class SelfProposal:
    """One self-improvement recommendation."""
    probe_id: str
    trigger: str        # EMERGENCE | SCHEDULE | MANUAL
    pattern_type: str   # SPIKE | RISING | STABLE_HIGH | QTABLE | ESCORE | CONFIG
    severity: float     # [0, 1]
    dimension: str      # QTABLE | ESCORE | RESIDUAL | CONFIG
    target: str         # dog name, state_key:action, or parameter name
    recommendation: str # Human-readable improvement suggestion
    current_value: float
    suggested_value: float
    proposed_at: float = field(default_factory=time.time)
    status: str = "PENDING"   # PENDING | APPLIED | DISMISSED

    def to_dict(self) -> dict[str, Any]:
        return {
            "probe_id":       self.probe_id,
            "trigger":        self.trigger,
            "pattern_type":   self.pattern_type,
            "severity":       round(self.severity, 4),
            "dimension":      self.dimension,
            "target":         self.target,
            "recommendation": self.recommendation,
            "current_value":  round(self.current_value, 4),
            "suggested_value": round(self.suggested_value, 4),
            "proposed_at":    self.proposed_at,
            "status":         self.status,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> SelfProposal:
        return cls(
            probe_id=d["probe_id"],
            trigger=d["trigger"],
            pattern_type=d["pattern_type"],
            severity=float(d["severity"]),
            dimension=d["dimension"],
            target=d["target"],
            recommendation=d["recommendation"],
            current_value=float(d["current_value"]),
            suggested_value=float(d["suggested_value"]),
            proposed_at=float(d["proposed_at"]),
            status=d.get("status", "PENDING"),
        )


# ── SelfProber ───────────────────────────────────────────────────────────────

class SelfProber:
    """
    L4 CYNIC→CYNIC self-improvement loop.

    Subscribes to EMERGENCE_DETECTED. On each event, runs three analyses
    and generates SelfProposal recommendations. Persists to disk.

    Wire at build_kernel():
      prober = SelfProber()
      prober.set_qtable(qtable)
      prober.set_residual_detector(residual_detector)
      prober.set_escore_tracker(escore_tracker)
      prober.start(get_core_bus())
    """

    def __init__(self, proposals_path: str = _PROPOSALS_PATH) -> None:
        self._path = proposals_path
        self._proposals: list[SelfProposal] = []
        self._total_proposed: int = 0
        self._qtable: Any | None = None
        self._residual: Any | None = None
        self._escore: Any | None = None
        self._registered: bool = False
        self._load()

    # ── Injection ─────────────────────────────────────────────────────────

    def set_qtable(self, qtable: Any) -> None:
        self._qtable = qtable

    def set_residual_detector(self, detector: Any) -> None:
        self._residual = detector

    def set_escore_tracker(self, tracker: Any) -> None:
        self._escore = tracker

    def set_handler_registry(self, registry: Any) -> None:
        """Inject HandlerRegistry for architecture analysis."""
        self._handler_registry = registry

    # ── Lifecycle ─────────────────────────────────────────────────────────

    def start(self, bus=None) -> None:
        """Subscribe to EMERGENCE_DETECTED. Call once at kernel startup."""
        if self._registered:
            return
        target_bus = bus or get_core_bus()
        target_bus.on(CoreEvent.EMERGENCE_DETECTED, self._on_emergence)
        self._registered = True
        logger.info("SelfProber subscribed to EMERGENCE_DETECTED")

    # ── Public API ────────────────────────────────────────────────────────

    def all_proposals(self) -> list[SelfProposal]:
        return list(self._proposals)

    def pending(self) -> list[SelfProposal]:
        return [p for p in self._proposals if p.status == "PENDING"]

    def get(self, probe_id: str) -> SelfProposal | None:
        for p in self._proposals:
            if p.probe_id == probe_id:
                return p
        return None

    def dismiss(self, probe_id: str) -> SelfProposal | None:
        for p in self._proposals:
            if p.probe_id == probe_id:
                p.status = "DISMISSED"
                self._save()
                return p
        return None

    def apply(self, probe_id: str) -> SelfProposal | None:
        for p in self._proposals:
            if p.probe_id == probe_id:
                p.status = "APPLIED"
                self._save()
                return p
        return None

    def stats(self) -> dict[str, Any]:
        counts = {"PENDING": 0, "APPLIED": 0, "DISMISSED": 0}
        for p in self._proposals:
            counts[p.status] = counts.get(p.status, 0) + 1
        return {
            "proposed_total": self._total_proposed,
            "queue_size":     len(self._proposals),
            "pending":        counts["PENDING"],
            "applied":        counts["APPLIED"],
            "dismissed":      counts["DISMISSED"],
        }

    # ── Analysis ──────────────────────────────────────────────────────────

    def analyze(
        self,
        trigger: str = "MANUAL",
        pattern_type: str = "UNKNOWN",
        severity: float = 0.5,
    ) -> list[SelfProposal]:
        """
        Run all three analyses and return new proposals generated.

        Can be called manually (e.g. from CLI or test) or automatically
        via _on_emergence().
        """
        new_proposals: list[SelfProposal] = []
        new_proposals.extend(self._analyze_qtable(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_escore(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_residual(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_architecture(trigger, pattern_type, severity))

        for proposal in new_proposals:
            self._proposals.append(proposal)
            self._total_proposed += 1

        # Rolling cap — evict oldest first
        while len(self._proposals) > _MAX_PROPOSALS:
            self._proposals.pop(0)

        if new_proposals:
            self._save()

        return new_proposals

    # ── Analysis: QTable ──────────────────────────────────────────────────

    def _analyze_qtable(
        self, trigger: str, pattern_type: str, severity: float,
    ) -> list[SelfProposal]:
        """
        Find the worst-performing QTable entry (lowest Q-value with enough visits).
        Generates at most 1 proposal (the single worst entry).
        """
        if self._qtable is None:
            return []

        try:
            table = self._qtable._table  # {state_key: {action: {value, visits}}}
        except AttributeError:
            return []

        worst: tuple | None = None  # (value, visits, state_key, action)
        for state_key, actions in table.items():
            for action, entry in actions.items():
                value  = float(entry.get("value", 1.0))
                visits = int(entry.get("visits", 0))
                if visits >= _MIN_VISITS and value < _LOW_Q_THRESHOLD:
                    if worst is None or value < worst[0]:
                        worst = (value, visits, state_key, action)

        if worst is None:
            return []

        value, visits, state_key, action = worst
        rec = (
            f"QTable: '{state_key[:40]}' action '{action}' Q={value:.3f} "
            f"after {visits} visits (below φ² threshold {_LOW_Q_THRESHOLD:.3f}). "
            f"Investigate heuristic coverage or axiom weights for this cell."
        )
        return [SelfProposal(
            probe_id=_short_id(),
            trigger=trigger,
            pattern_type=pattern_type,
            severity=severity,
            dimension="QTABLE",
            target=f"{state_key[:30]}:{action}",
            recommendation=rec[:240],
            current_value=value,
            suggested_value=min(value + 0.10, _LOW_Q_THRESHOLD),
        )]

    # ── Analysis: EScore ──────────────────────────────────────────────────

    def _analyze_escore(
        self, trigger: str, pattern_type: str, severity: float,
    ) -> list[SelfProposal]:
        """
        Find dogs with JUDGE E-Score < 38.2 (below GROWL floor).
        Generates one proposal per underperforming dog (max 3).
        """
        if self._escore is None:
            return []

        try:
            entities = self._escore._scores  # {agent_id: {dim: score}}
        except AttributeError:
            return []

        proposals = []
        for agent_id, dim_scores in entities.items():
            if not agent_id.startswith("agent:"):
                continue
            judge_score = float(dim_scores.get("JUDGE", 100.0))
            if judge_score < 38.2:
                dog_name = agent_id.replace("agent:", "")
                rec = (
                    f"Dog '{dog_name}' JUDGE E-Score={judge_score:.1f} "
                    f"(below GROWL threshold 38.2). "
                    f"Consider excluding from MACRO cycles until score recovers."
                )
                proposals.append(SelfProposal(
                    probe_id=_short_id(),
                    trigger=trigger,
                    pattern_type=pattern_type,
                    severity=min(severity, (38.2 - judge_score) / 38.2),
                    dimension="ESCORE",
                    target=dog_name,
                    recommendation=rec[:240],
                    current_value=judge_score,
                    suggested_value=38.2,
                ))
            if len(proposals) >= 3:
                break

        return proposals

    # ── Analysis: Residual / Config ───────────────────────────────────────

    def _analyze_residual(
        self, trigger: str, pattern_type: str, severity: float,
    ) -> list[SelfProposal]:
        """
        Pattern-specific parameter suggestions.
          STABLE_HIGH → re-run probe calibration (axiom weights drift)
          RISING      → reduce ANOMALY_THRESHOLD or add dog diversity
        """
        if self._residual is None:
            return []
        if pattern_type not in ("STABLE_HIGH", "RISING"):
            return []

        if pattern_type == "STABLE_HIGH":
            rec = (
                f"STABLE_HIGH residual (severity={severity:.3f}): "
                f"Dogs consistently disagree across {fibonacci(5)} judgments. "
                f"Run P1-P5 probe calibration to re-baseline axiom weights."
            )
            return [SelfProposal(
                probe_id=_short_id(),
                trigger=trigger,
                pattern_type=pattern_type,
                severity=severity,
                dimension="CONFIG",
                target="axiom_weights",
                recommendation=rec[:240],
                current_value=severity,
                suggested_value=0.0,
            )]

        # RISING
        rec = (
            f"RISING residual pattern (severity={severity:.3f}): "
            f"Dog disagreement is escalating. "
            f"Consider lowering ANOMALY_THRESHOLD (0.382→0.300) or adding dog diversity."
        )
        return [SelfProposal(
            probe_id=_short_id(),
            trigger=trigger,
            pattern_type=pattern_type,
            severity=severity,
            dimension="CONFIG",
            target="residual_threshold",
            recommendation=rec[:240],
            current_value=0.382,
            suggested_value=0.300,
        )]

    # ── Analysis: Architecture ────────────────────────────────────────────────

    def _analyze_architecture(
        self, trigger: str, pattern_type: str, severity: float,
    ) -> list[SelfProposal]:
        """Handler coupling analysis — the organism understands its own structure."""
        proposals: list[SelfProposal] = []
        if not hasattr(self, "_handler_registry") or self._handler_registry is None:
            return proposals

        try:
            topo = self._handler_registry.introspect()
            total_deps = topo.get("total_deps", 0)

            # Check 1: Total dependency count
            if total_deps > 25:
                severity_score = min(total_deps / 30, 1.0)
                proposals.append(SelfProposal(
                    probe_id=_short_id(),
                    trigger=trigger,
                    pattern_type="ARCHITECTURE_COUPLING",
                    severity=severity_score,
                    dimension="COUPLING",
                    target="handler_registry",
                    recommendation=f"Total handler deps={total_deps} (cap: 25). Consider splitting groups.",
                    current_value=float(total_deps),
                    suggested_value=25.0,
                ))

            # Check 2: Individual group coupling
            for group in topo.get("groups", []):
                if len(group.get("dependencies", [])) > 8:
                    proposals.append(SelfProposal(
                        probe_id=_short_id(),
                        trigger=trigger,
                        pattern_type="ARCHITECTURE_COUPLING",
                        severity=0.7,
                        dimension="COUPLING",
                        target=group["name"],
                        recommendation=f"{group['name']}: {len(group['dependencies'])} deps (cap: 8). Decompose.",
                        current_value=float(len(group["dependencies"])),
                        suggested_value=8.0,
                    ))

        except httpx.RequestError:
            logger.debug("_analyze_architecture error", exc_info=True)

        return proposals

    # ── Event handler ─────────────────────────────────────────────────────

    async def _on_emergence(self, event: Event) -> None:
        """Handle EMERGENCE_DETECTED → analyze → emit SELF_IMPROVEMENT_PROPOSED."""
        try:
            payload = event.payload if isinstance(event.payload, dict) else {}
            pattern_type = payload.get("pattern_type", "UNKNOWN")
            severity     = float(payload.get("severity", 0.5))

            new_proposals = self.analyze(
                trigger=     "EMERGENCE",
                pattern_type=pattern_type,
                severity=    severity,
            )

            if not new_proposals:
                return

            await get_core_bus().emit(Event.typed(
                CoreEvent.SELF_IMPROVEMENT_PROPOSED,
                SelfImprovementProposedPayload(
                    trigger="EMERGENCE",
                    pattern_type=pattern_type,
                    severity=severity,
                    proposals=[p.to_dict() for p in new_proposals],
                    total_pending=len(self.pending()),
                ),
                source="self_prober",
            ))
            logger.info(
                "SelfProber: %d proposals generated (pattern=%s severity=%.3f)",
                len(new_proposals), pattern_type, severity,
            )
        except CynicError as exc:
            logger.warning("SelfProber._on_emergence error: %s", exc)

    # ── Persistence ───────────────────────────────────────────────────────

    def _save(self) -> None:
        try:
            os.makedirs(os.path.dirname(self._path), exist_ok=True)
            with open(self._path, "w", encoding="utf-8") as fh:
                json.dump([p.to_dict() for p in self._proposals], fh, indent=2)
        except OSError as exc:
            logger.debug("SelfProber._save failed: %s", exc)

    def _load(self) -> None:
        try:
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            if isinstance(data, list):
                for d in data:
                    self._proposals.append(SelfProposal.from_dict(d))
                self._total_proposed = len(self._proposals)
        except (json.JSONDecodeError, FileNotFoundError):
            pass
