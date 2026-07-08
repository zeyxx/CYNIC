# Tier 1
import sys
from pathlib import Path

# Add the project root to the Python path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.validators import find_disallowed_cfg_in_domain

def main():
    print("▶ Checking for disallowed #[cfg] in domain (K1)...")
    project_root = Path(__file__).parent.parent.parent.parent
    
    violations = find_disallowed_cfg_in_domain(project_root)
    
    if violations:
        print("FAIL K1: Disallowed #[cfg] attributes found in domain/:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    
    print("✓ Domain purity (K1) check passed.")
    sys.exit(0)

if __name__ == "__main__":
    main()
