"""Live NEAR testnet integration tests

NOTE: These tests require:
- NEAR testnet account with funds (for gas)
- Contract deployed to testnet
- Environment variables: NEAR_ACCOUNT_ID, NEAR_PRIVATE_KEY, NEAR_CONTRACT_ID
"""

import pytest
pytestmark = pytest.mark.skip(reason="Old architecture: module imports not available in V5")

# Block all imports that would fail
pytest.skip("Skipping old architecture test module", allow_module_level=True)


import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - governance_bot module not found")

import os

from governance_bot.near_integration import NearRpcClient, TransactionSigner


@pytest.fixture
def near_config():
    """Load NEAR configuration from environment"""
    return {
        "account_id": os.getenv("NEAR_ACCOUNT_ID"),
        "private_key": os.getenv("NEAR_PRIVATE_KEY"),
        "contract_id": os.getenv("NEAR_CONTRACT_ID"),
        "rpc_url": os.getenv("NEAR_RPC_URL", "https://rpc.testnet.near.org")
    }


class TestNearIntegrationLive:
    """Live NEAR testnet integration tests"""

    @pytest.mark.asyncio
    async def test_account_exists_on_testnet(self, near_config):
        """Test that configured NEAR account exists on testnet"""
        async with NearRpcClient(near_config["rpc_url"]) as client:
            # Should not raise exception
            nonce = await client.get_account_nonce(near_config["account_id"])
            assert nonce >= 0

    @pytest.mark.asyncio
    async def test_contract_exists_on_testnet(self, near_config):
        """Test that deployed contract exists on testnet"""
        async with NearRpcClient(near_config["rpc_url"]) as client:
            # Try to call view method
            try:
                result = await client._call_rpc("query", {
                    "request_type": "view_code",
                    "account_id": near_config["contract_id"],
                    "finality": "final"
                })

                assert result.get("result") is not None
            except Exception as e:
                pytest.fail(f"Contract not accessible: {e}")

    @pytest.mark.asyncio
    async def test_create_proposal_transaction(self, near_config):
        """Test creating a proposal via transaction"""
        signer = TransactionSigner(
            near_config["private_key"],
            near_config["account_id"]
        )

        async with NearRpcClient(near_config["rpc_url"]) as client:
            # Get current nonce
            nonce = await client.get_account_nonce(near_config["account_id"])

            # Get block hash
            block_hash = await client.get_block_hash()

            # Create transaction
            actions = [{
                "type": "FunctionCall",
                "params": {
                    "methodName": "create_proposal",
                    "args": {
                        "id": "test_proposal_001",
                        "title": "Test Proposal",
                        "description": "Integration test proposal"
                    },
                    "gas": 30000000000000,
                    "deposit": "0"
                }
            }]

            transaction = signer.create_transaction(
                near_config["contract_id"],
                actions,
                nonce,
                block_hash
            )

            assert transaction["signer_id"] == near_config["account_id"]
            assert transaction["receiver_id"] == near_config["contract_id"]

    @pytest.mark.asyncio
    async def test_vote_on_proposal_transaction(self, near_config):
        """Test voting on a proposal via transaction"""
        signer = TransactionSigner(
            near_config["private_key"],
            near_config["account_id"]
        )

        async with NearRpcClient(near_config["rpc_url"]) as client:
            nonce = await client.get_account_nonce(near_config["account_id"])
            block_hash = await client.get_block_hash()

            actions = [{
                "type": "FunctionCall",
                "params": {
                    "methodName": "vote",
                    "args": {
                        "proposal_id": "test_proposal_001",
                        "choice": True
                    },
                    "gas": 30000000000000,
                    "deposit": "0"
                }
            }]

            transaction = signer.create_transaction(
                near_config["contract_id"],
                actions,
                nonce,
                block_hash
            )

            assert transaction["nonce"] > 0

    @pytest.mark.asyncio
    async def test_get_block_hash(self, near_config):
        """Test fetching current block hash"""
        async with NearRpcClient(near_config["rpc_url"]) as client:
            block_hash = await client.get_block_hash()

            # Block hash should be non-empty
            assert len(block_hash) > 0
