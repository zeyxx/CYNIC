"""CYNIC Discord passive capture proxy.

mitmproxy addon that observes Discord web API responses already loaded by the
operator's own browser session. It does not authenticate, initiate requests, or
use a bot/user token. Capture is limited to channel IDs explicitly allowlisted in
~/.cynic/organs/discord/config.yaml.

Usage:
    mitmdump -s organs/discord/src/passive_proxy.py \
      --listen-host 127.0.0.1 -p 8890 \
      --set "allow_hosts=(?:.*\\.)?discord\\.com"
"""
from __future__ import annotations

import html
import json
import logging
import os
import re
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse

from mitmproxy import http

from organs.discord.src.config import load_config, validate_allowlist
from organs.discord.src.schema import bootstrap_db

logger = logging.getLogger("discord-passive-proxy")

_DEFAULT_CONFIG = Path.home() / ".cynic" / "organs" / "discord" / "config.yaml"
_CHANNEL_MESSAGES = re.compile(r"/api/v\d+/channels/(\d+)/messages")
_EARN_LISTING = re.compile(r"^/listing/([^/?#]+)/?")
_TITLE = re.compile(r"<title[^>]*>(.*?)</title>", re.IGNORECASE | re.DOTALL)
_TAG = re.compile(r"<[^>]+>")


class DiscordPassiveCapture:
    """Persist allowlisted Discord channel messages from web responses."""

    def __init__(self) -> None:
        config_path = os.environ.get("DISCORD_ORGAN_CONFIG", str(_DEFAULT_CONFIG))
        self.cfg = load_config(config_path)
        validate_allowlist(self.cfg)
        self.allowed_channels = set(self.cfg.allowed_channel_ids)
        self.conn = bootstrap_db(self.cfg.db_path)

    def response(self, flow: http.HTTPFlow) -> None:
        parsed = urlparse(flow.request.pretty_url)
        if parsed.netloc.endswith("earn.superteam.fun"):
            self._capture_earn(flow, parsed)
            return

        if not parsed.netloc.endswith("discord.com"):
            return

        match = _CHANNEL_MESSAGES.search(parsed.path)
        if not match:
            return

        channel_id = int(match.group(1))
        if channel_id not in self.allowed_channels:
            return

        content_type = flow.response.headers.get("content-type", "")
        if "application/json" not in content_type:
            return

        try:
            payload = json.loads(flow.response.get_text())
        except (json.JSONDecodeError, UnicodeDecodeError):
            logger.warning("failed to parse Discord response for channel %s", channel_id)
            return

        if not isinstance(payload, list):
            return

        inserted = 0
        for item in payload:
            if isinstance(item, dict) and self._store_message(channel_id, item):
                inserted += 1
        if inserted:
            logger.info("captured %d Discord messages from channel %s", inserted, channel_id)

    def _capture_earn(self, flow: http.HTTPFlow, parsed) -> None:
        """Track visits to Superteam Earn listing pages."""
        if not _EARN_LISTING.match(parsed.path):
            return
        content_type = flow.response.headers.get("content-type", "")
        if "text/html" not in content_type:
            return
        try:
            html_text = flow.response.get_text()
        except UnicodeDecodeError:
            return

        canonical = f"https://earn.superteam.fun{parsed.path}"
        title = _extract_title(html_text)
        excerpt = _extract_excerpt(html_text)
        captured_at = datetime.now(timezone.utc).isoformat()
        raw_path = self._write_snapshot(canonical, captured_at, html_text)

        self.conn.execute(
            """INSERT INTO opportunity_snapshots
               (url, captured_at, status_code, title, excerpt, raw_path)
               VALUES (?, ?, ?, ?, ?, ?)""",
            (canonical, captured_at, flow.response.status_code, title, excerpt, raw_path),
        )
        self.conn.execute(
            """UPDATE opportunities
               SET platform_status = 'seen',
                   platform_last_seen = ?,
                   platform_title = COALESCE(NULLIF(?, ''), platform_title),
                   platform_excerpt = COALESCE(NULLIF(?, ''), platform_excerpt)
               WHERE url LIKE ?""",
            (captured_at, title, excerpt, canonical + "%"),
        )
        self.conn.commit()
        logger.info("captured Superteam Earn listing: %s", canonical)

    def _store_message(self, channel_id: int, item: dict) -> bool:
        message_id = _as_int(item.get("id"))
        if message_id is None:
            return False
        author = item.get("author") if isinstance(item.get("author"), dict) else {}
        author_id = _as_int(author.get("id"))
        guild_id = _as_int(item.get("guild_id"))
        date = item.get("timestamp") or datetime.now(timezone.utc).isoformat()
        text = item.get("content") or ""
        reply_to = None
        if isinstance(item.get("message_reference"), dict):
            reply_to = _as_int(item["message_reference"].get("message_id"))

        cursor = self.conn.execute(
            """INSERT OR IGNORE INTO messages
               (guild_id, channel_id, message_id, author_id, author_name, date,
                text, reply_to, raw_json)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                guild_id,
                channel_id,
                message_id,
                author_id,
                author.get("username"),
                date,
                text,
                reply_to,
                json.dumps(item, separators=(",", ":")),
            ),
        )
        self.conn.commit()
        return cursor.rowcount > 0

    def _write_snapshot(self, url: str, captured_at: str, content: str) -> str:
        path = _snapshot_path(url, captured_at)
        path.write_text(content, encoding="utf-8")
        return str(path)


def _as_int(value: object) -> int | None:
    try:
        return int(value) if value is not None else None
    except (TypeError, ValueError):
        return None


def _extract_title(html_text: str) -> str:
    match = _TITLE.search(html_text)
    if not match:
        return ""
    return html.unescape(" ".join(match.group(1).split()))[:300]


def _extract_excerpt(html_text: str) -> str:
    text = _TAG.sub(" ", html_text)
    text = html.unescape(" ".join(text.split()))
    return text[:1000]


def _safe_name(value: str) -> str:
    return re.sub(r"[^a-zA-Z0-9_.-]+", "_", value).strip("_")[:120] or "listing"


def _snapshot_dir() -> Path:
    path = Path.home() / ".cynic" / "organs" / "discord" / "earn-snapshots"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _now_slug(value: str) -> str:
    return value.replace(":", "").replace(".", "")


def _canonical_slug(url: str) -> str:
    return _safe_name(urlparse(url).path.strip("/"))


def _snapshot_path(url: str, captured_at: str) -> Path:
    return _snapshot_dir() / f"{_canonical_slug(url)}-{_now_slug(captured_at)}.html"


addons = [DiscordPassiveCapture()]
