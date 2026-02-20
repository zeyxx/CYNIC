#!/bin/bash
# MCP Bridge launcher — activates venv and starts the bridge
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
source .venv313/Scripts/activate
exec python -m cynic.mcp.claude_code_bridge "$@"
