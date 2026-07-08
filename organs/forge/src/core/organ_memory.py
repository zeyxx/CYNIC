"""Organ-forge local memory store.

This is peripheral memory: it buffers, indexes, and replays organ-local facts.
The Kernel remains the canonical organism memory after successful read-back.
"""

from __future__ import annotations

import hashlib
import json
import sqlite3
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Protocol

from organs.forge.src.adapters.kernel_task_client import KernelTaskClient
from organs.contracts.src.adapters.kernel_http import kernel_json_request


SCHEMA = "cynic.organ_forge_memory.v1"
DB_SCHEMA_VERSION = 1


@dataclass(frozen=True)
class LocalMemoryEvent:
    idempotency_key: str
    source_organ: str
    event_type: str
    storage_class: str
    observed_at: str
    payload_hash: str
    payload: dict[str, Any]


class OutboxDelivery(Protocol):
    def deliver(self, event: dict[str, Any]) -> dict[str, Any]: ...


class KernelOutboxDelivery:
    def __init__(self, base_url: str, api_key: str):
        self.base_url = base_url
        self.api_key = api_key
        self.tasks = KernelTaskClient(base_url, api_key)

    def deliver(self, event: dict[str, Any]) -> dict[str, Any]:
        destination = event.get("destination")
        payload = event.get("payload", {})
        if destination == "kernel:/observe":
            return self._deliver_observe(event, payload)
        if destination == "kernel:/agent-tasks/result":
            return self._deliver_agent_task_result(payload)
        return {
            "status": "failed",
            "error_kind": "unsupported_outbox_destination",
            "destination": destination,
        }

    def _deliver_observe(self, event: dict[str, Any], payload: dict[str, Any]) -> dict[str, Any]:
        observe_payload = {
            "tool": payload.get("tool", "organ-forge-outbox"),
            "domain": payload.get("domain", "organ_memory"),
            "status": payload.get("status", "observed"),
            "agent_id": payload.get("agent_id", "organ-forge"),
            "target": payload.get("target"),
            "context": payload.get("context") or _stable_json(payload),
            "tags": payload.get("tags", ["organ-memory", "outbox"]),
            "value": {
                "schema": "cynic.organ_forge_outbox_event.v1",
                "idempotency_key": event["idempotency_key"],
                "source_organ": event["source_organ"],
                "event_type": event["event_type"],
                "payload_hash": event["payload_hash"],
                "payload": payload,
            },
            "confidence": payload.get("confidence", "observed"),
        }
        status, text, obj = kernel_json_request(
            "POST",
            self.base_url,
            "/observe",
            self.api_key,
            payload=observe_payload,
            timeout=10,
        )
        if status in (200, 201) and isinstance(obj, dict) and obj.get("status") == "observed":
            return {"status": "ok", "read_back": obj}
        return {
            "status": "failed",
            "error_kind": "observe_delivery_failed",
            "http_status": status,
            "error": text[:500],
        }

    def _deliver_agent_task_result(self, payload: dict[str, Any]) -> dict[str, Any]:
        task_id = str(payload.get("task_id", ""))
        if not task_id:
            return {"status": "failed", "error_kind": "missing_task_id"}
        self.tasks.submit_agent_task_result(
            task_id,
            success=bool(payload.get("success")),
            result=payload.get("result") if isinstance(payload.get("result"), dict) else {},
            error=payload.get("error"),
            kind=payload.get("kind"),
            domain=payload.get("domain"),
        )
        if payload.get("kind") and payload.get("domain"):
            self.tasks.assert_agent_task_visible(
                task_id,
                kind=str(payload["kind"]),
                domain=str(payload["domain"]),
            )
        return {"status": "ok", "read_back": {"task_id": task_id, "visible": True}}


class OrganMemoryStore:
    def __init__(self, state_root: str | Path):
        self.state_root = Path(state_root)
        self.db_path = self.state_root / "db" / "organ-forge.sqlite"
        self.journal_path = self.state_root / "journal" / "events.jsonl"

    def status(self) -> dict[str, Any]:
        db_exists = self.db_path.exists()
        counts = {
            "outbox_pending": 0,
            "outbox_sent": 0,
            "journal_events": self._jsonl_line_count(self.journal_path),
            "deadletter": 0,
        }
        if db_exists:
            with sqlite3.connect(self.db_path) as conn:
                self._ensure_schema(conn)
                counts["outbox_pending"] = self._count(
                    conn,
                    "outbox_events",
                    "status = 'pending'",
                )
                counts["outbox_sent"] = self._count(
                    conn,
                    "outbox_events",
                    "status = 'sent'",
                )
                counts["deadletter"] = self._count(conn, "dead_letters")
        return {
            "schema": "cynic.organ_forge_memory_status.v1",
            "status": "ok",
            "state_root": str(self.state_root),
            "backend": {
                "sqlite": str(self.db_path),
                "sqlite_exists": db_exists,
                "journal_jsonl": str(self.journal_path),
                "journal_exists": self.journal_path.exists(),
            },
            "counts": counts,
            "authority": {
                "local_memory": "peripheral_buffer",
                "canonical_dynamic_memory": "kernel:/observe,/agent-tasks,proof_of_history",
                "canonical_desired_state": "git:infra/registry.json,axioms,manifests",
            },
        }

    def append_journal(
        self,
        *,
        event_type: str,
        payload: dict[str, Any],
        storage_class: str = "local_journal",
        source_organ: str = "organ-forge",
    ) -> LocalMemoryEvent:
        event = self._event(
            event_type=event_type,
            payload=payload,
            storage_class=storage_class,
            source_organ=source_organ,
        )
        self._ensure_parent_dirs()
        self._append_jsonl(self.journal_path, asdict(event))
        return event

    def enqueue_outbox(
        self,
        *,
        event_type: str,
        payload: dict[str, Any],
        destination: str,
        source_organ: str = "organ-forge",
    ) -> LocalMemoryEvent:
        event = self._event(
            event_type=event_type,
            payload=payload,
            storage_class="outbox",
            source_organ=source_organ,
        )
        self._ensure_parent_dirs()
        with sqlite3.connect(self.db_path) as conn:
            self._ensure_schema(conn)
            conn.execute(
                """
                INSERT OR IGNORE INTO outbox_events
                (idempotency_key, source_organ, event_type, destination, payload_hash, payload_json, status, observed_at, attempt_count)
                VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, 0)
                """,
                (
                    event.idempotency_key,
                    event.source_organ,
                    event.event_type,
                    destination,
                    event.payload_hash,
                    _stable_json(event.payload),
                    event.observed_at,
                ),
            )
        self._append_jsonl(self.journal_path, asdict(event) | {"destination": destination})
        return event

    def list_pending_outbox(self, limit: int = 50) -> list[dict[str, Any]]:
        if not self.db_path.exists():
            return []
        with sqlite3.connect(self.db_path) as conn:
            self._ensure_schema(conn)
            rows = conn.execute(
                """
                SELECT idempotency_key, source_organ, event_type, destination, payload_hash, payload_json, observed_at, attempt_count
                FROM outbox_events
                WHERE status = 'pending'
                ORDER BY observed_at ASC
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
        return [
            {
                "idempotency_key": row[0],
                "source_organ": row[1],
                "event_type": row[2],
                "destination": row[3],
                "payload_hash": row[4],
                "payload": json.loads(row[5]),
                "observed_at": row[6],
                "attempt_count": row[7],
            }
            for row in rows
        ]

    def flush_outbox(
        self,
        delivery: OutboxDelivery,
        *,
        limit: int = 25,
        dry_run: bool = False,
        max_attempts: int = 3,
    ) -> dict[str, Any]:
        pending = self.list_pending_outbox(limit=limit)
        if dry_run:
            return {
                "schema": "cynic.organ_forge_outbox_flush.v1",
                "status": "ok",
                "dry_run": True,
                "selected_count": len(pending),
                "events": [
                    {
                        "idempotency_key": item["idempotency_key"],
                        "destination": item["destination"],
                        "event_type": item["event_type"],
                        "attempt_count": item["attempt_count"],
                    }
                    for item in pending
                ],
            }

        delivered: list[dict[str, Any]] = []
        failed: list[dict[str, Any]] = []
        deadlettered: list[dict[str, Any]] = []
        for event in pending:
            try:
                result = delivery.deliver(event)
            except Exception as exc:
                result = {"status": "failed", "error": str(exc), "error_kind": "delivery_exception"}
            if result.get("status") == "ok":
                self.mark_outbox_sent(event["idempotency_key"], result)
                delivered.append(
                    {
                        "idempotency_key": event["idempotency_key"],
                        "destination": event["destination"],
                        "result": result,
                    }
                )
                continue
            failure = {
                "idempotency_key": event["idempotency_key"],
                "destination": event["destination"],
                "result": result,
            }
            if int(event.get("attempt_count", 0)) + 1 >= max_attempts:
                self.move_to_deadletter(event, _stable_json(result))
                deadlettered.append(failure)
            else:
                self.mark_outbox_failed(event["idempotency_key"], _stable_json(result))
                failed.append(failure)
        status = "ok" if not failed and not deadlettered else "degraded"
        return {
            "schema": "cynic.organ_forge_outbox_flush.v1",
            "status": status,
            "dry_run": False,
            "selected_count": len(pending),
            "delivered_count": len(delivered),
            "failed_count": len(failed),
            "deadlettered_count": len(deadlettered),
            "delivered": delivered,
            "failed": failed,
            "deadlettered": deadlettered,
        }

    def mark_outbox_sent(self, idempotency_key: str, result: dict[str, Any]) -> None:
        with sqlite3.connect(self.db_path) as conn:
            self._ensure_schema(conn)
            conn.execute(
                """
                UPDATE outbox_events
                SET status = 'sent', last_error = NULL
                WHERE idempotency_key = ?
                """,
                (idempotency_key,),
            )
        self._append_jsonl(
            self.journal_path,
            {
                "schema": "cynic.organ_forge_outbox_delivery.v1",
                "status": "sent",
                "idempotency_key": idempotency_key,
                "delivered_at": datetime.now(timezone.utc).isoformat(),
                "result": result,
            },
        )

    def mark_outbox_failed(self, idempotency_key: str, error: str) -> None:
        with sqlite3.connect(self.db_path) as conn:
            self._ensure_schema(conn)
            conn.execute(
                """
                UPDATE outbox_events
                SET attempt_count = attempt_count + 1, last_error = ?
                WHERE idempotency_key = ?
                """,
                (error, idempotency_key),
            )

    def move_to_deadletter(self, event: dict[str, Any], error: str) -> None:
        with sqlite3.connect(self.db_path) as conn:
            self._ensure_schema(conn)
            conn.execute(
                """
                INSERT OR REPLACE INTO dead_letters
                (idempotency_key, source_organ, event_type, payload_hash, payload_json, error, observed_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    event["idempotency_key"],
                    event["source_organ"],
                    event["event_type"],
                    event["payload_hash"],
                    _stable_json(event["payload"]),
                    error,
                    datetime.now(timezone.utc).isoformat(),
                ),
            )
            conn.execute(
                "UPDATE outbox_events SET status = 'deadletter', last_error = ? WHERE idempotency_key = ?",
                (error, event["idempotency_key"]),
            )

    def _event(
        self,
        *,
        event_type: str,
        payload: dict[str, Any],
        storage_class: str,
        source_organ: str,
    ) -> LocalMemoryEvent:
        payload_hash = _sha256(_stable_json(payload))
        idempotency_key = _sha256(
            _stable_json(
                {
                    "source_organ": source_organ,
                    "event_type": event_type,
                    "storage_class": storage_class,
                    "payload_hash": payload_hash,
                }
            )
        )
        return LocalMemoryEvent(
            idempotency_key=idempotency_key,
            source_organ=source_organ,
            event_type=event_type,
            storage_class=storage_class,
            observed_at=datetime.now(timezone.utc).isoformat(),
            payload_hash=payload_hash,
            payload=payload,
        )

    def _ensure_parent_dirs(self) -> None:
        (self.state_root / "db").mkdir(parents=True, exist_ok=True)
        (self.state_root / "journal").mkdir(parents=True, exist_ok=True)

    def _ensure_schema(self, conn: sqlite3.Connection) -> None:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              applied_at TEXT NOT NULL
            )
            """
        )
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS outbox_events (
              idempotency_key TEXT PRIMARY KEY,
              source_organ TEXT NOT NULL,
              event_type TEXT NOT NULL,
              destination TEXT NOT NULL,
              payload_hash TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              status TEXT NOT NULL,
              observed_at TEXT NOT NULL,
              attempt_count INTEGER NOT NULL DEFAULT 0,
              last_error TEXT
            )
            """
        )
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS dead_letters (
              idempotency_key TEXT PRIMARY KEY,
              source_organ TEXT NOT NULL,
              event_type TEXT NOT NULL,
              payload_hash TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              error TEXT NOT NULL,
              observed_at TEXT NOT NULL
            )
            """
        )
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, ?)",
            (DB_SCHEMA_VERSION, datetime.now(timezone.utc).isoformat()),
        )

    @staticmethod
    def _append_jsonl(path: Path, value: dict[str, Any]) -> None:
        with path.open("a", encoding="utf-8") as handle:
            handle.write(_stable_json(value) + "\n")

    @staticmethod
    def _jsonl_line_count(path: Path) -> int:
        if not path.exists():
            return 0
        with path.open(encoding="utf-8") as handle:
            return sum(1 for line in handle if line.strip())

    @staticmethod
    def _count(conn: sqlite3.Connection, table: str, where: str | None = None) -> int:
        query = f"SELECT COUNT(*) FROM {table}"
        if where:
            query = f"{query} WHERE {where}"
        return int(conn.execute(query).fetchone()[0])


def _stable_json(value: dict[str, Any]) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _sha256(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()
