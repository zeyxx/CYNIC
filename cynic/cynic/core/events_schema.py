"""
Typed event payload schemas — Pydantic v2.

WHY:
  Event payloads are Dict[str, Any] everywhere. Typos silently become
  None defaults, wrong types go undetected, IDE provides no completion.
  These schemas document the contract, enable IDE completion,
  and catch emitter/consumer disagreements at test time.

PATTERN (copy this in new handlers):

    from cynic.core.events_schema import JudgmentCreatedPayload

    async def _on_judgment_created(self, event: Event) -> None:
        try:
            p = JudgmentCreatedPayload.model_validate(event.payload or {})
            verdict = p.verdict        # str, typed, IDE-friendly
            q_score = p.q_score        # float, guaranteed
            reality = p.reality        # str, default "CODE"
            ...
        except Exception:
            pass

PHILOSOPHY:
  Extra fields allowed (extra='allow') — emitters often include fields
  not consumed by all subscribers. We never reject unknown fields.
  Defaults match the handler's existing .get("key", default) values
  so migration is non-breaking: old handlers keep working, new ones
  get type safety for free.

  Senior engineers set the patterns. This file IS the pattern.
"""
from __future__ import annotations

from typing import Any, Dict

from pydantic import BaseModel, ConfigDict, Field

# Shared config: never reject extra fields, allow post-init mutation
_BASE = ConfigDict(extra="allow", frozen=False)


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT LIFECYCLE
# ════════════════════════════════════════════════════════════════════════════

class JudgmentCreatedPayload(BaseModel):
    """
    JUDGMENT_CREATED — emitted by orchestrator after dog consensus.

    Most-subscribed event in the system: ANTIFRAGILITY axiom, BURN EScore,
    guidance writer, residual detector, and 8+ other handlers react to this.
    """
    model_config = _BASE

    state_key:       str                = ""
    verdict:         str                = "WAG"   # BARK|GROWL|WAG|HOWL
    q_score:         float              = 0.0     # [0, 100] — NOT φ-bounded
    confidence:      float              = 0.0     # φ-bounded ≤ 0.618
    reality:         str                = "CODE"  # CODE|CYNIC|SOLANA|HUMAN|MARKET|SOCIAL|COSMOS
    dog_votes:       dict[str, float]   = Field(default_factory=dict)
    judgment_id:     str                = ""
    level_used:      str                = ""      # REFLEX|MICRO|MACRO
    content_preview: str                = ""
    context:         str                = ""


class JudgmentRequestedPayload(BaseModel):
    """JUDGMENT_REQUESTED — orchestrator begins judgment pipeline for a cell."""
    model_config = _BASE

    cell_id: str = ""
    reality: str = "CODE"
    level:   str = ""     # REFLEX|MICRO|MACRO


class JudgmentFailedPayload(BaseModel):
    """JUDGMENT_FAILED — judgment pipeline failed (circuit breaker or exception)."""
    model_config = _BASE

    cell_id:       str = ""
    error:         str = ""
    circuit_state: str = ""      # CircuitState value (only set on circuit breaker path)
    failure_count: int = 0       # consecutive failures (only set on circuit breaker path)


class ConsensusReachedPayload(BaseModel):
    """CONSENSUS_REACHED — PBFT quorum met, judgment accepted."""
    model_config = _BASE

    q_score:  float = 0.0
    votes:    int   = 0
    quorum:   int   = 0
    verdict:  str   = "WAG"


class ConsensusFailedPayload(BaseModel):
    """CONSENSUS_FAILED — PBFT quorum not met, judgment rejected."""
    model_config = _BASE

    votes:   int = 0
    quorum:  int = 0
    reason:  str = ""


# ════════════════════════════════════════════════════════════════════════════
# LEARNING
# ════════════════════════════════════════════════════════════════════════════

class LearningEventPayload(BaseModel):
    """LEARNING_EVENT — TD(0) reward signal for Q-Table update."""
    model_config = _BASE

    reward:    float = 0.0   # [0, 1] normalised reward
    action:    str   = ""
    state_key: str   = ""


class QTableUpdatedPayload(BaseModel):
    """Q_TABLE_UPDATED — emitted after Q-Table flush to DB."""
    model_config = _BASE

    flushed: int = 0   # number of entries persisted


class EwcCheckpointPayload(BaseModel):
    """EWC_CHECKPOINT — elastic weight consolidation snapshot taken."""
    model_config = _BASE

    q_value:   float = 0.0   # [0, 1]
    state_key: str   = ""
    action:    str   = ""


class MetaCyclePayload(BaseModel):
    """
    META_CYCLE — emitted by orchestrator.evolve() every ~4 hours.

    The 'evolve' dict contains pass_rate and regression from
    the meta-evaluation sweep. Use the properties for convenience.
    """
    model_config = _BASE

    evolve: dict[str, Any] = Field(default_factory=dict)

    @property
    def pass_rate(self) -> float:
        return float(self.evolve.get("pass_rate", 0.0))

    @property
    def regression(self) -> bool:
        return bool(self.evolve.get("regression", False))


# ════════════════════════════════════════════════════════════════════════════
# PERCEPTION
# ════════════════════════════════════════════════════════════════════════════

class PerceptionReceivedPayload(BaseModel):
    """PERCEPTION_RECEIVED — raw perception input received from hook."""
    model_config = _BASE

    cell_id:  str = ""
    reality:  str = "CODE"
    source:   str = ""
    data:     str = ""     # truncated perception content


class AnomalyDetectedPayload(BaseModel):
    """ANOMALY_DETECTED — unusual signal detected in perception stream."""
    model_config = _BASE

    severity: float = 0.0   # [0, 1]
    reality:  str   = "CODE"
    analysis: str   = ""


# ════════════════════════════════════════════════════════════════════════════
# DECISION / ACT
# ════════════════════════════════════════════════════════════════════════════

class DecisionMadePayload(BaseModel):
    """
    DECISION_MADE — Decider selected an action from Q-Table/MCTS.

    Used by ActionProposer (L1), AxiomMonitor (AUTONOMY signal),
    and the feedback loop cross-feed (L2→L1 when verdict==BARK).
    """
    model_config = _BASE

    verdict:            str   = "WAG"
    reality:            str   = "CODE"
    state_key:          str   = ""
    q_value:            float = 0.0      # [0, 1] raw Q-value
    confidence:         float = 0.0
    recommended_action: str   = ""
    action_prompt:      str   = ""
    trigger:            str   = ""       # "auto_decide" | "manual" | ...
    mcts:               bool  = False


class ActRequestedPayload(BaseModel):
    """ACT_REQUESTED — execution requested for a proposed action."""
    model_config = _BASE

    action:      str = ""    # the prompt/command to execute
    target:      str = ""    # working directory (optional)
    action_id:   str = ""
    action_type: str = ""


class ActCompletedPayload(BaseModel):
    """ACT_COMPLETED — ClaudeCodeRunner execution finished."""
    model_config = _BASE

    action_id: str   = ""
    success:   bool  = False
    cost_usd:  float = 0.0
    exec_id:   str   = ""
    output:    str   = ""


class ActionProposedPayload(BaseModel):
    """
    ACTION_PROPOSED — ActionProposer queued a new ProposedAction.

    Priority mapping → EScore BUILD:
      1 (critical) → MAX_Q_SCORE (100.0)
      2 (high)     → HOWL_MIN    (82.0)
      3 (normal)   → WAG_MIN     (68.2)
      else         → GROWL_MIN   (38.2)
    """
    model_config = _BASE

    action_id:   str = ""
    action_type: str = ""
    verdict:     str = "WAG"
    reality:     str = "CODE"
    priority:    int = 3    # 1=critical, 2=high, 3=normal
    description: str = ""


# ════════════════════════════════════════════════════════════════════════════
# EMERGENCE
# ════════════════════════════════════════════════════════════════════════════

class EmergenceDetectedPayload(BaseModel):
    """EMERGENCE_DETECTED — ResidualDetector fired (anomaly pattern)."""
    model_config = _BASE

    pattern_type:    str              = ""    # SPIKE|STABLE_HIGH|RISING
    severity:        float            = 0.5   # [0, 1]
    evidence:        dict[str, Any]   = Field(default_factory=dict)
    judgment_id:     str              = ""
    reality:         str              = "CODE"
    analysis:        str              = ""
    total_anomalies: int              = 0
    total_patterns:  int              = 0


class ResidualHighPayload(BaseModel):
    """
    RESIDUAL_HIGH — unnameable signal detected (φ-level residual variance).

    Emitted by orchestrator when judgment.unnameable_detected == True.
    Triggers SelfProber L4 analysis and EMERGENCE EScore update.
    """
    model_config = _BASE

    cell_id:           str   = ""
    residual_variance: float = 0.0
    judgment_id:       str   = ""


class AxiomActivatedPayload(BaseModel):
    """AXIOM_ACTIVATED — emergent axiom crossed WAG_MIN threshold."""
    model_config = _BASE

    axiom:    str   = ""    # EMERGENCE|AUTONOMY|SYMBIOSIS|ANTIFRAGILITY|CONSCIOUSNESS
    maturity: float = 0.0
    trigger:  str   = ""    # which event caused the threshold crossing


class TranscendencePayload(BaseModel):
    """
    TRANSCENDENCE — all 4 emergent axioms simultaneously ACTIVE.

    Highest state in the system. Emitted by _on_axiom_activated when
    AUTONOMY + SYMBIOSIS + EMERGENCE + ANTIFRAGILITY all reach WAG_MIN.
    """
    model_config = _BASE

    active_axioms: list[str] = Field(default_factory=list)
    maturity:      float     = 0.0


class SelfImprovementProposedPayload(BaseModel):
    """SELF_IMPROVEMENT_PROPOSED — SelfProber L4 analysis complete."""
    model_config = _BASE

    trigger:       str              = ""    # EMERGENCE|SCHEDULE|MANUAL
    pattern_type:  str              = ""    # SPIKE|RISING|STABLE_HIGH|UNKNOWN
    severity:      float            = 0.5   # [0, 1]
    proposals:     list[dict]       = Field(default_factory=list)
    total_pending: int              = 0


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS
# ════════════════════════════════════════════════════════════════════════════

class ConsciousnessChangedPayload(BaseModel):
    """CONSCIOUSNESS_CHANGED — LOD (Level of Detail) transitioned."""
    model_config = _BASE

    direction:  str = ""    # "UP" | "DOWN"
    from_name:  str = ""
    to_name:    str = ""
    from_lod:   int = 0     # SurvivalLOD.value (0-3)
    to_lod:     int = 0


class BudgetWarningPayload(BaseModel):
    """BUDGET_WARNING — spend approaching session budget φ-threshold."""
    model_config = _BASE

    remaining_usd:      float = 0.0
    total_spent_usd:    float = 0.0
    ratio_remaining:    float = 0.0
    session_budget_usd: float = 0.0


class BudgetExhaustedPayload(BaseModel):
    """BUDGET_EXHAUSTED — session budget fully consumed."""
    model_config = _BASE

    total_spent_usd:    float = 0.0
    session_budget_usd: float = 0.0
    overspend_usd:      float = 0.0


# ════════════════════════════════════════════════════════════════════════════
# STORAGE / SYSTEM HEALTH
# ════════════════════════════════════════════════════════════════════════════

class DiskPressurePayload(BaseModel):
    """DISK_PRESSURE — DiskHealthMonitor reports disk > φ threshold."""
    model_config = _BASE

    pressure: str   = "WARN"   # "WARN" | "CRITICAL"
    used_pct: float = 0.0
    disk_pct: float = 0.0      # preferred alias for used_pct


class MemoryPressurePayload(BaseModel):
    """MEMORY_PRESSURE — MemoryWatcher reports RAM > φ threshold."""
    model_config = _BASE

    pressure:   str   = "WARN"
    used_pct:   float = 0.0
    memory_pct: float = 0.0    # preferred alias for used_pct


class DiskClearedPayload(BaseModel):
    """DISK_CLEARED — disk pressure resolved (back to OK)."""
    model_config = _BASE

    disk_pct: float = 0.0
    free_gb:  float = 0.0


class MemoryClearedPayload(BaseModel):
    """MEMORY_CLEARED — RAM pressure resolved (back to OK)."""
    model_config = _BASE

    memory_pct: float = 0.0
    free_gb:    float = 0.0


# ════════════════════════════════════════════════════════════════════════════
# SDK (Claude Code --sdk-url sessions)
# ════════════════════════════════════════════════════════════════════════════

class SdkSessionStartedPayload(BaseModel):
    """SDK_SESSION_STARTED — new Claude Code SDK session opened."""
    model_config = _BASE

    session_id: str = ""


class SdkToolJudgedPayload(BaseModel):
    """
    SDK_TOOL_JUDGED — GUARDIAN judged a Claude SDK tool invocation.

    Verdict → GRAPH EScore:
      HOWL  → HOWL_MIN (82.0) + SYMBIOSIS signal
      WAG   → WAG_MIN  (61.8)
      GROWL → GROWL_MIN (38.2)
      BARK  → 0.0 (trust breakdown)
    """
    model_config = _BASE

    session_id: str = ""
    tool:       str = ""    # "bash" | "read" | "write" | ...
    verdict:    str = ""    # "HOWL" | "WAG" | "GROWL" | "BARK"


class SdkResultReceivedPayload(BaseModel):
    """SDK_RESULT_RECEIVED — Claude Code SDK session completed."""
    model_config = _BASE

    session_id:     str   = ""
    is_error:       bool  = False
    cost_usd:       float = 0.0
    output_q_score: float = 0.0
    output:         str   = ""


# ════════════════════════════════════════════════════════════════════════════
# USER
# ════════════════════════════════════════════════════════════════════════════

class UserFeedbackPayload(BaseModel):
    """USER_FEEDBACK — human rated a judgment (1–5 stars)."""
    model_config = _BASE

    rating:      float = 3.0   # [1, 5]
    reward:      float = 0.5   # normalised reward [0.1, 0.9]
    sentiment:   float = 0.0   # (rating - 3) / 2.0
    state_key:   str   = ""
    action:      str   = ""
    judgment_id: str   = ""
    comment:     str   = ""


class UserCorrectionPayload(BaseModel):
    """USER_CORRECTION — human explicitly corrected a decision."""
    model_config = _BASE

    action:    str = ""
    state_key: str = ""
    reason:    str = ""
