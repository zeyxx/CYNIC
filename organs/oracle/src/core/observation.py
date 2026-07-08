"""Observation payloads for transparency anchoring."""

from __future__ import annotations

from dataclasses import asdict, dataclass

from .anchoring import AnchorReceipt


@dataclass(frozen=True)
class OracleObservation:
    domain: str
    event: str
    state_root: str | None
    transaction_hash: str | None
    network: str
    mode: str
    anchor_status: str

    def to_dict(self) -> dict[str, object]:
        return asdict(self)


def observation_from_receipt(receipt: AnchorReceipt) -> OracleObservation:
    return OracleObservation(
        domain="oracle",
        event="transparency_anchor",
        state_root=receipt.state_root,
        transaction_hash=receipt.transaction_hash,
        network=receipt.network,
        mode=receipt.mode,
        anchor_status=receipt.status,
    )

