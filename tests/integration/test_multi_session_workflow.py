"""
Integration tests for multi-session coordination system.

These tests verify that all layers work together correctly:
1. Pre-commit gates
2. GitHub Actions workflows
3. Documentation completeness
4. Dashboard generation
5. Blocker detection
"""

import subprocess
from pathlib import Path
import pytest


def test_pre_commit_gates():
    """Test that pre-commit gates work."""
    # Check that scripts exist
    assert Path("scripts/validate_encoding.py").exists()
    assert Path("scripts/audit_factory_wiring.py").exists()
    assert Path("scripts/validate_commit_message.py").exists()

    # Run each script
    result = subprocess.run(
        ["python", "scripts/validate_encoding.py"], capture_output=True
    )
    assert (
        result.returncode == 0
    ), f"Encoding validation failed: {result.stderr.decode()}"

    result = subprocess.run(
        ["python", "scripts/analyze_imports.py", "--fail-on-cycle"], capture_output=True
    )
    assert result.returncode == 0, f"Import analysis failed: {result.stderr.decode()}"

    result = subprocess.run(
        ["python", "scripts/audit_factory_wiring.py"], capture_output=True
    )
    assert result.returncode == 0, f"Factory audit failed: {result.stderr.decode()}"


def test_github_actions_workflows():
    """Test that GitHub Actions workflow files are valid YAML."""
    import yaml

    workflow_files = [
        ".github/workflows/ci-gates.yml",
        ".github/workflows/update-status.yml",
        ".github/workflows/auto-continue.yml",
    ]

    for wf_path in workflow_files:
        with open(wf_path, "r", encoding="utf-8") as f:
            try:
                yaml.safe_load(f)
            except yaml.YAMLError as e:
                raise AssertionError(f"{wf_path} has invalid YAML: {e}")


def test_documentation_exists():
    """Test that all required documentation files exist."""
    docs = [
        "docs/MULTI_SESSION_GUIDE.md",
        "docs/status.md",
        "docs/BRANCH_PROTECTION_SETUP.md",
        "docs/plans/2026-03-02-multi-session-coordination-design.md",
    ]

    for doc in docs:
        assert Path(doc).exists(), f"Missing documentation: {doc}"
        # Check that it has content
        content = Path(doc).read_text(encoding="utf-8")
        assert len(content) > 100, f"Documentation file too short: {doc}"


def test_dashboard_generator():
    """Test that dashboard generator produces valid markdown."""
    result = subprocess.run(
        ["python", "scripts/generate_status_dashboard.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0

    dashboard = result.stdout
    # Check for expected sections
    assert "CYNIC Project Status" in dashboard
    assert "Main Branch Health" in dashboard
    assert "In-Flight Sessions" in dashboard


def test_blocker_detector():
    """Test that blocker detection script works."""
    result = subprocess.run(
        ["python", "scripts/detect_blockers.py"], capture_output=True, text=True
    )
    # Should always succeed (even if no blockers)
    assert result.returncode == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
