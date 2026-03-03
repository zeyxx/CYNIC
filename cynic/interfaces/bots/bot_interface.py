"""
Unified Bot Interface and command/response dataclasses.

This module defines the abstract interface and data structures for all CYNIC bot
implementations (Discord, Telegram, CLI, Web), consolidating 4 separate
implementations into a single contract.

Key principles:
1. All bots implement BotInterface (abstract base class)
2. All commands are BotCommand objects (platform-agnostic)
3. All responses are BotResponse objects (platform-agnostic)
4. Platform adapters handle: Platform Format " BotCommand/BotResponse
5. Core logic is platform-independent

Example flow:
    Discord Message
        " (Discord adapter)
    BotCommand(name="propose", args={...}, platform="discord")
        " (Core logic)
    BotResponse(success=True, message="...", data={...})
        " (Discord adapter)
    Discord Embed + Buttons
"""

from __future__ import annotations

import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class BotCommand:
    """Platform-agnostic command representation.

    Encapsulates a user command regardless of platform.
    Platform adapters translate: Platform Event ' BotCommand ' Logic ' BotResponse.

    Attributes:
        name: Command name ("propose", "vote", "status", etc)
        args: Command arguments as dictionary
        user_id: Platform-specific user ID (required for tracking, permissions)
        platform: Platform name ("discord", "telegram", "cli", "web")
        guild_id: Optional group/server/chat ID for platform context
        timestamp: Unix timestamp when command was issued (for ordering)

    Raises:
        ValueError: If platform is not in {"discord", "telegram", "cli", "web"}
    """

    name: str
    args: dict[str, Any]
    user_id: str
    platform: str
    guild_id: str | None = None
    timestamp: float = field(default_factory=time.time)

    VALID_PLATFORMS = {"discord", "telegram", "cli", "web"}

    def __post_init__(self):
        """Validate platform is in allowed set."""
        if self.platform not in self.VALID_PLATFORMS:
            raise ValueError(
                f"platform must be in {self.VALID_PLATFORMS}, got '{self.platform}'"
            )


@dataclass(frozen=True)
class BotResponse:
    """Platform-agnostic response representation.

    Platform adapters translate: BotResponse ' Platform Format.
    Represents the result of handling a BotCommand.

    Attributes:
        success: Whether command execution succeeded
        message: Human-readable response message
        data: Structured response data (proposal ID, vote results, etc)
        ephemeral: If True, response is only visible to the requester
        error: Error message if success=False (optional debugging info)

    Raises:
        TypeError: If success is not a boolean
    """

    success: bool
    message: str
    data: dict[str, Any] = field(default_factory=dict)
    ephemeral: bool = False
    error: str | None = None

    def __post_init__(self):
        """Validate success is boolean."""
        if not isinstance(self.success, bool):
            raise TypeError(f"success must be bool, got {type(self.success).__name__}")


class BotInterface(ABC):
    """Abstract interface for all CYNIC bot implementations.

    All bots regardless of platform must:
    1. Support async start/stop lifecycle
    2. Handle BotCommand objects
    3. Return BotResponse objects
    4. Be platform-agnostic (adapters handle translation)

    Subclasses implement start/stop for platform connection and
    handle_command for business logic.

    Example implementation:
        class MyBot(BotInterface):
            async def start(self):
                # Connect to platform API
                pass

            async def stop(self):
                # Disconnect cleanly
                pass

            async def handle_command(self, command: BotCommand) -> BotResponse:
                if command.name == "propose":
                    # Handle proposal creation
                    return BotResponse(success=True, message="Proposal created")
                else:
                    return BotResponse(success=False, message="Unknown command")
    """

    @abstractmethod
    async def start(self) -> None:
        """Start the bot (connect to platform, register commands, etc).

        Called during bot initialization. Should:
        - Connect to platform API
        - Register command handlers
        - Set up event listeners
        - Start background tasks if needed

        Raises:
            Exception: If connection or setup fails
        """
        pass

    @abstractmethod
    async def stop(self) -> None:
        """Stop the bot cleanly.

        Called during shutdown. Should:
        - Cancel background tasks
        - Disconnect from platform API
        - Clean up resources
        - Not raise exceptions (cleanup should be best-effort)
        """
        pass

    @abstractmethod
    async def handle_command(self, command: BotCommand) -> BotResponse:
        """Handle a command and return response.

        Core business logic dispatcher. Translates BotCommand to
        appropriate handler and returns BotResponse.

        Args:
            command: Parsed command from user

        Returns:
            BotResponse: Result to send back to user

        Raises:
            ValueError: If command is invalid
            Exception: If business logic fails (should be caught and returned as BotResponse)
        """
        pass
