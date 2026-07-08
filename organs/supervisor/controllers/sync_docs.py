# Tier 1
import sys
from pathlib import Path

# Add the project root to the Python path
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.validators import sync_mcp_tools_docs
from organs.supervisor.src.adapters.filesystem_adapter import FilesystemDocsAdapter

def main():
    print("▶ Synchronizing agent documentation...")
    project_root = Path(__file__).parent.parent.parent.parent
    
    docs_adapter = FilesystemDocsAdapter(project_root)
    tool_count, update_count = sync_mcp_tools_docs(docs_adapter)
    
    if update_count > 0:
        print(f"✓ Successfully updated AGENTS.md with {tool_count} tools.")
        sys.exit(0)
    else:
        print("⚠ No update performed. Markers may be missing from AGENTS.md.")
        sys.exit(1)

if __name__ == "__main__":
    main()
