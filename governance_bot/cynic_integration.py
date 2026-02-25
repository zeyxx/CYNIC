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


async def ask_cynic(question: str, context: str = "", reality: str = "GOVERNANCE") -> dict:
    """
    Ask CYNIC a governance question and get judgment.

    Returns:
        {
            "verdict": "HOWL" | "WAG" | "GROWL" | "BARK",
            "q_score": float,
            "confidence": float,
            "dogs_voting": {...},
            "reasoning": str,
            "estimated_outcome": str
        }
    """
    try:
        from cynic.mcp.claude_code_bridge import call_tool

        logger.info(f"Asking CYNIC: {question[:100]}...")

        result = await call_tool("ask_cynic", {
            "question": question,
            "context": context,
            "reality": reality
        })

        if result and len(result) > 0:
            judgment_text = result[0].text

            # Parse judgment response
            judgment_data = parse_judgment_response(judgment_text)
            logger.info(f"CYNIC response: {judgment_data.get('verdict')} (Q-Score: {judgment_data.get('q_score')})")

            return judgment_data
        else:
            logger.error("No response from CYNIC")
            return {
                "verdict": "PENDING",
                "q_score": 0.0,
                "confidence": 0.0,
                "error": "No response from CYNIC"
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
    outcome: bool,
    actual_metrics: dict = None,
    feedback_rating: int = 3,
    comment: str = ""
) -> dict:
    """
    Teach CYNIC from a proposal outcome.

    Args:
        judgment_id: ID of the judgment to learn from
        outcome: Whether proposal succeeded (bool)
        actual_metrics: Actual metrics achieved vs predicted
        feedback_rating: Community rating 1-5 stars
        comment: Optional comment

    Returns:
        {
            "learning_status": "completed",
            "q_table_updated": bool,
            "confidence_change": float
        }
    """
    try:
        from cynic.mcp.claude_code_bridge import call_tool

        logger.info(f"Teaching CYNIC from judgment {judgment_id}...")

        result = await call_tool("learn_cynic", {
            "judgment_id": judgment_id,
            "outcome": outcome,
            "actual_metrics": actual_metrics or {},
            "feedback_rating": feedback_rating,
            "comment": comment
        })

        if result and len(result) > 0:
            logger.info(f"CYNIC learning completed")
            return {
                "learning_status": "completed",
                "q_table_updated": True,
                "message": result[0].text
            }
        else:
            logger.warning("No confirmation from CYNIC learning")
            return {
                "learning_status": "pending",
                "q_table_updated": False,
                "message": "Learning queued for later processing"
            }

    except Exception as e:
        logger.error(f"Error calling learn_cynic: {e}", exc_info=True)
        return {
            "learning_status": "error",
            "q_table_updated": False,
            "error": str(e)
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
        from cynic.mcp.claude_code_bridge import call_tool

        logger.info(f"Observing CYNIC {aspect}...")

        result = await call_tool("observe_cynic", {
            "aspect": aspect,
            "detailed": detailed
        })

        if result and len(result) > 0:
            logger.info(f"CYNIC observation complete")
            return {
                "status": "success",
                "observation": result[0].text
            }
        else:
            logger.warning("No observation from CYNIC")
            return {
                "status": "no_data",
                "observation": "CYNIC not responding to observation request"
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
        from cynic.mcp.claude_code_bridge import call_tool

        result = await call_tool("cynic_status", {})

        if result and len(result) > 0:
            return {
                "status": "online",
                "data": result[0].text
            }
        else:
            return {
                "status": "offline",
                "error": "No response from CYNIC"
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
