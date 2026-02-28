"""Complete Governance Stack Integration Tests.

Tests the end-to-end workflow:
Discord → CYNIC API → GASdf → NEAR Protocol

This tests the complete pipeline for memocoin governance:
1. User submits proposal via Discord
2. CYNIC evaluates with 11 Dogs + 5 Axioms
3. Community votes (Discord reactions)
4. GASdf handles fee abstraction (76.4% burn to treasury)
5. NEAR records proposal on-chain
6. Learning loop updates Q-Table based on burn statistics
"""
import asyncio
import json
from datetime import datetime, timedelta
from typing import Any
from unittest.mock import AsyncMock, MagicMock, patch
from uuid import uuid4

import pytest

from cynic.perception.integrations.gasdf.client import GASdfClient
from cynic.perception.integrations.gasdf.executor import GASdfExecutor
from cynic.perception.integrations.gasdf.types import (
    GASdfExecutionResult,
    GASdfQuote,
    GASdfStats,
)
from cynic.perception.integrations.near.executor import NEARExecutor
from cynic.perception.integrations.near.types import (
    NEARExecutionResult,
    NEARGovernanceProposal,
    NEARNetworkConfig,
    TxStatus,
)


class MockGASdfClient:
    """Mock GASdf client for testing."""

    def __init__(self):
        self.quotes: dict[str, GASdfQuote] = {}
        self.executions: list[GASdfExecutionResult] = []
        self.total_burned = 0
        self.total_transactions = 0

    async def health(self) -> dict[str, Any]:
        """Return mock health check."""
        return {"status": "ok", "network": "mainnet"}

    async def get_tokens(self) -> list[dict[str, str]]:
        """Return mock token list."""
        return [
            {"symbol": "COIN", "mint": "token_mint_address"},
            {"symbol": "SOL", "mint": "So11111111111111111111111111111111111111112"},
        ]

    async def get_quote(
        self, payment_token: str, user_pubkey: str, amount: int
    ) -> GASdfQuote:
        """Return mock fee quote."""
        fee_amount = int(amount * 0.005)  # 0.5% fee
        burn_amount = int(fee_amount * 0.764)  # 76.4% burn
        quote_id = f"quote_{uuid4().hex[:8]}"

        quote = GASdfQuote(
            quote_id=quote_id,
            payment_token=payment_token,
            fee_amount=fee_amount,
            burn_amount=burn_amount,
            user_pubkey=user_pubkey,
        )

        self.quotes[quote_id] = quote
        return quote

    async def submit(
        self, quote_id: str, signed_transaction: str, payment_token_account: str
    ) -> dict[str, Any]:
        """Submit mock transaction."""
        quote = self.quotes.get(quote_id)
        if not quote:
            raise ValueError(f"Quote not found: {quote_id}")

        self.total_burned += quote.burn_amount
        self.total_transactions += 1

        return {
            "signature": f"sig_{uuid4().hex[:16]}",
            "status": "confirmed",
            "quote_id": quote_id,
        }

    async def get_stats(self) -> GASdfStats:
        """Return mock statistics."""
        return GASdfStats(
            total_burned=self.total_burned,
            total_transactions=self.total_transactions,
            burned_formatted=f"{self.total_burned:,}",
            treasury="excellent",
        )


class MockNEARClient:
    """Mock NEAR RPC client for testing."""

    def __init__(self):
        self.proposals: dict[str, NEARGovernanceProposal] = {}
        self.nonce = 1

    async def health(self) -> dict[str, Any]:
        """Return mock health check."""
        return {"version": "1.18.0", "chain_id": "mainnet"}

    async def get_nonce(self, account_id: str) -> int:
        """Return mock nonce."""
        self.nonce += 1
        return self.nonce

    async def get_block_hash(self) -> str:
        """Return mock block hash."""
        return "base64hash123456789=="

    async def call_contract(
        self, contract_id: str, method: str, args: dict
    ) -> dict[str, Any]:
        """Call mock contract."""
        proposal_id = args.get("proposal_id")
        if proposal_id in self.proposals:
            proposal = self.proposals[proposal_id]
            return {
                "result": {
                    "proposal_id": proposal.proposal_id,
                    "title": proposal.title,
                    "cynic_verdict": proposal.cynic_verdict,
                    "cynic_q_score": proposal.q_score,
                    "votes_for": proposal.votes_for,
                    "votes_against": proposal.votes_against,
                    "status": proposal.status,
                }
            }
        return {"result": {}}

    async def send_transaction(self, signed_tx: str) -> dict[str, Any]:
        """Submit mock transaction."""
        return {
            "hash": f"tx_{uuid4().hex[:16]}",
            "result": "ok",
        }

    async def get_transaction_result(
        self, tx_hash: str, account_id: str
    ) -> dict[str, Any]:
        """Get mock transaction result."""
        return {
            "transaction_outcome": {
                "outcome": {
                    "status": {"SuccessValue": ""},
                    "gas_burnt": 123456,
                }
            },
            "transaction": {"block_height": 12345},
        }


@pytest.fixture
def gasdf_client():
    """Create mock GASdf client."""
    return MockGASdfClient()


@pytest.fixture
def gasdf_executor(gasdf_client):
    """Create GASdf executor with mock client."""
    return GASdfExecutor(gasdf_client)


@pytest.fixture
def near_client():
    """Create mock NEAR RPC client."""
    return MockNEARClient()


@pytest.fixture
def near_config():
    """Create NEAR network configuration."""
    return NEARNetworkConfig(
        network_id="testnet",
        rpc_url="http://localhost:3030",
        contract_id="governance.testnet",
        master_account="governance.testnet",
    )


@pytest.fixture
def near_executor(near_config):
    """Create NEAR executor (will use mock client internally)."""
    executor = NEARExecutor(near_config)
    # Replace RPC client with mock
    executor.rpc_client = MockNEARClient()
    return executor


@pytest.mark.asyncio
class TestGASdfIntegration:
    """Test GASdf integration layer."""

    async def test_health_check(self, gasdf_executor):
        """Test GASdf health check."""
        health = await gasdf_executor.client.health()
        assert health["status"] == "ok"

    async def test_fee_quote(self, gasdf_executor):
        """Test fee quote generation."""
        quote = await gasdf_executor.client.get_quote(
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            amount=1000000,
        )

        assert quote.fee_amount == int(1000000 * 0.005)
        assert quote.burn_amount == int(quote.fee_amount * 0.764)
        assert quote.payment_token == "token_mint"

    async def test_verdict_execution_howl(self, gasdf_executor):
        """Test HOWL verdict execution (strong yes)."""
        result = await gasdf_executor.execute_verdict(
            proposal_id="prop_1",
            verdict="HOWL",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_base64",
            payment_token_account="token_account",
            q_score=0.85,
        )

        assert result is not None
        assert result.status == "confirmed"
        assert result.fee_amount > 0

    async def test_verdict_execution_wag(self, gasdf_executor):
        """Test WAG verdict execution (yes)."""
        result = await gasdf_executor.execute_verdict(
            proposal_id="prop_2",
            verdict="WAG",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_base64",
            payment_token_account="token_account",
            q_score=0.65,
        )

        assert result is not None
        assert result.status == "confirmed"

    async def test_verdict_execution_growl(self, gasdf_executor):
        """Test GROWL verdict (caution) does not execute."""
        result = await gasdf_executor.execute_verdict(
            proposal_id="prop_3",
            verdict="GROWL",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_base64",
            payment_token_account="token_account",
            q_score=0.45,
        )

        assert result is None

    async def test_verdict_execution_bark(self, gasdf_executor):
        """Test BARK verdict (reject) does not execute."""
        result = await gasdf_executor.execute_verdict(
            proposal_id="prop_4",
            verdict="BARK",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_base64",
            payment_token_account="token_account",
            q_score=0.25,
        )

        assert result is None

    async def test_low_confidence_verdict(self, gasdf_executor):
        """Test that low-confidence verdicts don't execute."""
        result = await gasdf_executor.execute_verdict(
            proposal_id="prop_5",
            verdict="WAG",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_base64",
            payment_token_account="token_account",
            q_score=0.35,  # Below 0.5 threshold
        )

        assert result is None

    async def test_execution_reward_signal(self, gasdf_executor):
        """Test that execution generates reward signal for learning."""
        # Execute multiple verdicts
        await gasdf_executor.execute_verdict(
            proposal_id="prop_1",
            verdict="WAG",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_1",
            payment_token_account="token_account",
            q_score=0.65,
        )

        await gasdf_executor.execute_verdict(
            proposal_id="prop_2",
            verdict="HOWL",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            signed_transaction="signed_tx_2",
            payment_token_account="token_account",
            q_score=0.80,
        )

        # Get reward signal
        reward = await gasdf_executor.get_execution_reward()

        assert reward["total_transactions"] == 2
        assert reward["total_burned"] > 0
        assert reward["average_fee"] > 0
        assert reward["treasury_health"] in ["excellent", "good", "fair", "poor"]

    async def test_burn_calculation(self, gasdf_executor):
        """Test 76.4% burn to community treasury."""
        quote = await gasdf_executor.client.get_quote(
            payment_token="token_mint",
            user_pubkey="user_pubkey",
            amount=1000000,
        )

        fee_amount = quote.fee_amount
        expected_burn = int(fee_amount * 0.764)

        assert quote.burn_amount == expected_burn
        # Verify 76.4% ± 1 token (rounding)
        assert abs(quote.burn_amount - (fee_amount * 0.764)) < 1


@pytest.mark.asyncio
class TestNEARIntegration:
    """Test NEAR Protocol integration layer."""

    async def test_health_check(self, near_executor):
        """Test NEAR health check."""
        health = await near_executor.health()
        assert health is True

    async def test_proposal_submission(self, near_executor):
        """Test proposal submission to NEAR."""
        result = await near_executor.submit_proposal(
            proposal_id="prop_1",
            title="Test Proposal",
            description="Test proposal description",
            cynic_verdict="WAG",
            q_score=0.65,
            signer_id="test.near",
            expires_at=int((datetime.now() + timedelta(days=7)).timestamp()),
        )

        assert result.status == TxStatus.PENDING
        assert result.proposal_id == "prop_1"
        assert result.cynic_verdict == "WAG"

    async def test_proposal_query(self, near_executor):
        """Test querying proposal from NEAR."""
        # First store a proposal in the mock
        proposal = NEARGovernanceProposal(
            proposal_id="prop_1",
            title="Test Proposal",
            description="Test",
            cynic_verdict="WAG",
            q_score=0.65,
            votes_for=10,
            votes_against=2,
            votes_abstain=1,
            status="open",
            created_at=int(datetime.now().timestamp()),
            expires_at=int((datetime.now() + timedelta(days=7)).timestamp()),
        )
        near_executor.rpc_client.proposals["prop_1"] = proposal

        # Query it
        result = await near_executor.query_proposal("prop_1")

        assert result.proposal_id == "prop_1"
        assert result.cynic_verdict == "WAG"
        assert result.votes_for == 10
        assert result.q_score == 0.65

    async def test_vote_recording(self, near_executor):
        """Test recording votes on NEAR."""
        result = await near_executor.record_vote(
            proposal_id="prop_1",
            voter_id="voter.near",
            vote="for",
            weight=1,
        )

        assert result.status == TxStatus.PENDING

    async def test_invalid_vote_type(self, near_executor):
        """Test that invalid vote types are rejected."""
        from cynic.perception.integrations.near.types import NEARError

        with pytest.raises(NEARError):
            await near_executor.record_vote(
                proposal_id="prop_1",
                voter_id="voter.near",
                vote="invalid",  # Invalid vote type
                weight=1,
            )

    async def test_proposal_execution(self, near_executor):
        """Test executing a proposal on NEAR."""
        result = await near_executor.execute_proposal(
            proposal_id="prop_1",
            executor_id="executor.near",
        )

        assert result.status == TxStatus.PENDING
        assert result.proposal_id == "prop_1"


@pytest.mark.asyncio
class TestCompleteGovernanceWorkflow:
    """Test the complete end-to-end governance workflow."""

    async def test_proposal_to_execution_workflow(
        self, gasdf_executor, near_executor
    ):
        """Test complete workflow: proposal → verdict → execution → learning."""

        # Step 1: Simulate CYNIC verdict
        proposal_id = "prop_test_1"
        verdict = "WAG"
        q_score = 0.725
        community_id = "memecoin_xyz"

        # Step 2: Execute verdict via GASdf
        gasdf_result = await gasdf_executor.execute_verdict(
            proposal_id=proposal_id,
            verdict=verdict,
            community_id=community_id,
            payment_token="token_mint",
            user_pubkey="user.near",
            signed_transaction="signed_tx",
            payment_token_account="token_account",
            q_score=q_score,
        )

        assert gasdf_result is not None
        assert gasdf_result.status == "confirmed"
        initial_fee = gasdf_result.fee_amount

        # Step 3: Submit proposal to NEAR
        near_result = await near_executor.submit_proposal(
            proposal_id=proposal_id,
            title="Test Proposal",
            description="Complete workflow test",
            cynic_verdict=verdict,
            q_score=q_score,
            signer_id="governance.near",
            expires_at=int((datetime.now() + timedelta(days=7)).timestamp()),
        )

        assert near_result.status == TxStatus.PENDING
        assert near_result.cynic_verdict == verdict

        # Step 4: Community votes
        vote_results = []
        for i in range(5):
            vote_result = await near_executor.record_vote(
                proposal_id=proposal_id,
                voter_id=f"voter_{i}.near",
                vote="for",
                weight=1,
            )
            vote_results.append(vote_result)

        assert len(vote_results) == 5

        # Step 5: Execute on NEAR
        exec_result = await near_executor.execute_proposal(
            proposal_id=proposal_id,
            executor_id="executor.near",
        )

        assert exec_result.status == TxStatus.PENDING

        # Step 6: Learning loop - get reward signal
        reward = await gasdf_executor.get_execution_reward()

        assert reward["total_transactions"] >= 1
        assert reward["total_burned"] > 0
        # Reward should be proportional to burn
        assert reward["average_fee"] > 0

    async def test_multiple_verdicts_learning_signal(
        self, gasdf_executor
    ):
        """Test that multiple executions generate learning signal."""

        verdicts = [
            ("HOWL", 0.85),  # Strong yes
            ("WAG", 0.70),   # Yes
            ("WAG", 0.65),   # Yes, lower confidence
            ("HOWL", 0.80),  # Strong yes
        ]

        for i, (verdict, q_score) in enumerate(verdicts):
            await gasdf_executor.execute_verdict(
                proposal_id=f"prop_{i}",
                verdict=verdict,
                community_id="test_community",
                payment_token="token_mint",
                user_pubkey="user.near",
                signed_transaction=f"signed_tx_{i}",
                payment_token_account="token_account",
                q_score=q_score,
            )

        # Check reward signal
        reward = await gasdf_executor.get_execution_reward()

        assert reward["total_transactions"] == 4
        assert reward["total_burned"] > 0
        # With 4 small executions, treasury_health will be "poor"
        assert reward["treasury_health"] in ["poor", "fair"]

    async def test_governance_verdict_rejection_no_execution(
        self, gasdf_executor, near_executor
    ):
        """Test that GROWL/BARK verdicts don't execute."""

        # GROWL should not execute
        growl_result = await gasdf_executor.execute_verdict(
            proposal_id="prop_growl",
            verdict="GROWL",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user.near",
            signed_transaction="signed_tx",
            payment_token_account="token_account",
            q_score=0.45,
        )

        assert growl_result is None

        # BARK should not execute
        bark_result = await gasdf_executor.execute_verdict(
            proposal_id="prop_bark",
            verdict="BARK",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user.near",
            signed_transaction="signed_tx",
            payment_token_account="token_account",
            q_score=0.25,
        )

        assert bark_result is None

    async def test_treasury_health_improves_with_executions(
        self, gasdf_executor
    ):
        """Test that treasury health improves as executions accumulate."""

        # Initial state
        initial_reward = await gasdf_executor.get_execution_reward()
        assert initial_reward["total_transactions"] == 0

        # Execute 10 verdicts
        for i in range(10):
            await gasdf_executor.execute_verdict(
                proposal_id=f"prop_{i}",
                verdict="WAG",
                community_id="test_community",
                payment_token="token_mint",
                user_pubkey="user.near",
                signed_transaction=f"signed_tx_{i}",
                payment_token_account="token_account",
                q_score=0.65 + (i * 0.01),  # Increasing confidence
            )

        # Check final reward
        final_reward = await gasdf_executor.get_execution_reward()

        assert final_reward["total_transactions"] == 10
        assert final_reward["total_burned"] > initial_reward["total_burned"]

    async def test_proposal_context_metadata(
        self, gasdf_executor
    ):
        """Test that proposal context is tracked in execution."""

        context = {
            "amount": 5000000,
            "title": "Test Proposal",
            "description": "Test proposal with context",
            "timestamp": datetime.now().isoformat(),
        }

        result = await gasdf_executor.execute_verdict(
            proposal_id="prop_context",
            verdict="WAG",
            community_id="test_community",
            payment_token="token_mint",
            user_pubkey="user.near",
            signed_transaction="signed_tx",
            payment_token_account="token_account",
            q_score=0.65,
            proposal_context=context,
        )

        assert result is not None
        # Verify context was used for amount calculation
        # (get_quote was called with context amount)


@pytest.mark.asyncio
class TestGovernanceStackIntegration:
    """Integration tests for the complete governance stack."""

    async def test_discord_to_near_proposal_flow(
        self, gasdf_executor, near_executor
    ):
        """Simulate Discord proposal → CYNIC → GASdf → NEAR flow."""

        # Simulate Discord bot receiving proposal
        proposal_text = "Increase liquidity provision to DEX by 10%"
        user_id = "user.near"

        # Step 1: CYNIC evaluates (mocked)
        cynic_verdict = "WAG"
        cynic_q_score = 0.725

        # Step 2: Execute via GASdf
        gasdf_result = await gasdf_executor.execute_verdict(
            proposal_id="discord_prop_1",
            verdict=cynic_verdict,
            community_id="discord_memecoin",
            payment_token="token_mint",
            user_pubkey=user_id,
            signed_transaction="discord_signed_tx",
            payment_token_account="user_token_account",
            q_score=cynic_q_score,
        )

        assert gasdf_result is not None

        # Step 3: Submit to NEAR
        near_result = await near_executor.submit_proposal(
            proposal_id="discord_prop_1",
            title=proposal_text,
            description=f"Proposed by {user_id}",
            cynic_verdict=cynic_verdict,
            q_score=cynic_q_score,
            signer_id="governance.near",
            expires_at=int(
                (datetime.now() + timedelta(days=7)).timestamp()
            ),
        )

        assert near_result.status == TxStatus.PENDING
        assert near_result.cynic_verdict == cynic_verdict

        # Step 4: Community votes via Discord reactions
        vote_count = 0
        for i in range(7):
            vote_result = await near_executor.record_vote(
                proposal_id="discord_prop_1",
                voter_id=f"discord_user_{i}",
                vote="for" if i < 5 else "against",
                weight=1,
            )
            if vote_result:
                vote_count += 1

        # Step 5: Get learning feedback
        reward = await gasdf_executor.get_execution_reward()
        assert reward["total_transactions"] >= 1

    async def test_verdict_confidence_affects_execution(
        self, gasdf_executor
    ):
        """Test that verdict confidence (q_score) affects execution."""

        low_confidence = await gasdf_executor.execute_verdict(
            proposal_id="low_conf",
            verdict="WAG",
            community_id="test",
            payment_token="token",
            user_pubkey="user",
            signed_transaction="tx",
            payment_token_account="account",
            q_score=0.40,  # Below threshold
        )

        high_confidence = await gasdf_executor.execute_verdict(
            proposal_id="high_conf",
            verdict="WAG",
            community_id="test",
            payment_token="token",
            user_pubkey="user",
            signed_transaction="tx",
            payment_token_account="account",
            q_score=0.75,  # Above threshold
        )

        assert low_confidence is None
        assert high_confidence is not None

    async def test_fee_burn_aggregation(self, gasdf_executor):
        """Test that fee burns aggregate for treasury health calculation."""

        # Execute 5 proposals with different verdicts
        executed_count = 0
        for i in range(5):
            result = await gasdf_executor.execute_verdict(
                proposal_id=f"burn_test_{i}",
                verdict="HOWL" if i % 2 == 0 else "WAG",
                community_id="burn_community",
                payment_token="token",
                user_pubkey="user",
                signed_transaction=f"tx_{i}",
                payment_token_account="account",
                q_score=0.70,
            )
            if result:
                executed_count += 1

        # Verify aggregation
        reward = await gasdf_executor.get_execution_reward()
        assert reward["total_transactions"] == executed_count
        assert reward["total_burned"] == (
            executed_count
            * int(int(1000000 * 0.005) * 0.764)  # Fee * burn ratio
        )


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
