"""
CYNIC chat router â€” message handling endpoints.

Endpoints:
  POST /api/chat/message    â†’ Send a chat message and receive response

Note: Learning endpoints are in core.py (/api/learn)
"""
from __future__ import annotations

import logging

from fastapi import APIRouter
from pydantic import BaseModel, Field

from cynic.interfaces.chat.session import ChatSession

logger = logging.getLogger("cynic.interfaces.api.server")

router = APIRouter(prefix="/api", tags=["chat"])


class ChatMessageRequest(BaseModel):
    """POST /api/chat/message â€” send a chat message."""
    text: str = Field(description="User message text")
    session_id: str = Field(description="Session identifier")


class ChatMessageResponse(BaseModel):
    """Response from chat message endpoint."""
    text: str = Field(description="Response text from CYNIC")
    session_id: str = Field(description="Session identifier")
    message_count: int = Field(description="Total messages in session")


@router.post("/chat/message", response_model=ChatMessageResponse)
async def chat_message(req: ChatMessageRequest) -> ChatMessageResponse:
    """
    Send a chat message to CYNIC.

    Real implementation using ChatSession:
    1. Load or create session
    2. Add user message
    3. Generate response (echo + message count for MVP)
    4. Save session
    5. Return response

    Note: Full LLM integration planned for Phase 2.
    """
    try:
        # Try to load existing session, or create new one
        try:
            session = ChatSession.load(req.session_id)
            logger.debug(f"Loaded session {req.session_id} with {session.message_count} messages")
        except (FileNotFoundError, ValueError):
            session = ChatSession(session_id=req.session_id)
            logger.debug(f"Created new session {req.session_id}")

        # Add user message to session
        session.add_user(req.text)
        logger.debug(f"User message added: {req.text[:50]}...")

        # Generate response (MVP: echo confirmation + message count)
        # Phase 2: Wire to LLM orchestrator for real responses
        response_text = f"*wag* Message received. Session has {session.message_count} message(s)."
        session.add_assistant(response_text)

        # Persist session
        session.save()
        logger.debug(f"Session {req.session_id} saved with {session.message_count} messages")

        return ChatMessageResponse(
            text=response_text,
            session_id=req.session_id,
            message_count=session.message_count
        )

    except Exception as exc:
        logger.error(f"Chat message error: {exc}", exc_info=True)
        # Fallback response on error
        return ChatMessageResponse(
            text=f"*growl* Error processing message: {str(exc)[:100]}",
            session_id=req.session_id,
            message_count=0
        )
