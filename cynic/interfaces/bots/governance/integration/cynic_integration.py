"""
CYNIC Integration - Call CYNIC MCP tools
"""

import asyncio
import logging
import sys
from pathlib import Path

logger = logging.getLogger(__name__)

# Add CYNIC to path
cynic_path = Path(__file__).parent.parent
sys.path.insert(0, str(cynic_path))


async def ask_cynic(question: str, context: str = "", reality: str = "SOCIAL") -> dict:
    """
    Ask CYNIC a governance question and get judgment via HTTP API.
    Calls CYNIC API /judge and polls for results.

    Returns:
        {
            "verdict": "HOWL" | "WAG" | "GROWL" | "BARK",
            "q_score": float,
            "confidence": float,
            "dog_votes": {...},
            "axiom_scores": {...},
            "judgment_id": str,
            "verdict_explanation": str
        }
    """

    import aiohttp

    from cynic.interfaces.bots.governance.core.config import CYNICSettings
    
    settings = CYNICSettings()
    api_url = str(settings.url).rstrip("/")
    timeout = aiohttp.ClientTimeout(total=settings.timeout_seconds)

    try:
        logger.info(f"Asking CYNIC API: {question[:100]}...")

        # Map governance reality to valid CYNIC realities
        cynic_reality = reality.upper() if reality.upper() in {"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"} else "SOCIAL"

        payload = {
            "content": question,
            "context": context,
            "reality": cynic_reality,
            "analysis": "JUDGE",
            "level": "MICRO",  # MICRO consciousness level (~500ms) is good for governance decisions
            "budget_usd": 0.05
        }

        async with aiohttp.ClientSession(timeout=timeout) as session:
            # 1. Submit judgment request
            async with session.post(f"{api_url}/judge", json=payload) as resp:
                if resp.status != 200:
                    error_text = await resp.text()
                    logger.error(f"CYNIC API error (status={resp.status}): {error_text}")
                    return {"verdict": "PENDING", "error": f"API error: {resp.status}"}
                
                initial_data = await resp.json()
                judgment_id = initial_data.get("judgment_id")
                
                if not judgment_id:
                    logger.error("No judgment_id returned from CYNIC API")
                    return {"verdict": "PENDING", "error": "No judgment_id returned"}

            # 2. Poll for results (Track E: event-first API)
            max_attempts = 30
            poll_interval = 0.5
            
            for _attempt in range(max_attempts):
                async with session.get(f"{api_url}/judge/{judgment_id}") as resp:
                    if resp.status == 200:
                        judgment_data = await resp.json()
                        verdict = judgment_data.get("verdict")
                        
                        if verdict != "PENDING":
                            logger.info(f"CYNIC response: {verdict} (Q={judgment_data.get('q_score', 0):.1f}, conf={judgment_data.get('confidence', 0):.2%})")
                            return {
                                "verdict": verdict,
                                "q_score": judgment_data.get("q_score", 0.0),
                                "confidence": judgment_data.get("confidence", 0.0),
                                "judgment_id": judgment_id,
                                "dog_votes": judgment_data.get("dog_votes", {}),
                                "axiom_scores": judgment_data.get("axiom_scores", {}),
                                "verdict_explanation": judgment_data.get("reasoning", ""),
                                "consensus_reason": judgment_data.get("consensus_reason", ""),
                            }
                    
                    elif resp.status != 404:
                        logger.warning(f"Unexpected status while polling judgment {judgment_id}: {resp.status}")
                
                await asyncio.sleep(poll_interval)

            logger.error(f"Timeout waiting for judgment {judgment_id} after {max_attempts} attempts")
            return {
                "verdict": "PENDING",
                "q_score": 0.0,
                "confidence": 0.0,
                "judgment_id": judgment_id,
                "error": "Timeout waiting for judgment"
            }

    except Exception as e:
            logger.error(f"Error calling ask_cynic API: {e}", exc_info=True)
        return {
            "verdict": "PENDING",
            "q_score": 0.0,
            "confidence": 0.0,
            "error": str(e)
        }


async def learn_cynic(
    judgment_id: str,
    verdict: str,
    approved: bool,
    satisfaction: float = 3.0,
    comment: str = ""
) -> dict:
    """
    Teach CYNIC from a proposal outcome via MCP HTTP server.

    Args:
        judgment_id: ID of the judgment to learn from
        verdict: CYNIC's original verdict (HOWL/WAG/GROWL/BARK)
        approved: Whether proposal was actually approved
        satisfaction: Community satisfaction rating (1.0-5.0 stars)
        comment: Optional comment

    Returns:
        {
            "learning_status": "completed" | "skipped",
            "q_table_updated": bool (if completed)
        }
    """
    try:
        logger.info(f"Learning from judgment {judgment_id} (verdict={verdict}, approved={approved})...")

        import aiohttp

        from cynic.interfaces.bots.governance.core.config import CYNIC_MCP_URL

        # Normalize rating: 1-5 stars  -1.0 to +1.0
        base = (satisfaction / 5.0) * 2.0 - 1.0  # 1-0.6, 30.2, 51.0
        if not approved:
            base = -abs(base)  # rejected  always negative
        rating = max(-1.0, min(1.0, base))

        payload = {
            "signal": {
                "judgment_id": judgment_id,
                "rating": rating,
                "comment": comment
            },
            "update_qtable": True
        }

        timeout = aiohttp.ClientTimeout(total=10)
        async with aiohttp.ClientSession(timeout=timeout) as client:
            async with client.post(f"{CYNIC_MCP_URL}/learn", json=payload) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    qtable_updated = data.get("result", {}).get("qtable_updated", False)
                    logger.info(f"CYNIC learning completed: {judgment_id} (rating={rating:.2f}, qtable_updated={qtable_updated})")
                    return {
                        "learning_status": "completed",
                        "q_table_updated": qtable_updated
                    }
                else:
                    logger.warning(f"MCP server returned status {resp.status}")
                    return {
                        "learning_status": "skipped",
                        "q_table_updated": False
                    }

    except aiohttp.ClientConnectorError as e:
        # MCP not running - non-fatal, bot continues
        logger.warning(f"MCP server not available: {e}")
        return {
            "learning_status": "skipped",
            "q_table_updated": False
        }
    except Exception as e:
            logger.error(f"Error calling learn_cynic: {e}", exc_info=True)
        return {
            "learning_status": "skipped",
            "q_table_updated": False
        }


async def observe_cynic(aspect: str = "consciousness", detailed: bool = False) -> dict:
    """
    Observe CYNIC organism state.

    Args:
        aspect: Which aspect to observe (consciousness, learning, health, etc.)
        detailed: Get detailed breakdown

    Returns:
        Organism state snapshot
    """
    try:
        logger.info(f"Observing CYNIC {aspect}...")

        from cynic.kernel.core.consciousness import get_consciousness

        consciousness = get_consciousness()

        # Get state snapshot using available properties
        state_dict = consciousness.to_dict() if hasattr(consciousness, 'to_dict') else {}

        # Build observation based on aspect
        if aspect == "consciousness":
            level = state_dict.get("active_level", "UNKNOWN")
            cycles = state_dict.get("cycles", {}).get("total", 0)
            snapshot = f"level={level}, total_cycles={cycles}"
            observation = f"Consciousness state: {snapshot}"
        elif aspect == "learning":
            cycles = state_dict.get("cycles", {})
            macro_cycles = cycles.get("MACRO", 0)
            snapshot = f"macro_cycles={macro_cycles} (judgments completed)"
            observation = f"Learning metrics: {snapshot}"
        elif aspect == "health":
            level = state_dict.get("active_level", "UNKNOWN")
            timers = state_dict.get("timers", {})
            critical_count = sum(1 for t in timers.values() if isinstance(t, dict) and t.get("health") == "CRITICAL")
            snapshot = f"status=online, level={level}, critical_timers={critical_count}"
            observation = f"Organism health: {snapshot}"
        else:
            # Full snapshot
            snapshot = str(state_dict)
            observation = f"Full organism snapshot: {snapshot}"

        logger.info("CYNIC observation complete")
        return {
            "status": "success",
            "observation": observation
        }

    except Exception as e:
            logger.error(f"Error calling observe_cynic: {e}", exc_info=True)
        return {
            "status": "error",
            "error": str(e)
        }


async def get_cynic_status() -> dict:
    """Get CYNIC kernel health status."""
    try:
        from cynic.kernel.core.consciousness import get_consciousness

        consciousness = get_consciousness()
        # Get available state snapshot
        state_dict = consciousness.to_dict() if hasattr(consciousness, 'to_dict') else {}

        return {
            "status": "online",
            "data": {
                "active_level": state_dict.get("active_level", "UNKNOWN"),
                "total_cycles": state_dict.get("cycles", {}).get("total", 0),
                "gradient": state_dict.get("gradient", 0),
            }
        }

    except Exception as e:
            logger.error(f"Error getting CYNIC status: {e}")
        return {
            "status": "error",
            "error": str(e)
        }


def parse_judgment_response(judgment_text: str) -> dict:
    """
    Parse CYNIC judgment response text into structured data.

    This is a simple parser - can be enhanced to extract more details.
    """
    data = {
        "verdict": "PENDING",
        "q_score": 0.0,
        "confidence": 0.618,
        "reasoning": judgment_text,
        "raw_response": judgment_text
    }

    # Extract verdict
    for verdict in ["HOWL", "WAG", "GROWL", "BARK"]:
        if verdict in judgment_text:
            data["verdict"] = verdict
            break

    # Extract Q-Score (pattern: Q-Score: XX.X/100 or Q-Score: XX.X)
    import re
    q_match = re.search(r"Q-Score[:\s]+([0-9.]+)", judgment_text, re.IGNORECASE)
    if q_match:
        try:
            data["q_score"] = float(q_match.group(1))
        except ValueError:
            pass

    # Extract confidence (pattern: XX.X% or XX.X)
    conf_match = re.search(r"Confidence[:\s]+([0-9.]+)", judgment_text, re.IGNORECASE)
    if conf_match:
        try:
            data["confidence"] = float(conf_match.group(1)) / 100.0
        except ValueError:
            pass

    return data


# Helper to format CYNIC verdict for Discord
def format_verdict_embed(judgment_data: dict, proposal_title: str = "") -> dict:
    """
    Format judgment data as Discord embed.

    Returns Discord embed dict for message.post()
    """
    verdict = judgment_data.get("verdict", "PENDING")
    q_score = judgment_data.get("q_score", 0.0)
    confidence = judgment_data.get("confidence", 0.0)

    # Verdict emoji
    verdict_emoji = {
        "HOWL": "",
        "WAG": "",
        "GROWL": "",
        "BARK": "",
        "PENDING": ""
    }

    embed = {
        "title": f"CYNIC Judgment {verdict_emoji.get(verdict, '')}",
        "description": f"**{verdict}** | Q-Score: **{q_score}/100** | Confidence: **{confidence:.1%}**",
        "color": {
            "HOWL": 0x00FF00,      # Green
            "WAG": 0xFFFF00,       # Yellow
            "GROWL": 0xFF8800,     # Orange
            "BARK": 0xFF0000,      # Red
            "PENDING": 0x888888    # Gray
        }.get(verdict, 0x888888),
        "fields": [
            {
                "name": "Recommendation",
                "value": {
                    "HOWL": "Highly Recommended - Strong consensus for approval",
                    "WAG": "Lean Toward Approval - Majority of Dogs favorable",
                    "GROWL": "Lean Toward Rejection - Proceed with caution",
                    "BARK": "Not Recommended - Strong consensus against",
                    "PENDING": "Judgment pending..."
                }.get(verdict, "Judgment unavailable"),
                "inline": False
            }
        ]
    }

    if judgment_data.get("raw_response"):
        embed["fields"].append({
            "name": "Details",
            "value": judgment_data["raw_response"][:1000],  # Truncate to 1000 chars
            "inline": False
        })

    return embed
