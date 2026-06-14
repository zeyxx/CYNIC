#!/usr/bin/env python3
"""Discord signal pipeline for bounties, grants, infra, and community signals."""
from __future__ import annotations

import argparse
import json
import logging
import os
import re
import sqlite3
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import requests

logger = logging.getLogger("discord.pipeline")

_BOUNTY = re.compile(r"\b(bounty|bounties|prime|reward|paid task)\b", re.IGNORECASE)
_GRANT = re.compile(r"\b(grant|funding|microgrant|foundation|superteam|colosseum)\b", re.IGNORECASE)
_INFRA = re.compile(r"\b(rpc|validator|indexer|geyser|yellowstone|agave|node|infra)\b", re.IGNORECASE)
_SECURITY = re.compile(r"\b(audit|security|exploit|rug|scam|wallet|sybil)\b", re.IGNORECASE)
_COMMUNITY = re.compile(r"\b(community|builder|collab|cofounder|partner|sponsor)\b", re.IGNORECASE)
_SOLANA = re.compile(r"\b(solana|spl|svm|anchor|pinocchio|jito)\b", re.IGNORECASE)
_URL = re.compile(r"https?://\S+")
_EARN_URL = re.compile(r"https://earn\.superteam\.fun/listing/[^\s>]+")
_CATEGORY = re.compile(r"^\s*(?:[^\w\s]+)?\s*(Development|Design|Content|Growth)\s*$", re.IGNORECASE)
_REWARD = re.compile(r"\((?:\$|USDG\s*)?([\d,]+)\s*(?:USDG|USDC|USD)?\)", re.IGNORECASE)


@dataclass
class DiscordSignal:
    signal_type: str
    domain: str
    confidence: float
    content: str
    links: list[str]


@dataclass
class Opportunity:
    url: str
    title: str
    category: str
    capability: str
    reward: str
    reward_usd: int | None
    fit_score: int
    raw_text: str


def classify_message(text: str) -> Optional[DiscordSignal]:
    """Classify one message into a kernel-ready signal."""
    clean = " ".join(text.split())
    if len(clean) < 12:
        return None

    links = _URL.findall(clean)
    is_solana = bool(_SOLANA.search(clean))
    matches = {
        "bounty": bool(_BOUNTY.search(clean)),
        "grant": bool(_GRANT.search(clean)),
        "infra": bool(_INFRA.search(clean)),
        "security": bool(_SECURITY.search(clean)),
        "community": bool(_COMMUNITY.search(clean)),
    }

    if not any(matches.values()):
        return None

    confidence = 0.25
    if is_solana:
        confidence += 0.12
    if links:
        confidence += 0.08
    if matches["bounty"] or matches["grant"]:
        confidence += 0.12
    if matches["infra"] or matches["security"]:
        confidence += 0.08
    confidence = min(confidence, 0.618)

    if matches["bounty"]:
        signal_type = "bounty"
        domain = "dev"
    elif matches["grant"]:
        signal_type = "grant"
        domain = "governance"
    elif matches["infra"]:
        signal_type = "infrastructure"
        domain = "D3"
    elif matches["security"]:
        signal_type = "security"
        domain = "security"
    else:
        signal_type = "community"
        domain = "community"

    return DiscordSignal(
        signal_type=signal_type,
        domain=domain,
        confidence=confidence,
        content=clean[:2000],
        links=links[:5],
    )


def extract_opportunities(text: str) -> list[Opportunity]:
    """Extract Superteam Earn listings from one Discord announcement."""
    lines = [line.strip() for line in text.splitlines()]
    opportunities: list[Opportunity] = []
    current_category = ""

    for idx, line in enumerate(lines):
        cat = _CATEGORY.match(line)
        if cat:
            current_category = cat.group(1).title()
            continue

        match = _EARN_URL.search(line)
        if not match:
            continue

        url = match.group(0).rstrip(">")
        title = ""
        for prev in reversed(lines[max(0, idx - 4):idx]):
            if prev and not prev.startswith("<") and not _CATEGORY.match(prev):
                title = prev
                break
        if not title:
            title = url.rsplit("/", 2)[-2].replace("-", " ").title()

        reward_match = _REWARD.search(title)
        reward = reward_match.group(0) if reward_match else ""
        reward_usd = None
        if reward_match:
            reward_usd = int(reward_match.group(1).replace(",", ""))

        opportunities.append(
            opportunity_from_listing(
                url=url,
                title=title,
                category=current_category,
                reward=reward,
                reward_usd=reward_usd,
                raw_text=text[:4000],
            )
        )

    return opportunities


def opportunity_from_listing(
    url: str,
    title: str,
    category: str,
    reward: str,
    reward_usd: int | None,
    raw_text: str,
) -> Opportunity:
    capability = classify_capability(title, category)
    return Opportunity(
        url=url,
        title=title,
        category=category,
        capability=capability,
        reward=reward,
        reward_usd=reward_usd,
        fit_score=score_opportunity(title, category, capability, reward_usd),
        raw_text=raw_text,
    )


def classify_capability(title: str, category: str) -> str:
    """Map Superteam categories/listing titles to CYNIC capability tracks."""
    text = f"{category} {title}".lower()
    if re.search(r"\b(mobile|android|ios|react native|flutter|seeker)\b", text):
        return "mobile"
    if re.search(r"\b(frontend|front-end|ui|ux|react|next|website|dashboard)\b", text):
        return "frontend"
    if re.search(r"\b(backend|api|indexer|rpc|validator|data|database|agent|eliza|integration)\b", text):
        return "backend"
    if re.search(r"\b(thread|write|writing|article|video|explainer|content|correspondent)\b", text):
        return "content"
    if re.search(r"\b(growth|community|ambassador|promote|campaign|referral)\b", text):
        return "growth"
    if re.search(r"\b(design|mascot|fanart|visual|vertical format)\b", text):
        return "design"
    if category.lower() == "development":
        return "backend"
    if category.lower() in {"content", "growth", "design"}:
        return category.lower()
    return "general"


def score_opportunity(
    title: str,
    category: str,
    capability: str,
    reward_usd: int | None,
) -> int:
    """Score fit for CYNIC treasury/community strategy across Superteam tracks."""
    text = f"{category} {capability} {title}".lower()
    base_by_capability = {
        "backend": 58,
        "frontend": 55,
        "mobile": 52,
        "content": 45,
        "growth": 42,
        "design": 34,
        "general": 35,
    }
    score = base_by_capability.get(capability, 35)
    if "feedback" in text or "product" in text:
        score += 18
    if "prediction" in text or "markets" in text or "futarchy" in text:
        score += 18
    if "agent" in text or "eliza" in text:
        score += 12
    if "solana" in text or "summit" in text:
        score += 8
    if "rpc" in text or "indexer" in text or "infra" in text or "data" in text:
        score += 16
    if reward_usd and reward_usd >= 1000:
        score += 8
    if reward_usd and reward_usd >= 3000:
        score += 5
    return max(0, min(100, score))


def legacy_score_opportunity(title: str, category: str, reward_usd: int | None) -> int:
    """Backward-compatible wrapper for old callers."""
    capability = classify_capability(title, category)
    return score_opportunity(title, category, capability, reward_usd)


def upsert_opportunities(
    conn: sqlite3.Connection,
    opportunities: list[Opportunity],
    source: str,
) -> int:
    """Store extracted opportunities. Returns number of upserts attempted."""
    now = datetime.now(timezone.utc).isoformat()
    for opp in opportunities:
        conn.execute(
            """INSERT INTO opportunities
               (url, title, category, capability, reward, reward_usd, source, fit_score,
                status, first_seen, last_seen, raw_text)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'new', ?, ?, ?)
               ON CONFLICT(url) DO UPDATE SET
                 title=excluded.title,
                 category=excluded.category,
                 capability=excluded.capability,
                 reward=excluded.reward,
                 reward_usd=excluded.reward_usd,
                 fit_score=excluded.fit_score,
                 last_seen=excluded.last_seen,
                 raw_text=excluded.raw_text""",
            (
                opp.url,
                opp.title,
                opp.category,
                opp.capability,
                opp.reward,
                opp.reward_usd,
                source,
                opp.fit_score,
                now,
                now,
                opp.raw_text,
            ),
        )
    conn.commit()
    return len(opportunities)


def post_observe(
    kernel_url: str,
    auth_key: str,
    signal: DiscordSignal,
    guild_id: int | None,
    channel_id: int,
    message_id: int,
) -> bool:
    """POST a Discord signal to /observe."""
    if not kernel_url:
        return False
    body = {
        "tool": "discord_pipeline",
        "target": signal.signal_type,
        "domain": signal.domain,
        "agent_id": "hermes-discord",
        "value": {
            "signal_type": signal.signal_type,
            "content": signal.content,
            "confidence": signal.confidence,
            "links": signal.links,
            "guild_id": guild_id,
            "channel_id": channel_id,
            "message_id": message_id,
        },
        "context": signal.content,
        "tags": [
            "discord",
            "community-perception",
            signal.signal_type,
            f"channel:{channel_id}",
        ],
    }
    try:
        resp = requests.post(
            f"{kernel_url}/observe",
            headers={
                "Authorization": f"Bearer {auth_key}",
                "Content-Type": "application/json",
            },
            data=json.dumps(body),
            timeout=15,
        )
        return resp.status_code == 200
    except Exception:
        logger.warning("observe POST failed", exc_info=True)
        return False


def get_unprocessed(conn: sqlite3.Connection, limit: int | None) -> list[tuple]:
    """Return unprocessed message rows."""
    query = """
        SELECT guild_id, channel_id, message_id, date, text
        FROM messages
        WHERE text IS NOT NULL AND length(text) > 5
        ORDER BY date ASC
    """
    rows = conn.execute(query).fetchall()
    out: list[tuple] = []
    for row in rows:
        key = f"{row[1]}:{row[2]}"
        exists = conn.execute(
            "SELECT 1 FROM pipeline_processed WHERE message_key = ?",
            (key,),
        ).fetchone()
        if exists:
            continue
        out.append(row)
        if limit and len(out) >= limit:
            break
    return out


def run_pipeline(
    db_path: str,
    kernel_url: str,
    auth_key: str,
    limit: int | None = None,
    dry_run: bool = False,
) -> None:
    """Process stored Discord messages."""
    conn = sqlite3.connect(db_path)
    from .schema import SCHEMA_SQL, migrate_db
    from .skills import bootstrap_skill_assessments

    conn.executescript(SCHEMA_SQL)
    migrate_db(conn)
    bootstrap_skill_assessments(conn)
    rows = get_unprocessed(conn, limit)
    stats = {"processed": 0, "sent": 0, "ignored": 0, "opportunities": 0}
    start = time.monotonic()

    for guild_id, channel_id, message_id, _date, text in rows:
        key = f"{channel_id}:{message_id}"
        opportunities = extract_opportunities(text or "")
        if opportunities and not dry_run:
            stats["opportunities"] += upsert_opportunities(
                conn, opportunities, source=f"discord:{channel_id}:{message_id}"
            )
        elif opportunities:
            stats["opportunities"] += len(opportunities)

        signal = classify_message(text or "")
        if not signal:
            stats["ignored"] += 1
            if not dry_run:
                conn.execute(
                    "INSERT OR IGNORE INTO pipeline_processed "
                    "(message_key, processed_at, signal_type, domain, sent) "
                    "VALUES (?, ?, ?, ?, 0)",
                    (key, datetime.now(timezone.utc).isoformat(), "ignored", "drop"),
                )
                conn.commit()
            continue

        sent = False
        if dry_run:
            logger.info("DRY [%s/%s] %s", signal.signal_type, signal.domain, signal.content[:120])
        else:
            sent = post_observe(kernel_url, auth_key, signal, guild_id, channel_id, message_id)
            conn.execute(
                "INSERT OR IGNORE INTO pipeline_processed "
                "(message_key, processed_at, signal_type, domain, sent) "
                "VALUES (?, ?, ?, ?, ?)",
                (
                    key,
                    datetime.now(timezone.utc).isoformat(),
                    signal.signal_type,
                    signal.domain,
                    1 if sent else 0,
                ),
            )
            conn.commit()

        stats["sent"] += 1 if sent else 0
        stats["processed"] += 1

    elapsed = time.monotonic() - start
    logger.info(
        "pipeline done: processed=%d sent=%d ignored=%d opportunities=%d elapsed=%.1fs",
        stats["processed"],
        stats["sent"],
        stats["ignored"],
        stats["opportunities"],
        elapsed,
    )
    conn.close()


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format='{"time":"%(asctime)s","name":"%(name)s","level":"%(levelname)s","msg":"%(message)s"}',
    )
    parser = argparse.ArgumentParser(description="Discord signal pipeline")
    parser.add_argument("--db", default=str(Path.home() / ".cynic/organs/discord/messages.db"))
    parser.add_argument("--limit", type=int, help="Max messages to process")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    kernel_url = os.environ.get("CYNIC_REST_ADDR", "")
    if kernel_url and not kernel_url.startswith("http"):
        kernel_url = f"http://{kernel_url}"
    auth_key = os.environ.get("CYNIC_API_KEY", "")
    run_pipeline(args.db, kernel_url, auth_key, limit=args.limit, dry_run=args.dry_run)


if __name__ == "__main__":
    main()
