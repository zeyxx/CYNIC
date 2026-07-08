"""Compatibility wrapper for `#[allow]` justification validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))
from organs.supervisor.src.core.validators import find_allow_without_why

def main():
    print("▶ Checking for #[allow] without WHY: (K12)...")
    project_root = Path(__file__).parent.parent.parent.parent
    violations = find_allow_without_why(project_root)
    if violations:
        print("FAIL K12: #[allow] without WHY: comment found:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    print("✓ Allow without WHY check (K12) passed.")
    sys.exit(0)
if __name__ == "__main__":
    main()
