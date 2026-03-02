"""
Stabilization tests — verify foundational integrity for 10k TPS.

Tests that the codebase is ready for high-throughput operation.
"""
import subprocess
import sys


def test_no_encoding_errors():
    """Encoding validation passes."""
    result = subprocess.run(
        [sys.executable, "scripts/validate_encoding.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Encoding validation failed:\n{result.stdout}\n{result.stderr}"


def test_no_circular_imports():
    """No circular import chains."""
    result = subprocess.run(
        [sys.executable, "scripts/analyze_imports.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Circular import detected:\n{result.stdout}\n{result.stderr}"


def test_factory_wiring_complete():
    """All components wired in factory."""
    result = subprocess.run(
        [sys.executable, "scripts/audit_factory_wiring.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Factory wiring incomplete:\n{result.stdout}\n{result.stderr}"


def test_api_routers_mounted():
    """All API routers are mounted."""
    result = subprocess.run(
        [sys.executable, "scripts/audit_api_routers.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Router mounting incomplete:\n{result.stdout}\n{result.stderr}"


def test_priority_tests_pass():
    """All Priority 5-7 tests pass (no regressions)."""
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "tests/test_priority5_event_protocol.py",
            "tests/test_priority6_state_reconstruction.py",
            "tests/test_priority7_event_metrics.py",
            "-v",
            "--tb=short",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Priority tests failed:\n{result.stdout}\n{result.stderr}"


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])
