"""
CYNIC DogSoul — Cross-Session Identity Memory (β2)

Each Dog maintains a SOUL.md file at:
    ~/.cynic/dogs/{dog_id}/soul.md

The file uses YAML front matter for structured stats and Markdown body
for accumulated narrative wisdom. This allows the Dog to "remember" who
it is across sessions — total judgments, average Q-Score, session count,
strengths, and watchouts.

Usage:
    soul = DogSoul.load("SCHOLAR")
    soul.update(q_score=72.5, signals=["clean code", "type hints"])
    soul.save()

Format (soul.md):
    ---
    dog_id: SCHOLAR
    total_judgments: 1247
    avg_q_score: 52.3
    session_count: 12
    last_seen: "2026-02-18T10:30:00"
    top_signals:
      - "clean code (73 hits)"
      - "type hints (41 hits)"
    ---
    # SCHOLAR Soul
    *"Mémoire sémantique persistante"*
    ...
"""
from __future__ import annotations

import logging
import re
import time
from datetime import datetime, timezone, UTC
from pathlib import Path
from typing import Dict, List, Optional

logger = logging.getLogger("cynic.soul")

# Default soul root — can be overridden in tests
_DEFAULT_SOUL_ROOT = Path.home() / ".cynic" / "dogs"

# Max top signals to track
_MAX_SIGNALS = 7


# ── YAML front matter helpers ──────────────────────────────────────────────

def _parse_front_matter(text: str) -> tuple[dict, str]:
    """
    Extract YAML-like front matter from a string.
    Returns (fields_dict, body) or ({}, text) if no front matter.
    Front matter: lines between first --- and second --- delimiters.
    Supports only flat key: value and list items (- item).
    """
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return {}, text

    end = -1
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            end = i
            break

    if end == -1:
        return {}, text

    front_lines = lines[1:end]
    body = "\n".join(lines[end + 1:])
    fields: dict = {}

    current_list_key: str | None = None
    for line in front_lines:
        # List item
        if re.match(r"^\s+-\s+", line):
            item = re.sub(r"^\s+-\s+", "", line).strip().strip('"')
            if current_list_key and isinstance(fields.get(current_list_key), list):
                fields[current_list_key].append(item)
            continue
        # Key: value
        m = re.match(r"^(\w+):\s*(.*)", line)
        if m:
            key, val = m.group(1), m.group(2).strip().strip('"')
            if val == "":
                fields[key] = []
                current_list_key = key
            else:
                # Try numeric
                try:
                    if "." in val:
                        fields[key] = float(val)
                    else:
                        fields[key] = int(val)
                except ValueError:
                    fields[key] = val
                current_list_key = None

    return fields, body


def _render_front_matter(fields: dict) -> str:
    """Render a flat dict + list fields to YAML front matter string."""
    lines = ["---"]
    for key, val in fields.items():
        if isinstance(val, list):
            lines.append(f"{key}:")
            for item in val:
                lines.append(f'  - "{item}"')
        elif isinstance(val, float):
            lines.append(f"{key}: {round(val, 2)}")
        elif isinstance(val, bool):
            lines.append(f"{key}: {str(val).lower()}")
        else:
            lines.append(f'{key}: "{val}"')
    lines.append("---")
    return "\n".join(lines)


# ── DogSoul ───────────────────────────────────────────────────────────────

class DogSoul:
    """
    Persistent cross-session identity for a Dog.

    Tracks cumulative stats (judgments, avg Q-Score) and top signals
    across sessions. Written to ~/.cynic/dogs/{dog_id}/soul.md.
    """

    def __init__(
        self,
        dog_id: str,
        total_judgments: int = 0,
        avg_q_score: float = 0.0,
        session_count: int = 0,
        top_signals: list[str] | None = None,
        last_seen: str = "",
        soul_root: Path | None = None,
    ) -> None:
        self.dog_id = dog_id.upper()
        self.total_judgments = total_judgments
        self.avg_q_score = avg_q_score
        self.session_count = session_count
        self.top_signals: list[str] = top_signals or []
        self.last_seen = last_seen
        self._soul_root = soul_root or _DEFAULT_SOUL_ROOT
        # Signal frequency counter (in-session, not persisted directly)
        self._signal_counts: dict[str, int] = {}

    # ── Path ──────────────────────────────────────────────────────────────

    @property
    def path(self) -> Path:
        return self._soul_root / self.dog_id.lower() / "soul.md"

    # ── Load / Save ───────────────────────────────────────────────────────

    @classmethod
    def load(cls, dog_id: str, soul_root: Path | None = None) -> DogSoul:
        """
        Load DogSoul from disk. Returns fresh DogSoul if file doesn't exist.
        """
        root = soul_root or _DEFAULT_SOUL_ROOT
        path = root / dog_id.lower() / "soul.md"

        if not path.exists():
            logger.debug("DogSoul: no soul file for %s, starting fresh", dog_id)
            return cls(dog_id=dog_id, soul_root=root)

        try:
            text = path.read_text(encoding="utf-8")
            fields, _ = _parse_front_matter(text)
            return cls(
                dog_id=dog_id,
                total_judgments=int(fields.get("total_judgments", 0)),
                avg_q_score=float(fields.get("avg_q_score", 0.0)),
                session_count=int(fields.get("session_count", 0)),
                top_signals=list(fields.get("top_signals", [])),
                last_seen=str(fields.get("last_seen", "")),
                soul_root=root,
            )
        except Exception as exc:
            logger.warning("DogSoul: failed to load %s: %s", path, exc)
            return cls(dog_id=dog_id, soul_root=root)

    def save(self) -> None:
        """Write soul.md to disk. Creates parent directories if needed."""
        try:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            self.last_seen = datetime.now(tz=UTC).strftime(
                "%Y-%m-%dT%H:%M:%S"
            )
            fields = {
                "dog_id": self.dog_id,
                "total_judgments": self.total_judgments,
                "avg_q_score": round(self.avg_q_score, 2),
                "session_count": self.session_count,
                "last_seen": self.last_seen,
                "top_signals": self.top_signals[:_MAX_SIGNALS],
            }
            front = _render_front_matter(fields)
            body = self._render_body()
            self.path.write_text(f"{front}\n{body}", encoding="utf-8")
            logger.debug("DogSoul: saved %s (judgments=%d)", self.dog_id, self.total_judgments)
        except Exception as exc:
            logger.warning("DogSoul: save failed for %s: %s", self.dog_id, exc)

    # ── Update ────────────────────────────────────────────────────────────

    def update(self, q_score: float, signals: list[str] | None = None) -> None:
        """
        Record one judgment result. Updates running average and signal counts.
        Call save() to persist to disk.
        """
        n = self.total_judgments
        self.avg_q_score = (self.avg_q_score * n + q_score) / (n + 1)
        self.total_judgments += 1

        if signals:
            for sig in signals:
                self._signal_counts[sig] = self._signal_counts.get(sig, 0) + 1

            # Rebuild top_signals from merged counts + existing
            self._sync_top_signals()

    def on_session_start(self) -> None:
        """Increment session counter at the start of a new session."""
        self.session_count += 1

    # ── Stats ─────────────────────────────────────────────────────────────

    def to_dict(self) -> dict:
        return {
            "dog_id": self.dog_id,
            "total_judgments": self.total_judgments,
            "avg_q_score": round(self.avg_q_score, 2),
            "session_count": self.session_count,
            "top_signals": self.top_signals[:_MAX_SIGNALS],
            "last_seen": self.last_seen,
        }

    # ── Private ───────────────────────────────────────────────────────────

    def _sync_top_signals(self) -> None:
        """Rebuild top_signals list from in-session signal_counts."""
        if not self._signal_counts:
            return
        sorted_sigs = sorted(
            self._signal_counts.items(), key=lambda x: x[1], reverse=True
        )
        self.top_signals = [
            f"{sig} ({count}×)" for sig, count in sorted_sigs[:_MAX_SIGNALS]
        ]

    def _render_body(self) -> str:
        verdict = (
            "HOWL" if self.avg_q_score >= 82 else
            "WAG" if self.avg_q_score >= 61.8 else
            "GROWL" if self.avg_q_score >= 38.2 else
            "BARK"
        )
        signals_text = ""
        if self.top_signals:
            signals_text = "\n".join(f"  - {s}" for s in self.top_signals[:_MAX_SIGNALS])
            signals_text = f"\n## Top Signals\n{signals_text}"

        return (
            f"# {self.dog_id} Soul\n\n"
            f"- Seen **{self.total_judgments}** judgments across {self.session_count} sessions\n"
            f"- Average Q-Score: **{self.avg_q_score:.1f}** ({verdict})\n"
            f"- Last seen: {self.last_seen}"
            f"{signals_text}\n"
        )
