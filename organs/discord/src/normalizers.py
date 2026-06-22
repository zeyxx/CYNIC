"""Normalizers for Discord passive-capture payloads."""
from __future__ import annotations

import hashlib
import json
import sqlite3
from datetime import datetime, timezone
from typing import Any


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def json_dumps(value: Any) -> str:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=True)


def hash_body(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def as_int(value: object) -> int | None:
    try:
        return int(value) if value is not None else None
    except (TypeError, ValueError):
        return None


def normalize_payload(conn: sqlite3.Connection, endpoint_type: str, payload: Any) -> None:
    if endpoint_type == "guild_channels" and isinstance(payload, list):
        for item in payload:
            if isinstance(item, dict):
                upsert_channel(conn, item)
        return

    if endpoint_type == "channel" and isinstance(payload, dict):
        upsert_channel(conn, payload)
        return

    if endpoint_type == "guild" and isinstance(payload, dict):
        upsert_server(conn, payload)
        return

    if endpoint_type == "threads":
        normalize_threads(conn, payload)
        return

    if endpoint_type == "relationships" and isinstance(payload, list):
        for item in payload:
            if isinstance(item, dict):
                upsert_relationship(conn, item)
        return

    if endpoint_type == "user" and isinstance(payload, dict):
        upsert_identity(conn, payload)


def upsert_identity(conn: sqlite3.Connection, item: dict) -> None:
    user_id = as_int(item.get("id"))
    if user_id is None:
        return
    now = now_utc()
    conn.execute(
        """INSERT INTO discord_identities
           (user_id, username, global_name, discriminator, bot, first_seen, last_seen, raw_json)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(user_id) DO UPDATE SET
             username=excluded.username,
             global_name=excluded.global_name,
             discriminator=excluded.discriminator,
             bot=excluded.bot,
             last_seen=excluded.last_seen,
             raw_json=excluded.raw_json""",
        (
            user_id,
            item.get("username"),
            item.get("global_name"),
            item.get("discriminator"),
            1 if item.get("bot") else 0,
            now,
            now,
            json_dumps(item),
        ),
    )


def upsert_server(conn: sqlite3.Connection, item: dict) -> None:
    guild_id = as_int(item.get("id") or item.get("guild_id"))
    if guild_id is None:
        return
    now = now_utc()
    conn.execute(
        """INSERT INTO discord_servers
           (guild_id, name, owner_id, first_seen, last_seen, raw_json)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(guild_id) DO UPDATE SET
             name=excluded.name,
             owner_id=excluded.owner_id,
             last_seen=excluded.last_seen,
             raw_json=excluded.raw_json""",
        (guild_id, item.get("name"), as_int(item.get("owner_id")), now, now, json_dumps(item)),
    )


def upsert_channel(conn: sqlite3.Connection, item: dict) -> None:
    channel_id = as_int(item.get("id"))
    if channel_id is None:
        return
    now = now_utc()
    conn.execute(
        """INSERT INTO discord_channels
           (channel_id, guild_id, parent_id, name, channel_type, topic, first_seen, last_seen, raw_json)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(channel_id) DO UPDATE SET
             guild_id=excluded.guild_id,
             parent_id=excluded.parent_id,
             name=excluded.name,
             channel_type=excluded.channel_type,
             topic=excluded.topic,
             last_seen=excluded.last_seen,
             raw_json=excluded.raw_json""",
        (
            channel_id,
            as_int(item.get("guild_id")),
            as_int(item.get("parent_id")),
            item.get("name"),
            as_int(item.get("type")),
            item.get("topic"),
            now,
            now,
            json_dumps(item),
        ),
    )


def normalize_threads(conn: sqlite3.Connection, payload: Any) -> None:
    threads: list[dict] = []
    if isinstance(payload, dict):
        raw_threads = payload.get("threads", [])
        if isinstance(raw_threads, list):
            threads = [item for item in raw_threads if isinstance(item, dict)]
    elif isinstance(payload, list):
        threads = [item for item in payload if isinstance(item, dict)]

    for item in threads:
        upsert_thread(conn, item)


def upsert_thread(conn: sqlite3.Connection, item: dict) -> None:
    thread_id = as_int(item.get("id"))
    if thread_id is None:
        return
    now = now_utc()
    metadata = item.get("thread_metadata") if isinstance(item.get("thread_metadata"), dict) else {}
    conn.execute(
        """INSERT INTO discord_threads
           (thread_id, channel_id, guild_id, parent_id, name, archived, first_seen, last_seen, raw_json)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(thread_id) DO UPDATE SET
             channel_id=excluded.channel_id,
             guild_id=excluded.guild_id,
             parent_id=excluded.parent_id,
             name=excluded.name,
             archived=excluded.archived,
             last_seen=excluded.last_seen,
             raw_json=excluded.raw_json""",
        (
            thread_id,
            as_int(item.get("id")),
            as_int(item.get("guild_id")),
            as_int(item.get("parent_id")),
            item.get("name"),
            1 if metadata.get("archived") else 0,
            now,
            now,
            json_dumps(item),
        ),
    )


def upsert_relationship(conn: sqlite3.Connection, item: dict) -> None:
    user = item.get("user") if isinstance(item.get("user"), dict) else item
    user_id = as_int(user.get("id"))
    if user_id is None:
        return
    upsert_identity(conn, user)
    now = now_utc()
    conn.execute(
        """INSERT INTO discord_relationships
           (user_id, relation_type, username, first_seen, last_seen, raw_json)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(user_id) DO UPDATE SET
             relation_type=excluded.relation_type,
             username=excluded.username,
             last_seen=excluded.last_seen,
             raw_json=excluded.raw_json""",
        (
            user_id,
            str(item.get("type", "unknown")),
            user.get("username"),
            now,
            now,
            json_dumps(item),
        ),
    )


def extract_message_entities(conn: sqlite3.Connection, channel_id: int, message_id: int, item: dict) -> None:
    for user in item.get("mentions", []) if isinstance(item.get("mentions"), list) else []:
        if not isinstance(user, dict):
            continue
        upsert_identity(conn, user)
        user_id = as_int(user.get("id"))
        if user_id is not None:
            insert_message_entity(conn, channel_id, message_id, "mention_user", str(user_id), user.get("username"), user)

    for attachment in item.get("attachments", []) if isinstance(item.get("attachments"), list) else []:
        if isinstance(attachment, dict) and attachment.get("id"):
            insert_message_entity(conn, channel_id, message_id, "attachment", str(attachment["id"]), attachment.get("filename"), attachment)

    for embed in item.get("embeds", []) if isinstance(item.get("embeds"), list) else []:
        if isinstance(embed, dict):
            entity_id = embed.get("url") or embed.get("title") or hash_body(json_dumps(embed))
            insert_message_entity(conn, channel_id, message_id, "embed", str(entity_id), embed.get("title"), embed)


def insert_message_entity(
    conn: sqlite3.Connection,
    channel_id: int,
    message_id: int,
    entity_type: str,
    entity_id: str,
    label: str | None,
    raw: dict,
) -> None:
    conn.execute(
        """INSERT OR IGNORE INTO discord_message_entities
           (channel_id, message_id, entity_type, entity_id, label, raw_json)
           VALUES (?, ?, ?, ?, ?, ?)""",
        (channel_id, message_id, entity_type, entity_id, label, json_dumps(raw)),
    )


def insert_raw_event(
    conn: sqlite3.Connection,
    *,
    event_id: str,
    captured_at: str,
    host: str,
    method: str,
    path: str,
    endpoint_type: str,
    policy: str,
    status_code: int,
    content_type: str,
    request_url: str,
    body_hash: str | None,
    raw_json: str | None,
) -> None:
    conn.execute(
        """INSERT OR IGNORE INTO raw_events
           (event_id, captured_at, host, method, path, endpoint_type, policy,
            status_code, content_type, request_url, body_hash, raw_json)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
        (
            event_id,
            captured_at,
            host,
            method,
            path,
            endpoint_type,
            policy,
            status_code,
            content_type,
            request_url,
            body_hash,
            raw_json,
        ),
    )
