"""Tests for governance REST API router.

Test governance endpoints for memecoin community governance.
Tests cover proposal submission, voting, verdict retrieval, and status checking.
"""
from __future__ import annotations

import pytest
from fastapi.testclient import TestClient
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.interfaces.api.server import app


@pytest.fixture(scope="class")
def governanceendpoints_client():
    """Class-scoped HTTP client - reuses single organism."""
    with TestClient(app) as c:
        yield c


class TestGovernanceEndpoints:
    """Test governance API endpoints."""

    def test_submit_proposal(self, governanceendpoints_client):
        """POST /api/governance/proposals — submit a new proposal."""
        response = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Increase Trading Fee",
                "description": "Propose increasing trading fee from 1% to 2%",
                "proposer": "user_123",
            },
        )
        assert response.status_code == 200
        data = response.json()
        assert "proposal_id" in data
        assert data["title"] == "Increase Trading Fee"
        assert data["community_id"] == "test_community"
        assert data["status"] in ("pending", "voting", "approved", "rejected", "executed")

    def test_submit_proposal_missing_fields(self, governanceendpoints_client):
        """POST /api/governance/proposals with missing fields should fail."""
        response = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Incomplete Proposal",
            },
        )
        # Should fail validation
        assert response.status_code == 422

    def test_cast_vote(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/vote — cast a vote."""
        # First submit a proposal
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Test Proposal",
                "description": "A test proposal",
                "proposer": "user_123",
            },
        )
        proposal_id = proposal_resp.json()["proposal_id"]

        # Cast a vote
        response = governanceendpoints_client.post(
            f"/api/governance/proposals/{proposal_id}/vote",
            json={
                "voter": "user_456",
                "vote": "yes",
            },
        )
        assert response.status_code == 200
        data = response.json()
        assert data["proposal_id"] == proposal_id
        assert data["vote"] == "yes"
        assert data["status"] == "recorded"

    def test_cast_vote_invalid_choice(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/vote with invalid choice should fail."""
        response = governanceendpoints_client.post(
            "/api/governance/proposals/invalid_id/vote",
            json={
                "voter": "user_456",
                "vote": "invalid",  # Should be yes/no/abstain
            },
        )
        # Should fail validation
        assert response.status_code == 422

    def test_get_verdict(self, governanceendpoints_client):
        """GET /api/governance/proposals/{id}/verdict — get CYNIC's verdict."""
        # Submit proposal to populate verdict cache
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Test Proposal",
                "description": "A test proposal",
                "proposer": "user_123",
            },
        )
        proposal_id = proposal_resp.json()["proposal_id"]

        # Try to get verdict (may not exist immediately, but endpoint should work)
        response = governanceendpoints_client.get(f"/api/governance/proposals/{proposal_id}/verdict")
        # Either we get a verdict or a 404 for new proposal
        assert response.status_code in (200, 404)

    def test_get_verdict_not_found(self, governanceendpoints_client):
        """GET /api/governance/proposals/{id}/verdict with non-existent id returns 404."""
        response = governanceendpoints_client.get("/api/governance/proposals/nonexistent_id/verdict")
        assert response.status_code == 404
        assert "No verdict found" in response.json()["detail"]

    def test_record_outcome(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/outcome — record community outcome."""
        # Submit proposal
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Test Proposal",
                "description": "A test proposal",
                "proposer": "user_123",
            },
        )
        proposal_id = proposal_resp.json()["proposal_id"]

        # Record outcome
        response = governanceendpoints_client.post(
            f"/api/governance/proposals/{proposal_id}/outcome",
            json={
                "outcome": "approved",
                "executor": "user_admin",
            },
        )
        assert response.status_code == 200
        data = response.json()
        assert data["proposal_id"] == proposal_id
        assert data["outcome"] == "approved"
        assert data["status"] == "recorded"

    def test_record_outcome_invalid_choice(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/outcome with invalid outcome."""
        response = governanceendpoints_client.post(
            "/api/governance/proposals/test_id/outcome",
            json={
                "outcome": "invalid",
                "executor": "user_admin",
            },
        )
        # Should accept any outcome string, no validation error expected
        # But endpoint should handle it
        assert response.status_code in (200, 503)  # 503 if governance not init

    def test_governance_status(self, governanceendpoints_client):
        """GET /api/governance/status — get governance system status."""
        response = governanceendpoints_client.get("/api/governance/status")
        assert response.status_code == 200
        data = response.json()
        assert "status" in data
        assert data["status"] in ("healthy", "degraded", "offline")
        assert "proposals_total" in data
        assert "proposals_active" in data
        assert "verdicts_issued" in data
        assert "executions_completed" in data
        assert "gasdf_enabled" in data
        assert "gasdf_status" in data
        assert "lnsp_sensors" in data
        assert "lnsp_handlers" in data
        assert isinstance(data["proposals_total"], int)
        assert isinstance(data["verdicts_issued"], int)

    def test_governance_status_gasdf_info(self, governanceendpoints_client):
        """GET /api/governance/status includes GASdf connectivity info."""
        response = governanceendpoints_client.get("/api/governance/status")
        assert response.status_code == 200
        data = response.json()
        assert "gasdf_enabled" in data
        assert "gasdf_status" in data
        # gasdf_status should be either 'connected' or 'disconnected'
        assert data["gasdf_status"] in ("connected", "disconnected", "error")

    def test_governance_status_lnsp_info(self, governanceendpoints_client):
        """GET /api/governance/status includes LNSP layer info."""
        response = governanceendpoints_client.get("/api/governance/status")
        assert response.status_code == 200
        data = response.json()
        assert "lnsp_sensors" in data
        assert "lnsp_handlers" in data
        assert isinstance(data["lnsp_sensors"], int)
        assert isinstance(data["lnsp_handlers"], int)

    def test_vote_choices(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/vote accepts yes/no/abstain."""
        # Submit proposal
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Test Proposal",
                "description": "A test proposal",
                "proposer": "user_123",
            },
        )
        proposal_id = proposal_resp.json()["proposal_id"]

        # Test all vote choices
        for vote_choice in ("yes", "no", "abstain"):
            response = governanceendpoints_client.post(
                f"/api/governance/proposals/{proposal_id}/vote",
                json={
                    "voter": f"user_{vote_choice}",
                    "vote": vote_choice,
                },
            )
            assert response.status_code == 200
            assert response.json()["vote"] == vote_choice

    def test_vote_case_insensitive(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/vote accepts mixed case votes."""
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Test Proposal",
                "description": "A test proposal",
                "proposer": "user_123",
            },
        )
        proposal_id = proposal_resp.json()["proposal_id"]

        # Test mixed case
        for vote_choice in ("YES", "No", "ABSTAIN"):
            response = governanceendpoints_client.post(
                f"/api/governance/proposals/{proposal_id}/vote",
                json={
                    "voter": "user_mixed",
                    "vote": vote_choice,
                },
            )
            # The endpoint converts to lowercase internally
            assert response.status_code == 200

    def test_proposal_lifecycle(self, governanceendpoints_client):
        """Test full proposal lifecycle: submit → vote → outcome → status."""
        # 1. Submit proposal
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "community_1",
                "title": "Budget Increase",
                "description": "Increase community treasury allocation",
                "proposer": "proposer_1",
            },
        )
        assert proposal_resp.status_code == 200
        proposal_id = proposal_resp.json()["proposal_id"]

        # 2. Cast votes
        for voter_id in ["voter_1", "voter_2", "voter_3"]:
            vote_resp = governanceendpoints_client.post(
                f"/api/governance/proposals/{proposal_id}/vote",
                json={
                    "voter": voter_id,
                    "vote": "yes",
                },
            )
            assert vote_resp.status_code == 200

        # 3. Record outcome
        outcome_resp = governanceendpoints_client.post(
            f"/api/governance/proposals/{proposal_id}/outcome",
            json={
                "outcome": "approved",
                "executor": "executor_1",
            },
        )
        assert outcome_resp.status_code == 200

        # 4. Check status
        status_resp = governanceendpoints_client.get("/api/governance/status")
        assert status_resp.status_code == 200
        status_data = status_resp.json()
        assert status_data["proposals_total"] >= 1

    def test_trigger_execution_no_gasdf(self, governanceendpoints_client):
        """POST /api/governance/proposals/{id}/execution without GASdf returns 503."""
        # Submit proposal to get a proposal_id
        proposal_resp = governanceendpoints_client.post(
            "/api/governance/proposals",
            json={
                "community_id": "test_community",
                "title": "Test Proposal",
                "description": "A test proposal",
                "proposer": "user_123",
            },
        )
        proposal_id = proposal_resp.json()["proposal_id"]

        # Try to execute (will fail if GASDF_ENABLED != 1)
        response = governanceendpoints_client.post(
            f"/api/governance/proposals/{proposal_id}/execution",
            json={
                "payment_token": "USDC",
                "user_pubkey": "user_pub_key",
                "signed_transaction": "base64_encoded_tx",
                "payment_token_account": "account_address",
            },
        )
        # Should either return 503 (executor not available) or 404 (no verdict)
        assert response.status_code in (503, 404)

    def test_endpoint_requires_governance_init(self, governanceendpoints_client):
        """All governance endpoints gracefully handle missing governance instance."""
        # These endpoints should handle missing governance instance
        response = governanceendpoints_client.get("/api/governance/status")
        # Should still return 200 (with degraded/offline status)
        assert response.status_code == 200
        data = response.json()
        assert "status" in data


@pytest.fixture(scope="class")
def governancemodels_client():
    """Class-scoped HTTP client - reuses single organism."""
    with TestClient(app) as c:
        yield c


class TestGovernanceModels:
    """Test Pydantic models for governance API."""

    def test_proposal_request_validation(self, governancemodels_client):
        """ProposalRequest validates required fields."""
        from cynic.interfaces.api.routers.governance import ProposalRequest

        # Valid request
        req = ProposalRequest(
            community_id="test",
            title="Test",
            description="Test description",
            proposer="user_1",
        )
        assert req.community_id == "test"

    def test_vote_request_validation(self, governancemodels_client):
        """VoteRequest validates vote choice."""
        from cynic.interfaces.api.routers.governance import VoteRequest

        # Valid votes
        for vote in ["yes", "no", "abstain"]:
            req = VoteRequest(voter="user_1", vote=vote)
            assert req.vote == vote.lower()

        # Invalid vote should raise error
        with pytest.raises(ValueError):
            VoteRequest(voter="user_1", vote="invalid")

    def test_verdict_response_model(self, governancemodels_client):
        """VerdictResponse includes all required fields."""
        from cynic.interfaces.api.routers.governance import VerdictResponse

        response = VerdictResponse(
            proposal_id="test_id",
            verdict_type="APPROVED",
            q_score=75.5,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.9},
            dog_votes={"GUARDIAN": 0.95},
            timestamp=1234567890.0,
        )
        assert response.proposal_id == "test_id"
        assert response.verdict_type == "APPROVED"
        assert response.q_score == 75.5

    def test_governance_status_response_model(self, governancemodels_client):
        """GovernanceStatusResponse includes all required fields."""
        from cynic.interfaces.api.routers.governance import GovernanceStatusResponse

        response = GovernanceStatusResponse(
            status="healthy",
            proposals_total=10,
            proposals_active=3,
            verdicts_issued=7,
            executions_completed=5,
            gasdf_enabled=True,
            gasdf_status="connected",
            lnsp_sensors=4,
            lnsp_handlers=1,
            message="System operational",
        )
        assert response.status == "healthy"
        assert response.proposals_total == 10
        assert response.gasdf_enabled is True
