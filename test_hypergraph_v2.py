#!/usr/bin/env python3.13
"""Test the new /mcp/hypergraph/recent endpoint - fresh start."""
import sys
import time
import subprocess
import urllib.request
import json
import os
import signal

os.chdir(r"C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC")

print("=" * 70)
print("HYPERGRAPH ENDPOINT TEST - FRESH START")
print("=" * 70)

# Force kill ALL Python processes
print("\nKilling all Python processes...")
for _ in range(3):
    try:
        subprocess.run(["taskkill", "/F", "/IM", "python.exe"], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
    except:
        pass
    time.sleep(1)

print("Starting CYNIC server with fresh code...")
proc = subprocess.Popen(
    [sys.executable, "-m", "cynic.api.entry", "--port", "8000"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

# Wait for server
print("Waiting for server...", end="", flush=True)
for i in range(30):
    try:
        req = urllib.request.Request("http://localhost:8000/health")
        with urllib.request.urlopen(req, timeout=1) as resp:
            print(" OK")
            time.sleep(1)  # extra wait for auto-register
            break
    except:
        print(".", end="", flush=True)
        time.sleep(1)
else:
    print("\nServer failed")
    proc.terminate()
    proc.wait()
    sys.exit(1)

# Test hypergraph endpoint
print("\nTesting /mcp/hypergraph/recent...")
success = False
try:
    url = "http://localhost:8000/mcp/hypergraph/recent?limit=10"
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=2) as resp:
        data = json.loads(resp.read().decode())
        print(f"HTTP {resp.status}")
        print(f"Edges returned: {data.get('count', 0)}")
        print(f"Limit: {data.get('limit')}")

        if "edges" in data:
            print(f"\nResponse structure valid: YES")
            print(f"Edge count: {len(data['edges'])}")
            if data["edges"]:
                edge = data["edges"][0]
                print(f"\nFirst edge keys: {list(edge.keys())}")
                dims = ["signal", "symbol", "meaning", "value", "decision", "action", "integration"]
                present = [d for d in dims if d in edge]
                print(f"7D dimensions present: {len(present)}/7 ({', '.join(present)})")
            success = True
        else:
            print("Invalid response: missing 'edges' key")
except urllib.error.HTTPError as e:
    print(f"HTTP {e.code}")
    try:
        error_data = json.loads(e.read().decode())
        print(f"Error: {error_data.get('detail', 'Unknown')}")
    except:
        pass
except Exception as e:
    print(f"Connection error: {e}")

print("\n" + "=" * 70)
if success:
    print("RESULT: Hypergraph endpoint OPERATIONAL")
    exit_code = 0
else:
    print("RESULT: Hypergraph endpoint FAILED")
    exit_code = 1
print("=" * 70)

proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()
    proc.wait()

sys.exit(exit_code)
