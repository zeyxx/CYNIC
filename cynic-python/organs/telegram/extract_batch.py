"""
Batch extraction — runs Qwen 7B on core against Telegram message blocks.

Reads messages grouped by block_id from SQLite, sends each block to the LLM,
writes structured JSON results to an extractions table.

Usage:
    python -m organs.telegram.extract_batch [--channel CHANNEL_ID] [--limit N] [--dry-run]

Environment:
    CYNIC_SOVEREIGN_KEY — llama-server auth on core
"""
from __future__ import annotations

import argparse
import json
import logging
import os
import sqlite3
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import requests

from .config import load_config
from .schema import bootstrap_db

logger = logging.getLogger("telegram.extract")

_DEFAULT_CONFIG = Path.home() / ".cynic" / "organs" / "telegram" / "config.yaml"

SYSTEM_PROMPT = """Extract trading signals from these Telegram messages. For each actionable signal output JSON:
{"ticker": "$X", "direction": "long|short|neutral", "type": "call|update|exit|sentiment", "context": "brief description"}

Rules:
- Skip promotions, referral links, social engagement requests ("like and RT")
- Skip messages with no trading content (greetings, personal updates)
- If no actionable signal exists, output an empty array []
- Output ONLY a JSON array, no other text"""

LLAMA_URL = os.environ.get(
    "CYNIC_LLAMA_URL",
    f"http://{os.environ.get('CYNIC_REST_ADDR', 'localhost:3030').split(':')[0]}:8080/v1/chat/completions",
)


def ensure_extractions_table(conn: sqlite3.Connection) -> None:
    """Create extractions table if absent."""
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS extractions (
            block_id     TEXT PRIMARY KEY,
            channel_id   INTEGER NOT NULL,
            llm_output   TEXT NOT NULL,
            signal_count INTEGER NOT NULL DEFAULT 0,
            model        TEXT NOT NULL,
            tokens_used  INTEGER NOT NULL DEFAULT 0,
            extracted_at TEXT NOT NULL,
            version      INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_extractions_channel
            ON extractions(channel_id);
    """)
    conn.commit()


def get_blocks(
    conn: sqlite3.Connection,
    channel_id: Optional[int] = None,
    limit: Optional[int] = None,
) -> list[dict]:
    """Get message blocks not yet extracted, grouped by block_id or individual messages."""
    where = "WHERE m.text IS NOT NULL AND length(m.text) > 20"
    params: list = []

    if channel_id:
        where += " AND m.channel_id = ?"
        params.append(channel_id)

    # Get individual messages, we'll window them in Python
    query = f"""
        SELECT m.channel_id, m.message_id, m.date, m.text,
               m.author_name, m.block_id
        FROM messages m
        {where}
        ORDER BY m.channel_id, m.date ASC
    """

    cursor = conn.execute(query, params)
    rows = cursor.fetchall()

    # Group into windows of ~12 messages (fits in 4K context)
    # Skip blocks already extracted
    extracted_blocks: set[str] = set()
    try:
        for (bid,) in conn.execute("SELECT block_id FROM extractions").fetchall():
            extracted_blocks.add(bid)
    except sqlite3.OperationalError:
        pass  # extractions table may not exist yet

    window_size = 12
    blocks: list[dict] = []
    current_channel = None
    window: list[tuple] = []

    def flush_window() -> None:
        if not window:
            return
        ch = window[0][0]
        first_date = window[0][2]
        bid = f"{ch}_batch_{first_date}"
        if bid in extracted_blocks:
            return
        text = "\n".join(
            f"{r[2]} | {r[4] or 'anon'}: {r[3]}" for r in window
        )
        blocks.append({
            "channel_id": ch,
            "block_id": bid,
            "text": text,
            "msg_count": len(window),
        })

    for row in rows:
        ch = row[0]
        if ch != current_channel:
            flush_window()
            window = []
            current_channel = ch
        window.append(row)
        if len(window) >= window_size:
            flush_window()
            window = []

    flush_window()

    if limit:
        blocks = blocks[:limit]

    return blocks


def extract_block_gemini(text: str, timeout: int = 300) -> tuple[str, int, int]:
    """Send a block to Gemini CLI, return (json_str, signal_count, tokens_used)."""
    import subprocess

    prompt = SYSTEM_PROMPT + "\n\nMessages:\n" + text
    result = subprocess.run(
        ["gemini", "-m", "gemini-2.5-flash", "-p", prompt],
        capture_output=True, text=True, timeout=timeout,
    )
    if result.returncode != 0:
        raise RuntimeError(f"gemini CLI error: {result.stderr[:200]}")

    raw = result.stdout.strip()
    if raw.startswith("```"):
        lines = raw.split("\n")
        lines = [l for l in lines if not l.startswith("```")]
        raw = "\n".join(lines).strip()

    try:
        signals = json.loads(raw)
        if not isinstance(signals, list):
            signals = []
    except json.JSONDecodeError:
        signals = []
        raw = "[]"

    return raw, len(signals), 0


def extract_block_llama(text: str, api_key: str, timeout: int = 90) -> tuple[str, int, int]:
    """Send a block to local llama-server, return (json_str, signal_count, tokens_used)."""
    body = {
        "model": "qwen",
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": text},
        ],
        "temperature": 0.1,
        "max_tokens": 1024,
    }
    resp = requests.post(
        LLAMA_URL,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        data=json.dumps(body),
        timeout=timeout,
    )
    resp.raise_for_status()
    data = resp.json()

    content = data["choices"][0]["message"]["content"]
    tokens = data.get("usage", {}).get("total_tokens", 0)

    try:
        signals = json.loads(content)
        if not isinstance(signals, list):
            signals = []
    except json.JSONDecodeError:
        signals = []

    return content, len(signals), tokens


# Backend selection: "gemini" uses gemini CLI, "llama" uses local llama-server
EXTRACT_BACKEND = os.environ.get("EXTRACT_BACKEND", "gemini")


def extract_block(text: str, api_key: str, timeout: int = 300) -> tuple[str, int, int]:
    """Route to the configured backend."""
    if EXTRACT_BACKEND == "gemini":
        return extract_block_gemini(text, timeout)
    return extract_block_llama(text, api_key, timeout)


def run_batch(
    channel_id: Optional[int] = None,
    limit: Optional[int] = None,
    dry_run: bool = False,
) -> None:
    """Run batch extraction on pending blocks."""
    cfg = load_config(str(_DEFAULT_CONFIG))
    model_name = "gemini-2.5-flash" if EXTRACT_BACKEND == "gemini" else "qwen25-7b-core"
    logger.info("backend: %s, model: %s", EXTRACT_BACKEND, model_name)
    api_key = os.environ.get("CYNIC_SOVEREIGN_KEY", "")
    if EXTRACT_BACKEND == "llama" and not api_key and not dry_run:
        logger.error("CYNIC_SOVEREIGN_KEY not set")
        return

    conn = bootstrap_db(cfg.db_path)
    ensure_extractions_table(conn)

    blocks = get_blocks(conn, channel_id=channel_id, limit=limit)
    logger.info("found %d blocks to extract", len(blocks))

    if dry_run:
        for b in blocks[:5]:
            logger.info("DRY RUN block=%s msgs=%d text=%s",
                        b["block_id"], b["msg_count"], b["text"][:80])
        logger.info("... and %d more", max(0, len(blocks) - 5))
        conn.close()
        return

    extracted = 0
    signals_total = 0
    tokens_total = 0
    errors = 0
    start = time.monotonic()

    for i, block in enumerate(blocks):
        try:
            content, signal_count, tokens = extract_block(block["text"], api_key)
            conn.execute(
                """INSERT OR REPLACE INTO extractions
                   (block_id, channel_id, llm_output, signal_count, model,
                    tokens_used, extracted_at, version)
                   VALUES (?, ?, ?, ?, ?, ?, ?, 1)""",
                (
                    block["block_id"], block["channel_id"], content,
                    signal_count, model_name, tokens,
                    datetime.now(timezone.utc).isoformat(),
                ),
            )
            conn.commit()
            extracted += 1
            signals_total += signal_count
            tokens_total += tokens

            if (i + 1) % 10 == 0:
                elapsed = time.monotonic() - start
                rate = extracted / elapsed * 3600
                logger.info(
                    "progress: %d/%d blocks, %d signals, %d tokens, %.0f blocks/h",
                    extracted, len(blocks), signals_total, tokens_total, rate,
                )

        except Exception:
            errors += 1
            logger.error("extraction failed for block %s", block["block_id"],
                         exc_info=True)
            time.sleep(2)  # backoff on error

    elapsed = time.monotonic() - start
    conn.close()
    logger.info(
        "batch complete: %d extracted, %d signals, %d errors, %d tokens, %.1fs",
        extracted, signals_total, errors, tokens_total, elapsed,
    )


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format='{"time":"%(asctime)s","name":"%(name)s","level":"%(levelname)s","msg":"%(message)s"}',
    )

    parser = argparse.ArgumentParser(description="Batch LLM extraction on Telegram messages")
    parser.add_argument("--channel", type=int, help="Extract only this channel ID")
    parser.add_argument("--limit", type=int, help="Max blocks to extract")
    parser.add_argument("--dry-run", action="store_true", help="Show blocks without extracting")
    parser.add_argument("--config", type=str, default=str(_DEFAULT_CONFIG))
    args = parser.parse_args()

    run_batch(channel_id=args.channel, limit=args.limit, dry_run=args.dry_run)


if __name__ == "__main__":
    main()
