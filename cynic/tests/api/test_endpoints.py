"""
Task 2.1: Verify Chat & Learning Endpoints

Tests to verify that the POST /api/chat/message and POST /api/learn endpoints
respond with HTTP status code ≠ 404 (they exist, not "not found").
"""
import pytest
from fastapi.testclient import TestClient
from cynic.api.server import app

client = TestClient(app)


def test_chat_endpoint_exists():
    """Verify that POST /api/chat/message endpoint exists (status != 404)."""
    response = client.post("/api/chat/message", json={
        "text": "test",
        "session_id": "s1"
    })
    assert response.status_code != 404, f"Chat endpoint returned 404. Response: {response.text}"


def test_learn_endpoint_exists():
    """Verify that POST /api/learn endpoint exists (status != 404)."""
    response = client.post("/api/learn", json={
        "session_id": "s1",
        "prompt": "test",
        "code_generated": "def test(): pass",
        "user_feedback": "good"
    })
    assert response.status_code != 404, f"Learn endpoint returned 404. Response: {response.text}"
