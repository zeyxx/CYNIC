"""
Unified bot interface for CYNIC.

This module defines the abstract interface for all CYNIC bot implementations
(Discord, Telegram, CLI, Web) to consolidate 4 separate implementations.

Architecture:
â"Œâ"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"
â"‚          UNIFIED BOT ARCHITECTURE                   â"‚
â"œâ"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"¤
â"‚                                                     â"‚
â"‚  BotCommand (platform-agnostic)                     â"‚
â"‚  â"œâ"€ name: str ("propose", "vote", etc)             â"‚
â"‚  â"œâ"€ args: Dict[str, Any] (command args)            â"‚
â"‚  â"œâ"€ user_id: str (platform-specific user ID)       â"‚
â"‚  â"œâ"€ platform: str ("discord", "telegram", ...)     â"‚
â"‚  â"œâ"€ guild_id: Optional[str] (group context)        â"‚
â"‚  â""â"€ timestamp: float (command timestamp)           â"‚
â"‚                                                     â"‚
â"‚  BotResponse (platform-agnostic)                    â"‚
â"‚  â"œâ"€ success: bool (did command succeed?)           â"‚
â"‚  â"œâ"€ message: str (human-readable response)         â"‚
â"‚  â"œâ"€ data: Dict[str, Any] (structured data)         â"‚
â"‚  â"œâ"€ ephemeral: bool (only visible to user?)        â"‚
â"‚  â""â"€ error: Optional[str] (error if not success)    â"‚
â"‚                                                     â"‚
â"‚  BotInterface (abstract)                            â"‚
â"‚  â"œâ"€ start() â' None (connect to platform)           â"‚
â"‚  â"œâ"€ stop() â' None (disconnect cleanly)             â"‚
â"‚  â""â"€ handle_command(BotCommand) â' BotResponse       â"‚
â"‚                                                     â"‚
â"‚  Platform Adapters (concrete implementations)       â"‚
â"‚  â"œâ"€ DiscordBot: Platform Event â' BotCommand        â"‚
â"‚  â"œâ"€ TelegramBot: Platform Event â' BotCommand       â"‚
â"‚  â"œâ"€ CLIBot: Platform Input â' BotCommand            â"‚
â"‚  â""â"€ WebBot: Platform Request â' BotCommand          â"‚
â"‚                                                     â"‚
â""â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"˜

Key principle:
- All platform-specific logic is in adapters
- Core logic operates on BotCommand/BotResponse
- Adapters translate: Platform â' BotCommand â' Logic â' BotResponse â' Platform
"""

from cynic.interfaces.bots.bot_interface import BotCommand, BotInterface, BotResponse

__all__ = ["BotInterface", "BotCommand", "BotResponse"]
