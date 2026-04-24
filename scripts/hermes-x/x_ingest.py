"""
CYNIC X Ingest — watches for signals and POSTs them to the kernel for Dog evaluation.

Connects x_signal.py output to the CYNIC kernel via POST /judge.
Runs as a daemon alongside x_proxy.py.

Usage:
    python x_ingest.py --watch captures/
    python x_ingest.py signal.json        # one-shot

Environment:
    CYNIC_REST_ADDR  — kernel address (default from ~/.cynic-env)
    CYNIC_API_KEY    — kernel auth token
"""

import json
import sys
import os
import logging
import time
from pathlib import Path

import requests

from x_signal import process_capture, watch_directory

logger = logging.getLogger("x-ingest")

KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
API_KEY = os.environ.get("CYNIC_API_KEY", "")


def load_env():
    """Load kernel address and API key from ~/.cynic-env if not in environment."""
    global KERNEL_ADDR, API_KEY
    env_file = Path.home() / ".cynic-env"
    if env_file.exists() and (not KERNEL_ADDR or not API_KEY):
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("#") or "=" not in line:
                continue
            key, _, val = line.partition("=")
            key = key.strip()
            val = val.strip().strip('"').strip("'")
            if key == "CYNIC_REST_ADDR" and not KERNEL_ADDR:
                KERNEL_ADDR = val
            elif key == "CYNIC_API_KEY" and not API_KEY:
                API_KEY = val


def format_judge_content(signal: dict) -> str:
    """Format a signal into content string for POST /judge."""
    lines = [
        f"X/Twitter Social Signal — {signal['query']}",
        f"Source: {signal['source']} | Captured: {signal['timestamp']}",
        f"Tweets found: {signal['tweet_count']}",
        "",
    ]

    metrics = signal.get("metrics", {})
    if metrics:
        lines.extend([
            f"Metrics: {metrics.get('total_views', 0)} views, "
            f"{metrics.get('total_likes', 0)} likes, "
            f"{metrics.get('total_retweets', 0)} RTs, "
            f"{metrics.get('total_replies', 0)} replies",
            f"Engagement rate: {metrics.get('engagement_rate', 0):.4f}",
            f"Unique authors: {metrics.get('unique_authors', 0)} "
            f"({metrics.get('original_vs_rt', 0)} original, "
            f"{signal['tweet_count'] - metrics.get('original_vs_rt', 0)} RTs)",
            "",
        ])

    authors = signal.get("authors", {})
    if authors.get("names"):
        lines.append(f"Key authors: {', '.join(authors['names'][:5])}")
        lines.append(f"Max followers: {authors.get('max_followers', 0)}, "
                     f"Verified: {authors.get('verified_count', 0)}")
        lines.append("")

    cashtags = signal.get("cashtags", {})
    if cashtags:
        lines.append(f"Cashtags: {', '.join(f'${k}({v})' for k, v in cashtags.items())}")
        lines.append("")

    for sample in signal.get("content_sample", []):
        lines.append(f"@{sample['author']} ({sample['likes']}L, {sample['views']}V): {sample['text']}")
        lines.append("")

    return "\n".join(lines)


def post_to_kernel(signal: dict) -> dict | None:
    """POST signal to kernel /judge and return verdict."""
    if not KERNEL_ADDR:
        logger.error("CYNIC_REST_ADDR not set")
        return None

    content = format_judge_content(signal)
    payload = {
        "content": content,
        "context": f"Social signal from X/Twitter. Query: {signal['query']}. "
                   f"Automated capture via x-proxy pipeline.",
        "domain": "trading",
    }

    headers = {"Content-Type": "application/json"}
    if API_KEY:
        headers["Authorization"] = f"Bearer {API_KEY}"

    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    try:
        resp = requests.post(
            f"{addr}/judge",
            json=payload,
            headers=headers,
            timeout=120,
        )
        resp.raise_for_status()
        verdict = resp.json()
        logger.info(
            "Verdict: %s (Q=%.3f) — Dogs: %s",
            verdict.get("verdict"),
            verdict.get("q_score", {}).get("total", 0),
            verdict.get("dogs_used", ""),
        )
        return verdict
    except requests.RequestException as e:
        logger.error("Kernel POST failed: %s", e)
        return None


def handle_signal(signal: dict):
    """Process a signal: log + POST to kernel."""
    logger.info("Signal: %s — %d tweets, %d views",
                signal.get("query"), signal.get("tweet_count", 0),
                signal.get("metrics", {}).get("total_views", 0))

    if signal.get("tweet_count", 0) == 0:
        logger.info("Empty signal, skipping kernel POST")
        return

    verdict = post_to_kernel(signal)
    if verdict:
        # Save verdict alongside signal
        out_dir = Path(__file__).parent / "verdicts"
        out_dir.mkdir(exist_ok=True)
        ts = signal["timestamp"].replace(":", "").replace("-", "")[:15]
        out_path = out_dir / f"{ts}_{signal['query'][:20]}.json"
        out_path.write_text(json.dumps({
            "signal": signal,
            "verdict": verdict,
        }, indent=2))
        logger.info("Saved verdict → %s", out_path.name)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")
    load_env()

    if not KERNEL_ADDR:
        logger.error("Set CYNIC_REST_ADDR (or populate ~/.cynic-env)")
        sys.exit(1)

    if len(sys.argv) < 2:
        print("Usage: python x_ingest.py <capture.json | --watch <dir>>")
        sys.exit(1)

    if sys.argv[1] == "--watch":
        watch_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).parent / "captures"
        watch_directory(watch_dir, handle_signal)
    else:
        signal = process_capture(Path(sys.argv[1]))
        if signal:
            handle_signal(signal)
