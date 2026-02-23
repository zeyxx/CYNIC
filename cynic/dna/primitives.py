"""CYNIC DNA Primitives — Five core operations (PERCEIVE, JUDGE, DECIDE, ACT, LEARN)."""
from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Literal, Optional
from uuid import uuid4

from cynic.core.judgment import Cell, Judgment
from cynic.core.phi import MAX_Q_SCORE, PHI_INV, MAX_CONFIDENCE


# ============================================================================
# DATA TYPES (What primitives work with)
# ============================================================================

@dataclass
class DNA_Cell:
    """Input to PERCEIVE: what we observe from reality."""
    id: str = field(default_factory=lambda: str(uuid4())[:8])
    source: Literal["code", "git", "social", "market", "health", "solana"] = "code"
    content: str = ""
    metadata: dict = field(default_factory=dict)
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())

    def to_cell(self) -> Cell:
        """Convert to CYNIC Cell for judgment."""
        return Cell(
            cell_id=self.id,
            reality=self.source.upper(),
            analysis="JUDGE",
            content=self.content,
            context=self.metadata.get("context", ""),
            budget_usd=self.metadata.get("budget_usd", 0.01),
        )


@dataclass
class DNA_Judgment:
    """Output of JUDGE: what CYNIC thinks."""
    id: str = field(default_factory=lambda: str(uuid4())[:8])
    cell_id: str = ""
    q_score: float = 0.0  # [0, 100]
    verdict: str = "BARK"  # HOWL, WAG, GROWL, BARK
    confidence: float = 0.0  # [0, 0.618]
    dogs_votes: dict[str, float] = field(default_factory=dict)  # DogId: score
    reasoning: str = ""
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())

    @classmethod
    def from_judgment(cls, judgment: Judgment) -> DNA_Judgment:
        """Convert CYNIC Judgment to DNA_Judgment."""
        return cls(
            cell_id=judgment.cell_id,
            q_score=judgment.q_score,
            verdict=judgment.verdict.value if hasattr(judgment.verdict, 'value') else str(judgment.verdict),
            confidence=judgment.confidence,
            dogs_votes=judgment.dog_votes or {},
            reasoning=judgment.reasoning or "",
        )


@dataclass
class DNA_Decision:
    """Output of DECIDE: what to do."""
    id: str = field(default_factory=lambda: str(uuid4())[:8])
    judgment_id: str = ""
    axiom: str = "PHI"  # Which axiom applied
    action_type: str = ""  # What kind of action (alert, report, fix, learn)
    action_params: dict = field(default_factory=dict)  # Parameters for executor
    confidence: float = 0.0
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())


@dataclass
class DNA_Result:
    """Output of ACT: what happened."""
    id: str = field(default_factory=lambda: str(uuid4())[:8])
    decision_id: str = ""
    status: Literal["success", "failed", "partial"] = "success"
    output: str = ""
    error: str = ""
    metrics: dict = field(default_factory=dict)
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())


# ============================================================================
# THE FIVE PRIMITIVES
# ============================================================================

async def PERCEIVE(
    source: Literal["code", "git", "social", "market", "health", "solana"],
    content: str,
    metadata: dict = None,
) -> DNA_Cell:
    """
    PRIMITIVE 1: PERCEIVE
    Convert external reality into Cell.

    Args:
        source: What we're observing (code, git, social, etc)
        content: The actual content to observe
        metadata: Additional context (budget_usd, etc)

    Returns:
        DNA_Cell ready for judgment

    Example:
        >>> cell = await PERCEIVE("code", "def hello(): pass")
        >>> print(cell.q_score)
    """
    return DNA_Cell(
        source=source,
        content=content,
        metadata=metadata or {},
    )


async def JUDGE(
    cell: DNA_Cell,
    level: Literal["REFLEX", "MICRO", "MACRO", "META"] = "MACRO",
    orchestrator: Any = None,  # Injected at call time
) -> DNA_Judgment:
    """
    PRIMITIVE 2: JUDGE
    Run CYNIC's 7-step judgment cycle.

    Args:
        cell: DNA_Cell to judge
        level: Consciousness level (REFLEX=fast, MACRO=deep)
        orchestrator: CYNIC's orchestrator (injected from state)

    Returns:
        DNA_Judgment with q_score, verdict, dogs' votes

    Example:
        >>> judgment = await JUDGE(cell, level="MACRO")
        >>> print(judgment.verdict)  # "HOWL", "WAG", "GROWL", or "BARK"
    """
    if orchestrator is None:
        # Placeholder: in real use, orchestrator is injected
        raise RuntimeError("JUDGE requires orchestrator (injected from app state)")

    # Convert DNA_Cell to CYNIC Cell
    cynic_cell = cell.to_cell()

    # Run judgment through orchestrator
    judgment = await orchestrator.run(cynic_cell, level=level)

    # Convert back to DNA_Judgment
    return DNA_Judgment.from_judgment(judgment)


def DECIDE(
    judgment: DNA_Judgment,
    axiom: Literal["PHI", "VERIFY", "CULTURE", "BURN", "FIDELITY"] = "PHI",
) -> DNA_Decision:
    """
    PRIMITIVE 3: DECIDE
    Apply axiom to judgment, decide what to do.

    Args:
        judgment: DNA_Judgment from JUDGE
        axiom: Which axiom to apply

    Returns:
        DNA_Decision with action plan

    Example:
        >>> decision = DECIDE(judgment, axiom="VERIFY")
        >>> print(decision.action_type)
    """
    # Axiom weights (simplified)
    axiom_weights = {
        "PHI": {"alert": 0.5, "report": 0.3, "learn": 0.2},
        "VERIFY": {"alert": 0.6, "fix": 0.3, "learn": 0.1},
        "CULTURE": {"report": 0.5, "learn": 0.3, "alert": 0.2},
        "BURN": {"learn": 0.6, "report": 0.4},
        "FIDELITY": {"alert": 0.4, "report": 0.4, "learn": 0.2},
    }

    weights = axiom_weights.get(axiom, axiom_weights["PHI"])

    # Choose action based on judgment + axiom
    if judgment.q_score > 80:
        action_type = "report"  # Good news to share
    elif judgment.q_score > 60:
        action_type = "learn"  # Interesting patterns
    elif judgment.q_score > 40:
        action_type = "alert"  # Potential issues
    else:
        action_type = "fix"  # Serious problems

    return DNA_Decision(
        judgment_id=judgment.id,
        axiom=axiom,
        action_type=action_type,
        confidence=min(judgment.confidence * 1.2, MAX_CONFIDENCE),  # Boost slightly
    )


async def ACT(
    decision: DNA_Decision,
    executor: Literal["report", "alert", "fix", "learn"] = "report",
) -> DNA_Result:
    """
    PRIMITIVE 4: ACT
    Execute decision via specified executor.

    Args:
        decision: DNA_Decision to execute
        executor: How to execute (report, alert, fix, learn)

    Returns:
        DNA_Result with outcome

    Example:
        >>> result = await ACT(decision, executor="report")
        >>> print(result.status)
    """
    # Simulate execution (in real use, calls actual executors)
    await asyncio.sleep(0.01)  # Minimal delay

    try:
        if executor == "report":
            output = f"Report generated for decision {decision.id}"
        elif executor == "alert":
            output = f"Alert issued for decision {decision.id}"
        elif executor == "fix":
            output = f"Fix attempted for decision {decision.id}"
        elif executor == "learn":
            output = f"Learning signal emitted for decision {decision.id}"
        else:
            output = f"Unknown executor: {executor}"

        return DNA_Result(
            decision_id=decision.id,
            status="success",
            output=output,
            metrics={"executor": executor},
        )
    except EventBusError as e:
        return DNA_Result(
            decision_id=decision.id,
            status="failed",
            error=str(e),
        )


async def LEARN(
    result: DNA_Result,
    signal: Literal["success", "failure", "human_feedback"] = "success",
    qtable: Any = None,  # Injected at call time
) -> dict:
    """
    PRIMITIVE 5: LEARN
    Update QTable from result feedback.

    Args:
        result: DNA_Result from ACT
        signal: Was it successful/failed/feedback?
        qtable: CYNIC's QTable (injected from state)

    Returns:
        Learning metrics

    Example:
        >>> metrics = await LEARN(result, signal="success")
        >>> print(metrics)
    """
    if qtable is None:
        # Placeholder: in real use, qtable is injected
        return {"error": "LEARN requires qtable (injected from app state)"}

    # Map signal to reward
    reward_map = {
        "success": 0.8,
        "failure": 0.2,
        "human_feedback": 0.5,
    }
    reward = reward_map.get(signal, 0.5)

    # Update QTable (simplified)
    try:
        # In real use: qtable.update(state_key, action, reward)
        return {
            "status": "updated",
            "signal": signal,
            "reward": reward,
            "timestamp": datetime.now().isoformat(),
        }
    except asyncpg.Error as e:
        return {"error": str(e)}


# ============================================================================
# UTILITY: Chain primitives together
# ============================================================================

async def run_dna_chain(
    cell_input: DNA_Cell,
    level: str = "MACRO",
    axiom: str = "PHI",
    executor: str = "report",
    orchestrator: Any = None,
    qtable: Any = None,
) -> dict:
    """
    Run all 5 primitives in sequence (PERCEIVE → JUDGE → DECIDE → ACT → LEARN).

    Example:
        >>> result = await run_dna_chain(cell, orchestrator=orch, qtable=qtable)
        >>> print(result)
    """
    # Already have cell from PERCEIVE (input)
    cell = cell_input

    # JUDGE
    judgment = await JUDGE(cell, level=level, orchestrator=orchestrator)

    # DECIDE
    decision = DECIDE(judgment, axiom=axiom)

    # ACT
    act_result = await ACT(decision, executor=executor)

    # LEARN
    learn_result = await LEARN(act_result, signal="success", qtable=qtable)

    return {
        "cell": cell,
        "judgment": judgment,
        "decision": decision,
        "act_result": act_result,
        "learn_result": learn_result,
    }
