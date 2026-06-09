"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — circuit state persistence.

Tracks best gate×qubit product, tried levers, iteration history.
Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted to Tier 2.
"""
import json
import logging
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Optional


@dataclass
class CircuitState:
    best_product: int = 0
    best_gate_count: int = 0
    best_qubit_count: int = 0
    tried_levers: list[str] = field(default_factory=list)
    iterations: list[dict[str, Any]] = field(default_factory=list)
    baseline_product: Optional[int] = None


def load_state(path: Path) -> CircuitState:
    """Load CircuitState from JSON file, returning defaults if missing or partial."""
    if not path.exists():
        return CircuitState()
    try:
        data = json.loads(path.read_text())
        return CircuitState(
            best_product=data.get("best_product", 0),
            best_gate_count=data.get("best_gate_count", 0),
            best_qubit_count=data.get("best_qubit_count", 0),
            tried_levers=data.get("tried_levers", []),
            iterations=data.get("iterations", []),
            baseline_product=data.get("baseline_product"),
        )
    except (json.JSONDecodeError, KeyError) as e:
        logging.warning("state_load_failed", extra={"path": str(path), "error": str(e)})
        return CircuitState()


def save_state(state: CircuitState, path: Path) -> None:
    """Persist CircuitState to JSON file atomically."""
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(asdict(state), indent=2))
    tmp.replace(path)
