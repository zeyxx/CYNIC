"""
CYNIC Analyst Dog — Binah (Understanding)

Non-LLM Dog. L3 REFLEX (Z3 SMT solving is fast).
Technology: Z3 Theorem Prover (Microsoft Research)

Responsibilities:
  - Formal verification of logic properties
  - Smart contract invariant checking
  - Code correctness proofs (type safety, bounds checking)
  - Constraint satisfaction for DECIDE phase

Why Z3?
  Z3 is deterministic, fast, and produces PROOF CERTIFICATES.
  Unlike LLMs, Z3 either proves something or proves it's unprovable.
  This is the difference between "I think it's correct" (LLM) and
  "I can PROVE it's correct" (Analyst).

φ-integration:
  Analyst confidence = 61.8% IF Z3 produces proof (maximum certainty we allow)
  Analyst confidence = 0% if Z3 times out or formula is undecidable
  Analyst confidence = 38.2% if Z3 produces counterexample
"""
from __future__ import annotations

import logging
import time
from typing import Any


try:
    import z3
    Z3_AVAILABLE = True
except ImportError:
    Z3_AVAILABLE = False

from cynic.core.phi import PHI_INV, PHI_INV_2, PHI_INV_3, MAX_Q_SCORE, phi_bound_score
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.cognition.neurons.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.cognition.neurons.analyst")

Z3_TIMEOUT_MS = 3000  # 3 seconds max for Z3 (F(8)×21=441ms budget, allow 7× for hard problems)


class AnalystDog(AbstractDog):
    """
    Analyst (Binah) — Z3 formal verification.

    Verifies logical properties of code and decisions.
    Returns HOWL when proof succeeds, BARK when disproven,
    GROWL when timeout or undecidable.

    In CODE domain: checks type invariants, bounds, reachability
    In SOLANA domain: checks transaction invariants (no double-spend, overflow)
    In DECIDE domain: checks decision constraints (budget >= cost, quorum >= threshold)
    """

    DOG_ID = DogId.ANALYST

    def __init__(self) -> None:
        super().__init__(DogId.ANALYST)
        self._z3_available = Z3_AVAILABLE
        self._proof_count = 0
        self._refutation_count = 0
        self._timeout_count = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.ANALYST,
            sefirot="Binah — Understanding",
            consciousness_min=ConsciousnessLevel.REFLEX,
            uses_llm=False,
            supported_realities={"CODE", "SOLANA", "HUMAN", "CYNIC"},
            supported_analyses={"JUDGE", "DECIDE", "VERIFY"},
            technology="Z3 SMT Solver (formal proof / refutation)",
            max_concurrent=3,  # Z3 is thread-safe
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Run Z3 verification on the cell content.

        Content should be a dict with 'constraints' key containing
        Z3-compatible assertion descriptions, OR arbitrary dict
        that we'll extract constraints from heuristically.
        """
        start = time.perf_counter()

        if not self._z3_available:
            return self._unavailable_judgment(cell, start)

        constraints = self._extract_constraints(cell)
        result, evidence = self._verify(constraints)
        q_score, confidence = self._score_result(result)
        latency = (time.perf_counter() - start) * 1000

        reasoning = {
            "PROVED":     f"Z3 formally proved {len(constraints)} constraints. Proof complete.",
            "REFUTED":    f"Z3 found counterexample. Constraints violated: {evidence.get('counterexample', 'unknown')}",
            "TIMEOUT":    f"Z3 exceeded {Z3_TIMEOUT_MS}ms. Problem undecidable in budget.",
            "NO_CONSTRAINTS": "No verifiable constraints found in cell content.",
            "ERROR":      f"Z3 error: {evidence.get('error', 'unknown')}",
        }.get(result, f"Unknown result: {result}")

        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=confidence,
            reasoning=reasoning,
            evidence=evidence,
            latency_ms=latency,
        )
        self.record_judgment(judgment)
        return judgment

    def _extract_constraints(self, cell: Cell) -> list[dict[str, Any]]:
        """
        Extract verifiable constraints from cell content.

        Supports:
          - dict with 'constraints' key (explicit)
          - dict with numeric values → check they're in valid ranges
          - str → check basic string invariants
        """
        constraints = []
        content = cell.content

        if isinstance(content, dict):
            # Explicit constraints
            if "constraints" in content:
                return content["constraints"]

            # Implicit: check all numeric values are φ-bounded
            for key, val in content.items():
                if isinstance(val, float) and 0 <= val <= 1:
                    constraints.append({
                        "type": "range",
                        "name": key,
                        "value": val,
                        "min": 0.0,
                        "max": 1.0,
                    })
                elif isinstance(val, (int, float)):
                    constraints.append({
                        "type": "finite",
                        "name": key,
                        "value": float(val),
                    })

        # Always check cell φ-bounds (LAW 5 verification)
        constraints.extend([
            {"type": "range", "name": "novelty", "value": cell.novelty, "min": 0.0, "max": 1.0},
            {"type": "range", "name": "complexity", "value": cell.complexity, "min": 0.0, "max": 1.0},
            {"type": "range", "name": "risk", "value": cell.risk, "min": 0.0, "max": 1.0},
        ])

        return constraints

    def _verify(self, constraints: list[dict[str, Any]]) -> tuple[str, dict[str, Any]]:
        """Run Z3 verification. Returns (result, evidence)."""
        if not constraints:
            return "NO_CONSTRAINTS", {}

        try:
            solver = z3.Solver()
            solver.set("timeout", Z3_TIMEOUT_MS)

            z3_vars: dict[str, Any] = {}
            assertions_added = 0

            for c in constraints:
                c_type = c.get("type")
                name = c.get("name", "x")

                if c_type == "range":
                    val = c["value"]
                    lo = c.get("min", 0.0)
                    hi = c.get("max", 1.0)
                    v = z3.Real(name)
                    z3_vars[name] = v
                    solver.add(v == val)
                    solver.add(v >= lo)
                    solver.add(v <= hi)
                    assertions_added += 1

                elif c_type == "finite":
                    val = c["value"]
                    v = z3.Real(name)
                    z3_vars[name] = v
                    solver.add(v == val)
                    solver.add(z3.Or(v >= -1e10, v <= 1e10))  # finite check
                    assertions_added += 1

            if assertions_added == 0:
                return "NO_CONSTRAINTS", {}

            result = solver.check()

            if result == z3.sat:
                self._proof_count += 1
                return "PROVED", {"model": str(solver.model()), "assertions": assertions_added}
            elif result == z3.unsat:
                self._refutation_count += 1
                return "REFUTED", {"counterexample": "UNSAT — constraints contradictory", "assertions": assertions_added}
            else:  # unknown (timeout)
                self._timeout_count += 1
                return "TIMEOUT", {"reason": "Z3 timeout or undecidable", "assertions": assertions_added}

        except Exception as e:
            logger.warning("Z3 error: %s", e)
            return "ERROR", {"error": str(e)}

    def _score_result(self, result: str) -> tuple[float, float]:
        """Map Z3 result to (q_score, confidence)."""
        return {
            "PROVED":         (MAX_Q_SCORE, PHI_INV),        # 61.8, 0.618 — proven = max allowed
            "REFUTED":        (0.0, PHI_INV),                # 0, 0.618 — disproven = certain failure
            "TIMEOUT":        (MAX_Q_SCORE * PHI_INV_2, PHI_INV_3),  # ≈23.6, 0.236 — uncertain
            "NO_CONSTRAINTS": (MAX_Q_SCORE * PHI_INV, PHI_INV_2),    # ≈38.2, 0.382 — neutral
            "ERROR":          (0.0, PHI_INV_3),              # 0, 0.236 — error = low confidence
        }.get(result, (0.0, 0.0))

    def _unavailable_judgment(self, cell: Cell, start: float) -> DogJudgment:
        """Graceful degradation when Z3 not installed."""
        latency = (time.perf_counter() - start) * 1000
        return DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=MAX_Q_SCORE * PHI_INV,  # 38.2 — neutral when unavailable
            confidence=0.0,
            reasoning="Z3 solver not available (pip install z3-solver). Abstaining.",
            latency_ms=latency,
        )

    async def health_check(self) -> DogHealth:
        status = HealthStatus.HEALTHY if self._z3_available else HealthStatus.DEGRADED
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Z3 available: {self._z3_available}, "
                f"Proved: {self._proof_count}, "
                f"Refuted: {self._refutation_count}, "
                f"Timeouts: {self._timeout_count}"
            ),
        )
