"""
CYNIC Formula Constants — Business Logic & Thresholds

This module centralizes all hardcoded business logic constants:
- Consciousness blending weights
- Q-Table Fisher scaling
- LLM timeouts
- Rolling capacity caps (Fibonacci-derived)
- Cost efficiency thresholds
- Signal TTLs
- LOD latency targets

All constants are φ-derived or justified by first principles.
Changing these values directly impacts system behavior — consult architecture before modifying.

IMPORT RULE: Centralize ALL formulas here.
Always: `from cynic.core.formulas import CONSCIOUSNESS_BLENDING_WEIGHTS, ...`
"""
from __future__ import annotations

from cynic.core.phi import fibonacci, PHI_INV_2


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS BLENDING (L1/L2 Axiom Selector)
# ════════════════════════════════════════════════════════════════════════════

CONSCIOUSNESS_BLENDING_WEIGHTS: dict[str, float] = {
    "axiom_maturity": 0.4,      # 40% — Active axiom health (triggers MACRO)
    "e_score": 0.3,             # 30% — Dog reputation (quality signal)
    "oracle": 0.3,              # 30% — Oracle confidence (prediction strength)
}
"""
Blended consciousness formula:
  consciousness_level = (axiom_maturity/100 × 0.4) + (e_score/100 × 0.3) + (oracle_conf × 0.3)

Justification:
  - Axioms (40%): System readiness — if axioms inactive, whole system is in distress
  - E-Score (30%): Dog quality — which dogs are trustworthy to execute this judgment?
  - Oracle (30%): Prediction confidence — how certain is the forecasting engine?

Symmetry: 40+30+30 = 100, intentionally not φ-weighted (simple blend for fast computation)
"""


# ════════════════════════════════════════════════════════════════════════════
# Q-TABLE LEARNING
# ════════════════════════════════════════════════════════════════════════════

Q_TABLE_FISHER_SCALE: int = fibonacci(8)  # = 21
"""
Fisher Information weighting for EWC (Elastic Weight Consolidation).

When QEntry visits >= 21:
  effective_α = α × (1 - λ × fisher_weight)

Justification:
  F(8) = 21 is the "consolidation threshold" in the Fibonacci sequence.
  After 21 visits, an entry has collected enough data to be considered "consolidated."
  Consolidation means it should resist future changes more strongly (EWC penalty kicks in).

Reference: Q-Table benchmark (experiment #0) validates this via empirical convergence testing.
"""

AXIOM_MATURITY_FORMULA_DESC: str = "ratio × MAX_Q_SCORE"
"""
Axiom maturity score = (recent_signals / expected_signals_per_window) × 100

Justification:
  - Axiom is ACTIVE when it generates signals consistently
  - Maturity = confidence that axiom is actually doing work
  - Ranges [0, 100] on the Q-Score scale (same scale as judgments)
"""


# ════════════════════════════════════════════════════════════════════════════
# LLM INFERENCE
# ════════════════════════════════════════════════════════════════════════════

LLM_TIMEOUT_SEC: float = 120.0
"""
Maximum time (seconds) to wait for LLM response before aborting with timeout error.

Justification:
  - Ollama local: ~10-50s typical for gemma2:2b on modern GPU
  - Claude API: ~5-30s typical for Sonnet
  - 120s = 2 minutes = safe upper bound with room for network latency
  - Prevents infinite waits if LLM service becomes unresponsive

Note: Can be overridden per adapter via LLMAdapter.timeout config.
"""

LLM_DISCOVERY_TIMEOUT_SEC: float = 5.0
"""
Timeout for LLM discovery (list available models) operations.

Faster than inference timeout because discovery is a cheap operation.
"""


# ════════════════════════════════════════════════════════════════════════════
# ROLLING CAPACITY CAPS (Fibonacci-derived, prevent unbounded growth)
# ════════════════════════════════════════════════════════════════════════════

SOCIAL_SIGNAL_CAP: int = fibonacci(8)  # = 21
"""
Maximum number of social signals to retain in ~/.cynic/social.json

Justification:
  F(8) = 21 matches the "recent window" for judgment outcomes (confidence window).
  After 21 signals (representing ~3-5 hours of activity), older signals naturally decay.
  This prevents file bloat while keeping enough history for pattern detection.
"""

ACT_LOG_CAP: int = fibonacci(11)  # = 89
"""
Maximum number of ACT execution records to retain in ~/.cynic/act_log.jsonl

Justification:
  F(11) = 89 = number of Dogs' consensus + decisions capacity.
  89 acts = roughly 1 day of heavy MACRO cycles (1 act per 60-90s avg).
  Enough history for meta-learning without unbounded storage.
"""

CHAT_MESSAGE_CAP: int = fibonacci(11)  # = 89
"""
Maximum number of messages to retain in a ChatSession (CYNIC Code REPL).

Justification:
  F(11) = 89 = fractal balance between context window and efficiency.
  89 messages ≈ 100-150 exchanges = enough for sustained problem-solving without explosion.
"""

QTABLE_ENTRY_CAP: int = fibonacci(11)  # = 89
"""
Maximum Q-Table entries per state before pruning oldest entries.

Justification:
  Same as chat messages: F(11) = 89 balances coverage vs memory.
  Beyond 89 entries per state, marginal information gain decreases.
"""

SERVICE_REGISTRY_JUDGMENT_CAP: int = fibonacci(10)  # = 55
"""
Maximum judgments to store in ServiceRegistry (Tier 1 Nervous System).

Justification:
  F(10) = 55 = judgments from ~1-2 hours of normal operation.
  Enough to detect health trends without unlimited memory.
"""

DECISION_TRACE_CAP: int = fibonacci(10)  # = 55
"""
Maximum decision traces (DAGs) to retain in Tier 1 Decision Trace component.

Justification:
  F(10) = 55 matches judgment cap (same temporal window for causality analysis).
"""

LOOP_CLOSURE_CAP: int = fibonacci(9)  # = 34
"""
Maximum loop closure events to retain in Tier 1 Loop Closure Validator.

Justification:
  F(9) = 34 = ~half the judgment window.
  Focusing on recent cycles (stalls detected within 1 hour).
"""

EVENT_JOURNAL_CAP: int = fibonacci(11)  # = 89
"""
Maximum events to retain in Tier 1 Event Journal.

Justification:
  F(11) = 89 = full judgment history window for complete event sequencing.
"""

SELF_PROBE_CAP: int = fibonacci(10)  # = 55
"""
Maximum self-improvement proposals to store.

Justification:
  F(10) = 55 = same as decision traces (meta-cognition operates on recent decisions).
  Self-probes are reactions to recent patterns, stored rolling window.
"""


# ════════════════════════════════════════════════════════════════════════════
# TOOL OUTPUT TRUNCATION (Chat tool safety)
# ════════════════════════════════════════════════════════════════════════════

BASH_OUTPUT_CAP: int = 8192
"""
Maximum characters to return from bash command execution.

Prevents massive log dumps from filling context window.
"""

READ_FILE_CAP: int = 16384
"""
Maximum characters to return from file read operations.

Balances detail (files can be large) vs context efficiency.
"""

GLOB_MATCH_CAP: int = 4096
"""
Maximum characters for glob pattern matching results.

Prevents explosion of match counts in large directories.
"""

GREP_OUTPUT_CAP: int = 4096
"""
Maximum characters for grep search results.

Prevents massive search result dumps.
"""


# ════════════════════════════════════════════════════════════════════════════
# COST EFFICIENCY METRICS
# ════════════════════════════════════════════════════════════════════════════

COST_EFFICIENCY_CAP_USD: float = 0.02
"""
Cost per Q-point threshold for RUN score calculation.

Formula:
  run_score = max(0, 100 × (1 - cost_per_q_point / 0.02))

Justification:
  - $0.02 per Q-point = "expensive" threshold
  - Example: if cost_per_q = $0.01, run_score = 50 (medium efficiency)
  - Example: if cost_per_q = $0.02, run_score = 0 (terrible efficiency)
  - For Ollama (free local): run_score ≈ 100 (best)
  - For Claude API (~$0.01/call): run_score ≈ 50-100 depending on Q

Tuning: Adjust based on your LLM cost structure (update if pricing changes).
"""


# ════════════════════════════════════════════════════════════════════════════
# SIGNAL LIFECYCLE
# ════════════════════════════════════════════════════════════════════════════

SIGNAL_TTL_SEC: float = float(fibonacci(8) * 60)  # = 21 × 60 = 1260 seconds ≈ 21 minutes
"""
Time-to-live for axiom activation signals.

Justification:
  F(8) × 60 = 1260s = 21 minutes.
  After 21 minutes without new signals, axiom maturity decays (becomes inactive).
  Prevents stale axioms from inflating consciousness level.

Tuning: Adjust based on your typical judgment velocity (judgments/min).
"""


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS LEVEL (LOD) LATENCY TARGETS
# ════════════════════════════════════════════════════════════════════════════

LOD_LEVEL0_LATENCY_MS: float = 10.0
"""
L0 REFLEX: Maximum latency target (milliseconds).

Level 0 = instant judgments (no LLM, only heuristics).
Target: <10ms for pure logic operations.
"""

LOD_LEVEL1_LATENCY_MS: float = 100.0
"""
L1 MICRO: Maximum latency target.

Level 1 = single-dog judgment (one LLM call, REFLEX dogs only).
Target: <100ms for quick advisor.
"""

LOD_LEVEL2_LATENCY_MS: float = 500.0
"""
L2 MACRO: Maximum latency target.

Level 2 = 7-dog consensus (parallel SAGE + SCHOLAR + others).
Target: <500ms for balanced decisions.
"""

LOD_LEVEL3_LATENCY_MS: float = 2850.0
"""
L3 META: Maximum latency target.

Level 3 = full 11-dog PBFT + meta-cognition.
Formula: F(8) × 21ms × 100 ≈ 2850ms.
Justification: 11 dogs × ~200-300ms each + overhead ≈ 2.8s worst case.
Target: <2.85s for deep reasoning on critical decisions.
"""


# ════════════════════════════════════════════════════════════════════════════
# PATTERN DETECTION (Residual Anomaly Detection)
# ════════════════════════════════════════════════════════════════════════════

RESIDUAL_MIN_SAMPLES: int = fibonacci(4)  # = 3
"""
Minimum number of samples before anomaly detection patterns kick in.

Used by: ResidualDetector.SPIKE, RISING, STABLE_HIGH pattern recognition.

Justification:
  F(4) = 3 = minimum for trend detection (need at least 3 points to compute slope).
  Below 3 samples, patterns are noise.

Example:
  - 1-2 judgments: insufficient data
  - 3+ judgments: pattern detection active (SPIKE, RISING, STABLE_HIGH)
"""

RESIDUAL_STABLE_HIGH_N: int = fibonacci(5)  # = 5
"""
Number of consecutive high residuals before STABLE_HIGH pattern is flagged.

Used by: ResidualDetector._check_stable_high_pattern().

Justification:
  F(5) = 5 = minimum for "pattern persistence" (outlier threshold).
  A single high residual = noise; 5 consecutive = genuine anomaly.
"""


# ════════════════════════════════════════════════════════════════════════════
# AXIOM MONITORING
# ════════════════════════════════════════════════════════════════════════════

AXIOM_MATURITY_WINDOW_SIZE: int = fibonacci(8)  # = 21
"""
Number of recent signals to consider when computing axiom maturity.

Justification:
  F(8) = 21 = rolling window for "recent activity."
  Older signals (beyond 21) decay exponentially via TTL mechanism.
"""

AXIOM_ACTIVATION_THRESHOLD: float = 0.382  # PHI_INV_2
"""
Minimum maturity score (0-100) for axiom to be considered "active."

Justification:
  0.382 = φ⁻² = "above GROWL threshold" for axiom health.
  Below this, axiom is dormant (no impact on consciousness level).

Derivation: φ⁻² = minimum confidence threshold (system still functional but degraded).
"""

AXIOM_TRANSCENDENCE_THRESHOLD: float = 0.618  # PHI_INV
"""
Minimum maturity score (0-100) for axiom to trigger transcendence detection.

Justification:
  0.618 = φ⁻¹ = "WAG tier" for axiom confidence.
  When all 4 emergent axioms reach this, system enters TRANSCENDENCE phase.
"""


# ════════════════════════════════════════════════════════════════════════════
# VALIDATION (document all constants)
# ════════════════════════════════════════════════════════════════════════════

FORMULA_CONSTANTS = {
    "consciousness_blending": CONSCIOUSNESS_BLENDING_WEIGHTS,
    "q_table_fisher_scale": Q_TABLE_FISHER_SCALE,
    "llm_timeout_sec": LLM_TIMEOUT_SEC,
    "social_signal_cap": SOCIAL_SIGNAL_CAP,
    "act_log_cap": ACT_LOG_CAP,
    "chat_message_cap": CHAT_MESSAGE_CAP,
    "qtable_entry_cap": QTABLE_ENTRY_CAP,
    "cost_efficiency_cap_usd": COST_EFFICIENCY_CAP_USD,
    "signal_ttl_sec": SIGNAL_TTL_SEC,
    "lod_latency_targets": {
        "level0_ms": LOD_LEVEL0_LATENCY_MS,
        "level1_ms": LOD_LEVEL1_LATENCY_MS,
        "level2_ms": LOD_LEVEL2_LATENCY_MS,
        "level3_ms": LOD_LEVEL3_LATENCY_MS,
    },
}

__all__ = [
    "CONSCIOUSNESS_BLENDING_WEIGHTS",
    "Q_TABLE_FISHER_SCALE",
    "LLM_TIMEOUT_SEC",
    "LLM_DISCOVERY_TIMEOUT_SEC",
    "SOCIAL_SIGNAL_CAP",
    "ACT_LOG_CAP",
    "CHAT_MESSAGE_CAP",
    "QTABLE_ENTRY_CAP",
    "SERVICE_REGISTRY_JUDGMENT_CAP",
    "DECISION_TRACE_CAP",
    "LOOP_CLOSURE_CAP",
    "EVENT_JOURNAL_CAP",
    "SELF_PROBE_CAP",
    "BASH_OUTPUT_CAP",
    "READ_FILE_CAP",
    "GLOB_MATCH_CAP",
    "GREP_OUTPUT_CAP",
    "COST_EFFICIENCY_CAP_USD",
    "SIGNAL_TTL_SEC",
    "LOD_LEVEL0_LATENCY_MS",
    "LOD_LEVEL1_LATENCY_MS",
    "LOD_LEVEL2_LATENCY_MS",
    "LOD_LEVEL3_LATENCY_MS",
    "RESIDUAL_MIN_SAMPLES",
    "RESIDUAL_STABLE_HIGH_N",
    "AXIOM_MATURITY_WINDOW_SIZE",
    "AXIOM_ACTIVATION_THRESHOLD",
    "AXIOM_TRANSCENDENCE_THRESHOLD",
    "FORMULA_CONSTANTS",
]
