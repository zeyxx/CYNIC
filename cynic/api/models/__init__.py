"""
API models subpackage — All Pydantic models for CYNIC HTTP API.

Contains both:
1. Core judgment/perceive models (from core_models.py)
2. Organism state models (from organism_state.py)
"""

# Core judgment/perceive models
from cynic.api.models.core_models import (
    JudgeRequest,
    JudgeResponse,
    PerceiveRequest,
    PerceiveResponse,
    LearnRequest,
    LearnResponse,
    FeedbackRequest,
    FeedbackResponse,
    PolicyResponse,
    AccountRequest,
    AccountResponse,
    HealthResponse,
    StatsResponse,
    EventSnapshot,
    EcosystemStateResponse,
    DecisionPathStage,
    DecisionTraceResponse,
    TopologyConsciousnessResponse,
    GuardrailDecision,
    NervousSystemAuditResponse,
    SelfAwarenessResponse,
)

# Organism state models (from organism_state.py)
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogStatus,
    DogsResponse,
    ProposedAction,
    ActionsResponse,
)

__all__ = [
    # Core models
    "JudgeRequest",
    "JudgeResponse",
    "PerceiveRequest",
    "PerceiveResponse",
    "LearnRequest",
    "LearnResponse",
    "FeedbackRequest",
    "FeedbackResponse",
    "PolicyResponse",
    "AccountRequest",
    "AccountResponse",
    "HealthResponse",
    "StatsResponse",
    "EventSnapshot",
    "EcosystemStateResponse",
    "DecisionPathStage",
    "DecisionTraceResponse",
    "TopologyConsciousnessResponse",
    "GuardrailDecision",
    "NervousSystemAuditResponse",
    "SelfAwarenessResponse",
    # Organism state models
    "StateSnapshotResponse",
    "ConsciousnessResponse",
    "DogStatus",
    "DogsResponse",
    "ProposedAction",
    "ActionsResponse",
]
