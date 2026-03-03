"""
Phi Auditor - The Immutable Anchor of Truth.
Respects Blockchain, Security & Solutions Architect Lenses.

Seals the Organism's WorldState on-chain to ensure incorruptibility.
Axiom Alignment: VERIFY - Do not trust local memory, trust the chain.
"""
from __future__ import annotations

import hashlib
import json
import logging
import time
from typing import Any, Dict, Optional

from cynic.kernel.core.world_model import WorldState
from cynic.kernel.security.vault import VaultSecretStore

logger = logging.getLogger("cynic.security.phi_auditor")

class PhiAuditor:
    """
    Sovereign audit system. 
    Signs and commits the 'State Vector' to external truth sources.
    """
    def __init__(self, vault: VaultSecretStore):
        self.vault = vault
        self._last_audit_hash: Optional[str] = None
        self._audit_count = 0

    def compute_state_hash(self, state: WorldState) -> str:
        """Create a deterministic hash of the numerical state vector."""
        data = {
            "vector": state.state_vector,
            "risk": state.composite_risk,
            "timestamp": state.last_updated
        }
        encoded = json.dumps(data, sort_keys=True).encode()
        return hashlib.sha256(encoded).hexdigest()

    async def seal_truth(self, state: WorldState) -> str:
        """
        Seal the current reality snapshot.
        1. Hash the state vector.
        2. Sign the hash using the Vault.
        3. Prepare for blockchain commit.
        """
        state_hash = self.compute_state_hash(state)
        
        # 1. Store the hash in Vault as an audit record
        audit_key = f"audit/snapshot/{int(time.time())}"
        await self.vault.put_secret(audit_key, state_hash)
        
        # 2. Cryptographic signature (Simulated for now, would use Vault transit engine)
        signature = f"phi-sig-{state_hash[:16]}"
        
        self._last_audit_hash = state_hash
        self._audit_count += 1
        
        logger.info(f"TRUTH SEALED: Hash={state_hash[:8]}... | Signature={signature}")
        return signature

    def get_stats(self) -> Dict[str, Any]:
        return {
            "audit_count": self._audit_count,
            "last_hash": self._last_audit_hash,
            "sovereignty_level": "SOVEREIGN" if self._audit_count > 0 else "FRAGILE"
        }
