"""
CYNIC Governance REST API Router

Exposes governance endpoints for memecoin communities:
- POST /api/governance/proposals — submit a governance proposal
- POST /api/governance/proposals/{id}/vote — cast a vote
- POST /api/governance/proposals/{id}/execution — trigger on-chain execution
- POST /api/governance/proposals/{id}/outcome — record community outcome
- GET  /api/governance/proposals/{id}/verdict — get CYNIC's verdict
- GET  /api/governance/status — health/status of governance system

Design:
- Uses GovernanceLNSP from FastAPI app.state (set in core.py startup)
- GASdfExecutor wired if GASDF_ENABLED=1
- All responses include judgment_id for traceability
"""
from __future__ import annotations

import logging
import uuid
from typing import Any, Optional

from pydantic import BaseModel, Field
from fastapi import APIRouter, HTTPException, Request

logger = logging.getLogger("cynic.api.governance")

router_governance = APIRouter(prefix="/api/governance", tags=["governance"])


# ════════════════════════════════════════════════════════════════════════════
# PYDANTIC MODELS
# ════════════════════════════════════════════════════════════════════════════


class ProposalRequest(BaseModel):
    """Request to submit a governance proposal."""
    community_id: str = Field(description="Community identifier")
    title: str = Field(description="Proposal title")
    description: str = Field(description="Proposal description")
    proposer: str = Field(description="Proposer identifier")


class VoteRequest(BaseModel):
    """Request to cast a vote on a proposal."""
    voter: str = Field(description="Voter identifier")
    vote: str = Field(description="Vote choice: yes/no/abstain")

    def __init__(self, **data: Any) -> None:
        super().__init__(**data)
        self.vote = self.vote.lower()
        if self.vote not in ("yes", "no", "abstain"):
            raise ValueError("vote must be 'yes', 'no', or 'abstain'")


class ExecutionRequest(BaseModel):
    """Request to execute a governance verdict on-chain via GASdf."""
    payment_token: str = Field(description="Token address for fee payment")
    user_pubkey: str = Field(description="User's public key")
    signed_transaction: str = Field(description="Base64-encoded signed transaction")
    payment_token_account: str = Field(description="Token account for fee deduction")


class OutcomeRequest(BaseModel):
    """Request to record community outcome feedback."""
    outcome: str = Field(description="Outcome result (approved/rejected/expired)")
    executor: str = Field(description="Executor/recorder identifier")


class VerdictResponse(BaseModel):
    """Response containing CYNIC's governance verdict."""
    proposal_id: str
    verdict_type: str = Field(description="APPROVED/TENTATIVE_APPROVE/CAUTION/REJECT")
    q_score: float = Field(description="Quality score [0, 100]")
    confidence: float = Field(description="Confidence [0, 0.618]")
    axiom_scores: dict[str, float] = Field(default_factory=dict)
    dog_votes: dict[str, float] = Field(default_factory=dict)
    timestamp: float


class ExecutionResponse(BaseModel):
    """Response from execution request."""
    proposal_id: str
    execution_id: str
    status: str = Field(description="pending/success/failure")
    signature: Optional[str] = Field(default=None, description="Transaction signature if success")
    message: str


class ProposalResponse(BaseModel):
    """Response from proposal submission."""
    proposal_id: str
    community_id: str
    title: str
    status: str = Field(description="pending/voting/approved/rejected/executed")
    created_at: float


class GovernanceStatusResponse(BaseModel):
    """Response from /governance/status."""
    status: str = Field(description="healthy/degraded/offline")
    proposals_total: int
    proposals_active: int
    verdicts_issued: int
    executions_completed: int
    gasdf_enabled: bool
    gasdf_status: str = Field(description="connected/disconnected/error")
    lnsp_sensors: int
    lnsp_handlers: int
    message: str


# ════════════════════════════════════════════════════════════════════════════
# ENDPOINTS
# ════════════════════════════════════════════════════════════════════════════


@router_governance.post("/proposals", response_model=ProposalResponse)
async def submit_proposal(req: ProposalRequest, request: Request) -> ProposalResponse:
    """
    POST /api/governance/proposals

    Submit a new governance proposal to the community.
    CYNIC will analyze it and issue a verdict.
    """
    governance = getattr(request.app.state, "governance", None)
    if not governance:
        raise HTTPException(status_code=503, detail="Governance system not initialized")

    proposal_id = str(uuid.uuid4())[:8]
    import time
    proposal_data = {
        "proposal_id": proposal_id,
        "community_id": req.community_id,
        "title": req.title,
        "content": req.description,  # Map description to content
        "submitter_id": req.proposer,  # Map proposer to submitter_id
        "submission_timestamp": time.time(),
        "voting_period_hours": 24,  # Default voting period
    }

    try:
        await governance.process_proposal(proposal_data)
        return ProposalResponse(
            proposal_id=proposal_id,
            community_id=req.community_id,
            title=req.title,
            status="pending",
            created_at=__import__("time").time(),
        )
    except Exception as e:
        logger.error("Failed to process proposal: %s", e)
        raise HTTPException(status_code=500, detail=f"Failed to process proposal: {e}")


@router_governance.post("/proposals/{proposal_id}/vote")
async def cast_vote(proposal_id: str, req: VoteRequest, request: Request) -> dict[str, Any]:
    """
    POST /api/governance/proposals/{proposal_id}/vote

    Cast a vote (yes/no/abstain) on a proposal.
    """
    governance = getattr(request.app.state, "governance", None)
    if not governance:
        raise HTTPException(status_code=503, detail="Governance system not initialized")

    import time
    vote_data = {
        "proposal_id": proposal_id,
        "voter_id": req.voter,  # Map voter to voter_id
        "vote_choice": req.vote.upper(),  # Map vote to vote_choice and uppercase
        "timestamp": time.time(),
        "community_id": "unknown",  # Would come from context in real system
    }

    try:
        await governance.process_vote(vote_data)
        return {
            "proposal_id": proposal_id,
            "voter": req.voter,
            "vote": req.vote,
            "status": "recorded",
        }
    except Exception as e:
        logger.error("Failed to record vote: %s", e)
        raise HTTPException(status_code=500, detail=f"Failed to record vote: {e}")


@router_governance.get("/proposals/{proposal_id}/verdict", response_model=VerdictResponse)
async def get_verdict(proposal_id: str, request: Request) -> VerdictResponse:
    """
    GET /api/governance/proposals/{proposal_id}/verdict

    Get CYNIC's governance verdict for a proposal.
    Returns verdict type, confidence, axiom scores, and dog votes.
    """
    governance = getattr(request.app.state, "governance", None)
    if not governance:
        raise HTTPException(status_code=503, detail="Governance system not initialized")

    # Check verdict cache
    if proposal_id not in governance.verdict_cache:
        raise HTTPException(status_code=404, detail=f"No verdict found for proposal {proposal_id}")

    verdict_info = governance.verdict_cache[proposal_id]
    return VerdictResponse(
        proposal_id=proposal_id,
        verdict_type=verdict_info.get("verdict_type", "UNKNOWN"),
        q_score=verdict_info.get("q_score", 0.0),
        confidence=verdict_info.get("confidence", 0.0),
        axiom_scores=verdict_info.get("axiom_scores", {}),
        dog_votes=verdict_info.get("dog_votes", {}),
        timestamp=verdict_info.get("timestamp", 0.0),
    )


@router_governance.post("/proposals/{proposal_id}/execution", response_model=ExecutionResponse)
async def trigger_execution(
    proposal_id: str, req: ExecutionRequest, request: Request
) -> ExecutionResponse:
    """
    POST /api/governance/proposals/{proposal_id}/execution

    Trigger on-chain execution of a governance verdict via GASdf.
    Requires signed transaction and payment details.

    Only executes if GASdf is enabled and a verdict exists.
    """
    governance = getattr(request.app.state, "governance", None)
    if not governance:
        raise HTTPException(status_code=503, detail="Governance system not initialized")

    if not governance.gasdf_executor:
        raise HTTPException(
            status_code=503,
            detail="GASdf executor not available (GASDF_ENABLED=0 or client error)",
        )

    # Get verdict
    if proposal_id not in governance.verdict_cache:
        raise HTTPException(status_code=404, detail=f"No verdict found for proposal {proposal_id}")

    verdict_info = governance.verdict_cache[proposal_id]
    verdict_type = verdict_info.get("verdict_type", "UNKNOWN")

    execution_id = str(uuid.uuid4())[:8]

    try:
        # Execute via GASdf
        result = await governance.gasdf_executor.execute_verdict(
            proposal_id=proposal_id,
            verdict=verdict_type,
            community_id="unknown",  # Would come from proposal data in real system
            payment_token=req.payment_token,
            user_pubkey=req.user_pubkey,
            signed_transaction=req.signed_transaction,
            payment_token_account=req.payment_token_account,
        )

        if result:
            await governance.on_execution_completed(
                {
                    "proposal_id": proposal_id,
                    "success": True,
                    "signature": result.signature,
                }
            )
            return ExecutionResponse(
                proposal_id=proposal_id,
                execution_id=execution_id,
                status="success",
                signature=result.signature,
                message="Verdict executed on-chain",
            )
        else:
            # Execution returned None (verdict not eligible for execution)
            return ExecutionResponse(
                proposal_id=proposal_id,
                execution_id=execution_id,
                status="pending",
                message="Verdict not eligible for on-chain execution (not approved)",
            )
    except Exception as e:
        logger.error("Failed to execute verdict: %s", e)
        await governance.on_execution_completed(
            {
                "proposal_id": proposal_id,
                "success": False,
            }
        )
        raise HTTPException(status_code=500, detail=f"Execution failed: {e}")


@router_governance.post("/proposals/{proposal_id}/outcome")
async def record_outcome(
    proposal_id: str, req: OutcomeRequest, request: Request
) -> dict[str, Any]:
    """
    POST /api/governance/proposals/{proposal_id}/outcome

    Record community outcome feedback for learning loop.
    This feeds back into CYNIC's judgment for future proposals.
    """
    governance = getattr(request.app.state, "governance", None)
    if not governance:
        raise HTTPException(status_code=503, detail="Governance system not initialized")

    import time
    outcome_data = {
        "proposal_id": proposal_id,
        "accepted": req.outcome == "approved",  # Map outcome to accepted boolean
        "funds_received": req.outcome == "approved",  # Assume approved = funds received
        "community_sentiment": 0.5 if req.outcome == "approved" else -0.5,  # Simple sentiment mapping
        "feedback_text": f"Outcome: {req.outcome}, Executor: {req.executor}",
        "timestamp": time.time(),
        "community_id": "unknown",  # Would come from context in real system
    }

    try:
        await governance.process_outcome(outcome_data)
        await governance.on_outcome_feedback(outcome_data)
        return {
            "proposal_id": proposal_id,
            "outcome": req.outcome,
            "status": "recorded",
            "message": "Outcome recorded for learning loop",
        }
    except Exception as e:
        logger.error("Failed to record outcome: %s", e)
        raise HTTPException(status_code=500, detail=f"Failed to record outcome: {e}")


@router_governance.get("/status", response_model=GovernanceStatusResponse)
async def governance_status(request: Request) -> GovernanceStatusResponse:
    """
    GET /api/governance/status

    Get health and status of the governance system.
    Includes LNSP layer count, GASdf connectivity, and proposal stats.
    """
    governance = getattr(request.app.state, "governance", None)

    if not governance:
        return GovernanceStatusResponse(
            status="offline",
            proposals_total=0,
            proposals_active=0,
            verdicts_issued=0,
            executions_completed=0,
            gasdf_enabled=False,
            gasdf_status="disconnected",
            lnsp_sensors=0,
            lnsp_handlers=0,
            message="Governance system not initialized",
        )

    # Count verdicts issued
    verdicts_issued = len(governance.verdict_cache)

    # Count executions (those with execution_success field)
    executions_completed = sum(
        1 for v in governance.verdict_cache.values() if v.get("execution_success")
    )

    # GASdf status
    gasdf_enabled = governance.gasdf_executor is not None
    gasdf_status = "connected" if gasdf_enabled else "disconnected"

    # LNSP layer counts
    lnsp_sensors = len(governance.manager.layer1.sensors) if governance.manager else 0
    lnsp_handlers = len(governance.manager.layer4.handlers) if governance.manager else 0

    return GovernanceStatusResponse(
        status="healthy" if verdicts_issued > 0 else "degraded",
        proposals_total=len(governance.verdict_cache),
        proposals_active=max(0, len(governance.verdict_cache) - executions_completed),
        verdicts_issued=verdicts_issued,
        executions_completed=executions_completed,
        gasdf_enabled=gasdf_enabled,
        gasdf_status=gasdf_status,
        lnsp_sensors=lnsp_sensors,
        lnsp_handlers=lnsp_handlers,
        message="Governance system operational",
    )
