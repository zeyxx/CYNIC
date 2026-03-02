#!/usr/bin/env python3
"""
Branch Protection Setup via GitHub API using curl
Avoids gh CLI JSON formatting issues
"""

import subprocess
import json
import sys

# Get GitHub token
token = subprocess.run(
    ["gh", "auth", "token"],
    capture_output=True,
    text=True
).stdout.strip()

if not token:
    print("ERROR: Could not get GitHub token")
    print("Run: gh auth login")
    sys.exit(1)

# Configuration
repo = "zeyxx/CYNIC"
branch = "master"
api_url = f"https://api.github.com/repos/{repo}/branches/{branch}/protection"

# Protection configuration
protection_data = {
    "required_status_checks": {
        "strict": True,
        "contexts": [
            "tests (3.11)",
            "tests (3.12)",
            "tests (3.13)",
            "Code Quality",
            "Coverage Gate",
            "Security Scan"
        ]
    },
    "required_pull_request_reviews": {
        "dismiss_stale_reviews": True,
        "require_code_owner_reviews": False,
        "required_approving_review_count": 1
    },
    "enforce_admins": True,
    "required_linear_history": True,
    "allow_force_pushes": False,
    "allow_deletions": False
}

print("[BRANCH PROTECTION SETUP]")
print()
print(f"Repository: {repo}")
print(f"Branch: {branch}")
print()
print("Configuring...")
print()

# Make API request
headers = {
    "Authorization": f"token {token}",
    "Accept": "application/vnd.github.v3+json",
    "Content-Type": "application/json"
}

cmd = [
    "curl",
    "-X", "PUT",
    "-H", f"Authorization: token {token}",
    "-H", "Accept: application/vnd.github.v3+json",
    "-d", json.dumps(protection_data),
    api_url
]

result = subprocess.run(cmd, capture_output=True, text=True)

if result.returncode != 0:
    print("ERROR: API request failed")
    print(result.stderr)
    sys.exit(1)

try:
    response = json.loads(result.stdout)

    if "message" in response and "error" in response.get("message", "").lower():
        print("ERROR: API returned error")
        print(response.get("message", "Unknown error"))
        sys.exit(1)

    print("SUCCESS: Branch protection configured!")
    print()
    print("Active Rules:")
    print("  [Status Checks - 6 required]")
    for check in protection_data["required_status_checks"]["contexts"]:
        print(f"    - {check}")
    print()
    print("  [PR Reviews]")
    print(f"    - Approvals required: {protection_data['required_pull_request_reviews']['required_approving_review_count']}")
    print(f"    - Dismiss stale reviews: {protection_data['required_pull_request_reviews']['dismiss_stale_reviews']}")
    print()
    print("  [Enforcement]")
    print(f"    - Strict mode (up to date): {protection_data['required_status_checks']['strict']}")
    print(f"    - Linear history: {protection_data['required_linear_history']}")
    print(f"    - Force pushes: {'BLOCKED' if not protection_data['allow_force_pushes'] else 'ALLOWED'}")
    print(f"    - Deletions: {'BLOCKED' if not protection_data['allow_deletions'] else 'ALLOWED'}")
    print()
    print("DONE: Master branch is now protected!")
    print()
    print("Next steps:")
    print("1. Create a test PR to verify protection is working")
    print("2. Wait for all status checks to run")
    print("3. Verify PR cannot merge until all checks pass")

except json.JSONDecodeError:
    print("ERROR: Invalid JSON response")
    print(result.stdout)
    sys.exit(1)
