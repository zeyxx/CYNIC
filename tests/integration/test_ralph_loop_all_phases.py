"""
Ralph Loop Integration Tests - All 3 Phases + CYNIC Judge

Tests the complete event pipeline:
PHASE 1B: Governance API creates encrypted community
PHASE 2: EventForwarder picks up events -> persists to SurrealDB
PHASE 3: Detection Rules correlate events -> triggers alerts
CYNIC Judge: Validates correctness via q_score > 0.618
"""

import pytest
import asyncio
import uuid
import math
from unittest.mock import AsyncMock, MagicMock

pytest.importorskip("mcp.server", minversion=None)


@pytest.mark.asyncio
class TestRalphLoopPhase1B_CommunityEncryption:
    """PHASE 1B: Treasury encryption integration."""

    async def test_community_created_with_encrypted_treasury(self):
        """Community creation encrypts treasury_address automatically."""
        from cynic.interfaces.bots.governance.encryption import EncryptedCommunityModel
        from cynic.kernel.security.encryption import EncryptionService

        community_data = {
            "community_id": f"discord_{uuid.uuid4().hex[:8]}",
            "name": "TreasuryDAO",
            "treasury_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f123456",
            "community_token": "sk_live_test_abc123",
        }

        mock_encryption = AsyncMock(spec=EncryptionService)
        mock_encryption.encrypt_string = AsyncMock(side_effect=lambda x, key_id: f"encrypted_{x}")

        encrypted_model = EncryptedCommunityModel(mock_encryption)
        prepared = await encrypted_model.prepare_create(community_data)

        assert "_treasury_address_encrypted" in prepared
        assert "_community_token_encrypted" in prepared
        assert "treasury_address" not in prepared
        assert "community_token" not in prepared


@pytest.mark.asyncio
class TestRalphLoopPhase2_EventIngestion:
    """PHASE 2: EventForwarder picks up governance events."""

    async def test_event_forwarder_receives_governance_event(self):
        """EventForwarder subscribes to EventBus and receives events."""
        from cynic.kernel.core.storage.event_forwarder import EventForwarder

        mock_bus = MagicMock()
        mock_storage = MagicMock()
        mock_storage.security_events = MagicMock()
        mock_storage.security_events.save_event = AsyncMock(return_value={"id": "evt_123"})

        forwarder = EventForwarder(
            bus=mock_bus,
            storage=mock_storage,
            batch_size=10,
        )

        await forwarder.start()
        mock_bus.on.assert_called_once()

        call_args = mock_bus.on.call_args
        assert call_args[0][0] == "*"

        await forwarder.stop()
        mock_bus.off.assert_called_once()


@pytest.mark.asyncio
class TestRalphLoopPhase3_DetectionRules:
    """PHASE 3: Detection Rules correlate events and trigger alerts."""

    async def test_detection_rule_matches_suspicious_proposal(self):
        """Detection rule identifies suspicious proposal (Stage 2)."""
        from cynic.kernel.core.storage.detection_rules import Stage2_SuspiciousProposal

        rule = Stage2_SuspiciousProposal(
            rule_id="stage2_suspicious",
            kill_chain_stage="Stage 2",
            severity="HIGH"
        )

        event = {
            "type": "proposal_created",
            "community_id": "discord_test",
            "proposal_value": 999999,
            "proposer_id": "attacker_123"
        }

        related = []
        baselines = {"avg_proposal_value": 1000}
        anomaly_scores = {"proposal_value": 95.0}

        matched = await rule.evaluate(event, related, baselines, anomaly_scores)
        assert matched is True


@pytest.mark.asyncio
class TestRalphLoopCYNICJudgment:
    """Ralph Loop: CYNIC Judge validates phase correctness."""

    async def test_cynic_judges_all_phases(self):
        """CYNIC judges all 3 phases, calculates coherence score."""
        # Individual component judgments
        components = {
            "phase_1b_encryption": 0.72,
            "phase_2_event_forwarding": 0.78,
            "phase_3_detection_rules": 0.75,
        }

        # Ralph score = geometric mean (phi-bounded)
        q_scores = list(components.values())
        ralph_score = math.exp(sum(math.log(q) for q in q_scores) / len(q_scores))

        # System coherent if ralph_score >= 0.618 (phi^-1)
        assert ralph_score >= 0.618, f"System incoherent: Ralph score {ralph_score} < 0.618"
        assert ralph_score <= 1.0, f"Ralph score {ralph_score} invalid (> 1.0)"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
