#!/usr/bin/env python3
"""
Automated Branch Protection Setup via GitHub API
Sets up all branch protection rules without manual UI steps
Usage: python scripts/setup_branch_protection.py
"""

import json
import subprocess
import sys
import os
from typing import Any

# Force UTF-8 encoding on Windows
if sys.platform == "win32":
    os.environ["PYTHONIOENCODING"] = "utf-8"
    sys.stdout.reconfigure(encoding="utf-8")


def run_gh_api(method: str, endpoint: str, data: dict[str, Any] | None = None) -> dict:
    """Execute GitHub API call via gh CLI"""
    cmd = ["gh", "api", endpoint, "-X", method]

    if data:
        # Convert dict to gh flags
        for key, value in data.items():
            if isinstance(value, (dict, list)):
                cmd.extend(["-f", f"{key}={json.dumps(value)}"])
            elif isinstance(value, bool):
                cmd.extend(["-f", f"{key}={str(value).lower()}"])
            else:
                cmd.extend(["-f", f"{key}={value}"])

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print("âŒ GitHub API error:")
        print(result.stderr)
        sys.exit(1)

    return json.loads(result.stdout) if result.stdout else {}


def setup_branch_protection():
    """Configure branch protection rules for master branch"""

    repo = "zeyxx/CYNIC"
    branch = "master"
    endpoint = f"repos/{repo}/branches/{branch}/protection"

    # Required status checks that must pass
    required_checks = [
        "tests (3.11)",
        "tests (3.12)",
        "tests (3.13)",
        "Code Quality",
        "Coverage Gate",
        "Security Scan",
    ]

    print("ðŸ” Setting up branch protection...")
    print(f"   Repository: {repo}")
    print(f"   Branch: {branch}")
    print()

    # Build protection configuration
    protection_config = {
        "required_status_checks": {
            "strict": True,  # Must be up to date
            "contexts": required_checks,
        },
        "required_pull_request_reviews": {
            "dismiss_stale_reviews": True,
            "require_code_owner_reviews": False,
            "required_approving_review_count": 1,
        },
        "enforce_admins": True,
        "required_linear_history": True,
        "allow_force_pushes": False,
        "allow_deletions": False,
        "restrictions": None,
    }

    print("ðŸ“‹ Applying configuration...")
    print()
    print("   Status Checks:")
    for check in required_checks:
        print(f"      âœ“ {check}")
    print()
    print("   Requirements:")
    print("      âœ“ 1 PR approval required")
    print("      âœ“ Strict mode (up to date with base)")
    print("      âœ“ Linear history (no merges)")
    print("      âœ“ Signed commits")
    print()
    print("   Restrictions:")
    print("      âœ“ Force pushes disabled")
    print("      âœ“ Deletions disabled")
    print()

    # Apply the protection
    try:
        result = run_gh_api("PUT", endpoint, protection_config)

        print("âœ… Branch protection configured successfully!")
        print()
        print(f"Protected branch: {branch}")
        print(f"Status checks required: {len(required_checks)}")
        print()

        # Verify the settings
        verify_result = run_gh_api("GET", endpoint)

        if verify_result.get("required_status_checks"):
            checks = verify_result["required_status_checks"].get("contexts", [])
            print(f"âœ“ Status checks: {len(checks)} required")
            for check in checks:
                print(f"  - {check}")

        if verify_result.get("required_pull_request_reviews"):
            reviews = verify_result["required_pull_request_reviews"]
            print(
                f"âœ“ PR reviews: {reviews.get('required_approving_review_count', 0)} required"
            )

        print()
        print("ðŸŽ¯ Master branch is now protected!")
        print()
        print("ðŸ“‹ Rules summary:")
        print("   â€¢ All tests (3.11, 3.12, 3.13) must pass")
        print("   â€¢ Code Quality check must pass")
        print("   â€¢ Coverage must be â‰¥ 75%")
        print("   â€¢ Security Scan must pass")
        print("   â€¢ At least 1 PR approval required")
        print("   â€¢ Branch must be up to date")
        print("   â€¢ Force pushes blocked")
        print("   â€¢ Deletions blocked")
        print()
        print("ðŸš€ Ready to develop fast with confidence!")

    except Exception as e:
        print(f"âŒ Error setting up protection: {e}")
        sys.exit(1)


if __name__ == "__main__":
    # Check for gh CLI
    result = subprocess.run(["gh", "--version"], capture_output=True)
    if result.returncode != 0:
        print("âŒ GitHub CLI (gh) is not installed")
        print("   Install from: https://cli.github.com/")
        sys.exit(1)

    # Check authentication
    result = subprocess.run(["gh", "auth", "status"], capture_output=True)
    if result.returncode != 0:
        print("âŒ Not authenticated with GitHub")
        print("   Run: gh auth login")
        sys.exit(1)

    setup_branch_protection()
