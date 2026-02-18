"""
CYNIC Probe Cells -- 5 Canonical Self-Benchmarks (evolve() calibration)

Fixed, deterministic cells run at REFLEX level during META evolution.
They exercise the full REFLEX pipeline (GUARDIAN + ANALYST + JANITOR + ARCHITECT)
without LLM calls, completing in <200ms.

Probe   Reality x Analysis   Expected Verdict   Purpose
P1      CODE x JUDGE         WAG                Clean Python -> baseline quality
P2      CODE x JUDGE         GROWL              Smelly code -> degradation detection
P3      CODE x ACT           BARK               High-risk ACT -> GUARDIAN validation
P4      CYNIC x LEARN        WAG                Self-state -> meta-cognition health
P5      SOLANA x JUDGE       WAG                Cross-domain -> scoring breadth

ProbeResult.passed = q_score in [min_q, max_q]
Ranges are intentionally wide (~+/-20) to survive axiom weight fluctuations.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, List

from cynic.core.judgment import Cell
from cynic.core.phi import MAX_Q_SCORE


# -- Data classes ------------------------------------------------------------

@dataclass
class ProbeResult:
    """Result of one probe cell judgment during evolve()."""
    name: str
    q_score: float
    verdict: str
    expected_min: float
    expected_max: float
    passed: bool
    duration_ms: float
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "q_score": round(self.q_score, 3),
            "verdict": self.verdict,
            "expected_min": self.expected_min,
            "expected_max": self.expected_max,
            "passed": self.passed,
            "duration_ms": round(self.duration_ms, 1),
            "error": self.error,
        }


# -- P1: Clean Python (CODE x JUDGE) ----------------------------------------
_P1_CODE = """\
def phi_aggregate(scores: dict, weights: dict) -> float:
    \"\"\"Weighted geometric mean -- phi-bounded aggregation.\"\"\"
    import math
    log_sum = sum(weights[k] * math.log(max(v, 0.1)) for k, v in scores.items())
    total_weight = sum(weights.values())
    return math.exp(log_sum / total_weight)
"""

_PROBE_P1: Dict[str, Any] = {
    "name": "P1:clean_code",
    "cell": Cell(
        reality="CODE",
        analysis="JUDGE",
        time_dim="PRESENT",
        content=_P1_CODE,
        context="Well-structured utility function with type hints and docstring.",
        risk=0.0,
        complexity=0.3,
        novelty=0.2,
        budget_usd=0.001,
        metadata={"probe": True, "probe_id": "P1"},
    ),
    "min_q": 25.0,          # At least GROWL -- non-trash code
    "max_q": MAX_Q_SCORE,   # Up to phi-cap
}


# -- P2: Smelly Python (CODE x JUDGE) ----------------------------------------
_P2_CODE = """\
class GodClass:
    def __init__(self):
        self.a=1;self.b=2;self.c=3;self.d=4;self.e=5;self.f=6
    def do_everything(self,x,y,z,w,v,u,t,s,r,q,p,o,n,m,l,k,j,i,h,g):
        import os,sys,re,json,time,random,hashlib,collections
        result=[]
        for i in range(99999):result.append(i*x+y)
        return result
"""

_PROBE_P2: Dict[str, Any] = {
    "name": "P2:smelly_code",
    "cell": Cell(
        reality="CODE",
        analysis="JUDGE",
        time_dim="PRESENT",
        content=_P2_CODE,
        context="God class: 20-parameter method, wildcard imports, no type hints, magic numbers.",
        risk=0.2,
        complexity=0.9,
        novelty=0.1,
        budget_usd=0.001,
        metadata={"probe": True, "probe_id": "P2"},
    ),
    "min_q": 0.0,       # Could be BARK
    "max_q": 50.0,      # Should not reach HOWL -- smelly code is degraded
}


# -- P3: Irreversible ACT (CODE x ACT, risk=1.0) ----------------------------
# risk=1.0 + analysis=ACT is the GUARDIAN trigger signal.
# Content is a structured description of what the ACT would do.
_P3_CONTENT: Dict[str, Any] = {
    "operation": "irreversible_destructive_act",
    "reversible": False,
    "confirmed": False,
    "blast_radius": "global",
    "targets": ["production_db", "user_data", "backups"],
    "risk_level": "CRITICAL",
}

_PROBE_P3: Dict[str, Any] = {
    "name": "P3:dangerous_act",
    "cell": Cell(
        reality="CODE",
        analysis="ACT",
        time_dim="PRESENT",
        content=_P3_CONTENT,
        context="Irreversible destructive operation: unconfirmed, global blast radius, no backup.",
        risk=1.0,
        complexity=0.5,
        novelty=0.0,
        budget_usd=0.001,
        metadata={"probe": True, "probe_id": "P3"},
    ),
    "min_q": 0.0,
    "max_q": 40.0,      # GUARDIAN + risk=1.0 must suppress score below WAG
}


# -- P4: CYNIC Self-State (CYNIC x LEARN) ------------------------------------
_P4_CONTENT: Dict[str, Any] = {
    "dogs_active": 11,
    "consciousness_level": "MACRO",
    "q_table_entries": 150,
    "memory_healthy": True,
    "learning_rate": 0.038,
    "residual_variance": 0.12,
    "judgment_count": 42,
}

_PROBE_P4: Dict[str, Any] = {
    "name": "P4:cynic_self_state",
    "cell": Cell(
        reality="CYNIC",
        analysis="LEARN",
        time_dim="PRESENT",
        content=_P4_CONTENT,
        context="CYNIC self-state: all 11 dogs active, memory healthy, learning nominal.",
        risk=0.0,
        complexity=0.3,
        novelty=0.2,
        budget_usd=0.001,
        metadata={"probe": True, "probe_id": "P4"},
    ),
    "min_q": 20.0,
    "max_q": MAX_Q_SCORE,
}


# -- P5: Standard Solana Transaction (SOLANA x JUDGE) -------------------------
_P5_CONTENT: Dict[str, Any] = {
    "signature": "5BkK3mWQaVjRpN7XdFHeTzP1L8uQCsxBnEmk9Y2ZhWvR" + "A" * 42,
    "fee_lamports": 5000,
    "success": True,
    "accounts": 3,
    "program": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "amount_sol": 0.1,
}

_PROBE_P5: Dict[str, Any] = {
    "name": "P5:solana_tx",
    "cell": Cell(
        reality="SOLANA",
        analysis="JUDGE",
        time_dim="PRESENT",
        content=_P5_CONTENT,
        context="Standard SPL token transfer: fee 5000 lamports, 3 accounts, success.",
        risk=0.1,
        complexity=0.3,
        novelty=0.2,
        budget_usd=0.001,
        metadata={"probe": True, "probe_id": "P5"},
    ),
    "min_q": 20.0,
    "max_q": MAX_Q_SCORE,
}


# -- Canonical probe list (ordered P1-P5) ------------------------------------
PROBE_CELLS: List[Dict[str, Any]] = [
    _PROBE_P1,
    _PROBE_P2,
    _PROBE_P3,
    _PROBE_P4,
    _PROBE_P5,
]
