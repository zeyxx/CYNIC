#!/usr/bin/env python3
"""
Wallet Behavior Analysis — Helius Data Collector

Fetches on-chain wallet data from Helius API and converts to WalletProfile.
"""

import os
import logging
from typing import Optional, Set, Dict
from datetime import datetime
import requests

from wallet_behavior_scorer import WalletProfile, score_wallet

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] helius_collector: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


class HeliusWalletCollector:
    """Collect wallet behavior data from Helius API."""

    def __init__(self, api_key: Optional[str] = None, timeout: int = 30):
        """Initialize Helius client.

        Args:
            api_key: Helius API key. If None, reads from HELIUS_API_KEY env var.
            timeout: Request timeout in seconds.
        """
        self.api_key = api_key or os.getenv("HELIUS_API_KEY")
        if not self.api_key:
            raise ValueError(
                "Helius API key not provided. "
                "Set HELIUS_API_KEY env var or pass api_key parameter."
            )
        self.rpc_url = "https://mainnet.helius-rpc.com"
        self.rest_url = "https://api.helius.xyz/v1"
        self.timeout = timeout

    def _rpc_call(self, method: str, *args, **kwargs) -> dict:
        """Call Helius JSON-RPC API.

        Args:
            method: JSON-RPC method name (e.g., "getBalance")
            *args: Positional parameters for the RPC method
            **kwargs: Named parameters (converted to config object if needed)

        Returns:
            API response as dict

        Raises:
            requests.RequestException: On API error
        """
        url = f"{self.rpc_url}?api-key={self.api_key}"
        # Build params: positional args first, then config dict if kwargs exist
        params = list(args)
        if kwargs:
            params.append(kwargs)

        payload = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }
        try:
            response = requests.post(url, json=payload, timeout=self.timeout)
            response.raise_for_status()
            data = response.json()
            if "error" in data:
                raise ValueError(f"Helius error: {data['error']}")
            return data.get("result", {})
        except requests.RequestException as e:
            logger.error(f"Helius RPC error calling {method}: {e}")
            raise

    def _rest_call(self, endpoint: str, params: Optional[dict] = None) -> dict:
        """Call Helius REST API.

        Args:
            endpoint: REST endpoint path (e.g., "/wallet/ABC123/balances")
            params: Query parameters dict

        Returns:
            API response as dict

        Raises:
            requests.RequestException: On API error
        """
        url = f"{self.rest_url}{endpoint}"
        query_params = {"api-key": self.api_key}
        if params:
            query_params.update(params)

        try:
            response = requests.get(url, params=query_params, timeout=self.timeout)
            response.raise_for_status()
            return response.json()
        except requests.RequestException as e:
            logger.error(f"Helius REST error calling {endpoint}: {e}")
            raise

    def get_balance(self, wallet: str) -> int:
        """Get native SOL balance in lamports."""
        result = self._rpc_call("getBalance", wallet)
        return result.get("value", 0)

    def get_token_balances(self, wallet: str) -> list:
        """Get SPL token balances."""
        result = self._rest_call(f"/wallet/{wallet}/balances")
        return result.get("tokens", [])

    def get_transaction_history(
        self, wallet: str, limit: int = 100
    ) -> list:
        """Get transaction history (parsed mode)."""
        result = self._rest_call(f"/wallet/{wallet}/history", {"limit": limit})
        return result.get("transactions", [])

    def get_account_info(self, wallet: str) -> dict:
        """Get account metadata."""
        result = self._rpc_call("getAccountInfo", wallet, {"encoding": "jsonParsed"})
        return result

    def collect_wallet_profile(self, wallet: str) -> Optional[WalletProfile]:
        """
        Collect all data for a wallet and build WalletProfile.

        Args:
            wallet: Solana wallet address (base58)

        Returns:
            Scored WalletProfile, or None on error
        """
        try:
            logger.info(f"Collecting data for {wallet}...")

            # Fetch data (in parallel would be faster, but Helius has rate limits)
            balance = self.get_balance(wallet)
            token_balances = self.get_token_balances(wallet)
            transactions = self.get_transaction_history(wallet, limit=200)
            account_info = self.get_account_info(wallet)

            # Parse Tier 1: Age
            wallet_age_days = self._compute_age(account_info, transactions)

            # Parse Tier 2: Diversity
            token_count = len(token_balances)
            program_count = self._count_programs(transactions)
            unique_swap_pairs = self._count_swap_pairs(transactions)
            single_token_pct = self._compute_single_token_pct(transactions)

            # Parse Tier 3: Temporal
            activity_span_days = self._compute_activity_span(transactions)
            total_transactions = len(transactions)
            transaction_density = (
                total_transactions / activity_span_days
                if activity_span_days > 0
                else 0.0
            )
            gap_max_days = self._compute_max_gap(transactions)

            # Parse Tier 4: Anomalies
            all_txs_same_hour = self._check_same_hour(transactions)
            recent_whale_flag = (
                wallet_age_days < 1 and balance > 100 * 10**9
            )  # > 100 SOL
            transaction_frequency_anomaly = self._check_frequency_anomaly(transactions)

            # Build profile
            profile = WalletProfile(
                wallet_address=wallet,
                wallet_age_days=wallet_age_days,
                token_count=token_count,
                program_count=program_count,
                unique_swap_pairs=unique_swap_pairs,
                activity_span_days=activity_span_days,
                total_transactions=total_transactions,
                transaction_density=transaction_density,
                gap_max_days=gap_max_days,
                all_txs_same_hour=all_txs_same_hour,
                single_token_pct=single_token_pct,
                recent_whale_flag=recent_whale_flag,
                transaction_frequency_anomaly=transaction_frequency_anomaly,
            )

            # Score
            profile = score_wallet(profile)

            logger.info(
                f"Profile complete: "
                f"age={profile.wallet_age_days}d, "
                f"tokens={profile.token_count}, "
                f"programs={profile.program_count}, "
                f"score={profile.authenticity_score:.3f}"
            )

            return profile

        except Exception as e:
            logger.error(f"Failed to collect profile for {wallet}: {e}")
            return None

    # ========================================================================
    # Private helpers
    # ========================================================================

    @staticmethod
    def _compute_age(account_info: dict, transactions: list) -> int:
        """Compute wallet age in days.

        Tries multiple sources:
        1. Account creation from account_info (if available)
        2. First transaction timestamp from history
        """
        now = datetime.utcnow().timestamp()

        # Try account info
        if "executable" in account_info:
            # Fallback: assume ~4 days per transaction as proxy
            age_estimate = len(transactions) * 4 / 30  # rough
            return max(1, int(age_estimate))

        # Try transactions
        if transactions:
            first_tx_timestamp = transactions[-1].get("timestamp", now)
            age_seconds = now - first_tx_timestamp
            age_days = age_seconds / 86400
            return max(1, int(age_days))

        # Fallback: assume 1 day (unknown)
        return 1

    @staticmethod
    def _count_programs(transactions: list) -> int:
        """Count unique programs called."""
        programs = set()
        for tx in transactions:
            if "instructions" in tx:
                for instr in tx["instructions"]:
                    program = instr.get("programId")
                    if program:
                        programs.add(program)
        return len(programs)

    @staticmethod
    def _count_swap_pairs(transactions: list) -> int:
        """Count unique [token_a, token_b] swap pairs.

        Looks for Swap instruction type.
        """
        pairs = set()
        for tx in transactions:
            if tx.get("type") == "SWAP":
                # Extract token pair from token transfers in this tx
                # This is a simplified heuristic
                if "description" in tx:
                    desc = tx["description"]
                    if "swapped" in desc:
                        # Rough parse: "swapped X tokenA for Y tokenB"
                        # For now, just count as 1 pair per SWAP
                        pairs.add(str(tx.get("signature")))
        return len(pairs)

    @staticmethod
    def _compute_single_token_pct(transactions: list) -> float:
        """Compute % of interactions on single dominant token.

        Returns: 0-100 representing concentration on most-common token.
        """
        if not transactions:
            return 0.0

        # Count token interactions (simplified: look at token_transfers)
        token_counts: Dict[str, int] = {}
        for tx in transactions:
            if "tokenTransfers" in tx:
                for xfer in tx["tokenTransfers"]:
                    mint = xfer.get("mint", "unknown")
                    token_counts[mint] = token_counts.get(mint, 0) + 1

        if not token_counts:
            return 0.0

        max_count = max(token_counts.values())
        total_count = sum(token_counts.values())
        pct = (max_count / total_count) * 100.0
        return min(100.0, pct)

    @staticmethod
    def _compute_activity_span(transactions: list) -> int:
        """Compute span of activity in days (first to last tx)."""
        if len(transactions) < 2:
            return 1

        timestamps = [tx.get("timestamp", 0) for tx in transactions if "timestamp" in tx]
        if len(timestamps) < 2:
            return 1

        span_seconds = max(timestamps) - min(timestamps)
        span_days = span_seconds / 86400
        return max(1, int(span_days))

    @staticmethod
    def _compute_max_gap(transactions: list) -> int:
        """Compute longest gap (in days) between consecutive transactions."""
        if len(transactions) < 2:
            return 0

        timestamps = sorted([tx.get("timestamp", 0) for tx in transactions if "timestamp" in tx])
        if len(timestamps) < 2:
            return 0

        gaps = []
        for i in range(len(timestamps) - 1):
            gap_seconds = timestamps[i + 1] - timestamps[i]
            gap_days = gap_seconds / 86400
            gaps.append(gap_days)

        return int(max(gaps)) if gaps else 0

    @staticmethod
    def _check_same_hour(transactions: list) -> bool:
        """Check if all transactions occurred within 1 hour (bot signature)."""
        if len(transactions) < 2:
            return False

        timestamps = [tx.get("timestamp", 0) for tx in transactions if "timestamp" in tx]
        if len(timestamps) < 2:
            return False

        span_seconds = max(timestamps) - min(timestamps)
        return span_seconds < 3600  # < 1 hour

    @staticmethod
    def _check_frequency_anomaly(transactions: list) -> bool:
        """Check for transaction frequency anomaly (> 100 txs in 1 hour)."""
        if len(transactions) < 100:
            return False

        timestamps = [tx.get("timestamp", 0) for tx in transactions if "timestamp" in tx]
        if len(timestamps) < 100:
            return False

        # Check sliding 1-hour windows
        for i in range(len(timestamps) - 99):
            window_start = timestamps[i]
            window_end = window_start + 3600
            txs_in_window = sum(1 for t in timestamps if window_start <= t < window_end)
            if txs_in_window > 100:
                return True

        return False


if __name__ == "__main__":
    # Example usage (requires HELIUS_API_KEY env var)
    import sys

    if len(sys.argv) < 2:
        print("Usage: python wallet_behavior_helius.py <wallet_address>")
        sys.exit(1)

    wallet_address = sys.argv[1]

    try:
        collector = HeliusWalletCollector()
        profile = collector.collect_wallet_profile(wallet_address)
        if profile:
            print(f"\n=== Wallet Profile ===")
            print(f"Address: {profile.wallet_address}")
            print(f"Age: {profile.wallet_age_days} days")
            print(f"Tokens: {profile.token_count}")
            print(f"Programs: {profile.program_count}")
            print(f"Transactions: {profile.total_transactions}")
            print(f"Activity Span: {profile.activity_span_days} days")
            print(f"Single Token %: {profile.single_token_pct:.1f}%")
            print(f"\nScore: {profile.authenticity_score:.3f}")
            print(f"Verified Human: {profile.is_verified_human}")
        else:
            print("Failed to collect profile")
            sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)
