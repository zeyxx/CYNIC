"""Compatibility wrapper for pipeline-function reimplementation validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))
from organs.supervisor.src.core.validators import find_reimplemented_pipeline_functions

def main():
    print("▶ Checking for reimplemented pipeline functions (K3/K11)...")
    project_root = Path(__file__).parent.parent.parent.parent
    violations = find_reimplemented_pipeline_functions(project_root)
    if violations:
        print("FAIL K3/K11: Pipeline functions reimplemented in api/:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    print("✓ Pipeline function check (K3/K11) passed.")
    sys.exit(0)
if __name__ == "__main__":
    main()
