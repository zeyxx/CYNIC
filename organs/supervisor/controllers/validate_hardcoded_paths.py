"""Compatibility wrapper for hardcoded-path validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path

# Add the project root to the Python path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.validators import find_hardcoded_absolute_paths
from organs.supervisor.src.adapters.git_adapter import GitAdapter

def main():
    print("▶ Checking for hardcoded absolute paths...")
    
    git_adapter = GitAdapter()
    violations = find_hardcoded_absolute_paths(git_adapter)
    
    if violations:
        print("FAIL R1: hardcoded absolute paths found:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    
    print("✓ No hardcoded absolute paths found.")
    sys.exit(0)

if __name__ == "__main__":
    main()
