"""
CYNIC Opcode Semantics — Formal specification of 7-step instruction set

Each opcode has immutable semantics:
- Preconditions: What must be true before executing
- Postconditions: What is guaranteed after execution
- State transitions: Valid next opcodes
- Storage tier: Where data persists
- Cost model: Token/USD cost
- Consciousness gating: Which levels permit this opcode
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable

from cynic.core.phi import PHI_INV, PHI_INV_2, MAX_Q_SCORE
from cynic.core.consciousness import ConsciousnessLevel


# ════════════════════════════════════════════════════════════════════════════
# STORAGE TIER ENUM
# ════════════════════════════════════════════════════════════════════════════

class StorageTier(Enum):
    """Storage tier mapping (HOT, WARM, COLD, FROZEN)."""
    HOT = "hot"        # PostgreSQL — indexed, queryable, immediate
    WARM = "warm"      # Qdrant — semantic vectors, searchable, ~1-30 days
    COLD = "cold"      # Solana PoJ — immutable proof, archived, 30+ days
    FROZEN = "frozen"  # Deleted but hash recorded, audit trail only


# ════════════════════════════════════════════════════════════════════════════
# OPCODE SPECIFICATION
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class OpcodeSpec:
    """Formal specification of a single opcode."""

    name: str
    """Opcode name (e.g., "PERCEIVE", "JUDGE")"""

    preconditions: list[str]
    """What must be true before this opcode runs"""

    postconditions: list[str]
    """What is guaranteed after execution"""

    state_transitions: list[str]
    """List of valid next opcodes. Only these can follow this opcode."""

    storage_tiers: list[StorageTier]
    """Where this opcode writes data (can be multiple tiers)"""

    cost_usd: float | Callable[..., float]
    """Fixed cost or callable returning cost based on parameters"""

    consciousness_gates: dict[ConsciousnessLevel, bool]
    """Which consciousness levels permit this opcode: {L3: True, L2: True, ...}"""

    description: str = ""
    """Human-readable description of what this opcode does"""

    def gate_for_level(self, level: ConsciousnessLevel) -> bool:
        """Check if this opcode is allowed at given consciousness level."""
        return self.consciousness_gates.get(level, False)

    def estimated_cost(self) -> float:
        """Get estimated cost (or call cost_usd if callable)."""
        if callable(self.cost_usd):
            return self.cost_usd()  # Best effort
        return float(self.cost_usd)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 1: [PERCEIVE]
# ════════════════════════════════════════════════════════════════════════════

PERCEIVE_SPEC = OpcodeSpec(
    name="PERCEIVE",
    description=(
        "Receive external signal (code change, feedback, market data) into Cell. "
        "Immutable after creation. May trigger consciousness level escalation."
    ),
    preconditions=[
        "System is alive (any consciousness level ≥ L3)",
        "Source is validated (reality ∈ {CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS})",
        "No ongoing PERCEIVE for identical content (dedup)",
    ],
    postconditions=[
        "Cell is created and immutable",
        "Timestamp recorded",
        "Consciousness level MIGHT escalate based on signal importance",
        "Event PERCEPTION_RECEIVED emitted",
    ],
    state_transitions=["JUDGE"],
    storage_tiers=[StorageTier.HOT],
    cost_usd=0.001,  # Cheap — no LLM
    consciousness_gates={
        ConsciousnessLevel.REFLEX: True,   # Always runs
        ConsciousnessLevel.MICRO: True,
        ConsciousnessLevel.MACRO: True,
        ConsciousnessLevel.META: True,
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 2: [JUDGE]
# ════════════════════════════════════════════════════════════════════════════

JUDGE_SPEC = OpcodeSpec(
    name="JUDGE",
    description=(
        "Analyze Cell using Dogs at consciousness level. "
        "Produce immutable Judgment with Q-Score and Verdict. "
        "Output drives DECIDE governance gate."
    ),
    preconditions=[
        "PERCEIVE completed and Cell is immutable",
        "Consciousness level is known (L3/L2/L1/L4)",
        "Dogs available for this level exist",
        "Cell.confidence still in [0, φ⁻¹] (unknown state)",
    ],
    postconditions=[
        "Judgment created with immutable Q-Score ∈ [0, 100]",
        "Verdict assigned: HOWL (≥82) | WAG (≥61.8) | GROWL (≥38.2) | BARK (<38.2)",
        "Confidence φ-bounded to [0, 0.618]",
        "Dog votes recorded (for audit)",
        "PBFT consensus algorithm applied (if L1+)",
        "Event JUDGMENT_CREATED emitted with full breakdown",
    ],
    state_transitions=["DECIDE"],
    storage_tiers=[StorageTier.HOT, StorageTier.WARM, StorageTier.COLD],
    cost_usd=lambda level: {
        "L3": 0.02,      # Local pattern matching
        "L2": 0.15,      # Quick LLM calls (7 dogs)
        "L1": 2.50,      # Full MCTS (7 parallel LLM × ~350ms each)
        "L4": 5.00,      # Evolution + consolidation
    }.get(str(level), 1.0),
    consciousness_gates={
        ConsciousnessLevel.REFLEX: True,   # Non-LLM dogs only
        ConsciousnessLevel.MICRO: True,    # Quick scoring
        ConsciousnessLevel.MACRO: True,    # Full judgment
        ConsciousnessLevel.META: True,     # Evolution judgment
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 3: [DECIDE]
# ════════════════════════════════════════════════════════════════════════════

DECIDE_SPEC = OpcodeSpec(
    name="DECIDE",
    description=(
        "Apply governance policy to Judgment. "
        "Gate: Can this action proceed? Does TIER allow it? "
        "Output: APPROVED | REJECTED | HUMAN_REVIEW_REQUIRED"
    ),
    preconditions=[
        "JUDGE completed and Judgment immutable",
        "Judgment.confidence meets GOVERNANCE_THRESHOLD",
        "User TIER constraint is known",
        "Action type is known (bash, edit, read, git, etc)",
    ],
    postconditions=[
        "Decision status: APPROVED | REJECTED | HUMAN_REVIEW_REQUIRED",
        "If APPROVED: ProposedAction created and PENDING",
        "TIER constraints applied (low TIER → fewer action types)",
        "Event DECISION_MADE emitted",
    ],
    state_transitions=["ACT"],  # If approved. Otherwise loop back to PERCEIVE
    storage_tiers=[StorageTier.WARM],
    cost_usd=0.0,  # Local policy evaluation
    consciousness_gates={
        ConsciousnessLevel.REFLEX: False,  # No decisions
        ConsciousnessLevel.MICRO: False,   # Observation only
        ConsciousnessLevel.MACRO: True,    # Full DECIDE
        ConsciousnessLevel.META: True,     # Meta-decisions
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 4: [ACT]
# ════════════════════════════════════════════════════════════════════════════

ACT_SPEC = OpcodeSpec(
    name="ACT",
    description=(
        "Execute approved Decision on external systems. "
        "Spawn bash, apply edits, read files, run git, etc. "
        "Record execution outcome (success/failure)."
    ),
    preconditions=[
        "DECIDE approved the action",
        "Human confirmed (if required by TIER)",
        "Resource budget available (tokens, compute time)",
        "No contradictory action in progress",
    ],
    postconditions=[
        "Action executed (immutable execution record)",
        "Outcome recorded (exit code, duration, actual cost)",
        "Learning signal queued for LEARN opcode",
        "Event ACTION_EXECUTED emitted",
    ],
    state_transitions=["LEARN"],
    storage_tiers=[StorageTier.COLD],  # Immutable proof on Solana PoJ
    cost_usd=lambda: 0.0,  # Varies (bash, file I/O costs extracted separately)
    consciousness_gates={
        ConsciousnessLevel.REFLEX: False,  # No actions
        ConsciousnessLevel.MICRO: False,   # Read-only
        ConsciousnessLevel.MACRO: True,    # Full actions
        ConsciousnessLevel.META: True,     # Meta-actions
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 5: [LEARN]
# ════════════════════════════════════════════════════════════════════════════

LEARN_SPEC = OpcodeSpec(
    name="LEARN",
    description=(
        "Update Q-Table and E-Score from action outcome + human feedback. "
        "Fisher-weighted EWC protects high-visit states. "
        "Track calibration: (CYNIC_confidence, actual_outcome)."
    ),
    preconditions=[
        "ACT executed and outcome recorded",
        "Human feedback available (rating, correction, or implicit from outcome)",
        "State-action pair identified for Q-Table",
    ],
    postconditions=[
        "Q-Table updated with Fisher-weighted α",
        "E-Score dimension updated (EMA α=0.618)",
        "Calibration tracked for confidence adjustment",
        "High-visit entries (21+) 4× more resistant to change (EWC)",
        "Event LEARNING_SIGNAL_PROCESSED emitted",
    ],
    state_transitions=["ACCOUNT"],
    storage_tiers=[StorageTier.WARM],  # Q-Table + E-Score rows
    cost_usd=0.01,  # Local computation
    consciousness_gates={
        ConsciousnessLevel.REFLEX: False,  # No learning
        ConsciousnessLevel.MICRO: False,   # No LLM training
        ConsciousnessLevel.MACRO: True,    # Full Q-Learning
        ConsciousnessLevel.META: True,     # Fisher locking
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 6: [ACCOUNT]
# ════════════════════════════════════════════════════════════════════════════

ACCOUNT_SPEC = OpcodeSpec(
    name="ACCOUNT",
    description=(
        "Record economic costs and signal BURN axiom. "
        "Track: tokens consumed, compute time, storage bytes. "
        "Update budget and axiom maturity signals."
    ),
    preconditions=[
        "LEARN completed",
        "Actual costs measured",
        "User budget is known",
    ],
    postconditions=[
        "Cost recorded immutably",
        "E-Score BURN dimension updated",
        "Axiom signal sent (BURN violated or excellent)",
        "Budget updated (spent += cost, remaining -= cost)",
        "If low budget: consciousness_level_request = throttle to MICRO",
        "Event COST_ACCOUNTED emitted",
    ],
    state_transitions=["EMERGE"],
    storage_tiers=[StorageTier.COLD],  # Immutable cost proof on Solana PoJ
    cost_usd=0.0,  # Local accounting
    consciousness_gates={
        ConsciousnessLevel.REFLEX: False,
        ConsciousnessLevel.MICRO: False,
        ConsciousnessLevel.MACRO: True,
        ConsciousnessLevel.META: True,
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE 7: [EMERGE]
# ════════════════════════════════════════════════════════════════════════════

EMERGE_SPEC = OpcodeSpec(
    name="EMERGE",
    description=(
        "Meta-pattern detection over rolling window of past judgments. "
        "Unlock axioms. Detect residuals. "
        "Suggest consciousness level escalation."
    ),
    preconditions=[
        "ACCOUNT completed",
        "Ring buffer has ≥ F(9)=34 entries (sufficient history)",
        "L4 META scheduler triggered (or manual probe)",
    ],
    postconditions=[
        "Patterns recorded (STABLE_HIGH, RISING, RESIDUAL, SPIKE)",
        "Axiom signals sent (AUTONOMY, SYMBIOSIS, EMERGENCE, ANTIFRAGILITY)",
        "Consciousness level recommendation updated",
        "E-Score consolidation (Fisher locking high-visit states)",
        "Event EMERGENCE_DETECTED emitted",
        "Event AXIOM_ACTIVATED emitted (if new axiom unlocked)",
    ],
    state_transitions=["PERCEIVE"],  # Loop back to start
    storage_tiers=[StorageTier.WARM, StorageTier.COLD],  # Patterns + PoJ proof
    cost_usd=0.50,  # Vector search + consolidation
    consciousness_gates={
        ConsciousnessLevel.REFLEX: False,
        ConsciousnessLevel.MICRO: False,
        ConsciousnessLevel.MACRO: False,
        ConsciousnessLevel.META: True,  # Only L4 (daily)
    },
)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE REGISTRY (Immutable)
# ════════════════════════════════════════════════════════════════════════════

OPCODE_REGISTRY: dict[str, OpcodeSpec] = {
    "PERCEIVE": PERCEIVE_SPEC,
    "JUDGE": JUDGE_SPEC,
    "DECIDE": DECIDE_SPEC,
    "ACT": ACT_SPEC,
    "LEARN": LEARN_SPEC,
    "ACCOUNT": ACCOUNT_SPEC,
    "EMERGE": EMERGE_SPEC,
}

OPCODE_NAMES: list[str] = [
    "PERCEIVE",
    "JUDGE",
    "DECIDE",
    "ACT",
    "LEARN",
    "ACCOUNT",
    "EMERGE",
]


# ════════════════════════════════════════════════════════════════════════════
# HELPER FUNCTIONS
# ════════════════════════════════════════════════════════════════════════════

def get_opcode_spec(name: str) -> OpcodeSpec | None:
    """Get opcode specification by name."""
    return OPCODE_REGISTRY.get(name.upper())


def verify_state_transition(from_opcode: str, to_opcode: str) -> bool:
    """
    Verify that a state transition is legal.

    Legal: to_opcode is in from_opcode's state_transitions list.
    """
    spec = get_opcode_spec(from_opcode)
    if not spec:
        return False
    return to_opcode.upper() in spec.state_transitions


def opcodes_for_level(level: ConsciousnessLevel) -> list[str]:
    """Return list of opcodes available at given consciousness level."""
    return [
        name
        for name, spec in OPCODE_REGISTRY.items()
        if spec.gate_for_level(level)
    ]


def all_opcodes_documented() -> bool:
    """Verify all 7 opcodes are documented."""
    return len(OPCODE_REGISTRY) == 7 and set(OPCODE_REGISTRY.keys()) == set(OPCODE_NAMES)
