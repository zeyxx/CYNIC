#!/usr/bin/env python3
"""CultScreener API client — ground truth token risk labels.

CultScreener is a Solana rug detection service that maintains a public database
of token risk assessments. Their labels are crowd-sourced and on-chain verified.

Usage:
    client = CultScreenenerClient()

    # Fetch risk assessment for a token
    result = client.get_token_risk(mint_address)
    # → {"mint": "...", "risk_level": "high", "reasons": [...], "last_updated": "..."}

    # Batch fetch (more efficient)
    results = client.batch_get_risks([mint1, mint2, ...])

    # Search tokens by risk level
    high_risk = client.search_tokens(risk_level="high", limit=100)
    low_risk = client.search_tokens(risk_level="low", limit=100)

Note: CultScreener API is public, no auth required.
Public endpoint: https://api.cultscreener.com/
"""

import requests
from typing import Optional, Dict, List
from dataclasses import dataclass
from enum import Enum


class RiskLevel(Enum):
    """CultScreener risk classification."""
    VERIFIED = "verified"  # Low risk, verified legitimate
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"  # Likely rug/scam
    UNKNOWN = "unknown"

    @classmethod
    def from_string(cls, value: str) -> "RiskLevel":
        """Parse risk level from string."""
        value_lower = value.lower().strip()
        for level in cls:
            if level.value == value_lower:
                return level
        return cls.UNKNOWN

    def to_verdict(self) -> str:
        """Map CultScreener risk to CYNIC verdict labels."""
        if self in (RiskLevel.HIGH, RiskLevel.UNKNOWN):
            return "Bark"  # High risk → treat as rug
        elif self == RiskLevel.VERIFIED:
            return "Howl"  # Verified legitimate
        elif self == RiskLevel.LOW:
            return "Howl"  # Low risk → treat as legitimate
        else:  # MEDIUM
            return "Growl"  # Ambiguous


@dataclass
class TokenRiskAssessment:
    """CultScreener risk assessment for a token."""
    mint: str
    name: Optional[str]
    symbol: Optional[str]
    risk_level: RiskLevel
    confidence: float  # 0.0-1.0
    reasons: List[str]  # e.g., ["active_mint_authority", "high_concentration"]
    last_updated: str  # ISO 8601 timestamp

    def to_verdict(self) -> str:
        """Convert risk assessment to CYNIC verdict."""
        return self.risk_level.to_verdict()


class CultScreenerClient:
    """CultScreener API client for token risk data."""

    BASE_URL = "https://api.cultscreener.com"
    TIMEOUT = 30

    def __init__(self):
        self.session = requests.Session()

    def get_token_risk(self, mint: str) -> Optional[TokenRiskAssessment]:
        """Fetch risk assessment for a single token.

        Args:
            mint: Token mint address (base58)

        Returns:
            TokenRiskAssessment or None if not found
        """
        try:
            response = self.session.get(
                f"{self.BASE_URL}/tokens/{mint}",
                timeout=self.TIMEOUT,
            )
            if response.status_code == 404:
                return None
            if response.status_code != 200:
                print(f"CultScreener error: {response.status_code}")
                return None

            data = response.json()
            return TokenRiskAssessment(
                mint=data.get("mint", mint),
                name=data.get("name"),
                symbol=data.get("symbol"),
                risk_level=RiskLevel.from_string(data.get("risk", "unknown")),
                confidence=data.get("confidence", 0.5),
                reasons=data.get("reasons", []),
                last_updated=data.get("last_updated", ""),
            )
        except Exception as e:
            print(f"Error fetching token risk for {mint}: {e}")
            return None

    def batch_get_risks(self, mints: List[str]) -> Dict[str, Optional[TokenRiskAssessment]]:
        """Fetch risk assessments for multiple tokens.

        Args:
            mints: List of token mint addresses

        Returns:
            Dict mapping mint → TokenRiskAssessment (or None if not found)
        """
        results = {}
        for mint in mints:
            results[mint] = self.get_token_risk(mint)
        return results

    def search_tokens(
        self,
        risk_level: Optional[str] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> List[TokenRiskAssessment]:
        """Search for tokens by risk level.

        Args:
            risk_level: "low", "medium", "high", "verified" (or None for all)
            limit: Max results (default 100)
            offset: Pagination offset (default 0)

        Returns:
            List of TokenRiskAssessment objects
        """
        try:
            params = {"limit": limit, "offset": offset}
            if risk_level:
                params["risk"] = risk_level

            response = self.session.get(
                f"{self.BASE_URL}/tokens/search",
                params=params,
                timeout=self.TIMEOUT,
            )
            if response.status_code != 200:
                print(f"CultScreener search error: {response.status_code}")
                return []

            data = response.json()
            tokens = []
            for item in data.get("results", []):
                tokens.append(
                    TokenRiskAssessment(
                        mint=item.get("mint"),
                        name=item.get("name"),
                        symbol=item.get("symbol"),
                        risk_level=RiskLevel.from_string(item.get("risk", "unknown")),
                        confidence=item.get("confidence", 0.5),
                        reasons=item.get("reasons", []),
                        last_updated=item.get("last_updated", ""),
                    )
                )
            return tokens
        except Exception as e:
            print(f"Error searching tokens: {e}")
            return []

    def get_statistics(self) -> Optional[Dict]:
        """Fetch CultScreener statistics (token count by risk level)."""
        try:
            response = self.session.get(
                f"{self.BASE_URL}/statistics",
                timeout=self.TIMEOUT,
            )
            if response.status_code == 200:
                return response.json()
            return None
        except Exception as e:
            print(f"Error fetching statistics: {e}")
            return None


if __name__ == "__main__":
    client = CultScreenerClient()

    # Example: fetch some tokens by risk level
    print("Fetching high-risk tokens from CultScreener...")
    high_risk = client.search_tokens(risk_level="high", limit=5)
    for token in high_risk:
        print(f"  {token.symbol:8s} {token.mint:44s} risk={token.risk_level.value} verdict={token.to_verdict()}")

    print("\nFetching low-risk tokens from CultScreener...")
    low_risk = client.search_tokens(risk_level="low", limit=5)
    for token in low_risk:
        print(f"  {token.symbol:8s} {token.mint:44s} risk={token.risk_level.value} verdict={token.to_verdict()}")

    print("\nStatistics:")
    stats = client.get_statistics()
    if stats:
        print(f"  Total tokens assessed: {stats.get('total_tokens', '?')}")
        print(f"  By risk level: {stats.get('by_risk_level', {})}")
