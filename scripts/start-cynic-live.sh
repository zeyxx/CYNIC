#!/bin/bash
# Start CYNIC kernel + MCP bridge for live Claude Code interaction

echo "*sniff* CYNIC Live Integration Startup"
echo "======================================="
echo ""
echo "This script starts two processes:"
echo "  1. CYNIC Python kernel (port 8765 + MCP on 8766)"
echo "  2. MCP Bridge (stdio to Claude Code)"
echo ""
echo "Then Claude Code can invoke tools: ask_cynic, observe_cynic, learn_cynic, etc."
echo ""
echo "USAGE:"
echo "  Terminal 1: bash scripts/start-cynic-live.sh kernel"
echo "  Terminal 2: bash scripts/start-cynic-live.sh bridge"
echo "  Terminal 3: claude  (then use MCP tools)"
echo ""

cd "$(dirname "$0")/.."

if [ "$1" == "kernel" ]; then
    echo "🟢 Starting CYNIC kernel..."
    cd cynic
    py -3.13 -m cynic.api.entry

elif [ "$1" == "bridge" ]; then
    echo "🟢 Starting MCP bridge (Claude Code interface)..."
    cd cynic
    py -3.13 -m cynic.mcp.claude_code_bridge

else
    echo "Usage: $0 [kernel|bridge]"
    echo ""
    echo "Example (3 terminals):"
    echo "  Terminal 1: bash scripts/start-cynic-live.sh kernel"
    echo "  Terminal 2: bash scripts/start-cynic-live.sh bridge"
    echo "  Terminal 3: claude"
fi
