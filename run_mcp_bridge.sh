#!/bin/bash
# CYNIC MCP Bridge Starter — Launches Claude Code bridge for autonomous CYNIC access
# Usage: ./run_mcp_bridge.sh
# This script is called by Claude Code's MCP configuration (~/.claude/mcp.json)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${SCRIPT_DIR}"

# Ensure we're in the right directory
cd "${REPO_ROOT}"

# Start the claude_code_bridge MCP server
# This runs on stdio (stdin/stdout) and handles requests from Claude Code
python -m cynic.mcp.claude_code_bridge
