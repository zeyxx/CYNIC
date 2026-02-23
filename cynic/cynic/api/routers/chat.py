"""
CYNIC chat router — message handling endpoints.

Endpoints:
  POST /api/chat/message    → Send a chat message and receive response

Note: Learning endpoints are in core.py (/api/learn)
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
