"""Persistent storage for dialogue history."""

from __future__ import annotations

import sqlite3
import asyncio
import json
from pathlib import Path
from typing import Optional, Any
from dataclasses import asdict
from datetime import datetime

from cynic.brain.dialogue.models import UserMessage, CynicMessage, DialogueMessage


class DialogueStore:
    """SQLite-backed dialogue storage with async interface."""

    def __init__(self, db_path: Path):
        """Initialize dialogue store with database path.

        Args:
            db_path: Path to SQLite database file.
        """
        self.db_path = db_path
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._connection: Optional[sqlite3.Connection] = None

    async def initialize(self) -> None:
        """Create tables and initialize database."""
        def _init():
            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS dialogue_messages (
                    id INTEGER PRIMARY KEY,
                    timestamp REAL NOT NULL,
                    is_user BOOLEAN NOT NULL,
                    message_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    confidence REAL,
                    axiom_scores TEXT,
                    related_judgment_id TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
            """)
            conn.commit()
            conn.close()

        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, _init)

    async def save_message(self, message: DialogueMessage) -> int:
        """Save message and return ID.

        Args:
            message: UserMessage or CynicMessage to persist.

        Returns:
            ID of inserted message row.
        """
        def _save():
            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()

            is_user = isinstance(message, UserMessage)
            axiom_scores = None
            if isinstance(message, CynicMessage) and message.axiom_scores:
                axiom_scores = json.dumps(dict(message.axiom_scores))

            # Get related_judgment_id from the message
            if is_user:
                related_id = message.related_judgment_id
            else:
                related_id = message.source_judgment_id

            cursor.execute("""
                INSERT INTO dialogue_messages
                (timestamp, is_user, message_type, content, confidence,
                 axiom_scores, related_judgment_id)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            """, (
                message.timestamp,
                is_user,
                message.message_type,
                message.content,
                message.user_confidence if is_user else message.confidence,
                axiom_scores,
                related_id
            ))
            conn.commit()
            msg_id = cursor.lastrowid
            conn.close()
            return msg_id

        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _save)

    async def get_last_n_messages(self, n: int = 100) -> list[dict[str, Any]]:
        """Get last N messages.

        Args:
            n: Number of messages to retrieve.

        Returns:
            List of message dictionaries in chronological order.
        """
        def _get():
            conn = sqlite3.connect(self.db_path)
            conn.row_factory = sqlite3.Row
            cursor = conn.cursor()
            cursor.execute("""
                SELECT * FROM dialogue_messages
                ORDER BY timestamp DESC
                LIMIT ?
            """, (n,))
            rows = [dict(row) for row in cursor.fetchall()]
            conn.close()
            return list(reversed(rows))  # Return in chronological order

        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _get)

    async def get_conversation_context(self, judgment_id: str,
                                       context_size: int = 10) -> list[dict[str, Any]]:
        """Get messages related to a specific judgment.

        Args:
            judgment_id: ID of the judgment to find context for.
            context_size: Maximum number of messages to retrieve (unused but available for future use).

        Returns:
            List of messages related to the judgment in chronological order.
        """
        def _get():
            conn = sqlite3.connect(self.db_path)
            conn.row_factory = sqlite3.Row
            cursor = conn.cursor()
            cursor.execute("""
                SELECT * FROM dialogue_messages
                WHERE related_judgment_id = ?
                ORDER BY timestamp ASC
            """, (judgment_id,))
            return [dict(row) for row in cursor.fetchall()]

        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _get)

    async def close(self) -> None:
        """Close database connection."""
        if self._connection:
            self._connection.close()


# Global dialogue store instance
_dialogue_store: Optional[DialogueStore] = None


async def get_dialogue_store() -> DialogueStore:
    """Get or create global dialogue store.

    Returns:
        Singleton DialogueStore instance initialized with default path.
    """
    global _dialogue_store
    if _dialogue_store is None:
        from pathlib import Path
        store_path = Path.home() / ".cynic" / "phase2" / "dialogue_history.db"
        _dialogue_store = DialogueStore(store_path)
        await _dialogue_store.initialize()
    return _dialogue_store
