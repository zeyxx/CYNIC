"""
Telegram signal pipeline — classify, pre-filter, extract, route to kernel.

Flow per message block:
  raw messages → classify() → drop | extract+route → POST /observe or /judge

Domains map to kernel slugs (twitter, token-analysis, trading, governance, dev).
Pre-filter drops structural BARK before spending Dog tokens on it.
"""
from __future__ import annotations

import json
import logging
import os
import re
import sqlite3
import subprocess
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import requests

logger = logging.getLogger("telegram.pipeline")

# ── Channel → domain hint ─────────────────────────────────────────────────────
# "hint" = default domain for this channel. Classifier may override per-message.

CHANNEL_DOMAIN_HINTS: dict[int, str] = {
    # Pump.fun ecosystem
    -1002634233787: "dev",          # Pump Developers
    -1002286234018: "dev",          # Pump Developer Updates
    -1002124010152: "launch",        # Pump Portal (launch announcements)
    -1001996676648: "mixed",        # Pump Community (shill + some signal)
    -1002075849201: "launch",       # Pumpdotpump
    -1002001857595: "launch",       # Pump Support Portal
    # SOL signals — mostly buy-bots
    -1002066575222: "buy-bot",      # SOL TRENDING
    # Crypto analysis
    -1001662330802: "twitter",      # Kucoin Insider Signals
    -1001556054753: "twitter",      # Watcher Guru
    -1002711741394: "trading",      # Jeets Trading Journal
    -1001864215962: "trading",      # CryptoCapo TG
    -1002434520452: "twitter",      # CypherWhale
    -1002222962024: "twitter",      # Scooby Call
    -1002079811589: "twitter",      # PROOF Launch Alerts
    -1002043408864: "twitter",      # Whales Market Bot
    -1002930428556: "twitter",      # TRENCHER
    # Memecoins
    -1003895129191: "launch",       # $asdfasdfa
    -1002460104630: "mixed",        # PILLNET AI (buy-bot + raids + launch)
    -1002531693653: "launch",       # PILLNET LABS
    # Governance / ecosystem
    -1002392373515: "governance",   # MetaDAO Community
    -1002049567527: "twitter",      # Onchain Agent Enthusiasts
    -1002031241087: "twitter",      # Solsniffer Portal
    # Talaria / CYNIC internal
    -1003732138279: "ops",          # Talaria channel
    -5297007075:    "ops",          # Talaria group
    -5084954904:    "ops",          # Talaria Ops
    -1003764606730: "ops",          # CYNIC channel
    -5052243009:    "ops",          # CYNIC group
}

# ── Pre-filter patterns ───────────────────────────────────────────────────────
# Structural BARK — drop before spending LLM tokens.

_BUY_BOT_SIGNALS = [
    re.compile(r"Market Cap \$[\d ,]+", re.IGNORECASE),
    re.compile(r"🔀 [\d.]+ SOL"),
    re.compile(r"solscan\.io/address/"),
    re.compile(r"dexscreener\.com/solana/"),
    re.compile(r"⬆️.*(Up!|New Holder|Position)"),
]

_SHILL_SIGNALS = [
    re.compile(r"telegram shilling", re.IGNORECASE),
    re.compile(r"twitter.{0,10}raid", re.IGNORECASE),
    re.compile(r"(listing|upvoting).{0,20}service", re.IGNORECASE),
    re.compile(r"message @\w+.{0,30}(service|more info|details)", re.IGNORECASE),
    re.compile(r"🤡.*(raid|shill|promo)", re.IGNORECASE),
    re.compile(r"(organic (shiller|raider)|raid (team|service))", re.IGNORECASE),
]

_RAID_SIGNALS = [
    re.compile(r"Raid Ended"),
    re.compile(r"Targets Reached"),
    re.compile(r"Likes \d+ \| \d+"),
    re.compile(r"Retweets \d+ \| \d+"),
]

_MINT_PATTERN = re.compile(r"[1-9A-HJ-NP-Za-km-z]{32,44}pump\b")
_SOLANA_ADDR = re.compile(r"\b[1-9A-HJ-NP-Za-km-z]{32,44}\b")


@dataclass
class ClassifiedBlock:
    channel_id: int
    block_id: str
    text: str
    msg_count: int
    message_type: str  # buy-bot | shill | raid | ops | signal | private
    domain: str        # kernel domain slug or "drop"
    mints: list[str]
    channel_type: str = "channel"


def classify_block(
    channel_id: int,
    block_id: str,
    text: str,
    msg_count: int,
    channel_type: str = "channel",
) -> ClassifiedBlock:
    """Classify a message block and determine routing."""
    hint = CHANNEL_DOMAIN_HINTS.get(channel_id)

    # If it's a DM, default to community unless hinted otherwise
    if channel_type == "private" and not hint:
        hint = "community"
    
    if not hint:
        hint = "twitter"

    # Ops channels — observe only, never judge
    if hint == "ops":
        return ClassifiedBlock(channel_id, block_id, text, msg_count, "ops", "ops", [], channel_type)

    # Detect buy-bot pattern
    bot_matches = sum(1 for p in _BUY_BOT_SIGNALS if p.search(text))
    if bot_matches >= 2 or hint == "buy-bot":
        mints = _MINT_PATTERN.findall(text)
        if not mints:
            mints = _SOLANA_ADDR.findall(text)[:3]
        return ClassifiedBlock(
            channel_id, block_id, text, msg_count, "buy-bot", "token-analysis", mints, channel_type
        )

    # Shill/spam — structural BARK, drop
    shill_matches = sum(1 for p in _SHILL_SIGNALS if p.search(text))
    if shill_matches >= 2:
        return ClassifiedBlock(channel_id, block_id, text, msg_count, "shill", "drop", [], channel_type)

    # Raid coordination — drop
    raid_matches = sum(1 for p in _RAID_SIGNALS if p.search(text))
    if raid_matches >= 2:
        return ClassifiedBlock(channel_id, block_id, text, msg_count, "raid", "drop", [], channel_type)

    # Map hint to kernel domain
    domain_map = {
        "launch":     "token-analysis",
        "trading":    "trading",
        "governance": "governance",
        "dev":        "dev",
        "twitter":    "twitter",
        "mixed":      "twitter",
        "community":  "community",
        "askesis":    "askesis",
    }
    domain = domain_map.get(hint, "twitter")

    # Extract any token mints found in the text for context
    mints = _MINT_PATTERN.findall(text)

    return ClassifiedBlock(
        channel_id, block_id, text, msg_count, "signal", domain, mints, channel_type
    )


# ── LLM extraction ────────────────────────────────────────────────────────────

_EXTRACT_SYSTEM = """\
You are a crypto signal extractor. Given a block of Telegram messages, extract structured signals.

For each actionable signal output JSON with these fields:
{
  "domain": "<twitter|trading|token-analysis|governance|dev>",
  "content": "<the signal, cleaned and condensed>",
  "token_mint": "<Solana address if mentioned, else null>",
  "token_ticker": "<$TICKER if mentioned, else null>",
  "signal_type": "<warning|call|analysis|news|update>",
  "confidence": <0.1-0.618>
}

Rules:
- Drop: raid coordination, shill service offers, buy-bot outputs, pure emoji spam
- Compress: remove engagement bait, keep the substance
- If no actionable signal exists, output []
- Output ONLY a JSON array, no other text
"""


def extract_signals_gemini(text: str, timeout: int = 120) -> list[dict]:
    """Run Gemini CLI on a block, return list of signal dicts."""
    prompt = _EXTRACT_SYSTEM + "\n\nMessages:\n" + text
    try:
        result = subprocess.run(
            ["gemini", "-m", "gemini-2.5-flash", "-p", prompt],
            capture_output=True, text=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        logger.warning("gemini timeout on block")
        return []
    except FileNotFoundError:
        logger.error("gemini CLI not found")
        return []

    if result.returncode != 0:
        logger.warning("gemini error: %s", result.stderr[:200])
        return []

    raw = result.stdout.strip()
    if raw.startswith("```"):
        lines = [l for l in raw.split("\n") if not l.startswith("```")]
        raw = "\n".join(lines).strip()

    try:
        signals = json.loads(raw)
        return signals if isinstance(signals, list) else []
    except json.JSONDecodeError:
        logger.warning("gemini output not valid JSON: %s", raw[:100])
        return []


# ── Kernel POST ───────────────────────────────────────────────────────────────

def post_observe(
    kernel_url: str,
    api_key: str,
    signal: dict,
    channel_id: int,
    block_id: str,
    source: str = "telegram",
) -> bool:
    """POST a signal to /observe. Returns True on success."""
    body = {
        "tool": "telegram_pipeline",
        "target": signal.get("token_ticker") or signal.get("token_mint") or "unknown",
        "domain": signal.get("domain", "twitter"),
        "agent_id": "hermes-telegram",
        "value": signal,
        "context": signal.get("content", ""),
        "tags": ["telegram", source, block_id, f"channel:{channel_id}"],
    }
    try:
        resp = requests.post(
            f"{kernel_url}/observe",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            data=json.dumps(body),
            timeout=15,
        )
        return resp.status_code == 200
    except Exception:
        logger.warning("observe POST failed", exc_info=True)
        return False


# ── Main pipeline run ─────────────────────────────────────────────────────────

def get_unprocessed_blocks(
    conn: sqlite3.Connection,
    channel_id: Optional[int] = None,
    limit: Optional[int] = None,
) -> list[dict]:
    """Get message blocks not yet processed by the pipeline."""
    # Ensure pipeline tracking table exists
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS pipeline_processed (
            block_id     TEXT PRIMARY KEY,
            processed_at TEXT NOT NULL,
            message_type TEXT NOT NULL,
            domain       TEXT NOT NULL,
            signals_sent INTEGER NOT NULL DEFAULT 0
        );
    """)
    conn.commit()

    processed: set[str] = {
        row[0] for row in conn.execute(
            "SELECT block_id FROM pipeline_processed"
        ).fetchall()
    }

    where = "WHERE m.text IS NOT NULL AND length(m.text) > 5"
    params: list = []
    if channel_id:
        where += " AND m.channel_id = ?"
        params.append(channel_id)

    # Use native block_id from messages table and join with channels
    query = f"""
        SELECT m.channel_id, m.block_id, c.channel_type,
               GROUP_CONCAT(m.date || ' | ' || COALESCE(m.author_name, 'anon') || ': ' || m.text, '\n') as full_text,
               COUNT(*) as msg_count
        FROM messages m
        LEFT JOIN channels c ON m.channel_id = c.channel_id
        {where}
        GROUP BY m.channel_id, m.block_id
        ORDER BY m.date ASC
    """

    rows = conn.execute(query, params).fetchall()

    blocks: list[dict] = []
    for row in rows:
        ch_id, bid, ch_type, text, count = row
        if bid in processed or not bid:
            continue
        
        blocks.append({
            "channel_id": ch_id,
            "block_id": bid,
            "channel_type": ch_type or "channel",
            "text": text,
            "msg_count": count,
        })

    if limit:
        blocks = blocks[:limit]
    return blocks


def run_pipeline(
    db_path: str,
    kernel_url: str,
    api_key: str,
    channel_id: Optional[int] = None,
    limit: Optional[int] = None,
    dry_run: bool = False,
) -> None:
    """Run the full pipeline on pending blocks."""
    conn = sqlite3.connect(db_path)
    blocks = get_unprocessed_blocks(conn, channel_id=channel_id, limit=limit)
    logger.info("pipeline: %d blocks to process", len(blocks))

    stats = {"processed": 0, "dropped": 0, "signals": 0, "errors": 0}
    start = time.monotonic()

    for block in blocks:
        classified = classify_block(
            block["channel_id"], block["block_id"],
            block["text"], block["msg_count"],
            channel_type=block["channel_type"],
        )

        if classified.domain == "drop":
            logger.debug("drop [%s] block=%s", classified.message_type, classified.block_id)
            stats["dropped"] += 1
            if not dry_run:
                conn.execute(
                    "INSERT OR IGNORE INTO pipeline_processed "
                    "(block_id, processed_at, message_type, domain, signals_sent) "
                    "VALUES (?, ?, ?, ?, 0)",
                    (block["block_id"], datetime.now(timezone.utc).isoformat(),
                     classified.message_type, "drop"),
                )
                conn.commit()
            continue

        if classified.domain == "ops":
            logger.debug("ops block=%s — observe only", classified.block_id)
            stats["processed"] += 1
            if not dry_run:
                conn.execute(
                    "INSERT OR IGNORE INTO pipeline_processed "
                    "(block_id, processed_at, message_type, domain, signals_sent) "
                    "VALUES (?, ?, ?, ?, 0)",
                    (block["block_id"], datetime.now(timezone.utc).isoformat(),
                     "ops", "ops"),
                )
                conn.commit()
            continue

        if dry_run:
            logger.info(
                "DRY [%s→%s] block=%s mints=%s text=%s",
                classified.message_type, classified.domain,
                classified.block_id, classified.mints,
                classified.text[:60],
            )
            stats["processed"] += 1
            continue

        # LLM extraction
        try:
            signals = extract_signals_gemini(classified.text)
        except Exception:
            logger.error("extraction failed block=%s", classified.block_id, exc_info=True)
            stats["errors"] += 1
            time.sleep(2)
            continue

        # Override domain from classifier for buy-bots with mints
        if classified.message_type == "buy-bot" and classified.mints:
            for sig in signals:
                sig["domain"] = "token-analysis"
                if not sig.get("token_mint") and classified.mints:
                    sig["token_mint"] = classified.mints[0]

        sent = 0
        for sig in signals:
            if post_observe(kernel_url, api_key, sig,
                            classified.channel_id, classified.block_id):
                sent += 1

        stats["signals"] += sent
        stats["processed"] += 1

        conn.execute(
            "INSERT OR IGNORE INTO pipeline_processed "
            "(block_id, processed_at, message_type, domain, signals_sent) "
            "VALUES (?, ?, ?, ?, ?)",
            (block["block_id"], datetime.now(timezone.utc).isoformat(),
             classified.message_type, classified.domain, sent),
        )
        conn.commit()
        time.sleep(1)  # K25 — rate-limit LLM calls

    elapsed = time.monotonic() - start
    logger.info(
        "pipeline done: processed=%d dropped=%d signals=%d errors=%d elapsed=%.1fs",
        stats["processed"], stats["dropped"], stats["signals"], stats["errors"], elapsed,
    )
    conn.close()


def main() -> None:
    import argparse
    logging.basicConfig(
        level=logging.INFO,
        format='{"time":"%(asctime)s","name":"%(name)s","level":"%(levelname)s","msg":"%(message)s"}',
    )

    parser = argparse.ArgumentParser(description="Telegram signal pipeline")
    parser.add_argument("--channel", type=int, help="Process only this channel ID")
    parser.add_argument("--limit", type=int, help="Max blocks to process")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--db", default=str(Path.home() / ".cynic/organs/telegram/messages.db")
    )
    args = parser.parse_args()

    kernel_url = os.environ.get("CYNIC_REST_ADDR", "")
    if kernel_url and not kernel_url.startswith("http"):
        kernel_url = f"http://{kernel_url}"
    api_key = os.environ.get("CYNIC_API_KEY", "")

    run_pipeline(
        db_path=args.db,
        kernel_url=kernel_url,
        api_key=api_key,
        channel_id=args.channel,
        limit=args.limit,
        dry_run=args.dry_run,
    )


if __name__ == "__main__":
    main()
