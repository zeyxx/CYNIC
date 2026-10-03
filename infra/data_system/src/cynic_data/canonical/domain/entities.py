"""Canonical entities.

Design rule (operator invariant): harnesses carry DIFFERENT models. Forcing them
into one uniform shape is tedious and sometimes impossible. So every entity keeps
two things side by side:

  * normalized fields  — best effort, harness-agnostic, NULL when unobserved
  * native fields      — the exact raw payload / model name / fields the harness used

Missing data is stored as ``None``, never as a default (anti-fabrication). The
normalized view is a convenience; the native view is the truth. Nothing here is
required to be uniform across harnesses.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class EdgeKind(str, Enum):
    """Typed relation between trajectory nodes."""

    REPLY = "reply"
    TOOL_CALL = "tool_call"
    TOOL_RESULT = "tool_result"
    REASONING = "reasoning"
    COMPACTION = "compaction"
    SIDECHAIN = "sidechain"
    SUBAGENT = "subagent"
    BATTLE = "battle"
    MODEL_CHANGE = "model_change"
    SEQUENCE = "sequence"


class EdgeProvenance(str, Enum):
    """Where an edge comes from. Native = the harness states it; derived = we infer it."""

    NATIVE = "native"
    DERIVED = "derived"


@dataclass(frozen=True)
class Turn:
    turn_uid: str
    session_uid: str
    index: int
    role: str
    content: str
    timestamp_iso: str | None = None
    native_model: str | None = None
    native_fields: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class ToolCall:
    exec_uid: str
    session_uid: str
    turn_uid: str | None
    index: int
    tool_name: str
    call_id: str | None = None
    command_or_args: str | None = None
    raw_output: str | None = None
    exit_code: int | None = None
    is_error: bool = False
    # links a result back to the call that produced it (harness-native when available)
    parent_exec_uid: str | None = None
    timestamp_iso: str | None = None
    native_fields: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class ReasoningTrace:
    trace_uid: str
    session_uid: str
    turn_uid: str | None
    content: str
    subject: str | None = None
    model: str | None = None
    timestamp_iso: str | None = None


@dataclass(frozen=True)
class Edge:
    """A typed relation. ``source_uids`` is a tuple to allow n-ary relations
    (one compaction summarizing N turns, one battle between M agents)."""

    edge_uid: str
    session_uid: str
    kind: EdgeKind
    source_uids: tuple[str, ...]
    target_uid: str | None
    provenance: EdgeProvenance
    payload: dict[str, Any] = field(default_factory=dict)
    timestamp_iso: str | None = None


@dataclass(frozen=True)
class StateSnapshot:
    """The state that evolves at a point in the trajectory. All fields optional —
    a harness that does not expose state simply leaves them None."""

    snapshot_uid: str
    session_uid: str
    turn_uid: str | None
    index: int
    model: str | None = None
    cwd: str | None = None
    sandbox_policy: str | None = None
    approval_policy: str | None = None
    context_prompt_tokens: int | None = None
    context_nonmessage_tokens: int | None = None
    cumulative_tokens: int | None = None
    compacted: bool | None = None
    native_fields: dict[str, Any] = field(default_factory=dict)
    timestamp_iso: str | None = None


class CorrectionKind(str, Enum):
    """Classification of a human intervention. UNKNOWN is the honest default —
    we never guess that a human turn was a correction unless a marker says so."""

    UNKNOWN = "unknown"
    REDIRECT = "redirect"
    CONSTRAINT = "constraint"
    REJECTION = "rejection"
    APPROVAL = "approval"


@dataclass(frozen=True)
class HumanSignal:
    """A human turn, tagged with whatever intervention markers the raw text carries.

    The human signal is the richest thing the corpus holds; it is first-class here,
    not a plain dialogue row."""

    human_uid: str
    session_uid: str
    turn_uid: str
    text: str
    kind: CorrectionKind = CorrectionKind.UNKNOWN
    markers: tuple[str, ...] = field(default_factory=tuple)
    timestamp_iso: str | None = None
