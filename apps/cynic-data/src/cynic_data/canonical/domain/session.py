"""The NormalizedSession aggregate.

One harness session, projected into canonical entities. This is the unit a
normalizer returns and the canonical store persists. Invariants are enforced here,
not in the store and not in a script.
"""

from __future__ import annotations

import dataclasses
import hashlib
import json
from dataclasses import dataclass, field

from ...kernel.contract import CANONICAL_CONTRACT, LayerContract
from ...kernel.errors import ContractViolation
from .entities import (
    Edge,
    HumanSignal,
    ReasoningTrace,
    StateSnapshot,
    ToolCall,
    Turn,
)


def session_content_hash(session: NormalizedSession) -> str:
    """Stable content hash of a normalized session (skip-if-unchanged + resume).

    Enums serialize via str(); key order is fixed. Two byte-identical
    normalizations produce the same hash on any run.
    """
    raw = json.dumps(
        dataclasses.asdict(session), ensure_ascii=False, sort_keys=True, default=str
    )
    return "sha256:" + hashlib.sha256(raw.encode("utf-8")).hexdigest()


@dataclass
class NormalizedSession:
    session_uid: str
    harness: str
    source_ref: str
    turns: list[Turn] = field(default_factory=list)
    tools: list[ToolCall] = field(default_factory=list)
    reasoning: list[ReasoningTrace] = field(default_factory=list)
    edges: list[Edge] = field(default_factory=list)
    states: list[StateSnapshot] = field(default_factory=list)
    human: list[HumanSignal] = field(default_factory=list)
    native_meta: dict = field(default_factory=dict)

    @property
    def contract(self) -> LayerContract:
        return CANONICAL_CONTRACT

    def check(self) -> None:
        """Enforce the canonical invariants. Raises ContractViolation on breach."""
        turn_ids = {t.turn_uid for t in self.turns}
        if len(turn_ids) != len(self.turns):
            raise ContractViolation(f"{self.session_uid}: duplicate turn_uid")

        for t in self.turns:
            if t.session_uid != self.session_uid:
                raise ContractViolation(
                    f"{self.session_uid}: turn {t.turn_uid} carries foreign session_uid"
                )
            if t.role is None or t.role == "":
                raise ContractViolation(f"{self.session_uid}: turn {t.turn_uid} has empty role")

        for e in self.edges:
            if e.session_uid != self.session_uid:
                raise ContractViolation(
                    f"{self.session_uid}: edge {e.edge_uid} carries foreign session_uid"
                )
            if not e.source_uids:
                raise ContractViolation(f"{self.session_uid}: edge {e.edge_uid} has no source")
            if e.target_uid is not None and e.target_uid not in turn_ids:
                raise ContractViolation(
                    f"{self.session_uid}: edge {e.edge_uid} targets unknown turn {e.target_uid}"
                )

        for h in self.human:
            if h.turn_uid not in turn_ids:
                raise ContractViolation(
                    f"{self.session_uid}: human signal {h.human_uid} references unknown turn"
                )

        for s in self.states:
            if s.turn_uid is not None and s.turn_uid not in turn_ids:
                raise ContractViolation(
                    f"{self.session_uid}: state {s.snapshot_uid} references unknown turn"
                )

    def stats(self) -> dict[str, int]:
        return {
            "turns": len(self.turns),
            "tools": len(self.tools),
            "reasoning": len(self.reasoning),
            "edges": len(self.edges),
            "native_edges": sum(1 for e in self.edges if e.provenance.value == "native"),
            "states": len(self.states),
            "human": len(self.human),
            "corrections": sum(1 for h in self.human if h.kind.value != "unknown"),
        }
