"""
CYNIC Ingest Daemon — tails a JSONL dataset and POSTs observations to the kernel.

Universal: works for any organ that writes enriched JSONL.
Seeks to end of file on start, then polls for new lines.

Usage:
    python x_ingest_daemon.py                          # defaults: hermes/x dataset
    python x_ingest_daemon.py --dataset /path/to.jsonl # custom dataset
    python x_ingest_daemon.py --replay                 # replay full file (one-shot)

Environment:
    CYNIC_REST_ADDR  — kernel address
    CYNIC_API_KEY    — kernel auth token
    X_DATASET_PATH   — default dataset path (set by systemd)
"""

import argparse
import json
import os
import logging
import signal
import sys
import time
from pathlib import Path

import requests

logger = logging.getLogger("x-ingest")

# ── Config ──

DEFAULT_DATASET = Path(os.environ.get(
    "X_DATASET_PATH",
    Path.home() / ".cynic/organs/hermes/x/dataset.jsonl"
))
KERNEL_ADDR = ""
API_KEY = ""
ORGAN_NAME = "x-organ"
DOMAIN = "social-signal"
POLL_INTERVAL = 2.0
BATCH_SIZE = 20


def load_env():
    global KERNEL_ADDR, API_KEY
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")
    if KERNEL_ADDR and API_KEY:
        return
    env_file = Path.home() / ".cynic-env"
    if not env_file.exists():
        return
    for line in env_file.read_text().splitlines():
        line = line.strip()
        if line.startswith("#") or "=" not in line:
            continue
        key, _, val = line.partition("=")
        val = val.strip().strip('"').strip("'")
        if key.strip() == "CYNIC_REST_ADDR" and not KERNEL_ADDR:
            KERNEL_ADDR = val
        elif key.strip() == "CYNIC_API_KEY" and not API_KEY:
            API_KEY = val


# ── Observe POST ──

def post_observe(row: dict, organ_name: str = ORGAN_NAME) -> bool:
    """POST one enriched tweet to /observe. Returns True on success."""
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    text = row.get("text", "")[:200]
    author = row.get("author_screen_name", "?")
    score = row.get("signal_score", 0)
    tags = row.get("cashtags", []) + row.get("narratives", [])

    payload = {
        "tool": organ_name,
        "target": row.get("tweet_id", ""),
        "domain": DOMAIN,
        "status": "captured",
        "context": f"@{author} [{score}]: {text}",
        "tags": tags[:10],
        "agent_id": "hermes-x",
    }

    try:
        resp = requests.post(
            f"{addr}/observe",
            json=payload,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {API_KEY}",
            },
            timeout=5,
        )
        return resp.status_code == 200
    except requests.RequestException as e:
        logger.warning("POST /observe failed: %s", e)
        return False


# ── State file for cursor persistence ──

def load_cursor(state_path: Path) -> int:
    if state_path.exists():
        try:
            return int(state_path.read_text().strip())
        except (ValueError, OSError):
            pass
    return 0


def save_cursor(state_path: Path, offset: int):
    try:
        state_path.write_text(str(offset))
    except OSError:
        pass


# ── Tail loop ──

def tail_dataset(dataset: Path, state_path: Path, organ_name: str = ORGAN_NAME, replay: bool = False):
    """Tail the dataset JSONL, POST new lines to /observe."""
    if not dataset.exists():
        logger.info("Dataset not found: %s — waiting for creation", dataset)
        while not dataset.exists():
            time.sleep(POLL_INTERVAL)

    offset = 0 if replay else load_cursor(state_path)
    if not replay and offset == 0:
        # First run: seek to end (don't replay history)
        offset = dataset.stat().st_size
        save_cursor(state_path, offset)
        logger.info("First run — seeking to end (offset %d)", offset)

    logger.info("Tailing %s from offset %d (replay=%s)", dataset, offset, replay)

    sent = 0
    errors = 0
    while True:
        try:
            size = dataset.stat().st_size
        except OSError:
            time.sleep(POLL_INTERVAL)
            continue

        if size <= offset:
            if not replay:
                time.sleep(POLL_INTERVAL)
                continue
            else:
                break

        batch = []
        with open(dataset) as f:
            f.seek(offset)
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    batch.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
                if len(batch) >= BATCH_SIZE:
                    for row in batch:
                        if post_observe(row, organ_name):
                            sent += 1
                        else:
                            errors += 1
                    batch = []
            new_offset = f.tell()

        # Flush remaining
        for row in batch:
            if post_observe(row, organ_name):
                sent += 1
            else:
                errors += 1

        offset = new_offset
        save_cursor(state_path, offset)

        if replay:
            break

        if sent % 100 == 0 and sent > 0:
            logger.info("Progress: %d sent, %d errors", sent, errors)

    logger.info("Done: %d sent, %d errors", sent, errors)


# ── Main ──

def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )
    load_env()
    if not KERNEL_ADDR:
        logger.error("CYNIC_REST_ADDR not set")
        sys.exit(1)

    parser = argparse.ArgumentParser(description="CYNIC ingest daemon")
    parser.add_argument("--dataset", type=Path, default=DEFAULT_DATASET)
    parser.add_argument("--replay", action="store_true",
                        help="Replay full dataset (one-shot, no tail)")
    parser.add_argument("--organ", default=ORGAN_NAME,
                        help="Organ name for /observe tool field")
    args = parser.parse_args()

    organ = args.organ

    state_path = args.dataset.parent / "ingest_cursor.txt"

    # Graceful shutdown
    running = True
    def handle_signal(sig, frame):
        nonlocal running
        running = False
        logger.info("Shutting down (signal %s)", sig)
    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    logger.info("Ingest daemon starting — organ=%s dataset=%s kernel=%s",
                ORGAN_NAME, args.dataset, KERNEL_ADDR)

    tail_dataset(args.dataset, state_path, organ_name=organ, replay=args.replay)


if __name__ == "__main__":
    main()
