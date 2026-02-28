"""
Governance Router — Bridge between the Bot and the Organism Memory.
"""
from __future__ import annotations
from typing import Any, List, Optional
from fastapi import APIRouter, Depends, HTTPException

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.core.unified_state import GovernanceCommunity, GovernanceProposal, GovernanceVote

router = APIRouter(prefix="/governance", tags=["governance"])

@router.post("/communities")
async def register_community(req: dict, container: AppContainer = Depends(get_app_container)):
    """Register or update a community."""
    community = GovernanceCommunity(**req)
    await container.organism.state.register_community(community)
    return {"status": "SUCCESS", "community_id": community.community_id}

@router.post("/proposals")
async def submit_proposal(req: dict, container: AppContainer = Depends(get_app_container)):
    """Submit a new proposal."""
    proposal = GovernanceProposal(**req)
    await container.organism.state.submit_proposal(proposal)
    return {"status": "SUCCESS", "proposal_id": proposal.proposal_id}

@router.get("/proposals/{proposal_id}")
async def get_proposal(proposal_id: str, container: AppContainer = Depends(get_app_container)):
    """Get a proposal by ID."""
    proposal = container.organism.state.get_proposal(proposal_id)
    if not proposal:
        raise HTTPException(status_code=404, detail="Proposal not found")
    return proposal

@router.post("/votes")
async def record_vote(req: dict, container: AppContainer = Depends(get_app_container)):
    """Record a user vote."""
    vote = GovernanceVote(**req)
    await container.organism.state.record_vote(vote)
    return {"status": "SUCCESS", "vote_id": vote.vote_id}
