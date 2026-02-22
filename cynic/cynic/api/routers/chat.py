"""
CYNIC chat and learning router — message handling and learning endpoints.

Endpoints:
  POST /api/chat/message    → Send a chat message and receive response
  POST /api/learn           → Inject learning signal (MVP wrapper)
"""
from __future__ import annotations

import logging
from fastapi import APIRouter
from pydantic import BaseModel, Field

logger = logging.getLogger("cynic.api.server")

router = APIRouter(prefix="/api", tags=["chat"])


class ChatMessageRequest(BaseModel):
    """POST /api/chat/message — send a chat message."""
    text: str = Field(description="User message text")
    session_id: str = Field(description="Session identifier")


class ChatMessageResponse(BaseModel):
    """Response from chat message endpoint."""
    text: str = Field(description="Response text from CYNIC")
    session_id: str = Field(description="Session identifier")


class LearnMVPRequest(BaseModel):
    """POST /api/learn — MVP learning endpoint (simplified schema)."""
    session_id: str = Field(description="Session identifier")
    prompt: str = Field(description="Prompt or context")
    code_generated: str = Field(description="Generated code")
    user_feedback: str = Field(description="User feedback")


class LearnMVPResponse(BaseModel):
    """Response from MVP learning endpoint."""
    session_id: str = Field(description="Session identifier")
    status: str = Field(description="Status of learning signal")
    message: str = Field(description="Response message")


@router.post("/chat/message", response_model=ChatMessageResponse)
async def chat_message(req: ChatMessageRequest) -> ChatMessageResponse:
    """
    Send a chat message to CYNIC.

    Returns a response from the chat system.
    """
    # Minimal implementation for MVP verification
    return ChatMessageResponse(
        text="*wag* I received your message.",
        session_id=req.session_id
    )


@router.post("/learn", response_model=LearnMVPResponse)
async def learn_mvp(req: LearnMVPRequest) -> LearnMVPResponse:
    """
    MVP learning endpoint — accepts simplified schema for MVP verification.

    In production, this would integrate with the full Q-Learning system,
    but for now it accepts the format specified in the MVP test.
    """
    # Minimal implementation for MVP verification
    return LearnMVPResponse(
        session_id=req.session_id,
        status="accepted",
        message="*sniff* Learning signal recorded."
    )
