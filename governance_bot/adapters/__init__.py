"""
Governance bot adapters for different platforms (Discord, Telegram, etc).

This package contains platform-specific adapters that implement BotInterface
and bridge platform-specific events/interactions to the unified CYNIC system.
"""

from governance_bot.adapters.discord_adapter import DiscordAdapter
from governance_bot.adapters.telegram_adapter import TelegramAdapter

__all__ = ["DiscordAdapter", "TelegramAdapter"]
