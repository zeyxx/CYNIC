#!/usr/bin/env python3.13
"""Test MCP routes after server startup."""
import sys
import time
import subprocess
import urllib.request
import json
import os

os.chdir(r"C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC")

# Kill existing (Windows way)
try:
    subprocess.run(["taskkill", "/F", "/IM", "python.exe"], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
    time.sleep(2)
except:
    pass

print("Starting CYNIC server...")
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
    print("\nServer failed to start")
    proc.terminate()
    proc.wait()
    sys.exit(1)

# Test /mcp/loops/status
print("\nTesting /mcp/loops/status directly...")
try:
    req = urllib.request.Request("http://localhost:8000/mcp/loops/status")
    with urllib.request.urlopen(req, timeout=2) as resp:
        data = json.loads(resp.read().decode())
        print(f"OK: {resp.status}")
        print(json.dumps(data, indent=2)[:300])
except urllib.error.HTTPError as e:
    print(f"HTTP {e.code}")
    try:
        error_body = e.read().decode()
        print(f"Error: {error_body[:300]}")
    except:
        pass
except Exception as e:
    print(f"Connection error: {e}")
finally:
    proc.terminate()
    try:
        proc.wait(timeout=3)
    except:
        proc.kill()
    print("\nServer stopped")
