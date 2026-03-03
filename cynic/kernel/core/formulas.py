"""
CYNIC Formulas " The Metabolic Regulator.

Translates abstract PHI constants into operational system parameters.
Banish 'Magic Numbers' by deriving all limits from PHI, Fibonacci, and Lucas.

Scaling Law:
  As system stress (entropy) increases, limits should contract or expand
  following PHI-geometric ratios.
"""

from __future__ import annotations

import math

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.phi import (
    PHI,
    PHI_INV,
    PHI_INV_2,
    fibonacci,
    lucas,
)

# Consciousness Blending Weights (for Task #8)
CONSCIOUSNESS_BLENDING_WEIGHTS = {
    ConsciousnessLevel.REFLEX: PHI_INV_2,  # 0.382
    ConsciousnessLevel.MICRO: PHI_INV,  # 0.618
    ConsciousnessLevel.MACRO: 1.0,  # 1.0
    ConsciousnessLevel.META: PHI,  # 1.618
}

# Service Registry Judgment Log Cap (rolling window)
SERVICE_REGISTRY_JUDGMENT_CAP = int(fibonacci(11))  # 89 judgments in rolling log

#
# 1. LATENCY BUDGETS (Time-to-Judgment)
#


def get_latency_budget_ms(level: ConsciousnessLevel) -> float:
    """
    Calculate the total time allowed for a cycle.
    Formula: Base(100ms) * PHI^(level_index * 2)
    """
    # Level indices: REFLEX=0, MICRO=1, MACRO=2, META=3
    level_map = {
        ConsciousnessLevel.REFLEX: 0,
        ConsciousnessLevel.MICRO: 1,
        ConsciousnessLevel.MACRO: 2,
        ConsciousnessLevel.META: 3,
    }
    idx = level_map.get(level, 0)

    # REFLEX: 100ms
    # MICRO: ~261ms (100 * PHI^2)
    # MACRO: ~685ms (100 * PHI^4)
    # META:  ~1794ms (100 * PHI^6)
    return 100.0 * (PHI ** (idx * 2))


def get_timeout_s(level: ConsciousnessLevel, metabolic_pressure: float = 1.0) -> float:
    """
    Hard timeout for network/LLM calls.
    Includes a 'safety margin' of PHI and scales with system pressure.
    """
    budget_ms = get_latency_budget_ms(level)
    # Convert to seconds and apply PHI safety margin (1.618x)
    timeout = (budget_ms / 1000.0) * PHI
    # Under high metabolic pressure (CPU/RAM > 80%), reduce timeout to fail fast
    return timeout / metabolic_pressure


#
# 2. MEMORY & BUFFER CAPACITIES
#


def get_buffer_capacity(importance: int = 1) -> int:
    """
    Standard buffer sizes using Fibonacci numbers.
    importance 1 (Low)  -> F(8)  = 21
    importance 2 (Med)  -> F(10) = 55
    importance 3 (High) -> F(12) = 144
    """
    fib_idx = 6 + (importance * 2)
    return fibonacci(fib_idx)


# Standardized Capacities
EVENT_HISTORY_CAP = get_buffer_capacity(2)  # 55
JUDGMENT_BUFFER_CAP = get_buffer_capacity(1)  # 21
LOG_TAIL_CAP = get_buffer_capacity(3)  # 144

# Residual and Axiom Maturity (Fibonacci-aligned)
AXIOM_MATURITY_WINDOW_SIZE = fibonacci(8)  # 21
RESIDUAL_MIN_SAMPLES = fibonacci(7)  # 13
RESIDUAL_STABLE_HIGH_N = fibonacci(5)  # 5

# Domain Specific Capacities
ACT_LOG_CAP = fibonacci(11)  # 89
EVENT_JOURNAL_CAP = fibonacci(12)  # 144
DECISION_TRACE_CAP = fibonacci(10)  # 55
LOOP_CLOSURE_CAP = fibonacci(8)  # 21
SERVICE_REGISTRY_JUDGMENT_CAP = fibonacci(11)  # 89
CHAT_MESSAGE_CAP = fibonacci(10)  # 55

# Monitoring and Health (Fibonacci-aligned)
SIGNAL_TTL_SEC = fibonacci(10)  # 55s
STALL_THRESHOLD_SEC = fibonacci(11)  # 89s
TOPOLOGY_SNAPSHOT_INTERVAL = fibonacci(12)  # 144s

# LOD Level Latency Thresholds (PHI-aligned)
LOD_LEVEL0_LATENCY_MS = 100.0  # REFLEX
LOD_LEVEL1_LATENCY_MS = 300.0  # MICRO
LOD_LEVEL2_LATENCY_MS = 1000.0  # MACRO
LOD_LEVEL3_LATENCY_MS = 3000.0  # META

# Metabolic Load Thresholds for LOD scaling
LOD_LOAD_THRESHOLD_HIGH = PHI_INV * 100  # 61.8%
LOD_LOAD_THRESHOLD_CRITICAL = 82.0  # HOWL_MIN equivalent

# MCTS and Decision Logic (PHI-aligned)
MCTS_UCT_C = math.sqrt(2)  # Standard UCT constant
MCTS_DEPTH_LIMIT = lucas(4)  # L(4)=7
MCTS_SIMULATIONS = fibonacci(10)  # F(10)=55

# LLM Operational Constants
LLM_TIMEOUT_SEC = 120.0  # Increased for CPU-bound local inference (Ryzen 5700G)
LLM_DEFAULT_TEMPERATURE = PHI_INV  # 0.618
MAX_CONTEXT_WINDOW = fibonacci(20)  # 6765 (High resolution reasoning)

# Learning Parameters
Q_LEARNING_GAMMA = PHI_INV  # 0.618 (Discount factor)
Q_LEARNING_ALPHA = PHI_INV_2 / 10  # ~0.038 (Learning rate)
EXPLORATION_EPSILON = PHI_INV_2  # 0.382 (Initial epsilon)

# Confidence Thresholds (PHI-aligned)
CONFIDENCE_ENRICHMENT_MIN_THRESHOLD = (
    PHI_INV  # 0.618 (Minimum confidence for context enrichment)
)

# Kernel Integrity Thresholds (Health-based)
KERNEL_INTEGRITY_WAG_THRESHOLD = 85.0  # 85% health = wagging (good)
KERNEL_INTEGRITY_GROWL_THRESHOLD = 60.0  # 60% health = growling (stressed)
KERNEL_INTEGRITY_HOWL_THRESHOLD = 30.0  # 30% health = howling (critical)

# Economic Constants (PHI-aligned)
COST_EFFICIENCY_CAP_USD = 0.01  # Base efficiency unit
BUDGET_HARD_CAP_USD = fibonacci(7)  # 13.0
BUDGET_WARNING_PCT = PHI_INV * 100  # 61.8%

# Temporal and History (Fibonacci-aligned)
TEMPORAL_MEMORY_WINDOW = fibonacci(8)  # 21
HISTORY_REPLAY_BATCH = fibonacci(6)  # 8

# Tool Executor Output Capacities
BASH_OUTPUT_CAP = 30000
READ_FILE_CAP = 20000
GLOB_MATCH_CAP = 1000
GREP_OUTPUT_CAP = 10000

#
# 3. RESILIENCE & BACKOFF

#


def get_backoff_delay_s(retry_count: int) -> float:
    """
    Geometric backoff using PHI.
    Delay = PHI ^ retry_count
    1st retry: 1.61s
    2nd retry: 2.61s
    3rd retry: 4.23s
    """
    return PHI**retry_count


def get_max_retries(criticality: float = 0.5) -> int:
    """
    Number of allowed retries before giving up.
    Derived from Lucas(4)=7 scaled by criticality.
    """
    return max(1, round(lucas(4) * criticality))


#
# 4. RESPIRATION & HEARTBEAT
#


def get_respiration_interval_s(health_score: float = 100.0) -> float:
    """
    Heartbeat interval (SONA_TICK).
    Formula: Base(F(9)=34s) * (MAX_HEALTH / health_score)
    If health is 50%, interval doubles to 68s (slower breathing to recover).
    """
    base_interval = float(fibonacci(9))
    health_factor = 100.0 / max(health_score, 1.0)
    return base_interval * health_factor


#
# 5. ECONOMIC SCALING
#


def get_cost_threshold_usd(level: ConsciousnessLevel) -> float:
    """
    Max USD cost allowed per judgment.
    Scales geometrically by PHI_INV_2 (0.382).
    """
    base_cost = 0.01  # $0.01 per REFLEX
    level_map = {
        ConsciousnessLevel.REFLEX: 0,
        ConsciousnessLevel.MICRO: 1,
        ConsciousnessLevel.MACRO: 2,
        ConsciousnessLevel.META: 3,
    }
    idx = level_map.get(level, 0)
    # REFLEX: $0.01
    # MICRO:  $0.026 (base * PHI^2)
    # MACRO:  $0.068
    # META:   $0.179
    return base_cost * (PHI ** (idx * 2))
