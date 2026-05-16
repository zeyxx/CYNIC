#!/usr/bin/env python3
"""Helius Token Profiler — complete on-chain profile for a Solana token.

Combines multiple Helius APIs into one TokenProfile:
  - getAsset (DAS): metadata, supply, authorities, price
  - getTokenLargestAccounts (RPC): top holder distribution
  - getTransactionsForAddress (Helius RPC): age, volume, activity

Cost: ~31 credits per token (free tier = 1M/month).

Usage:
    profiler = HeliusTokenProfiler()
    profile = profiler.profile("9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump")
    print(profile)
"""

import os
import time
import json
import requests
from dataclasses import dataclass, field, asdict
from typing import Optional, List
from datetime import datetime, timezone

PUMPFUN_AUTHORITY = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM"


@dataclass
class TokenProfile:
    """Complete on-chain profile for a Solana token."""
    mint: str
    name: Optional[str] = None
    symbol: Optional[str] = None

    # Supply & price (getAsset)
    supply_raw: int = 0
    supply_human: float = 0.0
    decimals: int = 0
    price_usd: Optional[float] = None
    market_cap_usd: Optional[float] = None

    # Authorities (getAsset)
    mint_authority_active: bool = False
    freeze_authority_active: bool = False
    is_immutable: bool = False

    # Origin
    origin_pump_fun: bool = False

    # Distribution (getTokenLargestAccounts)
    top1_pct_supply: float = 0.0
    top5_pct_supply: float = 0.0
    top10_pct_supply: float = 0.0
    top20_pct_supply: float = 0.0
    top_holder_count: int = 0  # how many returned by getTokenLargestAccounts

    # Holder count (from CultScreener or external source — not available from Helius)
    holder_count: Optional[int] = None

    # Age & activity (getTransactionsForAddress)
    first_tx_timestamp: Optional[int] = None  # unix epoch
    last_tx_timestamp: Optional[int] = None
    age_hours: Optional[float] = None
    recent_tx_count: int = 0  # txs in last fetch window
    recent_swap_count: int = 0
    recent_transfer_count: int = 0
    hours_since_last_tx: Optional[float] = None

    # Data quality
    asset_found: bool = False
    account_exists: bool = False
    tx_history_available: bool = False

    # Conviction (from CultScreener, injected externally)
    conviction_1m: Optional[float] = None

    def to_stimulus(self, population_stats: Optional[dict] = None) -> str:
        """Generate enriched stimulus text for Dog judgment.

        Args:
            population_stats: Optional dict with percentile lookup arrays
                {'conviction': [...], 'holders': [...], 'mcap': [...]}
        """
        lines = [f"TOKEN ANALYSIS: {self.symbol or self.name or self.mint[:12]}"]
        lines.append(f"Mint: {self.mint}")
        lines.append("")

        # Population context
        if population_stats:
            lines.append(f"POPULATION CONTEXT (vs {population_stats.get('label', 'tracked tokens')}, n={population_stats.get('n', '?')}):")
            if self.conviction_1m is not None:
                pct = _percentile_of(self.conviction_1m, population_stats.get('conviction', []))
                med = _median(population_stats.get('conviction', []))
                lines.append(f"- Conviction (1m): {self.conviction_1m:.1%} — percentile {pct} (median: {med:.1%})")
            if self.holder_count:
                pct = _percentile_of(self.holder_count, population_stats.get('holders', []))
                med = _median(population_stats.get('holders', []))
                lines.append(f"- Holders: {self.holder_count:,} — percentile {pct} (median: {med:,.0f})")
            if self.market_cap_usd:
                pct = _percentile_of(self.market_cap_usd, population_stats.get('mcap', []))
                med = _median(population_stats.get('mcap', []))
                lines.append(f"- Market cap: ${self.market_cap_usd:,.0f} — percentile {pct} (median: ${med:,.0f})")
            lines.append("")

        # On-chain facts
        lines.append("ON-CHAIN FACTS:")
        lines.append(f"- Origin: {'pump.fun (permissionless launch)' if self.origin_pump_fun else 'standard SPL token'}")
        lines.append(f"- Mint authority: {'ACTIVE (can inflate supply)' if self.mint_authority_active else 'Revoked (supply locked)'}")
        lines.append(f"- Freeze authority: {'ACTIVE (can freeze wallets)' if self.freeze_authority_active else 'Revoked (wallets free)'}")
        if self.supply_human > 0:
            lines.append(f"- Total supply: {self.supply_human:,.0f} tokens")
        if self.price_usd:
            lines.append(f"- Price: ${self.price_usd:.8f}")
        if self.market_cap_usd:
            lines.append(f"- Market cap: ${self.market_cap_usd:,.0f}")
        lines.append("")

        # Distribution
        lines.append("HOLDER DISTRIBUTION:")
        lines.append(f"- Top 1 holder: {self.top1_pct_supply:.1f}% of supply")
        lines.append(f"- Top 10 holders: {self.top10_pct_supply:.1f}% of supply")
        lines.append(f"- Top 20 holders: {self.top20_pct_supply:.1f}% of supply")
        if self.holder_count:
            lines.append(f"- Total holders: {self.holder_count:,}")
        lines.append("")

        # Age & activity
        lines.append("AGE & ACTIVITY:")
        if self.age_hours is not None:
            days = self.age_hours / 24
            if days > 365:
                lines.append(f"- Token age: {days / 365:.1f} years ({days:.0f} days)")
            elif days > 30:
                lines.append(f"- Token age: {days:.0f} days ({days / 30:.1f} months)")
            else:
                lines.append(f"- Token age: {days:.1f} days ({self.age_hours:.0f} hours)")
        else:
            lines.append("- Token age: unknown")

        if self.hours_since_last_tx is not None:
            if self.hours_since_last_tx < 1:
                lines.append(f"- Last activity: {self.hours_since_last_tx * 60:.0f} minutes ago")
            elif self.hours_since_last_tx < 48:
                lines.append(f"- Last activity: {self.hours_since_last_tx:.1f} hours ago")
            else:
                lines.append(f"- Last activity: {self.hours_since_last_tx / 24:.1f} days ago")

        if self.recent_tx_count > 0:
            lines.append(f"- Recent transactions: {self.recent_tx_count} (swaps: {self.recent_swap_count}, transfers: {self.recent_transfer_count})")
        else:
            lines.append("- Recent transactions: none detected")

        # Conviction
        if self.conviction_1m is not None:
            lines.append("")
            lines.append("HOLDER BEHAVIOR:")
            lines.append(f"- 1-month conviction: {self.conviction_1m:.1%} of holders retained through last month")

        # What's missing
        lines.append("")
        lines.append("UNKNOWN:")
        unknowns = []
        if self.age_hours is None:
            unknowns.append("token age")
        if not self.holder_count:
            unknowns.append("exact holder count")
        if self.conviction_1m is None:
            unknowns.append("holder conviction/retention")
        unknowns.extend(["social media presence", "community quality", "liquidity depth"])
        lines.append(f"- {', '.join(unknowns)}")

        return "\n".join(lines)


def _percentile_of(value, sorted_list):
    if not sorted_list or value is None:
        return "?"
    count_below = sum(1 for x in sorted_list if x <= value)
    return round((count_below / len(sorted_list)) * 100, 1)


def _median(sorted_list):
    if not sorted_list:
        return 0
    n = len(sorted_list)
    return sorted(sorted_list)[n // 2]


class HeliusTokenProfiler:
    """Build complete on-chain profiles for Solana tokens via Helius APIs."""

    def __init__(self, api_key: Optional[str] = None, timeout: int = 30):
        self.api_key = api_key or os.getenv("HELIUS_API_KEY")
        if not self.api_key:
            raise ValueError("HELIUS_API_KEY not set")
        self.rpc_url = f"https://mainnet.helius-rpc.com/?api-key={self.api_key}"  # noqa: env var at runtime
        self.rest_url = f"https://api-mainnet.helius-rpc.com/v0"
        self.timeout = timeout

    def _rpc(self, method: str, params) -> Optional[dict]:
        """Call Helius JSON-RPC."""
        try:
            resp = requests.post(self.rpc_url, json={
                "jsonrpc": "2.0", "id": 1,
                "method": method, "params": params,
            }, timeout=self.timeout)
            data = resp.json()
            if "error" in data:
                return None
            return data.get("result")
        except Exception:
            return None

    def _rest_get(self, path: str, params: Optional[dict] = None) -> Optional[dict]:
        """Call Helius REST API."""
        try:
            url = f"{self.rest_url}/{path}?api-key={self.api_key}"  # noqa: env var at runtime
            resp = requests.get(url, params=params or {}, timeout=self.timeout)
            if resp.status_code != 200:
                return None
            return resp.json()
        except Exception:
            return None

    def profile(self, mint: str, holder_count: Optional[int] = None,
                conviction_1m: Optional[float] = None) -> TokenProfile:
        """Build complete token profile from Helius APIs.

        Args:
            mint: Token mint address (base58)
            holder_count: Inject holder count from external source (CultScreener)
            conviction_1m: Inject conviction from CultScreener
        """
        p = TokenProfile(mint=mint, holder_count=holder_count, conviction_1m=conviction_1m)

        # Phase 1: getAsset — metadata, supply, price, authorities
        self._fetch_asset(p)

        # Phase 2: getTokenLargestAccounts — distribution
        self._fetch_distribution(p)

        # Phase 3: getTransactionsForAddress — age, activity
        self._fetch_activity(p)

        return p

    def _fetch_asset(self, p: TokenProfile):
        """Fetch token metadata via DAS getAsset."""
        asset = self._rpc("getAsset", {"id": p.mint})
        if not asset:
            return

        p.asset_found = True
        p.account_exists = True

        # Metadata
        content = asset.get("content", {})
        metadata = content.get("metadata", {})
        p.name = metadata.get("name") or asset.get("content", {}).get("metadata", {}).get("name")
        p.symbol = metadata.get("symbol")

        # Token info
        ti = asset.get("token_info", {})
        p.supply_raw = int(ti.get("supply", 0))
        p.decimals = int(ti.get("decimals", 0))
        if p.decimals > 0 and p.supply_raw > 0:
            p.supply_human = p.supply_raw / (10 ** p.decimals)

        # Authorities — from token_info (actual on-chain state)
        p.mint_authority_active = ti.get("mint_authority") is not None
        p.freeze_authority_active = ti.get("freeze_authority") is not None

        # Immutability
        mutable = asset.get("mutable", True)
        p.is_immutable = not mutable

        # Price
        price_info = ti.get("price_info", {})
        p.price_usd = price_info.get("price_per_token")
        if p.price_usd and p.supply_human > 0:
            p.market_cap_usd = p.price_usd * p.supply_human

        # Pump.fun detection
        authorities = asset.get("authorities", [])
        for auth in authorities:
            if auth.get("address") == PUMPFUN_AUTHORITY:
                p.origin_pump_fun = True
                break
        if not p.origin_pump_fun and p.mint.endswith("pump"):
            p.origin_pump_fun = True

    def _fetch_distribution(self, p: TokenProfile):
        """Fetch top holder distribution via getTokenLargestAccounts."""
        result = self._rpc("getTokenLargestAccounts", [p.mint])
        if not result or not isinstance(result, dict):
            return

        accounts = result.get("value", [])
        if not accounts:
            return

        amounts = [int(acc.get("amount", "0")) for acc in accounts]
        p.top_holder_count = len(amounts)

        denom = p.supply_raw if p.supply_raw > 0 else sum(amounts)
        if denom <= 0:
            return

        if len(amounts) >= 1:
            p.top1_pct_supply = (amounts[0] / denom) * 100
        if len(amounts) >= 5:
            p.top5_pct_supply = (sum(amounts[:5]) / denom) * 100
        if len(amounts) >= 10:
            p.top10_pct_supply = (sum(amounts[:10]) / denom) * 100
        p.top20_pct_supply = (sum(amounts[:20]) / denom) * 100

    def _fetch_activity(self, p: TokenProfile):
        """Fetch age and recent activity via Helius APIs.

        Uses getTransactionsForAddress (Helius exclusive, Developer+ only):
        - sortOrder=asc, limit=1: first ever transaction → token age
        - sortOrder=desc, limit=100 via REST: recent activity level

        Credits: 10 per 100 txs (full), 10 flat (signatures-only).
        """
        now = int(time.time())

        # First transaction (oldest) — token age
        # getTransactionsForAddress uses positional params: [address, {options}]
        first_result = self._rpc("getTransactionsForAddress", [
            p.mint,
            {"limit": 1, "sortOrder": "asc", "transactionDetails": "signatures"},
        ])
        if first_result and isinstance(first_result, dict):
            txs = first_result.get("data", [])
            if txs:
                p.tx_history_available = True
                ts = txs[0].get("blockTime")
                if ts:
                    p.first_tx_timestamp = ts
                    p.age_hours = (now - ts) / 3600

        # Most recent transaction — last activity
        last_result = self._rpc("getTransactionsForAddress", [
            p.mint,
            {"limit": 1, "sortOrder": "desc", "transactionDetails": "signatures"},
        ])
        if last_result and isinstance(last_result, dict):
            txs = last_result.get("data", [])
            if txs:
                ts = txs[0].get("blockTime")
                if ts:
                    p.last_tx_timestamp = ts
                    p.hours_since_last_tx = (now - ts) / 3600

        # Recent activity — via Enhanced Transactions REST (parsed types)
        recent_txs = self._rest_get(f"addresses/{p.mint}/transactions", {"limit": 100})
        if recent_txs and isinstance(recent_txs, list):
            p.tx_history_available = True
            p.recent_tx_count = len(recent_txs)

            for tx in recent_txs:
                tx_type = tx.get("type", "UNKNOWN")
                if tx_type == "SWAP":
                    p.recent_swap_count += 1
                elif tx_type == "TRANSFER":
                    p.recent_transfer_count += 1


def load_env():
    """Load env vars from .env and ~/.cynic-env."""
    for env_file in [
        os.path.join(os.path.dirname(__file__), '.env'),
        os.path.expanduser('~/.cynic-env'),
    ]:
        if os.path.exists(env_file):
            with open(env_file) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        key = key.replace('export ', '').strip()
                        if key not in os.environ:
                            os.environ[key] = value.strip()


if __name__ == "__main__":
    load_env()
    profiler = HeliusTokenProfiler()

    # Test on Fartcoin
    mint = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump"
    print(f"Profiling {mint[:12]}...")
    profile = profiler.profile(mint, holder_count=160121, conviction_1m=0.967)

    print(f"\n=== {profile.symbol} ===")
    print(f"Supply: {profile.supply_human:,.0f}")
    print(f"Price: ${profile.price_usd}")
    print(f"Mcap: ${profile.market_cap_usd:,.0f}" if profile.market_cap_usd else "Mcap: N/A")
    print(f"Mint auth: {'ACTIVE' if profile.mint_authority_active else 'revoked'}")
    print(f"Freeze auth: {'ACTIVE' if profile.freeze_authority_active else 'revoked'}")
    print(f"Pump.fun: {profile.origin_pump_fun}")
    print(f"Top1: {profile.top1_pct_supply:.1f}%  Top10: {profile.top10_pct_supply:.1f}%  Top20: {profile.top20_pct_supply:.1f}%")
    print(f"Age: {profile.age_hours:.0f}h ({profile.age_hours/24:.0f}d)" if profile.age_hours else "Age: unknown")
    print(f"Last activity: {profile.hours_since_last_tx:.1f}h ago" if profile.hours_since_last_tx is not None else "Last activity: unknown")
    print(f"Recent txs: {profile.recent_tx_count} (swaps: {profile.recent_swap_count})")
    print(f"\n--- STIMULUS ---")
    print(profile.to_stimulus())
