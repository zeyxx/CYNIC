# Tier 1
import sys
from pathlib import Path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))
from organs.supervisor.src.core.validators import find_infra_type_leakage

def main():
    print("▶ Checking for infra type leakage (K5)...")
    project_root = Path(__file__).parent.parent.parent.parent
    violations = find_infra_type_leakage(project_root)
    if violations:
        print("FAIL K5: Infrastructure type leakage found in domain/:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    print("✓ Infra leakage check (K5) passed.")
    sys.exit(0)
if __name__ == "__main__":
    main()
