#!/usr/bin/env python3
"""
Wallet Behavior Integration for B&C Personality Card Minting

This module provides a simple interface for B&C to check if a wallet
passes the "verified human" gate during card minting.

Usage in B&C /mint-permit:
  from wallet_behavior_integration import check_wallet_verified_human

  @app.post("/mint-permit")
  async def mint_permit(body: MintPermitRequest):
      wallet = body.wallet
      game_result = body.game_result

      # Check wallet behavior
      verdict, profile = check_wallet_verified_human(wallet)

      return {
          "mint_authorized": True,
          "verified_by": "game + wallet_behavior" if verdict else "game_only",
          "authenticity_score": profile.authenticity_score if profile else None,
          "metadata": {
              "wallet_age_days": profile.wallet_age_days if profile else None,
              "token_count": profile.token_count if profile else None,
          }
      }
"""

import logging
import os
from typing import Tuple, Optional

from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_scorer import WalletProfile

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] b2c_integration: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


# Gate threshold: φ⁻¹ = 0.618
VERIFIED_HUMAN_THRESHOLD = 0.618


def check_wallet_verified_human(
    wallet: str, timeout: int = 30
) -> Tuple[bool, Optional[WalletProfile]]:
    """
    Check if a wallet passes the verified human gate.

    Args:
        wallet: Solana wallet address (base58)
        timeout: Request timeout in seconds (default 30)

    Returns:
        Tuple of (is_verified_human: bool, profile: WalletProfile or None)

        is_verified_human: True if authenticity_score >= 0.618
        profile: Full WalletProfile with scoring details, or None on error

    Error Handling:
        - If Helius API is unavailable or rate limited, returns (False, None)
        - Logs error details for debugging
        - B&C should treat (False, None) as "wallet behavior unavailable, allow minting with game_only verification"
    """
    try:
        # Read from env var; never hardcoded in source
        api_key = os.getenv("HELIUS_API_KEY")
        if not api_key:
            logger.warning(
                "HELIUS_API_KEY not set. Wallet behavior check disabled. "
                "Set env var to enable: export HELIUS_API_KEY=..."
            )
            return False, None

        collector = HeliusWalletCollector(api_key=api_key, timeout=timeout)
        profile = collector.collect_wallet_profile(wallet)

        if profile is None:
            logger.warning(f"Failed to collect profile for {wallet}")
            return False, None

        is_verified = profile.authenticity_score >= VERIFIED_HUMAN_THRESHOLD

        logger.info(
            f"Wallet {wallet}: score={profile.authenticity_score:.3f}, "
            f"verified={is_verified}"
        )

        return is_verified, profile

    except Exception as e:
        logger.error(f"Wallet behavior check failed for {wallet}: {e}")
        return False, None


def mint_permit_response(
    wallet: str, game_verified: bool, timeout: int = 30
) -> dict:
    """
    Generate full B&C /mint-permit response.

    Args:
        wallet: Solana wallet address
        game_verified: Whether the game result was verified (game_result.verified)
        timeout: Helius timeout in seconds

    Returns:
        Dict with mint_authorized, verified_by, authenticity_score, metadata
    """
    if not game_verified:
        return {"mint_authorized": False, "reason": "game_not_verified"}

    is_verified_human, profile = check_wallet_verified_human(wallet, timeout=timeout)

    response = {
        "mint_authorized": True,
        "verified_by": "game + wallet_behavior" if is_verified_human else "game_only",
        "authenticity_score": profile.authenticity_score if profile else None,
        "metadata": {
            "wallet_age_days": profile.wallet_age_days if profile else None,
            "token_count": profile.token_count if profile else None,
            "program_count": profile.program_count if profile else None,
        },
    }

    return response


if __name__ == "__main__":
    # Example usage / testing
    import sys

    if len(sys.argv) < 2:
        print("Usage: python wallet_behavior_integration.py <wallet_address>")
        sys.exit(1)

    wallet = sys.argv[1]

    is_verified, profile = check_wallet_verified_human(wallet)

    if profile:
        print(f"\n=== Wallet Verification Result ===")
        print(f"Address: {profile.wallet_address}")
        print(f"Score: {profile.authenticity_score:.3f}")
        print(f"Verified Human: {is_verified}")
        print(f"\nDetails:")
        print(f"  Age: {profile.wallet_age_days} days")
        print(f"  Tokens: {profile.token_count}")
        print(f"  Programs: {profile.program_count}")
        print(f"  Activity Span: {profile.activity_span_days} days")
        print(f"  Transactions: {profile.total_transactions}")
    else:
        print(f"Failed to collect profile for {wallet}")
        sys.exit(1)
