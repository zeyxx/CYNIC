"""Human intervention detection — shared, harness-agnostic, conservative.

We never *invent* a correction. A human turn is only tagged when its text carries
an explicit marker. Everything else stays UNKNOWN. This is the anti-fabrication
rule applied to the richest signal in the corpus.
"""

from __future__ import annotations

import re

from ...domain.entities import CorrectionKind

# Explicit markers, matched case-insensitively. Kept small and literal on purpose:
# a wrong tag is worse than an honest UNKNOWN.
_REJECTION = [
    r"\bnon\b\s*[,.]", r"\bno\b\s*[,.]", r"\bpas\b.{0,20}\bmais\b",
    r"that'?s wrong", r"c'?est faux", r"incorrect", r"stop\b", r"arrête",
    r"revert", r"undo", r"annule",
]
_REDIRECT = [
    r"\binstead\b", r"\bplutôt\b", r"\bplutot\b", r"au lieu de",
    r"\btry again\b", r"\brefais\b", r"\brecommence\b", r"\bchange\b",
    r"\buse\b.{0,20}\bnot\b", r"n'?utilise pas",
]
_CONSTRAINT = [
    r"\bdon'?t\b", r"\bnever\b", r"\bjamais\b", r"\btoujours\b", r"\balways\b",
    r"\bmust\b", r"\bdoit\b", r"\bonly\b", r"\bseulement\b", r"interdit",
]
_APPROVAL = [
    r"^ok\b", r"^oui\b", r"^yes\b", r"^go\b", r"^parfait\b", r"^nickel\b",
    r"^bien\b", r"^correct\b", r"^top\b",
]


def _match(text: str, patterns: list[str]) -> list[str]:
    found = []
    low = text.strip().lower()
    for pat in patterns:
        if re.search(pat, low):
            found.append(pat)
    return found


def classify_human_turn(text: str) -> tuple[CorrectionKind, tuple[str, ...]]:
    """Return (kind, markers). Priority: rejection > redirect > constraint > approval."""
    if not text or not text.strip():
        return CorrectionKind.UNKNOWN, ()

    for kind, patterns in (
        (CorrectionKind.REJECTION, _REJECTION),
        (CorrectionKind.REDIRECT, _REDIRECT),
        (CorrectionKind.CONSTRAINT, _CONSTRAINT),
        (CorrectionKind.APPROVAL, _APPROVAL),
    ):
        markers = _match(text, patterns)
        if markers:
            return kind, tuple(markers)
    return CorrectionKind.UNKNOWN, ()
