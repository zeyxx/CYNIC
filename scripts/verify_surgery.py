#!/usr/bin/env python3
import subprocess
import sys


def run_cmd(cmd: list[str]) -> bool:
    print(f"--- Running: {' '.join(cmd)} ---")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"❌ FAILED: {result.stderr or result.stdout}")
            return False
        print("✅ PASSED")
        return True
    except FileNotFoundError:
        print(f"❌ Tool not found: {cmd[0]}")
        return False


def audit_files(files: list[str]) -> bool:
    if not files:
        print("No files provided for audit.")
        return True

    all_passed = True

    # 1. RUFF CHECK (Linting + Basic Indentation)
    if not run_cmd(["ruff", "check", "--fix"] + files):
        all_passed = False

    # 2. RUFF FORMAT (Syntax + Indentation Style)
    if not run_cmd(["ruff", "format"] + files):
        all_passed = False

    # 3. MYPY (Typing & Signatures)
    # We use --follow-imports=silent to focus on the changed files only
    if not run_cmd(
        ["mypy", "--follow-imports=silent", "--ignore-missing-imports"] + files
    ):
        all_passed = False

    return all_passed


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python scripts/verify_surgery.py <file1> <file2> ...")
        sys.exit(1)

    files = sys.argv[1:]
    if audit_files(files):
        print("\n🛡️  HERESYGUARD: Surgery verified. No anomalies detected.")
        sys.exit(0)
    else:
        print(
            "\n☢️  ANOMALY DETECTED: Heresies found in your surgery. Fix them before claiming completion."
        )
        sys.exit(1)
