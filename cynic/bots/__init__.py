"""
Unified bot interface for CYNIC.

This module defines the abstract interface for all CYNIC bot implementations
(Discord, Telegram, CLI, Web) to consolidate 4 separate implementations.

Architecture:
┌─────────────────────────────────────────────────────┐
│          UNIFIED BOT ARCHITECTURE                   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  BotCommand (platform-agnostic)                     │
│  ├─ name: str ("propose", "vote", etc)             │
│  ├─ args: Dict[str, Any] (command args)            │
│  ├─ user_id: str (platform-specific user ID)       │
│  ├─ platform: str ("discord", "telegram", ...)     │
│  ├─ guild_id: Optional[str] (group context)        │
│  └─ timestamp: float (command timestamp)           │
│                                                     │
│  BotResponse (platform-agnostic)                    │
│  ├─ success: bool (did command succeed?)           │
│  ├─ message: str (human-readable response)         │
│  ├─ data: Dict[str, Any] (structured data)         │
│  ├─ ephemeral: bool (only visible to user?)        │
│  └─ error: Optional[str] (error if not success)    │
│                                                     │
│  BotInterface (abstract)                            │
│  ├─ start() → None (connect to platform)           │
│  ├─ stop() → None (disconnect cleanly)             │
│  └─ handle_command(BotCommand) → BotResponse       │
│                                                     │
│  Platform Adapters (concrete implementations)       │
│  ├─ DiscordBot: Platform Event → BotCommand        │
│  ├─ TelegramBot: Platform Event → BotCommand       │
│  ├─ CLIBot: Platform Input → BotCommand            │
│  └─ WebBot: Platform Request → BotCommand          │
│                                                     │
└─────────────────────────────────────────────────────┘

Key principle:
- All platform-specific logic is in adapters
- Core logic operates on BotCommand/BotResponse
- Adapters translate: Platform → BotCommand → Logic → BotResponse → Platform
"""

from cynic.bots.bot_interface import BotInterface, BotCommand, BotResponse

__all__ = ["BotInterface", "BotCommand", "BotResponse"]
