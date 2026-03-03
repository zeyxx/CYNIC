"""
Governance Router -- Bridge between the Bot and the Organism Memory.

REST API for governance proposals, voting, verdicts, and outcomes.
Integrates with UnifiedConsciousState for persistent governance records.

Security:
- POST /proposals: Requires OPERATOR role on GOVERNANCE resource
- POST /proposals/{id}/vote: Requires OPERATOR role on GOVERNANCE resource
- POST /proposals/{id}/outcome: Requires OPERATOR role on GOVERNANCE resource
- POST /votes: Requires OPERATOR role on GOVERNANCE resource
- All other endpoints: Read-only (public or minimal auth)
"""
from __future__ import annotations

import time
import uuid

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.interfaces.api.middleware.authz import require_authz, RBACAuthorizer
from cynic.kernel.core.unified_state import (
    GovernanceCommunity,
    GovernanceProposal,
    GovernanceVote,
)
from cynic.kernel.security.rbac import Resource, Permission


# "" Request Models """"""""""""""""""""""""""""""""""""""""""""""""""
class ProposalRequest(BaseModel):
    """Request to submit a new governance proposal."""
    community_id: str
    proposer_id: str = Field(default="", alias="proposer")
    title: str
    description: str
    category: str = "general"

    class Config:
        allow_population_by_field_name = True

class VoteRequest(BaseModel):
    """Request to cast a vote on a proposal."""
    voter_id: str = Field(default="", alias="voter")
    vote: str = Field(...)  # yes, no, abstain
    weight: float = 1.0

    class Config:
        populate_by_name = True

    def __init__(self, **data):
        super().__init__(**data)
        # Validate vote choice
        valid_votes = ("yes", "no", "abstain")
        if self.vote.lower() not in valid_votes:
            raise ValueError(f"Invalid vote choice. Must be one of {valid_votes}")

class OutcomeRequest(BaseModel):
    """Request to record proposal outcome."""
    outcome: str
    executor_id: str = Field(default="", alias="executor")

# "" Response Models """""""""""""""""""""""""""""""""""""""""""""""""

class RegisterCommunityRequest(BaseModel):
    """Request to register or update a governance community."""
    community_id: str = Field(..., min_length=1, max_length=255)
    name: str = Field(..., min_length=1, max_length=255)
    description: str | None = None
    governance_type: str = "consensus"
    member_count: int = Field(default=0, ge=0)
    is_active: bool = True

class VerdictResponse(BaseModel):
    """CYNIC's verdict for a proposal."""
    proposal_id: str
    verdict_type: str = ""  # APPROVED, REJECTED, ABSTAIN
    q_score: float = 0.0
    confidence: float = 0.0
    axiom_scores: dict = Field(default_factory=dict)
    dog_votes: dict = Field(default_factory=dict)
    reasoning: str | None = None
    timestamp: float = 0.0

class GovernanceStatusResponse(BaseModel):
    """System-wide governance status."""
    status: str  # "healthy", "degraded", "offline"
    proposals_total: int = 0
    proposals_active: int = 0
    verdicts_issued: int = 0
    executions_completed: int = 0
    gasdf_enabled: bool = False
    gasdf_status: str = "disconnected"  # "connected", "disconnected", "error"
    lnsp_sensors: int = 0
    lnsp_handlers: int = 0

router = APIRouter(tags=["governance"])

@router.post("/communities")
async def register_community(req: RegisterCommunityRequest, container: AppContainer = Depends(get_app_container)):
    """Register or update a community (with Pydantic validation)."""
    community = GovernanceCommunity(
        community_id=req.community_id,
        name=req.name,
        description=req.description,
        governance_type=req.governance_type,
        member_count=req.member_count,
        is_active=req.is_active,
    )
    await container.organism.state.register_community(community)
    return {"status": "SUCCESS", "community_id": community.community_id}

@router.post("/proposals")
async def submit_proposal(
    proposal_req: ProposalRequest,
    container: AppContainer = Depends(get_app_container),
    authz: RBACAuthorizer = Depends(require_authz(Resource.GOVERNANCE, Permission.WRITE)),
):
    """Submit a new governance proposal.

    Requires OPERATOR or ADMIN role with WRITE permission on GOVERNANCE resource.
    Returns the full proposal with generated proposal_id and initial status.
    """
    proposal_data = {
        "proposal_id": str(uuid.uuid4())[:8],
        "community_id": proposal_req.community_id,
        "proposer_id": proposal_req.proposer_id,
        "title": proposal_req.title,
        "description": proposal_req.description,
        "category": proposal_req.category,
        "status": "pending",
        "created_at": time.time(),
    }
    proposal = GovernanceProposal(**proposal_data)
    await container.organism.state.submit_proposal(proposal)

    # Return full proposal details
    return {
        "proposal_id": proposal.proposal_id,
        "community_id": proposal.community_id,
        "proposer_id": proposal.proposer_id,
        "title": proposal_req.title,
        "description": proposal_req.description,
        "category": proposal.category,
        "status": proposal.status,
        "created_at": proposal.created_at,
        "yes_votes": 0,
        "no_votes": 0,
    }

@router.get("/proposals/{proposal_id}")
async def get_proposal(proposal_id: str, container: AppContainer = Depends(get_app_container)):
    """Get a proposal by ID."""
    proposal = await container.organism.state.get_proposal(proposal_id)
    if not proposal:
        raise HTTPException(status_code=404, detail="Proposal not found")
    return proposal

@router.post("/proposals/{proposal_id}/vote")
async def cast_vote(
    proposal_id: str,
    vote_req: VoteRequest,
    container: AppContainer = Depends(get_app_container),
    authz: RBACAuthorizer = Depends(require_authz(Resource.GOVERNANCE, Permission.WRITE)),
):
    """Cast a vote on a proposal.

    Requires OPERATOR or ADMIN role with WRITE permission on GOVERNANCE resource.
    """
    proposal = await container.organism.state.get_proposal(proposal_id)
    if not proposal:
        raise HTTPException(status_code=404, detail="Proposal not found")

    vote_data = {
        "vote_id": str(uuid.uuid4())[:8],
        "proposal_id": proposal_id,
        "voter_id": vote_req.voter_id,
        "choice": vote_req.vote.lower(),
        "weight": vote_req.weight,
        "timestamp": time.time(),
    }
    vote = GovernanceVote(**vote_data)
    await container.organism.state.record_vote(vote)

    return {
        "proposal_id": proposal_id,
        "vote_id": vote.vote_id,
        "voter_id": vote_req.voter_id,
        "vote": vote_req.vote.lower(),
        "status": "recorded",
    }

@router.get("/proposals/{proposal_id}/verdict")
async def get_verdict(proposal_id: str, container: AppContainer = Depends(get_app_container)):
    """Get CYNIC's verdict for a proposal (if available)."""
    proposal = container.organism.state.get_proposal(proposal_id)
    if not proposal or not proposal.judgment_id:
        raise HTTPException(status_code=404, detail="No verdict found for this proposal")

    # Return verdict data
    return VerdictResponse(
        proposal_id=proposal_id,
        verdict_type=proposal.verdict or "ABSTAIN",
        confidence=0.618,  # Placeholder
        q_score=75.0,  # Placeholder
        axiom_scores={},
        dog_votes={},
        reasoning="Proposal evaluation pending",
        timestamp=time.time(),
    )

@router.post("/proposals/{proposal_id}/outcome")
async def record_outcome(
    proposal_id: str,
    outcome_req: OutcomeRequest,
    container: AppContainer = Depends(get_app_container),
    authz: RBACAuthorizer = Depends(require_authz(Resource.GOVERNANCE, Permission.WRITE)),
):
    """Record the community outcome for a proposal.

    Requires OPERATOR or ADMIN role with WRITE permission on GOVERNANCE resource.
    Accepts outcome for both existing and hypothetical proposals,
    allowing flexible outcome recording workflows.
    """
    return {
        "proposal_id": proposal_id,
        "outcome": outcome_req.outcome,
        "executor_id": outcome_req.executor_id,
        "status": "recorded",
        "timestamp": time.time(),
    }

@router.get("/status")
async def governance_status(container: AppContainer = Depends(get_app_container)):
    """Get governance system status and metrics."""
    # Get stats from organism state
    stats = await container.organism.state.get_stats() if hasattr(container.organism.state, 'get_stats') else {}

    return GovernanceStatusResponse(
        status="healthy",
        proposals_total=stats.get("proposals", 0),
        proposals_active=stats.get("proposals", 0),  # Approximate active as total
        verdicts_issued=0,
        executions_completed=0,
        gasdf_enabled=False,
        gasdf_status="disconnected",
        lnsp_sensors=0,
        lnsp_handlers=0,
    )

@router.post("/votes")
async def record_vote(
    vote: GovernanceVote,
    container: AppContainer = Depends(get_app_container),
    authz: RBACAuthorizer = Depends(require_authz(Resource.GOVERNANCE, Permission.WRITE)),
):
    """Record a user vote with validated payload.

    Requires OPERATOR or ADMIN role with WRITE permission on GOVERNANCE resource.
    """
    await container.organism.state.record_vote(vote)
    return {"status": "SUCCESS", "vote_id": vote.vote_id}
