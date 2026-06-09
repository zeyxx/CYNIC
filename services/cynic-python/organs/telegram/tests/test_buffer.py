"""Tests for message buffer — aggregation by author+time+reply."""
from datetime import datetime, timezone
from typing import Optional

from organs.telegram.buffer import MessageBuffer, RawMessage


def _msg(
    channel_id: int = 1,
    message_id: int = 1,
    author_id: int = 100,
    author_name: str = "trader",
    text: str = "hello",
    date: Optional[datetime] = None,
    reply_to: Optional[int] = None,
) -> RawMessage:
    """Helper to build a RawMessage."""
    return RawMessage(
        channel_id=channel_id,
        message_id=message_id,
        author_id=author_id,
        author_name=author_name,
        text=text,
        date=date or datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc),
        reply_to=reply_to,
        media_type=None,
        media_path=None,
        forward_from=None,
        forward_msg_id=None,
        raw_json="{}",
    )


def test_single_message_flush() -> None:
    """A single message flushes after the window expires."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 3, tzinfo=timezone.utc))
    assert len(blocks) == 0
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 6, tzinfo=timezone.utc))
    assert len(blocks) == 1
    assert len(blocks[0].messages) == 1


def test_same_author_aggregates() -> None:
    """Messages from same author+channel within window form one block."""
    buf = MessageBuffer(window_seconds=60)
    buf.add(_msg(message_id=1, date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=2, date=datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=3, date=datetime(2026, 5, 20, 12, 0, 20, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 1, 30, tzinfo=timezone.utc))
    assert len(blocks) == 1
    assert len(blocks[0].messages) == 3


def test_different_authors_separate_blocks() -> None:
    """Messages from different authors in same channel are separate blocks."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(message_id=1, author_id=100, date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=2, author_id=200, date=datetime(2026, 5, 20, 12, 0, 1, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc))
    assert len(blocks) == 2


def test_reply_merges_into_parent_block() -> None:
    """A reply from author B merges into author A's block."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(message_id=1, author_id=100, date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=2, author_id=200, reply_to=1, date=datetime(2026, 5, 20, 12, 0, 2, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc))
    assert len(blocks) == 1
    assert len(blocks[0].messages) == 2


def test_block_id_format() -> None:
    """block_id = {channel_id}_{author_id}_{first_msg_iso8601}."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(channel_id=42, author_id=100, message_id=1,
                 date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc))
    assert blocks[0].block_id == "42_100_2026-05-20T12:00:00+00:00"


def test_flush_all_drains_buffer() -> None:
    """flush_all returns all blocks regardless of window expiry."""
    buf = MessageBuffer(window_seconds=60)
    buf.add(_msg(date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    blocks = buf.flush_all()
    assert len(blocks) == 1
    assert len(buf.flush_all()) == 0
