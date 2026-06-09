"""Tests for state.py — CircuitState read/write."""
import json
import tempfile
from pathlib import Path

from state import CircuitState, load_state, save_state


def test_load_state_creates_default_when_missing() -> None:
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "state.json"
        state = load_state(path)
        assert state.best_product == 0
        assert state.tried_levers == []
        assert state.iterations == []
        assert state.baseline_product is None


def test_save_and_load_roundtrip() -> None:
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "state.json"
        original = CircuitState(
            best_product=5_000_000_000,
            best_gate_count=2_500_000,
            best_qubit_count=2_000,
            tried_levers=["windowed_scalar_mult"],
            iterations=[{"lever": "windowed_scalar_mult", "product": 5_000_000_000}],
            baseline_product=6_500_000_000,
        )
        save_state(original, path)
        loaded = load_state(path)
        assert loaded.best_product == 5_000_000_000
        assert loaded.tried_levers == ["windowed_scalar_mult"]
        assert loaded.baseline_product == 6_500_000_000


def test_load_state_handles_missing_optional_fields() -> None:
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "state.json"
        path.write_text(json.dumps({"best_product": 7_000_000_000}))
        state = load_state(path)
        assert state.best_product == 7_000_000_000
        assert state.baseline_product is None
        assert state.tried_levers == []
