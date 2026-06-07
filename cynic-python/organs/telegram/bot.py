#!/usr/bin/env python3
"""
Tier 2 ORGANS: Telegram Organ — Bot Service (@TalariaBuild)

Responsible for public engagement, community management, PoH verification gate,
and operational validation routing (Ops).

Operational Separation:
  This bot is independent from the 'listener.service' (Personal account ingestion).
  They share the same infrastructure space but run as separate processes with
  different security scopes.

Usage:
  export CYNIC_API_KEY=...
  export CYNIC_REST_ADDR=http://<TAILSCALE_CORE>:3030
  export CYNIC_TELEGRAM_BOT_TOKEN=...
  python3 bot.py
"""

import os
import json
import logging
import asyncio
import re
from datetime import datetime
from typing import Optional, Dict, Any, List
from pathlib import Path

import aiohttp
from telegram import Update, ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup
from telegram.ext import Application, CommandHandler, MessageHandler, ChatMemberHandler, CallbackQueryHandler, filters, ContextTypes
from telegram.constants import ChatMemberStatus

# ============================================================================
# Config
# ============================================================================

BOT_TOKEN = os.getenv("CYNIC_TELEGRAM_BOT_TOKEN")
CYNIC_API_KEY = os.getenv("CYNIC_API_KEY")
CYNIC_REST_ADDR = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
PUBLIC_GROUP_ID = os.getenv("TALARIA_PUBLIC_GROUP_ID") # e.g. -100...
OPS_CHAT_ID = os.getenv("TALARIA_OPS_CHAT_ID")
BC_API_URL = os.getenv("BC_API_URL", "https://blitzchill.space/api/verify")

# Local log fallback
OBSERVATION_LOG = Path.home() / ".cynic/organs/hermes/x/observation_log.jsonl"
LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO")
SOLANA_ADDRESS_RE = re.compile(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$")

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

async def fetch_bnc_poh(wallet: str) -> Dict[str, Any]:
    """Read B&C PoH state for a wallet. B&C is the producer; Telegram only consumes it."""
    url = f"{BC_API_URL}?wallet={wallet}"
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(url, timeout=aiohttp.ClientTimeout(total=10)) as resp:
                if resp.status == 200:
                    return await resp.json()
                if resp.status == 404:
                    return {"verified": False, "reason": "not found"}
                return {"error": f"HTTP {resp.status}", "text": await resp.text()}
    except Exception as e:
        return {"error": str(e)}

def _configured_public_group_id() -> Optional[int]:
    if not PUBLIC_GROUP_ID:
        return None
    try:
        return int(PUBLIC_GROUP_ID)
    except ValueError:
        logger.warning("TALARIA_PUBLIC_GROUP_ID is not an integer: %s", PUBLIC_GROUP_ID)
        return None

def _configured_ops_chat_id() -> Optional[int]:
    if not OPS_CHAT_ID:
        return None
    try:
        return int(OPS_CHAT_ID)
    except ValueError:
        logger.warning("TALARIA_OPS_CHAT_ID is not an integer: %s", OPS_CHAT_ID)
        return None

# ============================================================================
# Public Telegram Surface
# ============================================================================

async def start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Start handler."""
    logger.info(f"Received /start from user {update.effective_user.id} (@{update.effective_user.username}) in chat {update.effective_chat.id}")
    msg = """🦋 **Talaria Oracle**

I am the voice of the Talaria organism.

Available commands:
  /judge <domain> <content> — Request a formal verdict
  /ask <question> — Ask the Community Agent about Talaria
  /verify <wallet> — Verify your B&C PoH wallet for public group access
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

async def on_user_join(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Public group gate: new members must verify a B&C PoH wallet before speaking."""
    public_group_id = _configured_public_group_id()
    if public_group_id is None or update.effective_chat.id != public_group_id:
        return

    result = update.chat_member
    if result.new_chat_member.status != ChatMemberStatus.MEMBER:
        return

    user = result.new_chat_member.user
    logger.info("New public user joined: %s (@%s)", user.id, user.username)

    try:
        await context.bot.restrict_chat_member(
            chat_id=public_group_id,
            user_id=user.id,
            permissions=ChatPermissions(can_send_messages=False),
        )
        logger.info("Restricted public user %s pending PoH", user.id)
    except Exception as e:
        logger.error("Failed to restrict user %s: %s", user.id, e)
        return

    poh_url = "https://blitzchill.space/en/poh"
    msg = (
        f"Welcome @{user.username or user.first_name}.\n\n"
        "To speak here, prove humanity through Blitz & Chill chess PoH.\n"
        f"Play: {poh_url}\n\n"
        "Then DM me or use here: /verify <wallet>"
    )
    await context.bot.send_message(chat_id=public_group_id, text=msg)

async def verify_poh(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Public PoH consumer: verify a wallet against B&C and unmute the Telegram user."""
    if not context.args or len(context.args) != 1:
        await update.message.reply_text("Usage: `/verify <wallet>`", parse_mode="Markdown")
        return

    wallet = context.args[0].strip()
    if not SOLANA_ADDRESS_RE.match(wallet):
        await update.message.reply_text("Invalid Solana wallet address.")
        return

    public_group_id = _configured_public_group_id()
    target_chat_id = public_group_id or update.effective_chat.id
    user = update.effective_user

    data = await fetch_bnc_poh(wallet)
    if "error" in data:
        await update.message.reply_text(f"B&C PoH unavailable: `{data['error']}`", parse_mode="Markdown")
        return

    verified = bool(data.get("verified"))
    await kernel_post("/observe", {
        "project": "talaria",
        "agent_id": "talaria-bot",
        "tool": "telegram_poh_verify",
        "target": f"telegram:{user.id}",
        "domain": "talaria.poh.user",
        "status": "verified" if verified else "not_verified",
        "context": f"B&C PoH verification consumed for wallet {wallet}",
        "session_id": "public-poh-gate",
        "tags": ["telegram", "public", "poh", "blitz-and-chill"],
        "value": {
            "telegram_user_id": user.id,
            "username": user.username,
            "wallet": wallet,
            "bnc": data,
        },
    })

    if not verified:
        reason = data.get("reason", "wallet has no indexed B&C PoH proof")
        await update.message.reply_text(f"PoH not verified yet: {reason}.")
        return

    try:
        await context.bot.restrict_chat_member(
            chat_id=target_chat_id,
            user_id=user.id,
            permissions=ChatPermissions(
                can_send_messages=True,
                can_send_media_messages=True,
                can_send_other_messages=True,
                can_add_web_page_previews=True,
            ),
        )
    except Exception as e:
        logger.error("Failed to unrestrict user %s in chat %s: %s", user.id, target_chat_id, e)
        await update.message.reply_text("PoH verified, but I could not update group permissions. Ops must check bot admin rights.")
        return

    archetype = data.get("archetype", "unknown")
    await update.message.reply_text(f"PoH verified via B&C. Welcome. Archetype: `{archetype}`", parse_mode="Markdown")

# ============================================================================
# Ops Telegram Surface
# ============================================================================

async def poll_engagement_tasks(app: Application):
    """Background task to poll kernel for pending validation tasks."""
    processed_tasks = set()
    logger.info("Starting engagement validation poller...")
    
    while True:
        try:
            logger.debug("Polling kernel for engagement tasks...")
            data = await kernel_get("/agent-tasks?kind=engagement-validation&status=pending")
            if "tasks" in data:
                tasks = data["tasks"]
                if tasks:
                    logger.info(f"Found {len(tasks)} pending tasks")
                for task in tasks:
                    task_id = task["id"]
                    if task_id not in processed_tasks:
                        logger.info(f"New task detected: {task_id}")
                        await notify_ops_of_task(app, task)
                        processed_tasks.add(task_id)
        except Exception as e:
            logger.error(f"Error polling tasks: {e}")
        await asyncio.sleep(30)

async def notify_ops_of_task(app: Application, task: Dict[str, Any]):
    """Send an interactive message to Ops chat with improved structured UX."""
    try:
        content = json.loads(task["content"])
        platform = content.get("platform", "X")
        target_url = content.get("target_url", "Unknown")
        proposed_text = content.get("proposed_text", "")
        reasoning = content.get("reasoning", "No automated reasoning provided.")
        original = content.get("original_text", "Context unavailable.")
        task_id = task["id"]

        msg = f"🛡️ **SOVEREIGN ACTION REQUIRED**\n"
        msg += f"━━━━━━━━━━━━━━━━━━━━\n"
        msg += f"📍 **Platform:** {platform}\n"
        msg += f"🔗 **Target:** [View Tweet]({target_url})\n\n"
        
        msg += f"📖 **Context:**\n`{original[:150]}...`\n\n"
        
        msg += f"🧠 **CYNIC Reasoning:**\n_{reasoning}_\n\n"
        
        msg += f"✍️ **Proposed Response:**\n{proposed_text}\n"
        msg += f"━━━━━━━━━━━━━━━━━━━━\n"
        msg += "Do you authorize this sovereign transmission?"

        keyboard = [
            [
                InlineKeyboardButton("🚀 AUTHORIZE (POST)", callback_data=f"engage_approve:{task_id}"),
            ],
            [
                InlineKeyboardButton("🌑 IGNORE", callback_data=f"engage_reject:{task_id}")
            ]
        ]
        reply_markup = InlineKeyboardMarkup(keyboard)

        await app.bot.send_message(
            chat_id=OPS_CHAT_ID,
            text=msg,
            reply_markup=reply_markup,
            parse_mode="Markdown",
            disable_web_page_preview=True
        )
        logger.info(f"Structured notification sent for task {task_id}")
    except Exception as e:
        logger.error(f"Failed to notify ops of task {task['id']}: {e}")

async def handle_engagement_callback(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle button clicks for engagement approval/rejection."""
    query = update.callback_query
    await query.answer()

    ops_chat_id = _configured_ops_chat_id()
    if ops_chat_id is not None and query.message and query.message.chat_id != ops_chat_id:
        await query.edit_message_text("This action is restricted to Talaria Ops.")
        return
    
    data = query.data
    action, task_id = data.split(":")
    
    if action == "engage_approve":
        # Update task in kernel to 'approved' (or similar)
        # Note: In a full system, the X organ would watch for this state change
        # For now, we'll mark it as completed with a result
        result = await kernel_post(f"/agent-tasks/{task_id}/result", {
            "result": "approved",
            "status": "completed"
        })
        await query.edit_message_text(text=f"{query.message.text}\n\n✅ **Approved for posting.**")
        
        # TRIGGER: Manual trigger of x_poster could happen here if needed
        # Or let the organ's next cycle see the 'completed/approved' status
        logger.info(f"Task {task_id} approved by user {update.effective_user.id}")
        
    elif action == "engage_reject":
        await kernel_post(f"/agent-tasks/{task_id}/result", {
            "result": "rejected",
            "status": "completed"
        })
        await query.edit_message_text(text=f"{query.message.text}\n\n❌ **Rejected/Ignored.**")
        logger.info(f"Task {task_id} rejected by user {update.effective_user.id}")

# ============================================================================
# Main
# ============================================================================

def main():
    """Run bot."""
    if not BOT_TOKEN:
        logger.error("CYNIC_TELEGRAM_BOT_TOKEN not set")
        return

    logger.info("Starting Talaria Telegram Bot...")

    async def post_init(application: Application):
        asyncio.create_task(poll_engagement_tasks(application))

    app = Application.builder().token(BOT_TOKEN).post_init(post_init).build()

    # Handlers
    app.add_handler(CommandHandler("start", start))
    app.add_handler(CommandHandler("help", start))
    app.add_handler(CommandHandler("judge", judge))
    app.add_handler(CommandHandler("ask", ask))
    app.add_handler(CommandHandler("status", status))
    app.add_handler(CommandHandler("observe", observe_cmd))
    app.add_handler(CommandHandler("verify", verify_poh))

    # Public PoH gate
    app.add_handler(ChatMemberHandler(on_user_join, ChatMemberHandler.CHAT_MEMBER))

    # Ops inline buttons
    app.add_handler(CallbackQueryHandler(handle_engagement_callback, pattern="^engage_"))
    
    app.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_all_messages))

    logger.info("✅ Bot ready. Polling for messages...")
    app.run_polling()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        logger.info("Bot stopped")
