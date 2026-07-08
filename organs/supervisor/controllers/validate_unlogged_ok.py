"""Compatibility wrapper for unlogged `.ok()` validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
wrapper until remaining callers are ported.
"""

# Tier 1
import sys
from pathlib import Path

# Add the project root to the Python path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.validators import find_unlogged_ok_calls
from organs.supervisor.src.adapters.git_adapter import GitAdapter

def main():
    print("▶ Checking for unlogged .ok() calls in Rust files...")
    
    git_adapter = GitAdapter()
    violations = find_unlogged_ok_calls(git_adapter)
    
    if violations:
        print("FAIL R2: .ok() without adjacent logging found:")
        for file_path, lines in violations.items():
            for line in lines:
                print(f"{file_path}:{line}")
        sys.exit(1)
    
    print("✓ No unlogged .ok() calls found.")
    sys.exit(0)

if __name__ == "__main__":
    main()
