#!/usr/bin/env python3.13
"""Test ALL MCP endpoints including new hypergraph."""
import sys
import time
import subprocess
import urllib.request
import json
import os

os.chdir(r"C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC")

# Kill all
for _ in range(3):
    try:
        subprocess.run(["taskkill", "/F", "/IM", "python.exe"], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
    except:
        pass
    time.sleep(0.5)

print("Starting server...")
proc = subprocess.Popen(
    [sys.executable, "-m", "cynic.api.entry", "--port", "8000"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

# Wait
for i in range(30):
    try:
        req = urllib.request.Request("http://localhost:8000/health")
        with urllib.request.urlopen(req, timeout=1) as resp:
            print("Server ready")
            break
    except:
        time.sleep(1)
else:
    print("Server startup failed")
    proc.terminate()
    sys.exit(1)

# Test endpoints
endpoints = [
    "/mcp/loops/status",
    "/mcp/learning/patterns",
    "/mcp/events/recent",
    "/mcp/hypergraph/recent",
]

print("\nTesting MCP endpoints:")
print("-" * 50)
for ep in endpoints:
    try:
        req = urllib.request.Request(f"http://localhost:8000{ep}")
        with urllib.request.urlopen(req, timeout=2) as resp:
            data = json.loads(resp.read().decode())
            status = "OK"
    except urllib.error.HTTPError as e:
        status = f"HTTP {e.code}"
    except Exception as e:
        status = f"ERROR: {str(e)[:20]}"

    print(f"{status:20} {ep}")

proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()
