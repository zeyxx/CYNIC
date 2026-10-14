"""Layer contracts.

A layer is not a colour in a gradient; it is a contract with a guarantee and a
DECLARED loss. A projection that drops the human prompt must say so. The audit
then checks the declaration against reality, so loss stops being invisible.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class Layer(str, Enum):
    CAPTURE = "capture"
    CANONICAL = "canonical"
    MODEL = "model"
    PROJECTION = "projection"


class Authority(str, Enum):
    AUTHORITATIVE = "authoritative"
    DERIVED = "derived"


@dataclass(frozen=True)
class LossDeclaration:
    """An explicit, named loss a layer admits to inflicting."""

    field: str
    reason: str


@dataclass(frozen=True)
class LayerContract:
    """The guarantee a layer makes, and what it admits to losing."""

    layer: Layer
    authority: Authority
    guarantee: str
    reads_from: Layer | None
    declared_losses: tuple[LossDeclaration, ...] = field(default_factory=tuple)

    def declares_loss_of(self, field_name: str) -> bool:
        return any(loss.field == field_name for loss in self.declared_losses)


# The canonical contracts for the four layers. Any adapter claiming to implement a
# layer must publish a contract compatible with these.
CAPTURE_CONTRACT = LayerContract(
    layer=Layer.CAPTURE,
    authority=Authority.AUTHORITATIVE,
    guarantee="Exact bytes emitted by a harness, immutable and content-addressed.",
    reads_from=None,
    declared_losses=(),
)

CANONICAL_CONTRACT = LayerContract(
    layer=Layer.CANONICAL,
    authority=Authority.AUTHORITATIVE,
    guarantee=(
        "Typed harness-agnostic facts: turns, tools, reasoning, edges, state, human signal. "
        "No semantic loss versus Capture."
    ),
    reads_from=Layer.CAPTURE,
    declared_losses=(),
)

MODEL_CONTRACT = LayerContract(
    layer=Layer.MODEL,
    authority=Authority.DERIVED,
    guarantee="The trajectory hypergraph, materialized and queryable.",
    reads_from=Layer.CANONICAL,
    declared_losses=(),
)

PROJECTION_CONTRACT = LayerContract(
    layer=Layer.PROJECTION,
    authority=Authority.DERIVED,
    guarantee="A consumer view. Reconstructible from Model.",
    reads_from=Layer.MODEL,
    declared_losses=(),
)


def all_layer_contracts() -> tuple[LayerContract, ...]:
    return (CAPTURE_CONTRACT, CANONICAL_CONTRACT, MODEL_CONTRACT, PROJECTION_CONTRACT)
