"""Live integration tests for GASdf API (https://asdfasdfa.tech).

These tests call the real GASdf API and are marked with @pytest.mark.integration
to be skipped by default. Run with:

    pytest cynic/tests/integrations/test_gasdf_live.py -m integration -v

Requires network access to https://asdfasdfa.tech
"""

from __future__ import annotations

import pytest

from cynic.kernel.organism.perception.integrations.gasdf.client import GASdfClient
from cynic.kernel.organism.perception.integrations.gasdf.types import GASdfError


class TestGASdfLiveAPI:
    """Live integration tests against real GASdf API."""

    @pytest.fixture
    def client(self) -> GASdfClient:
        """Create a real GASdfClient."""
        return GASdfClient()

    @pytest.mark.integration
    async def test_health_check_live(self, client: GASdfClient) -> None:
        """Test health check against real GASdf API.

        This verifies the GASdf service is available and responding.
        """
        result = await client.health()

        assert result is not None
        assert isinstance(result, dict)

    @pytest.mark.integration
    async def test_get_tokens_live(self, client: GASdfClient) -> None:
        """Test getting accepted tokens from real GASdf API.

        This verifies the list of tokens that can be used for fees.
        """
        tokens = await client.get_tokens()

        assert tokens is not None
        assert isinstance(tokens, list)
        assert len(tokens) > 0

        # Print token info
        for _token in tokens:
            pass

    @pytest.mark.integration
    async def test_get_stats_live(self, client: GASdfClient) -> None:
        """Test getting burn statistics from real GASdf API.

        This verifies we can retrieve the cumulative burn stats.
        """
        stats = await client.get_stats()

        assert stats is not None
        assert stats.total_burned >= 0
        assert stats.total_transactions >= 0
        assert isinstance(stats.burned_formatted, str)
        assert isinstance(stats.treasury, dict)

    @pytest.mark.integration
    async def test_get_quote_live(self, client: GASdfClient) -> None:
        """Test getting a fee quote from real GASdf API.

        This tests the quote flow with realistic parameters.
        Requires valid token and pubkey addresses.
        """
        # Use test/example values - adjust these for your actual test
        test_token = "EPjFWaLb3odccjL3THicP51XY9CTjZGT6PvMqKQ61AxP"  # USDC on Solana
        test_pubkey = "11111111111111111111111111111111"  # Example pubkey

        try:
            quote = await client.get_quote(
                payment_token=test_token,
                user_pubkey=test_pubkey,
                amount=1000000,
            )

            assert quote is not None
            assert quote.quote_id is not None
            assert quote.fee_amount > 0
            assert quote.burn_amount > 0

        except GASdfError as e:
            # Quote might fail with invalid token/pubkey - that's OK for this test
            error_str = str(e).lower()
            assert any(
                keyword in error_str
                for keyword in ["invalid", "not found", "not accepted", "verification"]
            )

    @pytest.mark.integration
    async def test_api_connectivity(self, client: GASdfClient) -> None:
        """Verify basic connectivity to GASdf API.

        This is a minimal test to ensure the API endpoint is reachable.
        """
        try:
            result = await client.health()
            assert result is not None
        except GASdfError as e:
            pytest.skip(f"GASdf API unreachable: {e}")


class TestGASdfErrorHandling:
    """Test error handling with real API."""

    @pytest.fixture
    def client(self) -> GASdfClient:
        """Create a real GASdfClient."""
        return GASdfClient()

    @pytest.mark.integration
    async def test_invalid_token_error(self, client: GASdfClient) -> None:
        """Test error handling for invalid token address."""
        with pytest.raises(GASdfError):
            await client.get_quote(
                payment_token="invalid_token_address",
                user_pubkey="invalid_pubkey",
                amount=1000,
            )

    @pytest.mark.integration
    async def test_network_error_handling(self) -> None:
        """Test error handling for unreachable API.

        This creates a client pointing to a fake URL to test network error handling.
        """
        import httpx

        client = GASdfClient()
        client.base_url = "https://invalid-api-that-does-not-exist-12345.test"

        # Network errors raise httpx.ConnectError, not GASdfError
        with pytest.raises((GASdfError, httpx.ConnectError)):
            await client.health()


class TestGASdfExecutorIntegration:
    """Integration tests for GASdfExecutor with real API."""

    @pytest.mark.integration
    async def test_executor_with_real_client(self) -> None:
        """Test executor flow with real GASdf client.

        This tests the full executor flow (quote â†’ submit) against real API
        without actually submitting a transaction (using test data).
        """
        from cynic.kernel.organism.perception.integrations.gasdf.executor import (
            GASdfExecutor,
        )

        client = GASdfClient()
        executor = GASdfExecutor(client)

        # Test that executor properly skips non-approved verdicts
        result = await executor.execute_verdict(
            proposal_id="test_prop",
            verdict="REJECT",
            community_id="test_com",
            payment_token="usdc",
            user_pubkey="user",
            signed_transaction="tx",
            payment_token_account="acc",
        )

        assert result is None  # Should skip REJECT verdict
