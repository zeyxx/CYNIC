"""Tests for evaluator.py — cargo build+eval runner."""
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from evaluator import EvalResult, parse_eval_output, run_eval


def test_parse_eval_output_success() -> None:
    output = "gates: 2800000\nqubits: 2300\nproduct: 6440000000\ncorrectness: PASS\n"
    result = parse_eval_output(output)
    assert result.gate_count == 2_800_000
    assert result.qubit_count == 2_300
    assert result.product == 6_440_000_000
    assert result.correctness is True


def test_parse_eval_output_fail() -> None:
    output = "gates: 2800000\nqubits: 2300\nproduct: 6440000000\ncorrectness: FAIL\n"
    result = parse_eval_output(output)
    assert result.correctness is False


def test_parse_eval_output_missing_field_raises() -> None:
    output = "gates: 2800000\nqubits: 2300\n"
    with pytest.raises(ValueError, match="product"):
        parse_eval_output(output)


def test_run_eval_returns_result_on_success() -> None:
    fake_output = "gates: 4000000\nqubits: 2700\nproduct: 10800000000\ncorrectness: PASS\n"
    with patch("evaluator.subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout=fake_output, stderr="")
        result = run_eval(Path("/fake/repo"))
    assert result.gate_count == 4_000_000
    assert result.product == 10_800_000_000


def test_run_eval_raises_on_cargo_failure() -> None:
    with patch("evaluator.subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=1, stdout="", stderr="error[E0308]: mismatched types"
        )
        with pytest.raises(RuntimeError, match="cargo"):
            run_eval(Path("/fake/repo"))
