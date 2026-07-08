"""Solana anchoring adapter boundary."""

from __future__ import annotations

from ..core.anchoring import AnchorReceipt, StateRoot, build_anchor_receipt


class MockSolanaAnchorAdapter:
    network = "solana-devnet"
    mode = "mock"

    def anchor(self, state_root: StateRoot) -> AnchorReceipt:
        transaction_hash = "MOCKED_TX_HASH" if state_root.cryptographic_hash else None
        return build_anchor_receipt(
            state_root,
            transaction_hash=transaction_hash,
            network=self.network,
            mode=self.mode,
        )

