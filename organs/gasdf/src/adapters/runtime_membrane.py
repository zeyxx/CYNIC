"""Organ-owned GASdf runtime membrane.

This adapter intentionally does not sign transactions. It replaces the old
external checkout projection with an owned runtime surface that can be supervised
by CYNIC while keeping fee-payer signing disabled until lifecycle approval is
explicit.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass(frozen=True)
class GasdfRuntimeEvidence:
    organ_root: str
    runtime_owner: str
    signer_adapter_present: bool
    external_checkout_required: bool

    def to_dict(self) -> dict[str, object]:
        return asdict(self)


class OrganOwnedRuntimeMembrane:
    def __init__(self, organ_root: Path) -> None:
        self.organ_root = organ_root.resolve()

    def evidence(self) -> GasdfRuntimeEvidence:
        return GasdfRuntimeEvidence(
            organ_root=str(self.organ_root),
            runtime_owner="organ-gasdf",
            signer_adapter_present=False,
            external_checkout_required=False,
        )
