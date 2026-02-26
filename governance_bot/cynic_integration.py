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
    Ask CYNIC a governance question and get judgment.
    Calls CYNIC organism's orchestrator directly.

    Returns:
        {
            "verdict": "HOWL" | "WAG" | "GROWL" | "BARK",
            "q_score": float,
            "confidence": float,
            "dog_votes": {...},
            "axiom_scores": {...},
            "judgment_id": str,
            "dog_reasoning": {...},  # Tier 1 explainability
            "verdict_explanation": str
        }
    """
    try:
        logger.info(f"Asking CYNIC: {question[:100]}...")

        # Import CYNIC components
        from cynic.organism.organism import awaken
        from cynic.core.judgment import Cell
        from cynic.core.consciousness import ConsciousnessLevel

        # Get or create the CYNIC organism
        organism = awaken()

        # Create a Cell for judgment (CYNIC's internal query format)
        # Map governance reality to valid CYNIC realities
        cynic_reality = reality.upper() if reality.upper() in {"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"} else "SOCIAL"

        cell = Cell(
            content=question,
            context=context,
            reality=cynic_reality,
            analysis="JUDGE",
            lod=1  # Level of detail: 1 is balanced
        )

        # Run judgment through orchestrator (CYNIC's main judgment engine)
        # MICRO consciousness level (~500ms) is good for governance decisions
        judgment = await organism.orchestrator.run(
            cell,
            level=ConsciousnessLevel.MICRO,
            budget_usd=0.05
        )

        if judgment:
            # Format response to match governance bot expectations
            judgment_data = {
                "verdict": judgment.verdict,
                "q_score": judgment.q_score,
                "confidence": judgment.confidence,
                "judgment_id": judgment.judgment_id,
                "dog_votes": dict(judgment.dog_votes) if hasattr(judgment, "dog_votes") else {},
                "axiom_scores": dict(judgment.axiom_scores) if hasattr(judgment, "axiom_scores") else {},
                # Tier 1 explainability (from our earlier implementation)
                "dog_reasoning": judgment.dog_reasoning if hasattr(judgment, "dog_reasoning") else {},
                "verdict_explanation": judgment.verdict_explanation if hasattr(judgment, "verdict_explanation") else "",
                "consensus_reason": judgment.consensus_reason if hasattr(judgment, "consensus_reason") else "",
            }

            logger.info(f"CYNIC response: {judgment_data.get('verdict')} (Q={judgment_data.get('q_score'):.1f}, conf={judgment_data.get('confidence'):.2%})")
            return judgment_data
        else:
            logger.error("No judgment returned from CYNIC")
            return {
                "verdict": "PENDING",
                "q_score": 0.0,
                "confidence": 0.0,
                "error": "No judgment returned"
            }

    except asyncio.TimeoutError:
        logger.error("CYNIC request timed out")
        return {
            "verdict": "PENDING",
            "q_score": 0.0,
            "confidence": 0.0,
            "error": "CYNIC request timed out"
        }
    except Exception as e:
        logger.error(f"Error calling ask_cynic: {e}", exc_info=True)
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

        from config import CYNIC_MCP_URL
        import aiohttp

        # Normalize rating: 1-5 stars → -1.0 to +1.0
        base = (satisfaction / 5.0) * 2.0 - 1.0  # 1→-0.6, 3→0.2, 5→1.0
        if not approved:
            base = -abs(base)  # rejected → always negative
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

        from cynic.core.consciousness import get_consciousness

        organism = get_consciousness()

        # Get organism snapshot based on aspect
        if aspect == "consciousness":
            # Get state from available properties
            snapshot = f"uptime={organism.uptime_s:.1f}s, dogs={len(organism.dogs)}"
            observation = f"Consciousness state: {snapshot}"
        elif aspect == "learning":
            # Get Q-table stats from learning loop
            qtable = organism.qtable
            q_entries = len(qtable.table) if hasattr(qtable, "table") else 0
            snapshot = f"q_entries={q_entries}"
            observation = f"Learning metrics: {snapshot}"
        elif aspect == "health":
            # Return basic health indicators without calling non-existent methods
            snapshot = f"status=online, uptime={organism.uptime_s:.1f}s, dogs_active={len(organism.dogs)}"
            observation = f"Organism health: {snapshot}"
        else:
            snapshot = f"uptime={organism.uptime_s:.1f}s, dogs={len(organism.dogs)}"
            observation = f"Full organism snapshot: {snapshot}"

        logger.info(f"CYNIC observation complete")
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
        from cynic.core.consciousness import get_consciousness

        organism = get_consciousness()
        # Return basic health indicators using available properties
        health_data = {
            "uptime_seconds": organism.uptime_s,
            "dogs_active": len(organism.dogs),
            "has_orchestrator": organism.orchestrator is not None,
            "has_learning": organism.learning_loop is not None
        }

        return {
            "status": "online",
            "data": health_data
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
        "HOWL": "🎺",
        "WAG": "👍",
        "GROWL": "⚠️",
        "BARK": "🚫",
        "PENDING": "⏳"
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
