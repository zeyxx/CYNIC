#!/usr/bin/env python3
"""
CYNIC Telegram Bot (@CynicOracle)

MVP: /observe <domain>, /status

Personal account (askesis) = manual: T. sends context, reads bot replies.

Usage:
  export CYNIC_API_KEY=...
  export CYNIC_REST_ADDR=http://<TAILSCALE_CORE>:3030
  export CYNIC_TELEGRAM_BOT_TOKEN=...
  python3 telegram_organ_bot.py
"""

import os
import json
import logging
import asyncio
from datetime import datetime
from typing import Optional, Dict, Any

import aiohttp
from telegram import Update
from telegram.ext import Application, CommandHandler, MessageHandler, filters, ContextTypes

# ============================================================================
# Config
# ============================================================================

BOT_TOKEN = os.getenv("CYNIC_TELEGRAM_BOT_TOKEN")
CYNIC_API_KEY = os.getenv("CYNIC_API_KEY")
CYNIC_REST_ADDR = os.getenv("CYNIC_REST_ADDR", "http://<TAILSCALE_CORE>:3030")

import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from hermes_paths import HERMES_X_DIR
OBSERVATION_LOG = str(HERMES_X_DIR / "observation_log.jsonl")
LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO")

logging.basicConfig(
    level=getattr(logging, LOG_LEVEL),
    format="[%(asctime)s] %(levelname)s | %(name)s — %(message)s"
)
logger = logging.getLogger("TelegramOrganBot")

# ============================================================================
# Kernel API
# ============================================================================

async def fetch_observations(domain: Optional[str] = None, limit: int = 5) -> Dict[str, Any]:
    """Fetch observations from kernel."""
    path = f"/observations?limit={limit}"
    if domain:
        path += f"&domain={domain}"

    url = f"{CYNIC_REST_ADDR.rstrip('/')}{path}"
    headers = {
        "Authorization": f"Bearer {CYNIC_API_KEY}",
        "Content-Type": "application/json",
    }

    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    return await resp.json()
                elif resp.status == 401:
                    logger.warning("Kernel auth failed (401)")
                    return {"error": "Auth failed"}
                else:
                    logger.warning(f"Kernel HTTP {resp.status}")
                    return {"error": f"HTTP {resp.status}"}
    except asyncio.TimeoutError:
        logger.warning("Kernel request timeout")
        return {"error": "Timeout"}
    except Exception as e:
        logger.error(f"Kernel request failed: {type(e).__name__}")
        return {"error": "Connection failed"}

async def fetch_health() -> Dict[str, Any]:
    """Fetch kernel health."""
    url = f"{CYNIC_REST_ADDR.rstrip('/')}/health"
    headers = {
        "Authorization": f"Bearer {CYNIC_API_KEY}",
        "Content-Type": "application/json",
    }

    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    return await resp.json()
                else:
                    logger.warning(f"Health check HTTP {resp.status}")
                    return {"error": f"HTTP {resp.status}"}
    except Exception as e:
        logger.error(f"Health check failed: {type(e).__name__}")
        return {"error": "Connection failed"}

# ============================================================================
# Formatters
# ============================================================================

def format_observations(data: Dict[str, Any], limit: int = 5) -> str:
    """Format observations for Telegram."""
    if "error" in data:
        return "⚠️ Kernel unavailable. Please try again."

    observations = data.get("observations", [])
    if not observations:
        return "📭 No observations yet."

    lines = [f"📋 Recent Observations ({len(observations)} total)\n"]
    for obs in observations[:limit]:
        domain = obs.get("domain", "?")
        status = obs.get("status", "?")
        emoji = {"success": "✅", "error": "❌", "pending": "⏳"}.get(status, "❓")
        lines.append(f"{emoji} {domain}")

    if len(observations) > limit:
        lines.append(f"… +{len(observations) - limit} more")

    return "\n".join(lines)

def format_health(data: Dict[str, Any]) -> str:
    """Format health for Telegram."""
    if "error" in data:
        return "⚠️ Kernel unavailable. Please try again."

    healthy = data.get("healthy", False)
    emoji = "✅" if healthy else "⚠️"

    lines = [f"{emoji} Kernel Status: {'HEALTHY' if healthy else 'DEGRADED'}\n"]
    lines.append(f"⏱️ {datetime.now().isoformat(timespec='seconds')}")

    return "\n".join(lines)

# ============================================================================
# Log Interactions
# ============================================================================

async def log_interaction(
    command: str,
    user_id: int,
    chat_id: int,
    domain: Optional[str],
    status: str
):
    """Log to observation_log.jsonl."""
    try:
        os.makedirs(os.path.dirname(OBSERVATION_LOG), exist_ok=True)

        event = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "tool": f"telegram_bot_{command}",
            "target": f"telegram_user_{user_id}_chat_{chat_id}",
            "domain": domain or "unspecified",
            "status": status,
        }

        with open(OBSERVATION_LOG, "a") as f:
            f.write(json.dumps(event) + "\n")
    except Exception as e:
        logger.error(f"Logging failed: {e}")

# ============================================================================
# Handlers
# ============================================================================

async def start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Start handler."""
    msg = """🔮 **CYNIC Telegram Organ**

Available commands:
  /observe [domain] — Recent observations (default: all domains)
  /status — Kernel health

Three-Voice Experiment (May 1-10):
  Bot: Reactive /observe /status
  Personal: Manual context + askesis learning

Example:
  /observe token
  /status
"""
    await update.message.reply_text(msg, parse_mode="Markdown")
    await log_interaction("start", update.effective_user.id, update.effective_chat.id, None, "success")

async def observe(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /observe command."""
    domain = None
    if context.args:
        domain = " ".join(context.args)

    await update.message.reply_text("⏳ Fetching observations...", parse_mode="Markdown")

    data = await fetch_observations(domain=domain, limit=5)
    message = format_observations(data, limit=5)

    await update.message.reply_text(message, parse_mode="Markdown")
    await log_interaction("observe", update.effective_user.id, update.effective_chat.id, domain, "success")

async def status(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /status command."""
    data = await fetch_health()
    message = format_health(data)

    await update.message.reply_text(message, parse_mode="Markdown")
    await log_interaction("status", update.effective_user.id, update.effective_chat.id, "infrastructure", "success")

async def help_command(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /help command."""
    await start(update, context)

# ============================================================================
# Main
# ============================================================================

def main():
    """Run bot."""
    if not BOT_TOKEN:
        logger.error("CYNIC_TELEGRAM_BOT_TOKEN not set")
        return

    logger.info("Starting CYNIC Telegram Bot (@CynicOracle)...")

    app = Application.builder().token(BOT_TOKEN).build()

    app.add_handler(CommandHandler("start", start))
    app.add_handler(CommandHandler("help", help_command))
    app.add_handler(CommandHandler("observe", observe))
    app.add_handler(CommandHandler("status", status))

    logger.info("✅ Bot ready. Polling for messages...")
    app.run_polling()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        logger.info("Bot stopped")
