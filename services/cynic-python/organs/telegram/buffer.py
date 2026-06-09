"""Message buffer — aggregation by author + time window + reply chain."""
import logging
from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional

logger = logging.getLogger("telegram.buffer")


@dataclass
class RawMessage:
    """A single Telegram message, ready for buffering."""

    channel_id: int
    message_id: int
    author_id: Optional[int]
    author_name: Optional[str]
    text: Optional[str]
    date: datetime
    reply_to: Optional[int]
    media_type: Optional[str]
    media_path: Optional[str]
    forward_from: Optional[int]
    forward_msg_id: Optional[int]
    raw_json: str


@dataclass
class Block:
    """A logical group of messages sharing a block_id."""

    channel_id: int
    originator_author_id: Optional[int]
    first_date: datetime
    messages: list[RawMessage] = field(default_factory=list)

    @property
    def block_id(self) -> str:
        return f"{self.channel_id}_{self.originator_author_id}_{self.first_date.isoformat()}"

    @property
    def last_date(self) -> datetime:
        return max(m.date for m in self.messages)


_BufKey = tuple[int, Optional[int]]


class MessageBuffer:
    """Aggregates messages into blocks by author+time window+reply chain.

    Flush condition: no new message from same (channel, author) for
    window_seconds. Reply messages merge into the parent block
    regardless of author or time window.
    """

    def __init__(self, window_seconds: int = 60) -> None:
        self.window_seconds = window_seconds
        self._blocks: dict[_BufKey, Block] = {}
        self._msg_to_key: dict[tuple[int, int], _BufKey] = {}

    def add(self, msg: RawMessage) -> None:
        """Add a message to the buffer."""
        if msg.reply_to is not None:
            parent_lookup = (msg.channel_id, msg.reply_to)
            if parent_lookup in self._msg_to_key:
                key = self._msg_to_key[parent_lookup]
                self._blocks[key].messages.append(msg)
                self._msg_to_key[(msg.channel_id, msg.message_id)] = key
                return

        key: _BufKey = (msg.channel_id, msg.author_id)
        if key not in self._blocks:
            self._blocks[key] = Block(
                channel_id=msg.channel_id,
                originator_author_id=msg.author_id,
                first_date=msg.date,
            )
        self._blocks[key].messages.append(msg)
        self._msg_to_key[(msg.channel_id, msg.message_id)] = key

    def flush_ready(self, now: datetime) -> list[Block]:
        """Return blocks where the last message is older than window_seconds."""
        ready: list[Block] = []
        to_remove: list[_BufKey] = []
        for key, block in self._blocks.items():
            elapsed = (now - block.last_date).total_seconds()
            if elapsed >= self.window_seconds:
                ready.append(block)
                to_remove.append(key)
        for key in to_remove:
            del self._blocks[key]
            self._msg_to_key = {
                k: v for k, v in self._msg_to_key.items() if v != key
            }
        return ready

    def flush_all(self) -> list[Block]:
        """Flush all blocks regardless of window expiry (for shutdown)."""
        blocks = list(self._blocks.values())
        self._blocks.clear()
        self._msg_to_key.clear()
        return blocks
