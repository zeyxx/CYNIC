"""Tests for evaluator.py — cargo build+eval runner."""
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from evaluator import EvalResult, parse_eval_output, run_eval


REAL_OUTPUT = """\
=== circuit metrics (secp256k1, n=256) ===
  avg executed Toffoli  : 1688703.000
  avg executed Clifford : 7925219.958
  total Toffoli (sum)   : 15238855872 over 9024 shots
  total Clifford (sum)  : 71517184902
  emitted ops           : 10827225
  qubits                : 1558

=== experiment OK ===
"""


def test_parse_eval_output_success() -> None:
    result = parse_eval_output(REAL_OUTPUT)
    assert result.gate_count == 1_688_703
    assert result.qubit_count == 1_558
    assert result.product == 1_688_703 * 1_558
    assert result.correctness is True


def test_parse_eval_output_fail() -> None:
    output = REAL_OUTPUT.replace("experiment OK", "experiment FAILED")
    result = parse_eval_output(output)
    assert result.correctness is False


def test_parse_eval_output_missing_field_raises() -> None:
    output = "  avg executed Clifford : 7925219.958\n  qubits : 1558\n"
    with pytest.raises(ValueError, match="Toffoli"):
        parse_eval_output(output)


def test_run_eval_returns_result_on_success() -> None:
    fake_output = REAL_OUTPUT.replace("1688703", "4000000").replace("1558", "2700")
    with patch("evaluator.subprocess.run") as mock_run:
        mock_run.side_effect = [
            MagicMock(returncode=0, stdout="", stderr=""),
            MagicMock(returncode=0, stdout=fake_output, stderr=""),
        ]
        result = run_eval(Path("/fake/repo"))
    assert result.gate_count == 4_000_000
    assert result.qubit_count == 2_700


def test_run_eval_raises_on_cargo_failure() -> None:
    with patch("evaluator.subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=1, stdout="", stderr="error[E0308]: mismatched types"
        )
        with pytest.raises(RuntimeError, match="cargo"):
            run_eval(Path("/fake/repo"))
