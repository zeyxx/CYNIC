"""Compatibility wrapper for reqwest-usage validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path

# Add the project root to the Python path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.validators import find_misplaced_reqwest_usage

def main():
    print("▶ Checking for misplaced reqwest usage (K2)...")
    project_root = Path(__file__).parent.parent.parent.parent
    
    violations = find_misplaced_reqwest_usage(project_root)
    
    if violations:
        print("FAIL K2: reqwest usage found outside of allowed locations:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    
    print("✓ Reqwest usage check (K2) passed.")
    sys.exit(0)

if __name__ == "__main__":
    main()
