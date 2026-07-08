"""Compatibility wrapper for duplicate-trait validation.

Stable deterministic authority is migrating into Rust. Keep this wrapper until
remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))
from organs.supervisor.src.core.validators import find_duplicate_trait_names

def main():
    print("▶ Checking for duplicate trait names (K4)...")
    project_root = Path(__file__).parent.parent.parent.parent
    duplicates = find_duplicate_trait_names(project_root)
    if duplicates:
        print("FAIL K4: Duplicate trait names found in domain/:")
        for trait in duplicates:
            print(trait)
        sys.exit(1)
    print("✓ Duplicate trait check (K4) passed.")
    sys.exit(0)
if __name__ == "__main__":
    main()
