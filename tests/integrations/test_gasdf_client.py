"""Tests for GASdf client."""
from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.kernel.organism.perception.integrations.gasdf.client import GASdfClient
from cynic.kernel.organism.perception.integrations.gasdf.types import (
    GASdfError,
    GASdfQuote,
    GASdfStats,
)


class TestGASdfClient:
    """Test suite for GASdfClient."""

    async def test_health_check_success(self) -> None:
        """Test successful health check."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {"status": "healthy"}

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.get.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            result = await client.health()

            assert result == {"status": "healthy"}
            mock_client.get.assert_called_once_with(
                f"{client.base_url}/health"
            )

    async def test_health_check_error(self) -> None:
        """Test health check error handling."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 500
            mock_response.text = "Internal Server Error"

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.get.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            with pytest.raises(GASdfError):
                await client.health()

    async def test_get_tokens(self) -> None:
        """Test get_tokens successfully retrieves token list."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {
                "note": "HolDex-verified community tokens...",
                "tokens": [
                    {"symbol": "USDC", "mint": "addr123"},
                    {"symbol": "SOL", "mint": "addr456"},
                ],
            }

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.get.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            result = await client.get_tokens()

            assert len(result) == 2
            assert result[0]["symbol"] == "USDC"

    async def test_get_quote(self) -> None:
        """Test get_quote returns properly structured GASdfQuote."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {
                "quote_id": "q123",
                "payment_token": "usdc_addr",
                "fee_amount": 5000,
                "burn_amount": 3820,
                "user_pubkey": "user_pub",
            }

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.post.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            result = await client.get_quote(
                payment_token="usdc_addr",
                user_pubkey="user_pub",
                amount=1000000,
            )

            assert isinstance(result, GASdfQuote)
            assert result.quote_id == "q123"
            assert result.fee_amount == 5000
            assert result.burn_amount == 3820

    async def test_submit_transaction(self) -> None:
        """Test submit transaction successfully."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {
                "signature": "sig_abc123",
                "status": "confirmed",
            }

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.post.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            result = await client.submit(
                quote_id="q123",
                signed_transaction="base64_signed",
                payment_token_account="token_acc",
            )

            assert result["signature"] == "sig_abc123"
            assert result["status"] == "confirmed"

    async def test_get_stats(self) -> None:
        """Test get_stats returns properly structured GASdfStats."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {
                "totalBurned": 1000000,
                "totalTransactions": 100,
                "burnedFormatted": "1000000 ASDF",
                "treasury": {"balance": 5000000},
            }

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.get.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            result = await client.get_stats()

            assert isinstance(result, GASdfStats)
            assert result.total_burned == 1000000
            assert result.total_transactions == 100

    async def test_error_handling_non_200_status(self) -> None:
        """Test error handling for non-200 responses."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 400
            mock_response.text = "Bad Request"

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.post.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            with pytest.raises(GASdfError) as exc_info:
                await client.get_quote(
                    payment_token="token",
                    user_pubkey="pubkey",
                    amount=1000,
                )
            assert "400" in str(exc_info.value)

    async def test_quote_error_handling(self) -> None:
        """Test error handling in quote request."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 404
            mock_response.text = "Token not found"

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.post.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            with pytest.raises(GASdfError):
                await client.get_quote(
                    payment_token="invalid",
                    user_pubkey="pubkey",
                    amount=1000,
                )

    async def test_submit_error_handling(self) -> None:
        """Test error handling in submit request."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.status_code = 500
            mock_response.text = "Transaction failed"

            mock_client = AsyncMock()
            mock_client.__aenter__.return_value = mock_client
            mock_client.__aexit__.return_value = None
            mock_client.post.return_value = mock_response

            mock_client_class.return_value = mock_client

            client = GASdfClient()
            with pytest.raises(GASdfError):
                await client.submit(
                    quote_id="q123",
                    signed_transaction="sig",
                    payment_token_account="token_acc",
                )
