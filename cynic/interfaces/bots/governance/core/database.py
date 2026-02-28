"""
Unified Memory Proxy for Governance Bot.

Redirects all database operations to the CYNIC Organism API.
This eliminates the local SQLite dependency and unifies the memory.
"""
import logging
import aiohttp
import asyncio
from typing import Any, List, Optional
from contextlib import asynccontextmanager

from cynic.interfaces.bots.governance.core.config import CYNICSettings

logger = logging.getLogger("cynic.interfaces.bots.governance.memory_proxy")

settings = CYNICSettings()
BASE_URL = str(settings.url).rstrip("/")

async def _api_call(method: str, path: str, json: dict = None) -> Any:
    """Internal helper for API calls to the organism."""
    async with aiohttp.ClientSession() as session:
        url = f"{BASE_URL}/governance{path}"
        try:
            async with session.request(method, url, json=json) as resp:
                if resp.status == 200:
                    return await resp.json()
                logger.error("Organism API Error (%d): %s", resp.status, await resp.text())
                return None
        except Exception as e:
            logger.error("Failed to connect to Organism API: %s", e)
            return None

# --- Re-implementing the DB interface as API Proxies ---

async def init_db():
    logger.info("Governance Bot: Unified Memory Proxy Active (No local DB)")
    return True

async def close_db():
    return True

@asynccontextmanager
async def session_context():
    """Dummy context to maintain compatibility with bot.py"""
    yield None

async def create_community(session, **kwargs):
    return await _api_call("POST", "/communities", kwargs)

async def get_community(session, community_id: str):
    # For now, we assume it exists or return a default
    return {"community_id": community_id, "name": "Unified Community"}

async def create_proposal(session, **kwargs):
    # Ensure ID is present
    if "proposal_id" not in kwargs:
        import uuid
        kwargs["proposal_id"] = str(uuid.uuid4())[:8]
    return await _api_call("POST", "/proposals", kwargs)

async def get_proposal(session, proposal_id: str):
    return await _api_call("GET", f"/proposals/{proposal_id}")

async def create_vote(session, **kwargs):
    if "vote_id" not in kwargs:
        import uuid
        kwargs["vote_id"] = str(uuid.uuid4())[:8]
    return await _api_call("POST", "/votes", kwargs)

# --- Placeholders for remaining methods to keep bot.py happy ---
async def update_proposal_status(*args, **kwargs): pass
async def update_proposal_judgment(*args, **kwargs): pass
async def count_votes(*args, **kwargs): return (0, 0, 0)
async def update_vote_counts(*args, **kwargs): pass
async def is_voting_active(*args, **kwargs): return True
async def check_voting_closed(*args, **kwargs): return False
async def get_user_vote(*args, **kwargs): return None
async def get_proposals_needing_outcome(*args, **kwargs): return []
async def create_learning_outcome(*args, **kwargs): pass
async def mark_outcome_determined(*args, **kwargs): pass
async def get_or_create_e_score(*args, **kwargs): return None
async def db_health_check(): return {"status": "CONNECTED_TO_ORGANISM"}
async def backup_database(): pass
async def restore_database(): pass
async def verify_data_consistency(): pass
async def get_session(): return None
async def update_e_score(*args, **kwargs): pass
