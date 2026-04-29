#!/usr/bin/env python3
"""CultScreener API client — conviction scoring (diamond hands metric).

CultScreener measures holder conviction: how long large holders have held the token.
Higher conviction = better (legitimate communities).
Lower conviction = riskier (potential rugs or pump-and-dumps).

Conviction scores map to CYNIC verdicts:
- conviction ≥ 0.7 → HOWL (strong community, real believers)
- conviction 0.4-0.7 → GROWL (mixed signals, need more time)
- conviction < 0.4 → BARK (weak conviction, suspicious)

Usage:
    client = CultScreenerClient(api_key="...")

    # Get top conviction tokens
    leaderboard = client.get_leaderboard(limit=50)

    # Get specific token's conviction
    token = client.get_token_conviction(mint_address)

Note: Requires API key (free tier available, sign up at cultscreener.com/api-keys).
API endpoint: https://cultscreener.com/api/v1/leaderboard
"""

import requests
import os
from typing import Optional, Dict, List
from dataclasses import dataclass
from enum import Enum

# Load .env file manually (no external dependency required)
def _load_env_file():
    """Load .env file from heuristics/ directory."""
    env_path = os.path.join(os.path.dirname(__file__), '.env')
    if os.path.exists(env_path):
        try:
            with open(env_path, 'r') as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        if key not in os.environ:  # Don't override existing env vars
                            os.environ[key] = value
        except Exception:
            pass  # Silently skip if .env file is malformed

_load_env_file()


class ConvictionTier(Enum):
    """CultScreener conviction classification."""
    STRONG = "strong"      # conviction ≥ 0.7 → legitimate
    MIXED = "mixed"        # conviction 0.4-0.7 → ambiguous
    WEAK = "weak"          # conviction < 0.4 → suspicious
    UNKNOWN = "unknown"

    @classmethod
    def from_conviction_score(cls, conviction: float) -> "ConvictionTier":
        """Map conviction score (0-1) to tier."""
        if conviction >= 0.7:
            return cls.STRONG
        elif conviction >= 0.4:
            return cls.MIXED
        elif conviction >= 0:
            return cls.WEAK
        else:
            return cls.UNKNOWN

    def to_verdict(self) -> str:
        """Map conviction to CYNIC verdict labels."""
        if self == ConvictionTier.STRONG:
            return "Howl"   # Strong conviction → legitimate
        elif self == ConvictionTier.MIXED:
            return "Growl"  # Mixed → ambiguous
        elif self == ConvictionTier.WEAK:
            return "Bark"   # Weak conviction → suspicious
        else:
            return "Bark"   # Unknown → default conservative


@dataclass
class TokenConvictionData:
    """CultScreener conviction data for a token."""
    mint: str
    name: Optional[str]
    symbol: Optional[str]
    conviction: float  # 0.0-1.0, higher = stronger community
    conviction_tier: ConvictionTier
    market_cap: Optional[float]  # USD market cap
    holders: Optional[int]  # Total token holders
    rank: Optional[int]  # Rank on leaderboard (if available)
    timestamp: str  # ISO 8601 timestamp

    def to_verdict(self) -> str:
        """Convert conviction to CYNIC verdict."""
        return self.conviction_tier.to_verdict()


class CultScreenerClient:
    """CultScreener API client for conviction-based token scoring."""

    BASE_URL = "https://cultscreener.com/api/v1"
    TIMEOUT = 30

    def __init__(self, api_key: Optional[str] = None):
        """Initialize client with API key.

        Args:
            api_key: CultScreener API key. If not provided, uses CULTSCREENER_API_KEY env var.
        """
        self.api_key = api_key or os.getenv("CULTSCREENER_API_KEY")
        if not self.api_key:
            raise ValueError(
                "CULTSCREENER_API_KEY not set.\n"
                "Option 1: export CULTSCREENER_API_KEY='<API_KEY>' && python3 token_dataset_ingester.py\n"
                "Option 2: Add to .env file: CULTSCREENER_API_KEY=<API_KEY>\n"
                "Get key from: https://cultscreener.com/api-keys"
            )

        self.session = requests.Session()
        self.session.headers.update({"Authorization": f"Bearer {self.api_key}"})

    def get_token_conviction(self, mint: str) -> Optional[TokenConvictionData]:
        """Fetch conviction data for a single token.

        Args:
            mint: Token mint address (base58)

        Returns:
            TokenConvictionData or None if not found
        """
        try:
            response = self.session.get(
                f"{self.BASE_URL}/leaderboard/{mint}",
                timeout=self.TIMEOUT,
            )
            if response.status_code == 404:
                return None
            if response.status_code == 401:
                raise ValueError("Invalid API key")
            if response.status_code != 200:
                print(f"CultScreener error: {response.status_code} {response.text}")
                return None

            data = response.json().get("data", {})
            if not data:
                return None

            conviction = data.get("conviction", 0.0)
            return TokenConvictionData(
                mint=data.get("mint_address", mint),
                name=data.get("name"),
                symbol=data.get("symbol"),
                conviction=conviction,
                conviction_tier=ConvictionTier.from_conviction_score(conviction),
                market_cap=data.get("market_cap"),
                holders=data.get("holders"),
                rank=data.get("rank"),
                timestamp=data.get("updated_at", ""),
            )
        except requests.exceptions.Timeout:
            print(f"Timeout fetching conviction for {mint}")
            return None
        except Exception as e:
            print(f"Error fetching conviction for {mint}: {e}")
            return None

    def get_leaderboard(
        self,
        limit: int = 50,
        offset: int = 0,
        min_conviction: Optional[float] = None,
        max_conviction: Optional[float] = None,
        min_mcap: Optional[float] = None,
        max_mcap: Optional[float] = None,
    ) -> List[TokenConvictionData]:
        """Fetch top conviction tokens (leaderboard).

        Args:
            limit: Max results (default 50, max 100)
            offset: Pagination offset (default 0)
            min_conviction: Filter tokens with conviction >= this value
            max_conviction: Filter tokens with conviction <= this value (client-side filter)
            min_mcap: Filter tokens with mcap >= this value (USD)
            max_mcap: Filter tokens with mcap <= this value (USD)

        Returns:
            List of TokenConvictionData sorted by conviction (descending)
        """
        try:
            params = {
                "limit": min(limit, 100),
                "offset": offset,
            }
            if min_conviction is not None:
                params["minConviction"] = min_conviction
            if min_mcap is not None:
                params["minMcap"] = min_mcap
            if max_mcap is not None:
                params["maxMcap"] = max_mcap

            response = self.session.get(
                f"{self.BASE_URL}/leaderboard",
                params=params,
                timeout=self.TIMEOUT,
            )
            if response.status_code == 401:
                raise ValueError("Invalid API key")
            if response.status_code != 200:
                print(f"CultScreener leaderboard error: {response.status_code}")
                return []

            data = response.json()
            tokens = []
            for item in data.get("data", []):
                conviction = item.get("conviction", 0.0)

                # Apply max_conviction filter (client-side, since API doesn't support it)
                if max_conviction is not None and conviction > max_conviction:
                    continue

                tokens.append(
                    TokenConvictionData(
                        mint=item.get("mint_address"),
                        name=item.get("name"),
                        symbol=item.get("symbol"),
                        conviction=conviction,
                        conviction_tier=ConvictionTier.from_conviction_score(conviction),
                        market_cap=item.get("market_cap"),
                        holders=item.get("holders"),
                        rank=item.get("rank"),
                        timestamp=item.get("updated_at", ""),
                    )
                )
            return tokens
        except requests.exceptions.Timeout:
            print("Timeout fetching leaderboard")
            return []
        except Exception as e:
            print(f"Error fetching leaderboard: {e}")
            return []


if __name__ == "__main__":
    import sys

    api_key = os.getenv("CULTSCREENER_API_KEY")
    if not api_key:
        print("Error: CULTSCREENER_API_KEY not set")
        print("Set via: export CULTSCREENER_API_KEY='<API_KEY>'")
        sys.exit(1)

    client = CultScreenerClient(api_key=api_key)

    # Example: fetch top conviction tokens
    print("Fetching top conviction tokens from CultScreener...")
    leaderboard = client.get_leaderboard(limit=10)
    for token in leaderboard:
        print(f"  {token.symbol:8s} conviction={token.conviction:.2f} tier={token.conviction_tier.value:6s} verdict={token.to_verdict()}")

    # Example: fetch specific token
    print("\nFetching specific token (BONK)...")
    bonk = client.get_token_conviction("DezXAZ8z7PnrnRJjz3wXBoRgixVqXaSo1S1zceA85q")
    if bonk:
        print(f"  {bonk.symbol} conviction={bonk.conviction:.2f} verdict={bonk.to_verdict()}")
