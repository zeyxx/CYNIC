"""
CYNIC Reality Schemas â€” Zero-Trust Data Contracts.

Defines the strict structure of data for each dimension of reality.
This prevents 'Silent Debt' by ensuring hallucinations or malformed
sensor data are rejected at the edge.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field

# â”€â”€ 1. SOMATIC (Hardware & Metabolism) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€


class SomaticPayload(BaseModel):
    """Data from HardwareBody/InternalSensor."""

    type: str = Field(description="CPU_STRESS, RAM_STRESS, DISK_STRESS, TEMP_ALERT, etc.")
    value: float | str | dict
    source: str = Field(default="hardware")
    metadata: dict[str, Any] = Field(default_factory=dict)


# â”€â”€ 2. CODE (Software Architecture) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€


class CodePayload(BaseModel):
    """Data from SourceWatcher/CLI."""

    filepath: str
    content: str
    language: str = "python"
    change_type: str = "MODIFIED"  # ADDED, MODIFIED, DELETED
    diff: str | None = None


# â”€â”€ 3. INTERNAL (Self-Observation) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€


class InternalPayload(BaseModel):
    """Signals from within the Mind (Anomalies, Residuals)."""

    signal_type: str
    message: str
    severity: float = Field(ge=0.0, le=1.0)
    origin_component: str


# â”€â”€ 4. MARKET & SOCIAL â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€


class MarketPayload(BaseModel):
    """Data from external market sensors."""
    symbol: str
    price: float
    change_24h: float | None = None
    volume_24h: float | None = None
    volatility: float = 0.0 # PHI-scaled volatility
    source: str

class SolanaPayload(BaseModel):
    """Data from Solana Blockchain sensors."""
    slot: int
    tps: float | None = None
    block_time: float | None = None
    recent_prioritization_fee: float | None = None
    health: str = "ok" # status of the cluster

# â”€â”€ DISPATCHER â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

REALITY_SCHEMAS = {
    "SOMATIC": SomaticPayload,
    "CODE": CodePayload,
    "INTERNAL": InternalPayload,
    "MARKET": MarketPayload,
    "SOLANA": SolanaPayload,
}


def validate_content(reality: str, content: Any) -> Any:
    """Validate and return typed content for a given reality."""
    # If content is already a string or something else, pass it through.
    # We only enforce schema if the source provides a dictionary.
    if not isinstance(content, dict):
        return content

    schema = REALITY_SCHEMAS.get(reality)
    if not schema:
        return content

    return schema.model_validate(content)
