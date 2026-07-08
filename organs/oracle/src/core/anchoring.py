"""Pure anchoring model for the transparency oracle."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Iterable
import json


@dataclass(frozen=True)
class StateRoot:
    cryptographic_hash: str | None
    source: str

    def to_dict(self) -> dict[str, str | None]:
        return asdict(self)


@dataclass(frozen=True)
class AnchorReceipt:
    status: str
    state_root: str | None
    transaction_hash: str | None
    network: str
    mode: str

    def to_dict(self) -> dict[str, str | None]:
        return asdict(self)


def latest_state_root_from_jsonl(lines: Iterable[str], source: str) -> StateRoot:
    latest_hash: str | None = None
    for raw_line in lines:
        line = raw_line.strip()
        if not line:
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        candidate = entry.get("cryptographic_hash")
        if isinstance(candidate, str) and candidate:
            latest_hash = candidate
    return StateRoot(cryptographic_hash=latest_hash, source=source)


def build_anchor_receipt(
    state_root: StateRoot,
    *,
    transaction_hash: str | None,
    network: str,
    mode: str,
) -> AnchorReceipt:
    if not state_root.cryptographic_hash:
        return AnchorReceipt(
            status="no_state_root",
            state_root=None,
            transaction_hash=None,
            network=network,
            mode=mode,
        )
    return AnchorReceipt(
        status="anchored",
        state_root=state_root.cryptographic_hash,
        transaction_hash=transaction_hash,
        network=network,
        mode=mode,
    )

