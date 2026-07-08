"""
Convergence detector — groups X organ observations by cashtag,
emits ConvergenceSignal when 3+ distinct authors mention the same item.

Tier 2 INFRASTRUCTURE: consumed by convergence consumer (kernel) via /observe.
K15 consumer: convergence signals trigger multi-source judgment.
"""
import logging
import re
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timezone, timedelta
from typing import Optional

logger = logging.getLogger("convergence-detector")


@dataclass
class ConvergenceSignal:
    cashtag: str
    resolved_mint: Optional[str]
    author_count: int
    authors: list[str]
    coordination_score: float  # 0=independent, 1=identical texts
    sentiment: str  # "bearish", "bullish", "neutral"
    key_quotes: list[dict]  # [{author, text, tweet_id}]
    window_hours: int
    domain: str


def _extract_author(context: str) -> str:
    """Extract @author from context string '@author [score]: text'."""
    m = re.match(r"@(\S+)", context or "")
    return m.group(1) if m else ""


def _extract_cashtags(tags: list[str]) -> list[str]:
    """Filter tags to cashtags only (uppercase, 2-10 chars)."""
    return [t for t in tags if t.isupper() and 2 <= len(t) <= 10 and t.isalpha()]


def detect_convergence(
    observations: list[dict],
    min_authors: int = 3,
    window_hours: int = 6,
) -> list[ConvergenceSignal]:
    """Scan observations, return convergence signals for items above threshold."""
    cutoff = datetime.now(timezone.utc) - timedelta(hours=window_hours)

    # Group by cashtag → set of (author, context, tweet_id)
    groups: dict[str, list[dict]] = defaultdict(list)
    for obs in observations:
        created = obs.get("created_at", "")
        try:
            ts = datetime.fromisoformat(created.replace("Z", "+00:00"))
        except (ValueError, AttributeError):
            continue
        if ts < cutoff:
            continue

        author = _extract_author(obs.get("context", ""))
        if not author:
            continue

        for tag in _extract_cashtags(obs.get("tags", [])):
            groups[tag].append({
                "author": author,
                "context": obs.get("context", ""),
                "target": obs.get("target", ""),
                "ts": ts,
            })

    signals = []
    for cashtag, entries in groups.items():
        # Deduplicate by author
        seen_authors: dict[str, dict] = {}
        for entry in entries:
            a = entry["author"]
            if a not in seen_authors:
                seen_authors[a] = entry

        if len(seen_authors) < min_authors:
            continue

        authors = list(seen_authors.keys())
        # Key quotes: one per author, most recent first, max 3
        quotes = [
            {"author": e["author"], "text": e["context"][:200], "tweet_id": e["target"]}
            for e in list(seen_authors.values())[:3]
        ]

        # Coordination: check for identical texts (basic)
        texts = [e["context"] for e in seen_authors.values()]
        unique_texts = len(set(texts))
        coord = 1.0 - (unique_texts / len(texts)) if texts else 0.0

        signals.append(ConvergenceSignal(
            cashtag=cashtag,
            resolved_mint=None,  # filled by caller via resolver
            author_count=len(seen_authors),
            authors=authors,
            coordination_score=round(coord, 3),
            sentiment="neutral",  # Phase C: sentiment analysis
            key_quotes=quotes,
            window_hours=window_hours,
            domain="D1",  # default; caller can override based on cashtag
        ))

    return signals
