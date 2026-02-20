#!/usr/bin/env python3.13
"""
Empirical Battle Run — Real data collection.

Start CYNIC server, run 1h of battles, collect and analyze results.
"""
import asyncio
import json
import logging
import os
import subprocess
import sys
import time
import urllib.request

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s | %(message)s",
)
log = logging.getLogger("empirical-run")

CYNIC_DIR = os.path.expanduser("~/.cynic")
BATTLES_FILE = os.path.join(CYNIC_DIR, "battles.jsonl")
API = "http://localhost:8000"


def _wait_for_server(max_wait=30):
    """Wait for CYNIC server to be ready."""
    log.info("Waiting for CYNIC server...")
    for i in range(max_wait):
        try:
            req = urllib.request.Request(f"{API}/health")
            with urllib.request.urlopen(req, timeout=2) as resp:
                data = json.loads(resp.read().decode())
                log.info(f"Server ready: {data['status']}")
                return True
        except:
            log.info(f"  ... {i+1}s", end="\r", flush=True)
            time.sleep(1)
    return False


def _start_server():
    """Start CYNIC server in background."""
    log.info("Starting CYNIC server on port 8000...")
    # Kill existing
    subprocess.run(
        ["taskkill", "/F", "/IM", "python.exe"],
        stderr=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
    )
    time.sleep(2)

    # Start new
    proc = subprocess.Popen(
        [sys.executable, "-m", "cynic.api.entry", "--port", "8000"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return proc


def _run_battles(duration_h=1):
    """Run autonomous battles."""
    log.info(f"\nStarting {duration_h}h battle run (empirical data collection)...")
    duration_str = f"{duration_h}h"

    subprocess.run(
        [
            sys.executable,
            "-m",
            "cynic.cli",
            "battles",
            f"--duration={duration_str}",
            "--interval=30s",
        ],
        cwd="C:\\Users\\zeyxm\\Desktop\\asdfasdfa\\CYNIC",
    )


def _analyze_battles():
    """Analyze battle results."""
    if not os.path.exists(BATTLES_FILE):
        log.info("No battles logged yet (expected on first run)")
        return None

    battles = []
    try:
        with open(BATTLES_FILE, "r", encoding="utf-8") as f:
            for line in f:
                battles.append(json.loads(line))
    except Exception as e:
        log.error(f"Failed to read battles: {e}")
        return None

    if not battles:
        log.info("No battles recorded")
        return None

    # Calculate metrics
    executed = sum(1 for b in battles if b.get("executed"))
    exec_rate = executed / len(battles) * 100 if battles else 0

    q_scores = [b.get("q_score_after", 0) for b in battles if b.get("q_score_after")]
    avg_q = sum(q_scores) / len(q_scores) if q_scores else 0

    learning_signals = [b.get("learning_signal", 0) for b in battles]
    total_learning = sum(learning_signals)

    # Report
    log.info(f"\n" + "=" * 70)
    log.info(f"EMPIRICAL RESULTS — {len(battles)} battles")
    log.info(f"=" * 70)
    log.info(f"Execution rate: {exec_rate:.1f}% ({executed}/{len(battles)})")
    log.info(f"Avg Q-score: {avg_q:.1f}")
    log.info(f"Total learning signal: {total_learning:.2f}")
    log.info(f"\nBattle log: {BATTLES_FILE}")
    log.info(f"=" * 70)

    return {
        "total_battles": len(battles),
        "executed": executed,
        "execution_rate": exec_rate,
        "avg_q_score": avg_q,
        "total_learning_signal": total_learning,
        "first_battle": battles[0].get("battle_id") if battles else None,
        "last_battle": battles[-1].get("battle_id") if battles else None,
    }


async def main():
    """Orchestrate empirical run."""
    log.info("EMPIRICAL BATTLE RUN — Real data collection")
    log.info("=" * 70)

    # 1. Start server
    server_proc = _start_server()
    if not _wait_for_server():
        log.error("Server failed to start")
        return False

    try:
        # 2. Run 1 hour of battles
        log.info("\nStarting battle execution phase...")
        _run_battles(duration_h=1)

        # 3. Analyze
        log.info("\nAnalyzing results...")
        results = _analyze_battles()

        if results:
            log.info("\nSuccess: Battle run complete, data collected")
            return True
        else:
            log.warning("No battle data collected")
            return False

    finally:
        log.info("\nShutting down server...")
        server_proc.terminate()
        try:
            server_proc.wait(timeout=3)
        except:
            server_proc.kill()


if __name__ == "__main__":
    os.chdir("C:\\Users\\zeyxm\\Desktop\\asdfasdfa\\CYNIC")
    success = asyncio.run(main())
    sys.exit(0 if success else 1)
