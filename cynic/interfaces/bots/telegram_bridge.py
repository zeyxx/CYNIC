"""
Telegram Bridge — The Organism's Outward Voice.

Simple async notifier to send system alerts and internal 'pain' signals
to the human operator.

Requirements:
  - TELEGRAM_BOT_TOKEN
  - TELEGRAM_CHAT_ID
"""
from __future__ import annotations

import logging
import os

import httpx

logger = logging.getLogger("cynic.interfaces.telegram")

class TelegramBridge:
    """
    Minimalist Telegram Notifier.
    Uses httpx directly to avoid heavy bot framework dependencies.
    """
    def __init__(self, token: str | None = None, chat_id: str | None = None):
        self.token = token or os.environ.get("TELEGRAM_BOT_TOKEN")
        self.chat_id = chat_id or os.environ.get("TELEGRAM_CHAT_ID")
        self.base_url = f"https://api.telegram.org/bot{self.token}"
        self.active = all([self.token, self.chat_id])
        
        if not self.active:
            logger.warning("TelegramBridge: Missing credentials. Notifications disabled.")

    async def notify(self, message: str, silent: bool = False) -> bool:
        """Send a message to the configured chat."""
        if not self.active:
            return False

        url = f"{self.base_url}/sendMessage"
        payload = {
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "HTML",
            "disable_notification": silent
        }

        try:
            async with httpx.AsyncClient(timeout=10.0) as client:
                resp = await client.post(url, json=payload)
                resp.raise_for_status()
                return True
        except Exception as e:
            logger.error(f"TelegramBridge failed to send message: {e}")
            return False

    async def notify_anomaly(self, source: str, error: str, details: str | None = None):
        """Standardized format for system errors."""
        msg = (
            f"🚨 <b>CYNIC ANOMALY DETECTED</b>\n\n"
            f"<b>Source:</b> <code>{source}</code>\n"
            f"<b>Error:</b> <code>{error}</code>\n"
        )
        if details:
            msg += f"\n<b>Context:</b>\n<pre>{details[:500]}</pre>"
            
        await self.notify(msg)

    async def notify_resource_stress(self, cpu: float, ram: float):
        """Standardized format for hardware stress."""
        msg = (
            f"⚠️ <b>CYNIC METABOLIC STRESS</b>\n\n"
            f"<b>CPU:</b> {cpu:.1f}%\n"
            f"<b>RAM:</b> {ram:.1f}%\n"
            f"<i>Organism is slowing down to survive.</i>"
        )
        await self.notify(msg, silent=True)
