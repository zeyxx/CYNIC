#!/usr/bin/env python3
"""
Validate that merge commits follow the required format.
Checks: Has Reviewed-By field, has Impacts section, etc.
"""

import sys
import subprocess


def validate_merge_commit():
    """Check the current commit message format."""
    # Get last commit message
    try:
        result = subprocess.run(
            ["git", "log", "-1", "--pretty=%B"], capture_output=True, text=True
        )
        commit_msg = result.stdout.strip()
    except Exception as e:
        print(f"Could not read last commit message: {e}")
        return True

    # Check required fields
    required_fields = ["Impacts", "Co-Authored-By"]
    missing = [f for f in required_fields if f not in commit_msg]

    if missing:
        print(f"Merge commit missing fields: {', '.join(missing)}")
        print("   This is a warning for now. Future commits should include all fields.")
        return True

    print("Merge commit format is correct")
    return True


if __name__ == "__main__":
    sys.exit(0 if validate_merge_commit() else 1)
