#!/usr/bin/env python3
"""
CYNIC Complete Local Validation Suite
Replace GitHub Actions for projects without CI/CD budget

Run this instead of waiting for GitHub Actions to validate code.
Validates everything that GitHub Actions would validate.
"""

import subprocess
import sys
from pathlib import Path

def run_command(name: str, cmd: list[str], timeout: int = 60) -> tuple[bool, str]:
    """Run a validation command and return (success, output)."""
    print(f"\n[*] {name}...", end=" ", flush=True)
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(Path(__file__).parent.parent)
        )
        output = result.stdout + result.stderr
        if result.returncode == 0:
            print("[PASS]")
            return True, output
        else:
            print("[FAIL]")
            return False, output
    except subprocess.TimeoutExpired:
        print(f"[TIMEOUT after {timeout}s]")
        return False, f"Command timed out after {timeout}s"
    except Exception as e:
        print("[ERROR]")
        return False, str(e)

def main():
    print("=" * 80)
    print("CYNIC LOCAL VALIDATION SUITE (GitHub Actions Replacement)")
    print("=" * 80)
    print("No GitHub budget? No problem. Validate locally.\n")

    gates = [
        ("UTF-8 Encoding", ["python", "scripts/validate_encoding.py"], 30),
        ("Circular Imports", ["python", "scripts/analyze_imports.py"], 30),
        ("Factory Wiring", ["python", "scripts/audit_factory_wiring.py"], 30),
        ("API Routers", ["python", "scripts/audit_api_routers.py"], 30),
        ("Commit Message Format", ["python", "scripts/validate_commit_message.py"], 30),
    ]

    test_commands = [
        ("Adapter Tests", ["python", "-m", "pytest", "tests/adapters/", "-q", "--tb=no"], 60),
        ("Change Analyzer Tests", ["python", "-m", "pytest", "tests/test_change_analyzer.py", "-q", "--tb=no"], 60),
        ("Core Tests (sample)", ["python", "-m", "pytest", "tests/", "-q", "--tb=no", "-k", "test_event or test_discord"], 120),
    ]

    results = {"gates": [], "tests": []}

    # Run infrastructure gates
    print("\n" + "=" * 80)
    print("INFRASTRUCTURE GATES")
    print("=" * 80)

    for name, cmd, timeout in gates:
        success, output = run_command(name, cmd, timeout)
        results["gates"].append((name, success, output))

    # Run tests
    print("\n" + "=" * 80)
    print("TEST SUITE")
    print("=" * 80)

    for name, cmd, timeout in test_commands:
        success, output = run_command(name, cmd, timeout)
        results["tests"].append((name, success, output))

    # Summary
    print("\n" + "=" * 80)
    print("VALIDATION SUMMARY")
    print("=" * 80)

    gates_passed = sum(1 for _, s, _ in results["gates"] if s)
    gates_total = len(results["gates"])
    tests_passed = sum(1 for _, s, _ in results["tests"] if s)
    tests_total = len(results["tests"])

    print(f"\nInfrastructure Gates: {gates_passed}/{gates_total} passed")
    print(f"Test Suites: {tests_passed}/{tests_total} passed")

    # Show failures
    all_failures = [
        (name, output) for name, success, output in results["gates"] + results["tests"]
        if not success
    ]

    if all_failures:
        print("\nFAILURES:")
        for name, output in all_failures:
            print(f"\n{name}:")
            print(output[:500])
        sys.exit(1)
    else:
        print("\n*** ALL VALIDATIONS PASSED ***")
        print("Code is ready to be pushed to GitHub.")
        print("Once GitHub billing is resolved, CI/CD will auto-validate on push.")
        sys.exit(0)

if __name__ == "__main__":
    main()
