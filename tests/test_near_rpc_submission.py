"""Tests for NEAR RPC transaction submission"""

import pytest
from governance_bot.near_integration import NearRpcClient
from unittest.mock import AsyncMock, MagicMock, patch


class TestNearRpcSubmission:
    """Test NEAR RPC interaction"""

    @pytest.mark.asyncio
    async def test_get_nonce_from_network(self):
        """Test fetching account nonce from NEAR network"""
        client = NearRpcClient(rpc_url="https://rpc.testnet.near.org")

        # Mock the RPC response
        with patch.object(client, '_call_rpc', new_callable=AsyncMock) as mock_rpc:
            mock_rpc.return_value = {
                "result": {
                    "nonce": 42
                }
            }

            nonce = await client.get_account_nonce("test.testnet")
            assert nonce == 42

    @pytest.mark.asyncio
    async def test_submit_transaction(self):
        """Test submitting transaction to NEAR"""
        client = NearRpcClient(rpc_url="https://rpc.testnet.near.org")

        transaction = {
            "signer_id": "test.testnet",
            "public_key": "ed25519:...",
            "nonce": 1,
            "receiver_id": "governance.testnet",
            "block_hash": "block123",
            "actions": []
        }

        with patch.object(client, '_call_rpc', new_callable=AsyncMock) as mock_rpc:
            mock_rpc.return_value = {
                "result": {
                    "hash": "txhash123"
                }
            }

            tx_hash = await client.send_transaction(transaction)
            assert tx_hash == "txhash123"

    @pytest.mark.asyncio
    async def test_poll_transaction_confirmation(self):
        """Test polling for transaction confirmation"""
        client = NearRpcClient(rpc_url="https://rpc.testnet.near.org")

        with patch.object(client, '_call_rpc', new_callable=AsyncMock) as mock_rpc:
            # First call: pending
            mock_rpc.side_effect = [
                {"result": None},
                {"result": None},
                # Third call: confirmed
                {"result": {
                    "status": {"SuccessValue": ""}
                }}
            ]

            status = await client.poll_transaction_status(
                "txhash123",
                max_polls=3,
                poll_interval=0.1
            )

            assert status["status"]["SuccessValue"] == ""

    @pytest.mark.asyncio
    async def test_transaction_timeout(self):
        """Test transaction confirmation timeout"""
        client = NearRpcClient(rpc_url="https://rpc.testnet.near.org")

        with patch.object(client, '_call_rpc', new_callable=AsyncMock) as mock_rpc:
            mock_rpc.return_value = {"result": None}  # Always pending

            with pytest.raises(TimeoutError):
                await client.poll_transaction_status(
                    "txhash123",
                    max_polls=2,
                    poll_interval=0.05
                )

    @pytest.mark.asyncio
    async def test_get_block_hash(self):
        """Test fetching block hash for transactions"""
        client = NearRpcClient(rpc_url="https://rpc.testnet.near.org")

        with patch.object(client, '_call_rpc', new_callable=AsyncMock) as mock_rpc:
            mock_rpc.return_value = {
                "result": {
                    "header": {
                        "hash": "blockhash123"
                    }
                }
            }

            block_hash = await client.get_block_hash()
            assert block_hash == "blockhash123"
