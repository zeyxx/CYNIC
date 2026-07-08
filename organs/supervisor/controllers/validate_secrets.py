"""Compatibility wrapper for staged-secret validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))
from organs.supervisor.src.core.validators import find_hardcoded_secrets
from organs.supervisor.src.adapters.git_adapter import GitAdapter

def main():
    print("▶ Checking for hardcoded secrets...")
    adapter = GitAdapter()
    violations = find_hardcoded_secrets(adapter)
    if violations["RAW_SECRETS"] or violations["HARDCODED_CREDS"]:
        print("FAIL Security: Possible hardcoded secrets found in staged changes:")
        if violations["RAW_SECRETS"]:
            print("Raw secrets:")
            for secret in violations["RAW_SECRETS"]:
                print(secret)
        if violations["HARDCODED_CREDS"]:
            print("Hardcoded credentials:")
            for cred in violations["HARDCODED_CREDS"]:
                print(cred)
        sys.exit(1)
    print("✓ Hardcoded secrets check passed.")
    sys.exit(0)
if __name__ == "__main__":
    main()
