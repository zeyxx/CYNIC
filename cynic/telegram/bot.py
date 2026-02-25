"""
CYNIC Telegram Bot — Governance for memecoin communities

Connects to CYNIC HTTP API and provides commands for:
- /propose — Submit a governance proposal
- /vote — Cast a vote on a proposal
- /verdict — Get CYNIC's verdict on a proposal
- /gov_status — Check governance system status

Usage: python -m cynic.telegram.bot
"""
from __future__ import annotations

import asyncio
import logging
import os
import sys
from typing import Any, Optional

import httpx
from dotenv import load_dotenv
from telegram import Update
from telegram.ext import Application, CommandHandler, ContextTypes

# Load environment variables
load_dotenv()

# Logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s — %(message)s",
)
logger = logging.getLogger("cynic.telegram")

# Configuration
TELEGRAM_TOKEN = os.getenv("TELEGRAM_TOKEN")
CYNIC_API_URL = os.getenv("CYNIC_API_URL", "http://localhost:8000")
CYNIC_API_TIMEOUT = int(os.getenv("CYNIC_API_TIMEOUT", "30"))

# HTTP client for API calls
http_client: Optional[httpx.AsyncClient] = None


async def init_http_client() -> None:
    """Initialize the HTTP client."""
    global http_client
    if http_client is None:
        http_client = httpx.AsyncClient(
            timeout=httpx.Timeout(CYNIC_API_TIMEOUT),
        )


async def close_http_client() -> None:
    """Close the HTTP client."""
    global http_client
    if http_client is not None:
        await http_client.aclose()
        http_client = None


# ════════════════════════════════════════════════════════════════════════════
# COMMAND HANDLERS
# ════════════════════════════════════════════════════════════════════════════


async def start(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Start command — show welcome message."""
    user = update.effective_user
    message = (
        f"👋 Welcome to CYNIC Governance, {user.mention_html()}!\n\n"
        "CYNIC helps memecoin communities make fair governance decisions.\n\n"
        "*Available Commands:*\n"
        "/propose — Submit a governance proposal\n"
        "/vote — Cast a vote on a proposal\n"
        "/verdict — Get CYNIC's verdict\n"
        "/gov_status — Check system status\n\n"
        "_Type /propose followed by a title and description._"
    )
    await update.message.reply_html(message)
    logger.info(f"Start command from {user.id}")


async def propose(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Submit a governance proposal."""
    user = update.effective_user
    chat = update.effective_chat

    # Parse arguments: /propose <title> | <description>
    if not context.args or len(context.args) < 3:
        await update.message.reply_text(
            "❌ Usage: /propose <title> | <description>\n"
            "Example: /propose Fee Increase | Increase trading fee from 1% to 2%"
        )
        return

    # Find the pipe separator
    try:
        pipe_index = context.args.index("|")
    except ValueError:
        await update.message.reply_text(
            "❌ Please use | to separate title from description.\n"
            "Example: /propose Fee Increase | Increase trading fee from 1% to 2%"
        )
        return

    title = " ".join(context.args[:pipe_index])
    description = " ".join(context.args[pipe_index + 1 :])

    if not title or not description:
        await update.message.reply_text("❌ Title and description cannot be empty.")
        return

    # Send request to CYNIC API
    try:
        async with http_client.post(
            f"{CYNIC_API_URL}/api/governance/proposals",
            json={
                "community_id": str(chat.id),
                "title": title,
                "description": description,
                "proposer": str(user.id),
            },
        ) as resp:
            if resp.status_code != 200:
                await update.message.reply_text(
                    f"❌ Failed to submit proposal (HTTP {resp.status_code})"
                )
                logger.warning(f"Proposal submission failed: HTTP {resp.status_code}")
                return

            data = resp.json()

        proposal_id = data.get("proposal_id", "unknown")
        message = (
            f"📋 *Proposal Submitted*\n\n"
            f"*Title:* {title}\n"
            f"*ID:* `{proposal_id}`\n"
            f"*Proposer:* {user.mention_html()}\n\n"
            "⏳ CYNIC is analyzing this proposal...\n\n"
            f"Run /verdict {proposal_id} to see the verdict"
        )
        await update.message.reply_html(message)
        logger.info(f"Proposal submitted: {proposal_id} by {user.id}")

    except httpx.TimeoutException:
        await update.message.reply_text("⏱️ Request timed out. Please try again.")
        logger.warning("Proposal submission timed out")
    except Exception as e:
        await update.message.reply_text(f"❌ Error: {str(e)[:200]}")
        logger.error(f"Error submitting proposal: {e}")


async def vote(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Cast a vote on a proposal."""
    user = update.effective_user

    # Parse arguments: /vote <proposal_id> <yes|no|abstain>
    if not context.args or len(context.args) < 2:
        await update.message.reply_text(
            "❌ Usage: /vote <proposal_id> <yes|no|abstain>\n"
            "Example: /vote abc12345 yes"
        )
        return

    proposal_id = context.args[0]
    choice = context.args[1].lower()

    if choice not in ("yes", "no", "abstain"):
        await update.message.reply_text("❌ Vote must be 'yes', 'no', or 'abstain'")
        return

    # Send request to CYNIC API
    try:
        async with http_client.post(
            f"{CYNIC_API_URL}/api/governance/proposals/{proposal_id}/vote",
            json={
                "voter": str(user.id),
                "vote": choice,
            },
        ) as resp:
            if resp.status_code != 200:
                await update.message.reply_text(
                    f"❌ Failed to record vote (HTTP {resp.status_code})"
                )
                logger.warning(f"Vote failed: HTTP {resp.status_code}")
                return

        message = (
            f"✅ *Vote Recorded*\n\n"
            f"*Proposal:* `{proposal_id}`\n"
            f"*Your Vote:* {choice.upper()}\n\n"
            "Community votes + CYNIC verdict determines outcome"
        )
        await update.message.reply_html(message)
        logger.info(f"Vote recorded: {proposal_id} — user {user.id} voted {choice}")

    except httpx.TimeoutException:
        await update.message.reply_text("⏱️ Request timed out. Please try again.")
        logger.warning("Vote submission timed out")
    except Exception as e:
        await update.message.reply_text(f"❌ Error: {str(e)[:200]}")
        logger.error(f"Error recording vote: {e}")


async def verdict(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Get CYNIC's verdict on a proposal."""
    user = update.effective_user

    # Parse arguments: /verdict <proposal_id>
    if not context.args:
        await update.message.reply_text(
            "❌ Usage: /verdict <proposal_id>\n"
            "Example: /verdict abc12345"
        )
        return

    proposal_id = context.args[0]

    # Send request to CYNIC API
    try:
        async with http_client.get(
            f"{CYNIC_API_URL}/api/governance/proposals/{proposal_id}/verdict"
        ) as resp:
            if resp.status_code == 404:
                await update.message.reply_text(
                    f"❌ No verdict found for proposal `{proposal_id}` yet."
                )
                return
            elif resp.status_code != 200:
                await update.message.reply_text(
                    f"❌ Error: HTTP {resp.status_code}"
                )
                logger.warning(f"Verdict fetch failed: HTTP {resp.status_code}")
                return

            data = resp.json()

        verdict_type = data.get("verdict_type", "UNKNOWN")
        q_score = data.get("q_score", 0)
        confidence = data.get("confidence", 0)

        # Emoji based on verdict
        verdict_emoji = {
            "APPROVED": "✅",
            "TENTATIVE_APPROVE": "⚠️",
            "CAUTION": "⚡",
            "REJECT": "❌",
        }.get(verdict_type, "❓")

        axiom_scores = data.get("axiom_scores", {})
        axioms_text = "\n".join(
            f"• {k}: {v:.2f}" for k, v in list(axiom_scores.items())[:5]
        ) if axiom_scores else "No axiom data"

        message = (
            f"🔮 *CYNIC Verdict*\n\n"
            f"{verdict_emoji} *Verdict:* {verdict_type}\n"
            f"📊 *Q-Score:* {q_score:.1f}/100\n"
            f"🎯 *Confidence:* {confidence:.3f}\n\n"
            f"*Axiom Scores:*\n{axioms_text}\n\n"
            f"`Proposal: {proposal_id}`"
        )
        await update.message.reply_html(message)
        logger.info(f"Verdict fetched: {proposal_id} — {verdict_type}")

    except httpx.TimeoutException:
        await update.message.reply_text("⏱️ Request timed out. Please try again.")
        logger.warning("Verdict fetch timed out")
    except Exception as e:
        await update.message.reply_text(f"❌ Error: {str(e)[:200]}")
        logger.error(f"Error fetching verdict: {e}")


async def gov_status(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Check governance system status."""
    user = update.effective_user

    # Send request to CYNIC API
    try:
        async with http_client.get(
            f"{CYNIC_API_URL}/api/governance/status"
        ) as resp:
            if resp.status_code != 200:
                await update.message.reply_text(
                    f"❌ Error: HTTP {resp.status_code}"
                )
                logger.warning(f"Status fetch failed: HTTP {resp.status_code}")
                return

            data = resp.json()

        status = data.get("status", "unknown").upper()
        gasdf_status = data.get("gasdf_status", "unknown").upper()
        proposals_total = data.get("proposals_total", 0)
        proposals_active = data.get("proposals_active", 0)
        verdicts_issued = data.get("verdicts_issued", 0)
        executions_completed = data.get("executions_completed", 0)
        lnsp_sensors = data.get("lnsp_sensors", 0)
        lnsp_handlers = data.get("lnsp_handlers", 0)

        message = (
            f"⚙️ *Governance System Status*\n\n"
            f"📡 *Status:* {status}\n"
            f"⛓️ *GASdf:* {gasdf_status}\n\n"
            f"*Proposals:*\n"
            f"• Total: {proposals_total}\n"
            f"• Active: {proposals_active}\n"
            f"• Verdicts: {verdicts_issued}\n"
            f"• Executions: {executions_completed}\n\n"
            f"*LNSP:*\n"
            f"• Sensors: {lnsp_sensors}\n"
            f"• Handlers: {lnsp_handlers}"
        )
        await update.message.reply_html(message)
        logger.info(f"Status fetched: {status}")

    except httpx.TimeoutException:
        await update.message.reply_text("⏱️ Request timed out. Please try again.")
        logger.warning("Status fetch timed out")
    except Exception as e:
        await update.message.reply_text(f"❌ Error: {str(e)[:200]}")
        logger.error(f"Error fetching status: {e}")


async def help_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Help command — show available commands."""
    message = (
        "*Available Commands:*\n\n"
        "/propose <title> | <description> — Submit proposal\n"
        "/vote <proposal_id> <yes|no|abstain> — Cast vote\n"
        "/verdict <proposal_id> — Get CYNIC's verdict\n"
        "/gov_status — Check system status\n\n"
        "*Examples:*\n"
        "`/propose Fee Increase | Raise trading fee to 2%`\n"
        "`/vote abc12345 yes`\n"
        "`/verdict abc12345`"
    )
    await update.message.reply_html(message)
    logger.info(f"Help command from user {update.effective_user.id}")


# ════════════════════════════════════════════════════════════════════════════
# MAIN
# ════════════════════════════════════════════════════════════════════════════


def main() -> None:
    """Run the Telegram bot."""
    if not TELEGRAM_TOKEN:
        logger.error("TELEGRAM_TOKEN not set. Set it in .env file or environment variable.")
        sys.exit(1)

    logger.info("Starting CYNIC Telegram bot...")
    logger.info(f"CYNIC API: {CYNIC_API_URL}")

    # Create application
    application = Application.builder().token(TELEGRAM_TOKEN).build()

    # Add command handlers
    application.add_handler(CommandHandler("start", start))
    application.add_handler(CommandHandler("help", help_command))
    application.add_handler(CommandHandler("propose", propose))
    application.add_handler(CommandHandler("vote", vote))
    application.add_handler(CommandHandler("verdict", verdict))
    application.add_handler(CommandHandler("gov_status", gov_status))

    # Initialize HTTP client on startup, close on shutdown
    async def post_init(application: Application) -> None:
        await init_http_client()

    async def post_stop(application: Application) -> None:
        await close_http_client()

    application.post_init = post_init
    application.post_stop = post_stop

    # Start the bot
    try:
        application.run_polling(allowed_updates=Update.ALL_TYPES)
    except KeyboardInterrupt:
        logger.info("Bot shutdown requested")
    finally:
        logger.info("CYNIC Telegram bot stopped")


if __name__ == "__main__":
    main()
