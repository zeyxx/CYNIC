"""
Telegram Listener Daemon — captures channel messages to SQLite.

Modes:
    --listen    Real-time daemon (systemd)
    --backfill  One-shot historical export
    --auth      Interactive Telethon authentication

Environment:
    CYNIC_REST_ADDR, CYNIC_API_KEY — kernel connection
    TELEGRAM_API_ID, TELEGRAM_API_HASH — Telegram API credentials
"""
from __future__ import annotations

import argparse
import asyncio
import json
import logging
import signal
import sqlite3
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional, TYPE_CHECKING

import requests

from . import __version__
from .config import (
    ensure_runtime_dirs,
    load_config,
    TelegramConfig,
    validate_telegram_credentials,
)
from .schema import bootstrap_db
from .buffer import MessageBuffer, RawMessage, Block

if TYPE_CHECKING:
    from telethon import TelegramClient

logger = logging.getLogger("telegram.listener")

_DEFAULT_CONFIG = Path.home() / ".cynic" / "organs" / "telegram" / "config.yaml"


# ── Heartbeat ──


def post_heartbeat(
    kernel_url: str,
    api_key: str,
    channels_active: int,
    messages_count: int,
    blocks_count: int,
    media_count: int,
) -> None:
    """POST heartbeat observation to kernel. Fire-and-forget."""
    if not kernel_url:
        return
    body = {
        "tool": "telegram_listener",
        "target": "heartbeat",
        "domain": "telegram",
        "agent_id": "hermes-telegram",
        "value": {
            "channels_active": channels_active,
            "messages_last_5min": messages_count,
            "blocks_last_5min": blocks_count,
            "media_downloaded": media_count,
        },
        "context": (
            f"{channels_active} channels active, {messages_count} messages, "
            f"{blocks_count} blocks, {media_count} media in last 5min"
        ),
        "tags": ["heartbeat", "telegram"],
    }
    try:
        resp = requests.post(
            f"{kernel_url}/observe",
            headers={"Authorization": f"Bearer {api_key}",
                     "Content-Type": "application/json"},
            data=json.dumps(body),
            timeout=10,
        )
        if resp.status_code != 200:
            logger.warning("heartbeat POST returned %d", resp.status_code)
    except Exception:
        logger.warning("heartbeat POST failed", exc_info=True)


# ── SQLite persistence ──


def store_block(conn: sqlite3.Connection, block: Block, media_dir: str) -> int:
    """Write a block's messages to SQLite. Returns count of rows inserted."""
    inserted = 0
    for msg in block.messages:
        try:
            conn.execute(
                """INSERT OR IGNORE INTO messages
                   (channel_id, message_id, author_id, author_name, date,
                    text, media_type, media_path, reply_to, forward_from,
                    forward_msg_id, block_id, raw_json)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    msg.channel_id, msg.message_id, msg.author_id,
                    msg.author_name, msg.date.isoformat(), msg.text,
                    msg.media_type, msg.media_path,
                    msg.reply_to, msg.forward_from, msg.forward_msg_id,
                    block.block_id, msg.raw_json,
                ),
            )
            inserted += 1
        except Exception:
            logger.error("failed to insert msg %d/%d",
                         msg.channel_id, msg.message_id, exc_info=True)
    conn.commit()
    return inserted


def upsert_channel(
    conn: sqlite3.Connection,
    channel_id: int,
    name: str,
    username: Optional[str],
    channel_type: str,
) -> None:
    """Insert or update a channel in the channels table."""
    conn.execute(
        """INSERT INTO channels (channel_id, name, username, channel_type, added_at)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT(channel_id) DO UPDATE SET
             name=excluded.name, username=excluded.username""",
        (channel_id, name, username, channel_type,
         datetime.now(timezone.utc).isoformat()),
    )
    conn.commit()


# ── Retention cleanup ──


def cleanup_expired(
    conn: sqlite3.Connection,
    media_dir: str,
    raw_days: int,
    media_days: int,
) -> tuple[int, int]:
    """Purge expired raw messages and media. Returns (rows_deleted, files_deleted)."""
    cursor = conn.execute(
        "DELETE FROM messages WHERE date < datetime('now', ?)",
        (f"-{raw_days} days",),
    )
    rows_deleted = cursor.rowcount

    files_deleted = 0
    if media_days > 0:
        old_media = conn.execute(
            "SELECT media_path FROM messages WHERE media_path IS NOT NULL "
            "AND date < datetime('now', ?)",
            (f"-{media_days} days",),
        ).fetchall()
        for (mpath,) in old_media:
            full = Path(media_dir) / mpath
            if full.exists():
                full.unlink()
                files_deleted += 1
        conn.execute(
            "UPDATE messages SET media_path = NULL, media_type = NULL "
            "WHERE date < datetime('now', ?)",
            (f"-{media_days} days",),
        )

    conn.commit()
    if rows_deleted or files_deleted:
        logger.info("retention cleanup: %d rows, %d media files purged",
                     rows_deleted, files_deleted)
    return rows_deleted, files_deleted


# ── Media download ──


async def download_media(
    client: Any,
    message: Any,
    media_dir: str,
) -> Optional[str]:
    """Download message media to local filesystem. Returns relative path or None."""
    if message.media is None:
        return None
    try:
        channel_dir = Path(media_dir) / str(message.chat_id)
        channel_dir.mkdir(parents=True, exist_ok=True)
        path = await client.download_media(
            message, file=str(channel_dir / str(message.id))
        )
        if path:
            return str(Path(path).relative_to(media_dir))
    except Exception:
        logger.warning("media download failed for %d/%d",
                       message.chat_id, message.id, exc_info=True)
    return None


# ── Telethon → RawMessage ──


def telethon_to_raw(message: Any) -> RawMessage:
    """Convert a Telethon Message object to RawMessage."""
    author_id: Optional[int] = None
    author_name: Optional[str] = None
    if message.sender:
        author_id = message.sender_id
        first = getattr(message.sender, "first_name", "") or ""
        last = getattr(message.sender, "last_name", "") or ""
        author_name = (f"{first} {last}".strip()
                       or getattr(message.sender, "title", None))

    forward_from: Optional[int] = None
    forward_msg_id: Optional[int] = None
    if message.forward:
        if message.forward.chat_id:
            forward_from = message.forward.chat_id
        if message.forward.channel_post:
            forward_msg_id = message.forward.channel_post

    media_type: Optional[str] = None
    if message.photo:
        media_type = "photo"
    elif message.document:
        media_type = "document"
    elif message.video:
        media_type = "video"

    return RawMessage(
        channel_id=message.chat_id,
        message_id=message.id,
        author_id=author_id,
        author_name=author_name,
        text=message.text or message.raw_text,
        date=message.date,
        reply_to=message.reply_to_msg_id if message.reply_to else None,
        media_type=media_type,
        media_path=None,
        forward_from=forward_from,
        forward_msg_id=forward_msg_id,
        raw_json=message.to_json(),
    )


# ── Main daemon loop ──


async def listen_loop(cfg: TelegramConfig) -> None:
    """Main listen loop — connect, buffer, flush to SQLite."""
    from telethon import TelegramClient, events
    from telethon.errors import FloodWaitError

    client = TelegramClient(
        cfg.session_path, cfg.telegram_api_id, cfg.telegram_api_hash
    )
    await client.start()
    logger.info("telegram-listener v%s connected", __version__)

    conn = bootstrap_db(cfg.db_path)
    buffer = MessageBuffer(window_seconds=cfg.buffer_window_seconds)
    Path(cfg.media_dir).mkdir(parents=True, exist_ok=True)

    async for dialog in client.iter_dialogs():
        if dialog.is_channel or dialog.is_group or dialog.is_user:
            if dialog.is_user:
                ch_type = "private"
            else:
                ch_type = "channel" if dialog.is_channel else "group"
            upsert_channel(
                conn, dialog.id, dialog.name,
                getattr(dialog.entity, "username", None), ch_type,
            )
    logger.info("channels/users registered")

    stats = {"messages": 0, "blocks": 0, "media": 0}
    last_heartbeat = time.monotonic()

    @client.on(events.NewMessage)
    async def on_new_message(event: events.NewMessage.Event) -> None:
        msg = event.message
        raw = telethon_to_raw(msg)

        if raw.media_type:
            try:
                media_path = await download_media(client, msg, cfg.media_dir)
                raw.media_path = media_path
                if media_path:
                    stats["media"] += 1
            except FloodWaitError as e:
                logger.warning("FloodWait on media download: %ds", e.seconds)
                await asyncio.sleep(e.seconds)

        buffer.add(raw)
        stats["messages"] += 1

        if msg.chat:
            if msg.is_private:
                ch_type = "private"
                first = getattr(msg.chat, "first_name", "") or ""
                last = getattr(msg.chat, "last_name", "") or ""
                title = f"{first} {last}".strip()
            else:
                ch_type = "channel" if getattr(msg.chat, "broadcast", False) else "group"
                title = getattr(msg.chat, "title", "")

            upsert_channel(
                conn, msg.chat_id,
                title,
                getattr(msg.chat, "username", None), ch_type,
            )

    async def periodic_flush() -> None:
        nonlocal last_heartbeat
        while True:
            await asyncio.sleep(5)
            now = datetime.now(timezone.utc)
            blocks = buffer.flush_ready(now)
            for block in blocks:
                store_block(conn, block, cfg.media_dir)
                stats["blocks"] += 1

            elapsed = time.monotonic() - last_heartbeat
            if elapsed >= cfg.heartbeat_interval_seconds:
                dialogs_count = 0
                try:
                    async for d in client.iter_dialogs():
                        if d.is_channel or d.is_group or d.is_user:
                            dialogs_count += 1
                except FloodWaitError as e:
                    logger.warning("FloodWait on iter_dialogs: %ds", e.seconds)
                    await asyncio.sleep(e.seconds)

                post_heartbeat(
                    kernel_url=cfg.kernel_url,
                    api_key=cfg.api_key,
                    channels_active=dialogs_count,
                    messages_count=stats["messages"],
                    blocks_count=stats["blocks"],
                    media_count=stats["media"],
                )
                stats["messages"] = 0
                stats["blocks"] = 0
                stats["media"] = 0
                last_heartbeat = time.monotonic()

                cleanup_expired(
                    conn, cfg.media_dir,
                    cfg.retention.raw_days, cfg.retention.media_days,
                )

    loop = asyncio.get_running_loop()
    shutdown_event = asyncio.Event()

    def handle_signal() -> None:
        logger.info("shutdown signal received, flushing buffers...")
        shutdown_event.set()

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, handle_signal)

    flush_task = asyncio.create_task(periodic_flush())

    await shutdown_event.wait()
    flush_task.cancel()

    for block in buffer.flush_all():
        store_block(conn, block, cfg.media_dir)
    conn.close()
    await client.disconnect()
    logger.info("telegram-listener stopped cleanly")


# ── Backfill ──


async def backfill(cfg: TelegramConfig, channel: str, limit: int) -> None:
    """One-shot historical export from a channel."""
    from telethon import TelegramClient
    from telethon.errors import FloodWaitError

    client = TelegramClient(
        cfg.session_path, cfg.telegram_api_id, cfg.telegram_api_hash
    )
    await client.start()

    conn = bootstrap_db(cfg.db_path)
    Path(cfg.media_dir).mkdir(parents=True, exist_ok=True)

    # Telethon needs int for numeric IDs, str for @usernames
    target = int(channel) if channel.lstrip("-").isdigit() else channel
    entity = await client.get_entity(target)
    ch_type = "channel" if getattr(entity, "broadcast", False) else "group"
    upsert_channel(
        conn, entity.id,
        getattr(entity, "title", channel),
        getattr(entity, "username", None), ch_type,
    )

    count = 0
    buf = MessageBuffer(window_seconds=cfg.buffer_window_seconds)
    try:
        async for message in client.iter_messages(entity, limit=limit):
            raw = telethon_to_raw(message)
            buf.add(raw)
            count += 1
            if count % 100 == 0:
                logger.info("backfill: %d messages processed", count)
    except FloodWaitError as e:
        logger.warning("FloodWait during backfill: %ds, flushing %d msgs",
                       e.seconds, count)

    for block in buf.flush_all():
        store_block(conn, block, cfg.media_dir)

    conn.close()
    await client.disconnect()
    logger.info("backfill complete: %d messages from %s", count, channel)


# ── Auth ──


async def auth(cfg: TelegramConfig) -> None:
    """Interactive Telethon authentication."""
    from telethon import TelegramClient

    client = TelegramClient(
        cfg.session_path, cfg.telegram_api_id, cfg.telegram_api_hash
    )
    await client.start()
    me = await client.get_me()
    logger.info("authenticated as %s (id=%d)", me.first_name, me.id)
    await client.disconnect()


# ── CLI ──


def main() -> None:
    """CLI entry point."""
    logging.basicConfig(
        level=logging.INFO,
        format='{"time":"%(asctime)s","name":"%(name)s","level":"%(levelname)s","msg":"%(message)s"}',
    )
    logger.info("telegram-listener v%s starting", __version__)

    parser = argparse.ArgumentParser(description="CYNIC Telegram Listener")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--listen", action="store_true",
                       help="Real-time daemon (default)")
    group.add_argument("--backfill", type=str, metavar="CHANNEL",
                       help="Backfill channel history")
    group.add_argument("--auth", action="store_true",
                       help="Interactive Telethon auth")
    parser.add_argument("--limit", type=int, default=5000,
                        help="Max messages for backfill")
    parser.add_argument("--config", type=str, default=str(_DEFAULT_CONFIG),
                        help="Config file path")
    args = parser.parse_args()

    cfg = load_config(args.config)
    try:
        validate_telegram_credentials(cfg)
    except ValueError as exc:
        raise SystemExit(str(exc)) from exc
    ensure_runtime_dirs(cfg)

    if args.auth:
        asyncio.run(auth(cfg))
    elif args.backfill:
        asyncio.run(backfill(cfg, args.backfill, args.limit))
    else:
        asyncio.run(listen_loop(cfg))


if __name__ == "__main__":
    main()
