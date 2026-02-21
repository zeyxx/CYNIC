"""Layer 0: Identity — CYNIC's Immutable DNA (Axioms as Constraints)

Axioms are NOT maturity scores or behavior labels.
Axioms are CONSTRAINTS that filter all behavior before execution.

This layer is present BEFORE consciousness awakens (before REFLEX).
This layer NEVER changes during runtime.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Callable, Any
import json
from cynic.core.phi import MAX_CONFIDENCE, MAX_Q_SCORE


class AxiomConstraint(Enum):
    """Immutable axioms as executable constraints.

    Each axiom has:
    - name: Human identifier
    - description: What it constrains
    - constraint: Function that validates judgments
    - violations: Accumulated violations (for learning)
    """

    FIDELITY = {
        "name": "FIDELITY",
        "description": "Loyal to truth over comfort. Never exceed φ-bounded confidence.",
        "constraint": lambda judgment: judgment.confidence <= MAX_CONFIDENCE,  # 0.618
        "violations": [],
    }

    PHI = {
        "name": "PHI",
        "description": "Harmonic balance: Q-Score in [0, 100], never magic numbers, never rescaled.",
        "constraint": lambda judgment: 0 <= judgment.q_score <= MAX_Q_SCORE,  # [0, 100]
        "violations": [],
    }

    VERIFY = {
        "name": "VERIFY",
        "description": "Don't trust, verify. Every judgment must explain itself.",
        "constraint": lambda judgment: hasattr(judgment, 'justification') and judgment.justification is not None,
        "violations": [],
    }

    CULTURE = {
        "name": "CULTURE",
        "description": "Patterns matter. Every judgment learns from history.",
        "constraint": lambda judgment: hasattr(judgment, 'precedent') and judgment.precedent is not None,
        "violations": [],
    }

    BURN = {
        "name": "BURN",
        "description": "Account for everything. No hidden costs, no free lunch.",
        "constraint": lambda judgment: hasattr(judgment, 'cost_usd') and judgment.cost_usd is not None,
        "violations": [],
    }


@dataclass
class OrganismIdentity:
    """Immutable organism identity layer.

    Present BEFORE first judgment.
    Cannot change.
    Is DNA (not phenotype).

    Public API:
    - __init__(...) — create identity
    - validate_judgment(judgment) -> list[str] — check axiom violations
    - __str__() — readable representation
    """

    axioms: dict[str, dict] = None
    name: str = "CYNIC"
    species: str = "Cynical Dog"
    birth_timestamp: float = None

    def __post_init__(self):
        """Load axioms as immutable constants on instantiation."""
        if self.axioms is None:
            # Freeze all axioms as immutable DNA
            self.axioms = {
                ax.name: {
                    "name": ax.value["name"],
                    "description": ax.value["description"],
                    "constraint": ax.value["constraint"],
                    "violations": [],  # Start clean
                }
                for ax in AxiomConstraint
            }

    def validate_judgment(self, judgment: Any) -> list[str]:
        """Check if judgment violates any axiom constraint.

        This is the organism's IMMUNE CHECK.
        Every judgment must pass before execution.

        Args:
            judgment: Object with confidence, q_score, justification, precedent, cost_usd

        Returns:
            List of axiom names violated (empty = valid, passes all constraints)
        """
        violations = []
        for axiom_name, axiom_def in self.axioms.items():
            try:
                constraint_fn = axiom_def["constraint"]
                if not constraint_fn(judgment):
                    violations.append(axiom_name)
                    axiom_def["violations"].append(axiom_name)
            except CynicError as e:
                # Constraint evaluation failed — also a violation
                # (object missing required attributes)
                violations.append(f"{axiom_name}:error")
                axiom_def["violations"].append(f"{axiom_name}:error:{str(e)}")

        return violations

    def can_execute_judgment(self, judgment: Any) -> bool:
        """True if judgment violates no axioms, False otherwise.

        Convenience method: not validate_judgment() but boolean result.
        """
        return len(self.validate_judgment(judgment)) == 0

    def __str__(self) -> str:
        """Human-readable DNA representation."""
        axiom_list = ", ".join(self.axioms.keys())
        return f"<{self.name} {self.species}> DNA: [{axiom_list}]"

    def to_dict(self) -> dict:
        """Serialize identity for persistence/logging."""
        return {
            "name": self.name,
            "species": self.species,
            "birth_timestamp": self.birth_timestamp,
            "axioms": {
                name: {
                    "description": ax["description"],
                    "violations_count": len(ax["violations"]),
                }
                for name, ax in self.axioms.items()
            },
        }

    def to_json(self) -> str:
        """Serialize to JSON for file storage."""
        return json.dumps(self.to_dict(), indent=2)
