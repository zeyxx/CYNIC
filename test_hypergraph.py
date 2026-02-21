#!/usr/bin/env python3.13
"""Test the new /mcp/hypergraph/recent endpoint."""
import sys
import time
import subprocess
import urllib.request
import json
import os

os.chdir(r"C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC")

# Kill existing
try:
    subprocess.run(["taskkill", "/F", "/IM", "python.exe"], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
    time.sleep(2)
except:
    pass

print("=" * 70)
print("HYPERGRAPH ENDPOINT TEST")
print("=" * 70)

print("\nStarting CYNIC server...")
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
            break
    except:
        print(".", end="", flush=True)
        time.sleep(1)
else:
    print("\nServer failed")
    proc.terminate()
    sys.exit(1)

# Test hypergraph endpoint
print("\nTesting /mcp/hypergraph/recent...")
try:
    url = "http://localhost:8000/mcp/hypergraph/recent?limit=10"
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=2) as resp:
        data = json.loads(resp.read().decode())
        print(f"Response: HTTP {resp.status}")
        print(f"Total edges: {data.get('count', 0)}")

        if "edges" in data and data["edges"]:
            edge = data["edges"][0]
            print("\nFirst edge structure:")
            print(f"  edge_id: {edge.get('edge_id')}")
            print(f"  symbol: {edge.get('symbol')}")
            print(f"  meaning (verdict): {edge.get('meaning', {}).get('verdict')}")
            print(f"  value (Q-score): {edge.get('value')}")
            print("\nHypergraph endpoint OPERATIONAL")
            exit_code = 0
        else:
            print("No edges yet (normal for fresh start)")
            exit_code = 0
except Exception as e:
    print(f"Error: {e}")
    exit_code = 1

proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()

sys.exit(exit_code)
