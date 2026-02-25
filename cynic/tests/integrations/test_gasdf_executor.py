"""Tests for GASdf executor."""
from __future__ import annotations

from unittest.mock import AsyncMock

import pytest

from cynic.integrations.gasdf.client import GASdfClient
from cynic.integrations.gasdf.executor import GASdfExecutor
from cynic.integrations.gasdf.types import GASdfError, GASdfExecutionResult, GASdfQuote


class TestGASdfExecutor:
    """Test suite for GASdfExecutor."""

    @pytest.fixture
    def mock_client(self) -> AsyncMock:
        """Create a mock GASdfClient."""
        return AsyncMock(spec=GASdfClient)

    async def test_execute_approved_verdict(self, mock_client: AsyncMock) -> None:
        """Test execution flow for APPROVED verdict."""
        # Setup mock responses
        mock_quote = GASdfQuote(
            quote_id="q123",
            payment_token="usdc",
            fee_amount=5000,
            burn_amount=3820,
            user_pubkey="user_pub",
        )
        mock_client.get_quote.return_value = mock_quote
        mock_client.submit.return_value = {
            "signature": "sig_abc",
            "status": "confirmed",
        }

        executor = GASdfExecutor(mock_client)
        result = await executor.execute_verdict(
            proposal_id="prop_1",
            verdict="APPROVED",
            community_id="com_1",
            payment_token="usdc",
            user_pubkey="user_pub",
            signed_transaction="base64_tx",
            payment_token_account="token_acc",
        )

        assert result is not None
        assert isinstance(result, GASdfExecutionResult)
        assert result.signature == "sig_abc"
        assert result.status == "confirmed"
        assert result.fee_amount == 5000
        assert result.quote_id == "q123"

    async def test_execute_tentative_approve_verdict(
        self, mock_client: AsyncMock
    ) -> None:
        """Test execution flow for TENTATIVE_APPROVE verdict."""
        mock_quote = GASdfQuote(
            quote_id="q456",
            payment_token="usdc",
            fee_amount=3000,
            burn_amount=2292,
            user_pubkey="user_pub",
        )
        mock_client.get_quote.return_value = mock_quote
        mock_client.submit.return_value = {
            "signature": "sig_def",
            "status": "pending",
        }

        executor = GASdfExecutor(mock_client)
        result = await executor.execute_verdict(
            proposal_id="prop_2",
            verdict="TENTATIVE_APPROVE",
            community_id="com_1",
            payment_token="usdc",
            user_pubkey="user_pub",
            signed_transaction="base64_tx",
            payment_token_account="token_acc",
        )

        assert result is not None
        assert result.signature == "sig_def"
        assert result.status == "pending"

    async def test_skip_caution_verdict(self, mock_client: AsyncMock) -> None:
        """Test that CAUTION verdicts are not executed."""
        executor = GASdfExecutor(mock_client)
        result = await executor.execute_verdict(
            proposal_id="prop_3",
            verdict="CAUTION",
            community_id="com_1",
            payment_token="usdc",
            user_pubkey="user_pub",
            signed_transaction="base64_tx",
            payment_token_account="token_acc",
        )

        assert result is None
        mock_client.get_quote.assert_not_called()
        mock_client.submit.assert_not_called()

    async def test_skip_reject_verdict(self, mock_client: AsyncMock) -> None:
        """Test that REJECT verdicts are not executed."""
        executor = GASdfExecutor(mock_client)
        result = await executor.execute_verdict(
            proposal_id="prop_4",
            verdict="REJECT",
            community_id="com_1",
            payment_token="usdc",
            user_pubkey="user_pub",
            signed_transaction="base64_tx",
            payment_token_account="token_acc",
        )

        assert result is None
        mock_client.get_quote.assert_not_called()
        mock_client.submit.assert_not_called()

    async def test_execution_result_structure(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that execution result has correct structure."""
        mock_quote = GASdfQuote(
            quote_id="q_struct",
            payment_token="sol",
            fee_amount=10000,
            burn_amount=7640,
            user_pubkey="pubkey",
        )
        mock_client.get_quote.return_value = mock_quote
        mock_client.submit.return_value = {
            "signature": "sig_struct",
            "status": "confirmed",
        }

        executor = GASdfExecutor(mock_client)
        result = await executor.execute_verdict(
            proposal_id="prop_struct",
            verdict="APPROVED",
            community_id="com_1",
            payment_token="sol",
            user_pubkey="pubkey",
            signed_transaction="tx_data",
            payment_token_account="token_acc",
        )

        assert result is not None
        assert hasattr(result, "signature")
        assert hasattr(result, "status")
        assert hasattr(result, "fee_amount")
        assert hasattr(result, "fee_token")
        assert hasattr(result, "quote_id")

    async def test_error_handling_on_quote_failure(
        self, mock_client: AsyncMock
    ) -> None:
        """Test error propagation when quote fails."""
        mock_client.get_quote.side_effect = GASdfError("Quote failed")

        executor = GASdfExecutor(mock_client)
        with pytest.raises(GASdfError) as exc_info:
            await executor.execute_verdict(
                proposal_id="prop_error",
                verdict="APPROVED",
                community_id="com_1",
                payment_token="usdc",
                user_pubkey="user_pub",
                signed_transaction="tx",
                payment_token_account="acc",
            )
        assert "Execution failed" in str(exc_info.value)

    async def test_error_handling_on_submit_failure(
        self, mock_client: AsyncMock
    ) -> None:
        """Test error propagation when submit fails."""
        mock_quote = GASdfQuote(
            quote_id="q_err",
            payment_token="usdc",
            fee_amount=5000,
            burn_amount=3820,
            user_pubkey="user_pub",
        )
        mock_client.get_quote.return_value = mock_quote
        mock_client.submit.side_effect = GASdfError("Submit failed")

        executor = GASdfExecutor(mock_client)
        with pytest.raises(GASdfError) as exc_info:
            await executor.execute_verdict(
                proposal_id="prop_submit_err",
                verdict="APPROVED",
                community_id="com_1",
                payment_token="usdc",
                user_pubkey="user_pub",
                signed_transaction="tx",
                payment_token_account="acc",
            )
        assert "Execution failed" in str(exc_info.value)
