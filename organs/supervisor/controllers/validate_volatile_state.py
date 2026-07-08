# Tier 1
import sys
import os
from pathlib import Path

# Add the project root to the Python path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.validators import find_staged_volatile_files
from organs.supervisor.src.adapters.git_adapter import GitAdapter

def main():
    print("▶ Checking volatile state policy...")
    
    if os.environ.get("ALLOW_VOLATILE_STATE_COMMIT") == "1":
        print("⚠ Volatile state commit override enabled")
        sys.exit(0)

    git_adapter = GitAdapter()
    volatile_files = find_staged_volatile_files(git_adapter)
    
    if volatile_files:
        print("FAIL volatile state: runtime organ snapshots are staged:")
        for f in volatile_files:
            print(f)
        print("Use kernel observations or local state, not git snapshots.")
        sys.exit(1)
    
    print("✓ Volatile state policy OK")
    sys.exit(0)

if __name__ == "__main__":
    main()
