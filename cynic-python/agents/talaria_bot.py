#!/usr/bin/env python3
"""
Tier 2 AGENTS: Talaria Bot (@TalariaBuild)

MVP: /observe <domain>, /status

Personal account (askesis) = manual: T. sends context, reads bot replies.

Usage:
  export CYNIC_API_KEY=...
  export CYNIC_REST_ADDR=http://<TAILSCALE_CORE>:3030
  export CYNIC_TELEGRAM_BOT_TOKEN=...
  python3 talaria_bot.py
"""

import os
import json
import logging
import asyncio
from datetime import datetime
from typing import Optional, Dict, Any, List
from pathlib import Path

import aiohttp
from telegram import Update, ChatPermissions
from telegram.ext import Application, CommandHandler, MessageHandler, ChatMemberHandler, filters, ContextTypes
from telegram.constants import ChatMemberStatus

# ============================================================================
# Config
# ============================================================================

BOT_TOKEN = os.getenv("CYNIC_TELEGRAM_BOT_TOKEN")
CYNIC_API_KEY = os.getenv("CYNIC_API_KEY")
CYNIC_REST_ADDR = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
PUBLIC_GROUP_ID = os.getenv("TALARIA_PUBLIC_GROUP_ID") # e.g. -100...

# Local log fallback
OBSERVATION_LOG = Path.home() / ".cynic/organs/hermes/x/observation_log.jsonl"
LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO")

logging.basicConfig(
    level=getattr(logging, LOG_LEVEL),
    format="[%(asctime)s] %(levelname)s | %(name)s — %(message)s"
)
logger = logging.getLogger("TalariaBot")

# ============================================================================
# Kernel API
# ============================================================================

async def kernel_post(path: str, data: Dict[str, Any]) -> Dict[str, Any]:
    """Generic POST to kernel."""
    url = f"{CYNIC_REST_ADDR.rstrip('/')}{path}"
    headers = {
        "Authorization": f"Bearer {CYNIC_API_KEY}",
        "Content-Type": "application/json",
    }
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json=data, headers=headers, timeout=aiohttp.ClientTimeout(total=30)) as resp:
                if resp.status in (200, 201):
                    return await resp.json()
                else:
                    return {"error": f"HTTP {resp.status}", "text": await resp.text()}
    except Exception as e:
        return {"error": str(e)}

async def kernel_get(path: str) -> Dict[str, Any]:
    """Generic GET from kernel."""
    url = f"{CYNIC_REST_ADDR.rstrip('/')}{path}"
    headers = {
        "Authorization": f"Bearer {CYNIC_API_KEY}",
    }
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    return await resp.json()
                else:
                    return {"error": f"HTTP {resp.status}"}
    except Exception as e:
        return {"error": str(e)}

# ============================================================================
# Handlers
# ============================================================================

async def start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Start handler."""
    logger.info(f"Received /start from user {update.effective_user.id} (@{update.effective_user.username}) in chat {update.effective_chat.id}")
    msg = """🦋 **Talaria Oracle**

I am the voice of the Talaria organism.

Available commands:
  /judge <domain> <content> — Request a formal verdict
  /ask <question> — Ask the Community Agent about Talaria
  /status — Kernel health
  /observe [domain] — Recent observations

Example:
  /judge token talaria
  /ask What is the current raise status?
"""
    await update.message.reply_text(msg, parse_mode="Markdown")
    await post_observation("start", update, "success")

async def judge(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /judge command."""
    if not context.args or len(context.args) < 2:
        await update.message.reply_text("Usage: `/judge <domain> <content>`\nExample: `/judge token talaria`")
        return

    domain = context.args[0]
    content = " ".join(context.args[1:])
    
    status_msg = await update.message.reply_text(f"🔍 **Judging {domain}...**\nCalling independent Dogs...", parse_mode="Markdown")

    payload = {
        "content": content,
        "domain": domain,
        "priority": "user"
    }
    
    result = await kernel_post("/judge", payload)
    
    if "error" in result:
        await status_msg.edit_text(f"❌ **Judgment failed:**\n`{result['error']}`", parse_mode="Markdown")
        return

    v = result.get("verdict", "BARK")
    q = result.get("q_score", {}).get("total", 0.0)
    
    reply = f"⚖️ **Verdict: {v.upper()}** (Q-Score: {q:.3f})\n\n"
    
    # Add axiom breakdown if available
    axioms = result.get("q_score", {})
    reply += f"• Fidelity: {axioms.get('fidelity', 0):.3f}\n"
    reply += f"• Phi: {axioms.get('phi', 0):.3f}\n"
    reply += f"• Verify: {axioms.get('verify', 0):.3f}\n"
    reply += f"• Culture: {axioms.get('culture', 0):.3f}\n"
    reply += f"• Burn: {axioms.get('burn', 0):.3f}\n"
    reply += f"• Sovereignty: {axioms.get('sovereignty', 0):.3f}\n"
    
    await status_msg.edit_text(reply, parse_mode="Markdown")
    await post_observation("judge", update, "success", domain=domain, context=content)

async def ask(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /ask command — dispatches an agent task."""
    if not context.args:
        await update.message.reply_text("Usage: `/ask <your question>`")
        return

    question = " ".join(context.args)
    await update.message.reply_text("📡 **Dispatching question to Community Agent...**\nI will reply once the Oracle has reasoned.", parse_mode="Markdown")

    payload = {
        "kind": "community-manager",
        "domain": "community-engagement",
        "content": json.dumps({
            "chat_id": update.effective_chat.id,
            "message_id": update.effective_message.id,
            "user_id": update.effective_user.id,
            "username": update.effective_user.username,
            "question": question
        }),
        "agent_id": "talaria-bot"
    }
    
    result = await kernel_post("/agent-tasks", payload)
    
    if "error" in result:
        await update.message.reply_text(f"❌ **Failed to queue question:** {result['error']}")
    else:
        await post_observation("ask", update, "success", context=question)

async def handle_all_messages(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Ingest all messages as observations."""
    if not update.message or not update.message.text:
        return
        
    # Ignore commands (already handled)
    if update.message.text.startswith("/"):
        return

    await post_observation("chat", update, "received", context=update.message.text)

async def post_observation(tool_suffix: str, update: Update, status: str, domain: str = "community", context: str = ""):
    """Send observation to kernel."""
    payload = {
        "project": "talaria",
        "agent_id": "talaria-bot",
        "tool": f"telegram_{tool_suffix}",
        "target": f"chat_{update.effective_chat.id}",
        "domain": domain,
        "status": status,
        "context": context[:200],
        "session_id": "organic-comms",
        "tags": ["community", "telegram"],
        "value": {
            "user_id": update.effective_user.id,
            "username": update.effective_user.username,
            "text": update.message.text if update.message else ""
        }
    }
    await kernel_post("/observe", payload)

async def status(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /status command."""
    data = await kernel_get("/health")
    if "error" in data:
        await update.message.reply_text("⚠️ Kernel unavailable.")
        return

    healthy = data.get("healthy", False)
    emoji = "✅" if healthy else "⚠️"
    msg = f"{emoji} **Kernel Status: {'HEALTHY' if healthy else 'DEGRADED'}**\n"
    msg += f"Dogs: {len(data.get('dogs', []))} active\n"
    msg += f"Uptime: {data.get('uptime_secs', 0)}s"
    
    await update.message.reply_text(msg, parse_mode="Markdown")

async def observe_cmd(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /observe command."""
    domain = context.args[0] if context.args else None
    path = f"/observations?limit=5"
    if domain: path += f"&domain={domain}"
    
    data = await kernel_get(path)
    if "error" in data:
        await update.message.reply_text("⚠️ Failed to fetch observations.")
        return

    obs_list = data.get("observations", [])
    if not obs_list:
        await update.message.reply_text("📭 No observations.")
        return

    lines = ["📋 **Recent Observations**"]
    for obs in obs_list:
        lines.append(f"• `{obs['domain']}`: {obs['tool']} ({obs['status']})")
    
    await update.message.reply_text("\n".join(lines), parse_mode="Markdown")

# ============================================================================
# PoH Gate
# ============================================================================

async def on_user_join(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Detect new user join, restrict them, and send PoH challenge."""
    result = update.chat_member
    if result.new_chat_member.status != ChatMemberStatus.MEMBER:
        return
        
    user = result.new_chat_member.user
    logger.info(f"New user joined: {user.id} (@{user.username})")

    # Restrict user
    try:
        await context.bot.restrict_chat_member(
            chat_id=update.effective_chat.id,
            user_id=user.id,
            permissions=ChatPermissions(can_send_messages=False)
        )
        logger.info(f"Restricted user {user.id}")
    except Exception as e:
        logger.error(f"Failed to restrict user {user.id}: {e}")
        return

    # Send welcome message
    poh_url = f"https://blitz-and-chill-web.vercel.app/en?poh_user_id={user.id}"
    msg = f"Welcome @{user.username or user.first_name}! ♟️\n\nTo speak in this group, you must prove you are human by beating the bot at chess (PoH L0).\n\nChallenge link: {poh_url}\n\nOnce done, click /verify to join the conversation."
    
    await context.bot.send_message(chat_id=update.effective_chat.id, text=msg)

async def verify_poh(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Check kernel for a 'human' verdict for this user."""
    user_id = update.effective_user.id
    # We look for a verdict in the 'human' domain where target is the telegram user_id
    path = f"/verdicts?domain=human&target=telegram_{user_id}&limit=1"
    
    data = await kernel_get(path)
    if "error" in data or not data:
        await update.message.reply_text("❌ No proof of humanity found yet. Keep playing!")
        return

    # If we found a verdict, assume it's positive for now (MVP)
    # Ideally check q_score > 0.618
    await context.bot.restrict_chat_member(
        chat_id=update.effective_chat.id,
        user_id=user_id,
        permissions=ChatPermissions(
            can_send_messages=True,
            can_send_media_messages=True,
            can_send_other_messages=True,
            can_add_web_page_previews=True
        )
    )
    await update.message.reply_text("✅ Proof of Humanity verified! Welcome to the sovereign collective.")

# ============================================================================
# Background Tasks
# ============================================================================

async def poll_agent_results(application: Application):
    """Poll kernel for completed community-manager tasks and reply in Telegram."""
    logger.info("Starting background task: poll_agent_results")
    while True:
        try:
            # Poll for completed tasks
            # This is a stub for the autonomous feedback loop
            path = "/agent-tasks?status=completed&limit=5"
            data = await kernel_get(path)
            
            if "tasks" in data:
                for task in data["tasks"]:
                    # Check if this task was meant for Telegram
                    content = json.loads(task.get("content", "{}"))
                    if "chat_id" in content and "result" in task:
                        # Send result to chat
                        await application.bot.send_message(
                            chat_id=content["chat_id"],
                            text=f"🔮 **Oracle Response:**\n\n{task['result']}",
                            reply_to_message_id=content.get("message_id"),
                            parse_mode="Markdown"
                        )
                        # Mark as handled (need an endpoint or local tracking)
                        logger.info(f"Delivered autonomous response for task {task['agent_id']}")
        except Exception as e:
            logger.error(f"Polling error: {e}")
        
        await asyncio.sleep(30)

# ============================================================================
# Main
# ============================================================================

def main():
    """Run bot."""
    if not BOT_TOKEN:
        logger.error("CYNIC_TELEGRAM_BOT_TOKEN not set")
        return

    logger.info("Starting Talaria Telegram Bot...")

    app = Application.builder().token(BOT_TOKEN).build()

    # Handlers
    app.add_handler(CommandHandler("start", start))
    app.add_handler(CommandHandler("help", start))
    app.add_handler(CommandHandler("judge", judge))
    app.add_handler(CommandHandler("ask", ask))
    app.add_handler(CommandHandler("status", status))
    app.add_handler(CommandHandler("observe", observe_cmd))
    app.add_handler(CommandHandler("verify", verify_poh))
    
    # PoH Gate
    app.add_handler(ChatMemberHandler(on_user_join, ChatMemberHandler.CHAT_MEMBER))
    
    app.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_all_messages))

    # Background tasks
    # asyncio.create_task(poll_agent_results(app)) # Not supported in run_polling directly

    logger.info("✅ Bot ready. Polling for messages...")
    app.run_polling()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        logger.info("Bot stopped")
