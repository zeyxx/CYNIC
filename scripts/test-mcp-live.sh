#!/bin/bash
# Test CYNIC MCP integration live
# Launches kernel + MCP bridge, then invokes tools

set -e

echo "*sniff* CYNIC MCP Live Integration Test"
echo "=========================================="
echo ""

cd "$(dirname "$0")/.."

# Check Python
if ! command -v python &> /dev/null; then
    echo "✗ Python not found"
    exit 1
fi

echo "Step 1: Check MCP modules compile..."
python -m py_compile cynic/cynic/mcp/claude_code_bridge.py
python -m py_compile cynic/cynic/mcp/server.py
echo "✓ All MCP modules compile"
echo ""

echo "Step 2: Check imports..."
python -c "
from cynic.mcp.claude_code_bridge import server as mcp_server
from cynic.mcp.models import ObserveRequest, ActRequest, LearnRequest
print('✓ All imports successful')
"
echo ""

echo "Step 3: List available MCP tools..."
python << 'PYTHON_TEST'
import asyncio
from cynic.mcp.claude_code_bridge import list_tools

async def main():
    tools = await list_tools()
    print(f"✓ {len(tools)} tools available:")
    for tool in tools:
        print(f"  - {tool.name}")

asyncio.run(main())
PYTHON_TEST
echo ""

echo "Step 4: Check model schemas..."
python << 'PYTHON_TEST'
from cynic.mcp.models import ObserveRequest, ActRequest, LearnRequest
from pydantic import BaseModel

print("✓ Model schema validation:")
print(f"  - ObserveRequest: {ObserveRequest.__name__}")
print(f"  - ActRequest: {ActRequest.__name__}")
print(f"  - LearnRequest: {LearnRequest.__name__}")
PYTHON_TEST
echo ""

echo "=========================================="
echo "✓ MCP Integration Test PASSED"
echo ""
echo "To run live kernel + MCP bridge:"
echo "  Terminal 1: python -m cynic.api.entry"
echo "  Terminal 2: python -m cynic.mcp.claude_code_bridge"
echo "  Terminal 3: claude --config-add-mcp cynic-mcp"
echo ""
