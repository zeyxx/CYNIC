"""Compatibility wrapper for hardcoded-IP validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))
from organs.supervisor.src.core.validators import find_hardcoded_ips
from organs.supervisor.src.adapters.git_adapter import GitAdapter

def main():
    print("▶ Checking for hardcoded IPs...")
    project_root = Path(__file__).parent.parent.parent.parent
    adapter = GitAdapter()
    violations = find_hardcoded_ips(adapter, project_root)
    if violations["STAGED_IPS"] or violations["BACKEND_IPS"]:
        print("FAIL Security: Possible hardcoded IPs found:")
        if violations["STAGED_IPS"]:
            print("Staged IPs:")
            for ip in violations["STAGED_IPS"]:
                print(ip)
        if violations["BACKEND_IPS"]:
            print("IPs in backends.toml:")
            for ip in violations["BACKEND_IPS"]:
                print(ip)
        sys.exit(1)
    print("✓ Hardcoded IPs check passed.")
    sys.exit(0)
if __name__ == "__main__":
    main()
