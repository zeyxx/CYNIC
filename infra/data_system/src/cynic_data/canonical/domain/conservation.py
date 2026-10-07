"""Conservation contract — the alpha every harness must satisfy.

A normalizer is only trustworthy if it can prove, for the producer it reads:

1. **Closure** — every field the producer emits has a declared fate, and the
   list of fields is derived from the producer's OWN schema, not typed by hand.
2. **Anchor** — the contract is pinned to the producer's schema version; when
   the producer bumps it, the contract fails instead of drifting silently.
3. **Resolution** — a concept that several models realise in different columns
   (Hermes: reasoning lives in 4 different places depending on the model) is
   resolved by a declared priority rule, never by a guess.
4. **Checkability** — every ``preserved`` fate names the canonical home so the
   value can be compared; every ``declared`` loss names why the bytes stay out
   and is re-derivable from the frozen capture.

This module is pure: no I/O, no producer knowledge. Producers supply a contract.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Mapping


class Fate(str, Enum):
    """The two, and only two, things that can happen to a raw field."""

    PRESERVED = "preserved"
    DECLARED = "declared"


def parse_fate(value: str) -> tuple[Fate, str]:
    """Split ``"preserved: turn.content"`` into ``(Fate.PRESERVED, "turn.content")``.

    A fate that does not start with a known prefix is a contract bug, not a
    default: raise, never guess.
    """
    prefix, _, detail = value.partition(":")
    try:
        fate = Fate(prefix.strip())
    except ValueError as exc:
        raise ValueError(f"fate must start with preserved:/declared:, got {value!r}") from exc
    return fate, detail.strip()


@dataclass(frozen=True)
class ConceptResolution:
    """How one concept is realised across a producer's models.

    ``homes`` is priority-ordered: the first non-null column wins. ``model_profiles``
    records which home each model was observed to use, so a known model that
    starts using a different home is *drift* (a failure), not silence.
    """

    concept: str
    homes: tuple[str, ...]
    model_profiles: Mapping[str, str] = field(default_factory=dict)

    def resolve(self, row: Mapping[str, Any]) -> tuple[str | None, Any]:
        """Return ``(home, value)`` for the first non-null home, else ``(None, None)``."""
        for home in self.homes:
            if row.get(home) is not None:
                return home, row[home]
        return None, None

    def expected_home(self, model: str | None) -> str | None:
        """The home this model was observed to use, or None if unknown."""
        if model is None:
            return None
        if model in self.model_profiles:
            return self.model_profiles[model]
        # longest-pattern-first so specific models beat family prefixes
        for pattern in sorted(self.model_profiles, key=len, reverse=True):
            if pattern and pattern in model:
                return self.model_profiles[pattern]
        return None


@dataclass(frozen=True)
class ConservationContract:
    """The guarantee a normalizer makes about a producer's raw store."""

    producer: str
    producer_schema_version: int
    producer_schema_source: str
    field_fate: Mapping[str, str]
    concepts: tuple[ConceptResolution, ...] = ()
    # Fields whose value is transformed on the way in (not copied verbatim).
    # Keyed by canonical home -> human description of the transform. The L2
    # value check must apply the transform before comparing.
    transforms: Mapping[str, str] = field(default_factory=dict)

    # -- closure ------------------------------------------------------------

    def missing_fates(self, observed_fields: set[str]) -> list[str]:
        """Observed raw fields with no declared fate. Must be empty."""
        return sorted(f for f in observed_fields if f not in self.field_fate)

    def malformed_fates(self) -> list[str]:
        """Fates that do not start with preserved:/declared:."""
        bad: list[str] = []
        for key, value in self.field_fate.items():
            try:
                parse_fate(value)
            except ValueError:
                bad.append(f"{key} -> {value}")
        return sorted(bad)

    def preserved_fields(self) -> dict[str, str]:
        return {
            k: parse_fate(v)[1]
            for k, v in self.field_fate.items()
            if parse_fate(v)[0] is Fate.PRESERVED
        }

    def declared_losses(self) -> dict[str, str]:
        return {
            k: parse_fate(v)[1]
            for k, v in self.field_fate.items()
            if parse_fate(v)[0] is Fate.DECLARED
        }

    def concept(self, name: str) -> ConceptResolution | None:
        for c in self.concepts:
            if c.concept == name:
                return c
        return None


# ---------------------------------------------------------------------------
# Schema parsing — derive the observed field set from the producer's own DDL,
# so closure is checked against the producer, not against our memory of it.
# ---------------------------------------------------------------------------

_CREATE_TABLE_RE = re.compile(
    r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)\s*\((.*?)\)\s*;",
    re.IGNORECASE | re.DOTALL,
)

# Column-type keywords that must not be mistaken for a column name.
_NOT_A_COLUMN = {
    "primary", "foreign", "unique", "check", "constraint", "key", "references",
    "not", "null", "default", "on", "delete", "cascade", "autoincrement",
}


def parse_ddl_columns(ddl: str) -> dict[str, list[str]]:
    """Extract ``{table: [column, ...]}`` from a CREATE TABLE script.

    Pure string work: the producer's DDL is the source of truth for what fields
    exist, so the contract is never checked against a stale hand-kept list.
    """
    tables: dict[str, list[str]] = {}
    for match in _CREATE_TABLE_RE.finditer(ddl):
        table = match.group(1).lower()
        body = match.group(2)
        cols: list[str] = []
        depth = 0
        for raw_line in body.splitlines():
            line = raw_line.strip()
            if not line or line.startswith("--"):
                continue
            depth += line.count("(") - line.count(")")
            # table-level constraints sit at depth 0 and start with a keyword
            first = line.split()[0].strip(",").lower() if line.split() else ""
            if depth <= 0 and first in _NOT_A_COLUMN:
                continue
            name = line.split()[0].strip(",")
            if name and name.lower() not in _NOT_A_COLUMN and not name.startswith("--"):
                cols.append(name)
        tables[table] = cols
    return tables
