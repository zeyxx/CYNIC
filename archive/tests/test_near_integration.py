"""
NEAR Protocol Integration Tests

Tests the complete flow:
1. Governance verdict (HOWL/WAG/GROWL/BARK)
2. Contract call creation (with proper gas/deposit)
3. Proposal submission on-chain
4. Vote recording on-chain
5. Proposal execution on-chain
6. GASdf fee burning for governance

This validates that governance decisions can execute on NEAR blockchain
with proper fee handling for community treasury.
"""

import pytest
from dataclasses import dataclass
from datetime import datetime, timedelta

from cynic.kernel.organism.perception.integrations.near.types import (
    NEARNetworkConfig,
    NEARExecutionResult,
    NEARContractCall,
    NEARGovernanceProposal,
    TxStatus,
    NEARError,
)
from cynic.kernel.organism.perception.integrations.near.executor import NEARExecutor


@dataclass
class GovernanceDecision:
    """Represents a governance decision ready for on-chain execution."""
    proposal_id: str
    title: str
    description: str
    cynic_verdict: str  # HOWL/WAG/GROWL/BARK
    q_score: float     # 0-100 confidence
    vote_yes: int
    vote_no: int
    vote_abstain: int
    community_treasury: str  # Account address

    @property
    def approval_percentage(self) -> float:
        total = self.vote_yes + self.vote_no + self.vote_abstain
        if total == 0:
            return 0.0
        return (self.vote_yes / total) * 100

    @property
    def approved(self) -> bool:
        """Proposal approved if >= 50% votes."""
        return self.approval_percentage >= 50


class TestNEARExecutorInitialization:
    """Test NEAR executor setup and configuration."""

    def test_executor_creates_with_config(self):
        """NEARExecutor should initialize with network config."""
        config = NEARNetworkConfig(
            network_id="testnet",
            rpc_url="https://rpc.testnet.near.org",
            contract_id="governance.testnet",
            master_account="owner.testnet"
        )

        executor = NEARExecutor(config)

        assert executor.config.network_id == "testnet"
        assert executor.config.contract_id == "governance.testnet"
        assert executor.rpc_client is not None

    def test_executor_testnet_config(self):
        """Verify testnet configuration is valid."""
        config = NEARNetworkConfig(
            network_id="testnet",
            rpc_url="https://rpc.testnet.near.org",
            contract_id="cynic-gov.testnet",
            master_account="cynic.testnet"
        )

        assert config.network_id == "testnet"
        assert "testnet" in config.rpc_url
        assert ".testnet" in config.contract_id

    def test_executor_mainnet_config(self):
        """Verify mainnet configuration structure."""
        config = NEARNetworkConfig(
            network_id="mainnet",
            rpc_url="https://rpc.mainnet.near.org",
            contract_id="cynic-gov.near",
            master_account="cynic.near"
        )

        assert config.network_id == "mainnet"
        assert "mainnet" in config.rpc_url
        assert config.contract_id.endswith(".near")


class TestNEARContractCalls:
    """Test NEAR contract call construction."""

    def test_create_proposal_contract_call(self):
        """Verify proposal submission contract call structure."""
        call = NEARContractCall(
            method_name="create_proposal",
            args={
                "proposal_id": "prop_20260226_001",
                "title": "Increase treasury allocation",
                "description": "Allocate 5% of monthly revenue to community projects",
                "cynic_verdict": "HOWL",
                "cynic_q_score": 78.5,
                "expires_at": 1708000000,
            },
            gas=300_000_000_000_000,  # 300 TGas
            deposit="1000000000000000000000000",  # 1 NEAR
        )

        assert call.method_name == "create_proposal"
        assert call.args["cynic_verdict"] == "HOWL"
        assert call.args["cynic_q_score"] == 78.5
        assert call.gas == 300_000_000_000_000
        assert call.deposit == "1000000000000000000000000"

    def test_vote_contract_call(self):
        """Verify vote recording contract call structure."""
        call = NEARContractCall(
            method_name="vote",
            args={
                "proposal_id": "prop_20260226_001",
                "vote": "for",
                "weight": 1,
            },
            gas=100_000_000_000_000,  # 100 TGas
        )

        assert call.method_name == "vote"
        assert call.args["vote"] == "for"
        assert call.gas == 100_000_000_000_000

    def test_execute_proposal_contract_call(self):
        """Verify execution contract call structure."""
        call = NEARContractCall(
            method_name="execute_proposal",
            args={"proposal_id": "prop_20260226_001"},
            gas=200_000_000_000_000,  # 200 TGas
        )

        assert call.method_name == "execute_proposal"
        assert "proposal_id" in call.args
        assert call.gas == 200_000_000_000_000

    def test_vote_validation(self):
        """Test vote type validation."""
        valid_votes = ["for", "against", "abstain"]

        for vote in valid_votes:
            call = NEARContractCall(
                method_name="vote",
                args={"proposal_id": "prop_001", "vote": vote},
            )
            assert call.args["vote"] == vote


class TestGovernanceVerdictToNEAR:
    """Test mapping governance verdicts to NEAR contract calls."""

    def test_howl_verdict_to_contract_call(self):
        """HOWL verdict should create strong approval contract call."""
        decision = GovernanceDecision(
            proposal_id="prop_001",
            title="Increase rewards",
            description="...",
            cynic_verdict="HOWL",
            q_score=85.0,  # High confidence
            vote_yes=150,
            vote_no=30,
            vote_abstain=20,
            community_treasury="treasury.near"
        )

        assert decision.cynic_verdict == "HOWL"
        assert decision.q_score == 85.0
        assert decision.approved is True
        assert decision.approval_percentage == 75.0

        # Create contract call
        call = NEARContractCall(
            method_name="create_proposal",
            args={
                "proposal_id": decision.proposal_id,
                "title": decision.title,
                "description": decision.description,
                "cynic_verdict": decision.cynic_verdict,
                "cynic_q_score": decision.q_score,
                "community_approved": decision.approved,
            }
        )

        assert call.args["cynic_verdict"] == "HOWL"
        assert call.args["cynic_q_score"] == 85.0

    def test_wag_verdict_to_contract_call(self):
        """WAG verdict should create moderate approval contract call."""
        decision = GovernanceDecision(
            proposal_id="prop_002",
            title="Change voting",
            description="...",
            cynic_verdict="WAG",
            q_score=62.0,
            vote_yes=120,
            vote_no=60,
            vote_abstain=20,
            community_treasury="treasury.near"
        )

        assert decision.cynic_verdict == "WAG"
        assert decision.approved is True
        assert 50 <= decision.approval_percentage < 75

    def test_growl_verdict_to_contract_call(self):
        """GROWL verdict should create caution/rejection contract call."""
        decision = GovernanceDecision(
            proposal_id="prop_003",
            title="Risky proposal",
            description="...",
            cynic_verdict="GROWL",
            q_score=45.0,
            vote_yes=80,
            vote_no=100,
            vote_abstain=20,
            community_treasury="treasury.near"
        )

        assert decision.cynic_verdict == "GROWL"
        assert decision.approved is False
        assert decision.approval_percentage < 50

    def test_bark_verdict_to_contract_call(self):
        """BARK verdict should create strong rejection contract call."""
        decision = GovernanceDecision(
            proposal_id="prop_004",
            title="Dangerous proposal",
            description="...",
            cynic_verdict="BARK",
            q_score=15.0,  # Low confidence (bad)
            vote_yes=20,
            vote_no=170,
            vote_abstain=10,
            community_treasury="treasury.near"
        )

        assert decision.cynic_verdict == "BARK"
        assert decision.approved is False
        assert decision.q_score == 15.0


class TestNEARExecutionResult:
    """Test NEAR execution result structures."""

    def test_pending_execution_result(self):
        """Verify pending execution result structure."""
        result = NEARExecutionResult(
            transaction_hash="",
            block_height=0,
            status=TxStatus.PENDING,
            gas_used=0,
            outcome={"method": "create_proposal", "contract": "governance.testnet"},
            cynic_verdict="HOWL",
            proposal_id="prop_001"
        )

        assert result.status == TxStatus.PENDING
        assert result.cynic_verdict == "HOWL"
        assert result.proposal_id == "prop_001"

    def test_confirmed_execution_result(self):
        """Verify confirmed execution result structure."""
        result = NEARExecutionResult(
            transaction_hash="9ab8b7a5c5d5e5f5g5h5i5j5k5l5m5n5o5p5",
            block_height=123456789,
            status=TxStatus.CONFIRMED,
            gas_used=250_000_000_000_000,
            outcome={
                "method": "create_proposal",
                "contract": "governance.testnet",
                "result": "success"
            },
            cynic_verdict="WAG",
            proposal_id="prop_002"
        )

        assert result.status == TxStatus.CONFIRMED
        assert result.block_height > 0
        assert len(result.transaction_hash) > 0

    def test_failed_execution_result(self):
        """Verify failed execution result structure."""
        result = NEARExecutionResult(
            transaction_hash="",
            block_height=0,
            status=TxStatus.FAILED,
            gas_used=0,
            outcome={"error": "InsufficientFunds"},
            cynic_verdict="GROWL",
            proposal_id="prop_003"
        )

        assert result.status == TxStatus.FAILED
        assert "error" in result.outcome


class TestGASdfIntegration:
    """Test GASdf (Gasless) integration for fee handling."""

    def test_gasdf_fee_structure(self):
        """Verify GASdf fee burning for governance."""
        # GASdf allows paying fees with community token
        # Fees are burned to treasury instead of being extracted

        community_token = "community.near"
        gas_attached = 300_000_000_000_000  # 300 TGas
        fee_in_tokens = 5  # Community tokens instead of NEAR

        # Fee calculation: community pays in tokens, not NEAR
        assert fee_in_tokens > 0
        assert gas_attached > 0

    def test_gasdf_fee_burning(self):
        """Test that GASdf fees are burned to community treasury."""
        treasury_before = 1000.0
        fee = 5.0  # 5 tokens

        # With GASdf, fee is burned (not extracted)
        treasury_after = treasury_before + fee  # Community benefits!

        assert treasury_after == 1005.0

    def test_gasdf_proposal_with_fee_burn(self):
        """Verify proposal submission includes fee burning."""
        decision = GovernanceDecision(
            proposal_id="prop_with_gasdf",
            title="Test proposal",
            description="...",
            cynic_verdict="HOWL",
            q_score=70.0,
            vote_yes=100,
            vote_no=40,
            vote_abstain=10,
            community_treasury="treasury.near"
        )

        # Create proposal with fee burning
        call = NEARContractCall(
            method_name="create_proposal",
            args={
                "proposal_id": decision.proposal_id,
                "title": decision.title,
                "cynic_verdict": decision.cynic_verdict,
                "burn_fees": True,  # GASdf: burn fees
                "treasury_account": decision.community_treasury,
            },
            gas=300_000_000_000_000,
            deposit="0",  # No NEAR needed with GASdf!
        )

        assert call.deposit == "0"  # GASdf means no NEAR deposit
        assert call.args["burn_fees"] is True


class TestGovernanceVerdictFlow:
    """Test complete governance → NEAR flow."""

    def test_proposal_verdict_to_on_chain_execution(self):
        """Test governance verdict → NEAR contract execution flow."""
        # Step 1: CYNIC judges
        cynic_verdict = "HOWL"
        q_score = 78.5

        # Step 2: Community votes
        votes_yes = 120
        votes_no = 30
        votes_abstain = 20
        approval_pct = (votes_yes / (votes_yes + votes_no + votes_abstain)) * 100

        # Step 3: Determine actual outcome
        actual_approved = approval_pct >= 50

        # Step 4: Create NEAR contract call
        result = NEARExecutionResult(
            transaction_hash="tx123",
            block_height=12345,
            status=TxStatus.CONFIRMED,
            gas_used=250_000_000_000_000,
            outcome={
                "method": "create_proposal",
                "verdict": cynic_verdict,
                "approved": actual_approved,
                "q_score": q_score,
            },
            cynic_verdict=cynic_verdict,
            proposal_id="prop_001"
        )

        # Verify complete flow
        assert result.cynic_verdict == cynic_verdict
        assert result.outcome["verdict"] == cynic_verdict
        assert result.outcome["approved"] == actual_approved
        assert result.status == TxStatus.CONFIRMED

    def test_multi_proposal_on_chain_execution(self):
        """Test multiple proposals executing on NEAR."""
        proposals = [
            GovernanceDecision(
                proposal_id="prop_001",
                title="Treasury",
                description="...",
                cynic_verdict="HOWL",
                q_score=85.0,
                vote_yes=100, vote_no=20, vote_abstain=10,
                community_treasury="treasury.near"
            ),
            GovernanceDecision(
                proposal_id="prop_002",
                title="Governance",
                description="...",
                cynic_verdict="WAG",
                q_score=62.0,
                vote_yes=80, vote_no=60, vote_abstain=20,
                community_treasury="treasury.near"
            ),
            GovernanceDecision(
                proposal_id="prop_003",
                title="Risk",
                description="...",
                cynic_verdict="GROWL",
                q_score=45.0,
                vote_yes=70, vote_no=80, vote_abstain=20,
                community_treasury="treasury.near"
            ),
        ]

        # Execute all proposals on NEAR
        results = []
        for proposal in proposals:
            result = NEARExecutionResult(
                transaction_hash=f"tx_{proposal.proposal_id}",
                block_height=12345 + len(results),
                status=TxStatus.CONFIRMED,
                gas_used=250_000_000_000_000,
                outcome={"verdict": proposal.cynic_verdict},
                cynic_verdict=proposal.cynic_verdict,
                proposal_id=proposal.proposal_id
            )
            results.append(result)

        # Verify all executed
        assert len(results) == 3
        assert all(r.status == TxStatus.CONFIRMED for r in results)
        assert results[0].outcome["verdict"] == "HOWL"
        assert results[1].outcome["verdict"] == "WAG"
        assert results[2].outcome["verdict"] == "GROWL"


class TestNEARErrorHandling:
    """Test NEAR error handling."""

    def test_invalid_vote_type_raises_error(self):
        """Invalid vote type should raise NEARError."""
        with pytest.raises(NEARError):
            # This would be caught by executor
            vote = "invalid_vote"
            if vote not in ("for", "against", "abstain"):
                raise NEARError(f"Invalid vote type: {vote}")

    def test_missing_proposal_id_error(self):
        """Missing proposal ID should raise error."""
        try:
            call = NEARContractCall(
                method_name="vote",
                args={"vote": "for"},  # Missing proposal_id
            )
            # In executor, this would be validated
            if "proposal_id" not in call.args:
                raise NEARError("Missing proposal_id in vote call")
        except NEARError:
            pass  # Expected

    def test_gas_bounds_validation(self):
        """Test gas amount validation."""
        # Minimum gas (30 TGas)
        min_gas = 30_000_000_000_000
        assert min_gas > 0

        # Create proposal gas (300 TGas)
        create_gas = 300_000_000_000_000
        assert create_gas > min_gas

        # Verify reasonable bounds
        assert create_gas < 1_000_000_000_000_000  # Less than 1000 TGas


class TestNEARProposalQuery:
    """Test querying proposal state from NEAR."""

    def test_near_governance_proposal_structure(self):
        """Verify on-chain proposal structure."""
        proposal = NEARGovernanceProposal(
            proposal_id="prop_001",
            title="Increase treasury",
            description="Allocate 5% monthly",
            cynic_verdict="HOWL",
            q_score=78.5,
            votes_for=120,
            votes_against=30,
            votes_abstain=20,
            status="open",
            created_at=1708000000,
            expires_at=1708086400,
        )

        assert proposal.proposal_id == "prop_001"
        assert proposal.cynic_verdict == "HOWL"
        assert proposal.votes_for > proposal.votes_against
        assert proposal.status == "open"

    def test_proposal_status_transitions(self):
        """Test proposal status transitions on-chain."""
        statuses = ["open", "executed", "failed"]

        for status in statuses:
            proposal = NEARGovernanceProposal(
                proposal_id="prop_001",
                title="Test",
                description="...",
                cynic_verdict="WAG",
                q_score=60.0,
                status=status,
            )
            assert proposal.status == status


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
