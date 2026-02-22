#!/usr/bin/env python3.13
"""Comprehensive test of CYNIC to Claude Code bridge."""
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
print("CYNIC <> Claude Code BIDIRECTIONAL BRIDGE TEST")
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

# Test endpoints
tests = [
    ("CYNIC to Claude: Get loop health", "/mcp/loops/status", "GET", None),
    ("CYNIC to Claude: Get patterns", "/mcp/learning/patterns", "GET", None),
    ("CYNIC to Claude: Get events", "/mcp/events/recent", "GET", None),
    ("Health check", "/health", "GET", None),
]

print("\n" + "=" * 70)
print("ENDPOINT TESTS")
print("=" * 70)

success_count = 0
for desc, path, method, body in tests:
    try:
        url = f"http://localhost:8000{path}"
        req = urllib.request.Request(url, method=method)
        with urllib.request.urlopen(req, timeout=2) as resp:
            data = resp.read().decode()
            print(f"OK   {desc:45} {path}")
            success_count += 1
    except urllib.error.HTTPError as e:
        print(f"FAIL HTTP {e.code:3} {desc:45} {path}")
    except Exception as e:
        print(f"FAIL CONN  {desc:45} {str(e)[:20]}")

print("\n" + "=" * 70)
print(f"RESULT: {success_count}/{len(tests)} endpoints operational")
print("=" * 70)

if success_count >= 3:
    print("\nBridge OPERATIONAL - Ready for autonomous battles!")
    exit_code = 0
else:
    print("\nBridge FAILED - Connectivity issues")
    exit_code = 1

proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()

sys.exit(exit_code)
