#!/usr/bin/env python3
"""Isolated test runner for cynic-protocol task_pipeline — avoids rustc ICE."""
import subprocess, sys

cmd = [
    "cargo", "test", "-p", "cynic-protocol",
    "--lib", "task_pipeline::tests",
    "--", "--nocapture"
]

env = {
    **__import__("os").environ,
    "RUST_MIN_STACK": "16777216",
    "RUSTUP_TOOLCHAIN": "1.96.0",
}

try:
    r = subprocess.run(
        cmd, env=env, timeout=540, capture_output=True, text=True
    )
    print("STDOUT:", r.stdout[-3000:])
    print("STDERR:", r.stderr[-1000:] if r.stderr else "none")
    print("EXIT:", r.returncode)
    sys.exit(r.returncode)
except subprocess.TimeoutExpired as e:
    print("TIMEOUT:", e)
    print("STDOUT:", e.stdout[-2000:] if e.stdout else "")
    print("STDERR:", e.stderr[-1000:] if e.stderr else "")
    sys.exit(124)
