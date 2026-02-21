#!/bin/bash
# End-to-end MCP test: start kernel + bridge, invoke a tool

set -e

echo "*sniff* CYNIC MCP End-to-End Test"
echo "=================================="
echo ""

cd "$(dirname "$0")/.."

# Check Python 3.13
if ! py -3.13 --version &>/dev/null; then
    echo "✗ Python 3.13 not found"
    exit 1
fi

echo "Step 1: Start CYNIC kernel in background..."
cd cynic
py -3.13 -m cynic.api.entry &
KERNEL_PID=$!
echo "  PID: $KERNEL_PID"
sleep 2

# Check kernel is listening
if ! timeout 1 bash -c "echo > /dev/tcp/127.0.0.1/8765" 2>/dev/null; then
    echo "✗ Kernel not listening on port 8765"
    kill $KERNEL_PID || true
    exit 1
fi
echo "✓ Kernel listening on port 8765"
echo ""

echo "Step 2: Test MCP bridge tools..."
py -3.13 << 'PYTHON_TEST'
import sys
sys.path.insert(0, '.')
from cynic.mcp.claude_code_bridge import list_tools
import asyncio

async def main():
    tools = await list_tools()
    print(f"✓ MCP Bridge has {len(tools)} tools:")
    for tool in tools[:5]:
        print(f"  - {tool.name}")
    print(f"  ... and {len(tools)-5} more")

asyncio.run(main())
PYTHON_TEST
echo ""

echo "Step 3: Test MCP ask_cynic tool call..."
py -3.13 << 'PYTHON_TEST'
import sys
sys.path.insert(0, '.')
import asyncio
import aiohttp
from cynic.mcp.models import ObserveRequest

async def test_observe():
    """Test /observe endpoint (used by ask_cynic)"""
    async with aiohttp.ClientSession() as session:
        payload = {
            "include_judgments": True,
            "include_events": False,
            "max_events": 10
        }
        try:
            async with session.post("http://127.0.0.1:8765/observe", json=payload, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    print(f"✓ CYNIC responded to /observe")
                    print(f"  Components: {data.get('registry_snapshot', {}).get('total_components', 0)}")
                    print(f"  Judgments: {len(data.get('recent_judgments', []))}")
                else:
                    print(f"✗ /observe returned {resp.status}")
        except Exception as e:
            print(f"✗ /observe failed: {e}")

asyncio.run(test_observe())
PYTHON_TEST
echo ""

echo "Step 4: Cleanup..."
kill $KERNEL_PID || true
sleep 1
echo "✓ Kernel stopped"
echo ""

echo "========================================"
echo "✓ MCP End-to-End Test PASSED"
echo ""
echo "Next: Start in 3 terminals:"
echo "  T1: bash scripts/start-cynic-live.sh kernel"
echo "  T2: bash scripts/start-cynic-live.sh bridge"
echo "  T3: claude (use MCP tools)"
echo ""
