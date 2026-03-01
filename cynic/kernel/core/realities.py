"""
CYNIC Reality Schemas — Zero-Trust Data Contracts.

Defines the strict structure of data for each dimension of reality.
This prevents 'Silent Debt' by ensuring hallucinations or malformed 
sensor data are rejected at the edge.
"""
from __future__ import annotations

from typing import Any, Dict, List, Optional, Union
from pydantic import BaseModel, Field

# ── 1. SOMATIC (Hardware & Metabolism) ───────────────────────────────────

class SomaticPayload(BaseModel):
    """Data from HardwareBody/InternalSensor."""
    type: str = Field(description="CPU_STRESS, RAM_STRESS, DISK_STRESS, TEMP_ALERT, etc.")
    value: Union[float, str, dict]
    source: str = Field(default="hardware")
    metadata: Dict[str, Any] = Field(default_factory=dict)

# ── 2. CODE (Software Architecture) ───────────────────────────────────────

class CodePayload(BaseModel):
    """Data from SourceWatcher/CLI."""
    filepath: str
    content: str
    language: str = "python"
    change_type: str = "MODIFIED" # ADDED, MODIFIED, DELETED
    diff: Optional[str] = None

# ── 3. INTERNAL (Self-Observation) ────────────────────────────────────────

class InternalPayload(BaseModel):
    """Signals from within the Mind (Anomalies, Residuals)."""
    signal_type: str
    message: str
    severity: float = Field(ge=0.0, le=1.0)
    origin_component: str

# ── 4. MARKET & SOCIAL ────────────────────────────────────────────────────

class MarketPayload(BaseModel):
    """Data from external market sensors."""
    symbol: str
    price: float
    volume_24h: Optional[float] = None
    source: str

# ── DISPATCHER ────────────────────────────────────────────────────────────

REALITY_SCHEMAS = {
    "SOMATIC": SomaticPayload,
    "CODE": CodePayload,
    "INTERNAL": InternalPayload,
    "MARKET": MarketPayload,
    # Fallback for others
}

def validate_content(reality: str, content: Any) -> Any:
    """Validate and return typed content for a given reality."""
    schema = REALITY_SCHEMAS.get(reality)
    if not schema:
        return content # Unknown realities pass through for now
    
    if isinstance(content, dict):
        return schema.model_validate(content)
    return content
