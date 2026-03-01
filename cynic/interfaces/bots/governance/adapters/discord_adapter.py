"""
Discord Adapter implementing unified BotInterface.

This module bridges Discord interactions to the unified CYNIC consciousness system.
It translates Discord events into platform-agnostic BotCommand objects and converts
BotResponse objects back to Discord embeds, buttons, and messages.

Architecture:
┌─────────────────────────────────────────────────────────┐
│          DISCORD ADAPTER FLOW                            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Discord Event (interaction, message, etc)              │
│  ↓ (Discord adapter translates)                         │
│  BotCommand(name, args, user_id, platform="discord")   │
│  ↓ (Core logic routes)                                  │
│  UnifiedConsciousState.judge/record/query              │
│  ↓ (Core returns)                                       │
│  BotResponse(success, message, data, ephemeral)        │
│  ↓ (Discord adapter converts)                          │
│  Discord Embed + Buttons + Message                     │
│                                                         │
└─────────────────────────────────────────────────────────┘

Key principles:
1. DiscordAdapter implements BotInterface contract
2. All commands route through handle_command()
3. All responses are BotResponse objects
4. Error handling is graceful (no exceptions escape)
5. Views/buttons persist across bot restarts
6. Async throughout
"""

from __future__ import annotations

import logging
import uuid
from typing import Any

from cynic.interfaces.bots.bot_interface import BotCommand, BotInterface, BotResponse

# Try importing discord, but don't fail if not available (for testing)
try:
    import discord
except ImportError:
    discord = None


logger = logging.getLogger(__name__)


class DiscordAdapter(BotInterface):
    """
    Discord adapter implementing unified BotInterface.

    Bridges Discord interactions to CYNIC consciousness system. Translates
    Discord events ↔ BotCommand/BotResponse and handles platform-specific
    formatting (embeds, buttons, views).

    Attributes:
        client: discord.Client instance
        conscious_state: UnifiedConsciousState instance (CYNIC consciousness)
    """

    def __init__(self, client: Any, conscious_state: Any):
        """
        Initialize Discord adapter.

        Args:
            client: discord.Client or discord.Bot instance
            conscious_state: UnifiedConsciousState or OrganismState
        """
        self.client = client
        self.conscious_state = conscious_state

        logger.debug(
            f"DiscordAdapter initialized with client={type(client).__name__}, "
            f"conscious_state={type(conscious_state).__name__}"
        )

    async def start(self) -> None:
        """
        Start the Discord adapter.

        Called during bot initialization. In practice, the discord.Client
        is started separately, but this method provides the BotInterface contract.

        In a full implementation, this would:
        - Set up command handlers
        - Register persistent views
        - Start background tasks
        """
        logger.info("Discord adapter starting")
        # Discord client start is handled by bot main loop
        # This method exists to satisfy BotInterface contract

    async def stop(self) -> None:
        """
        Stop the Discord adapter cleanly.

        Called during shutdown. Cleans up resources and closes client connection.

        In practice, shutdown is handled by discord.Client, but this method
        provides the BotInterface contract.
        """
        logger.info("Discord adapter stopping")
        # Cleanup handled by discord.Client shutdown
        # This method exists to satisfy BotInterface contract

    async def handle_command(self, command: BotCommand) -> BotResponse:
        """
        Handle a command and return response.

        Routes command by name to appropriate handler. All handlers return
        BotResponse objects. Exceptions are caught and returned as failure responses.

        Args:
            command: BotCommand with name, args, user_id, platform

        Returns:
            BotResponse with success, message, data, ephemeral flags

        Raises:
            ValueError: If command.name is invalid (optional, may return failure instead)
        """
        if not isinstance(command, BotCommand):
            return BotResponse(
                success=False,
                message="Invalid command: not a BotCommand",
                error="Type mismatch",
                ephemeral=True,
            )

        try:
            # Route by command name
            handlers = {
                "propose": self._handle_propose,
                "proposals": self._handle_proposals,
                "vote": self._handle_vote,
                "judgment_status": self._handle_status,
            }

            handler = handlers.get(command.name)

            if handler is None:
                logger.warning(f"Unknown command: {command.name}")
                return BotResponse(
                    success=False,
                    message=f"Unknown command: {command.name}",
                    ephemeral=True,
                )

            # Call handler
            response = await handler(command)
            logger.debug(
                f"Command '{command.name}' handled: success={response.success}"
            )
            return response

        except Exception as e:
            logger.error(f"Error handling command '{command.name}': {e}", exc_info=True)
            return BotResponse(
                success=False,
                message="Command processing failed",
                error=str(e),
                ephemeral=True,
            )

    # ════════════════════════════════════════════════════════════════════════════
    # COMMAND HANDLERS
    # ════════════════════════════════════════════════════════════════════════════

    async def _handle_propose(self, command: BotCommand) -> BotResponse:
        """
        Handle /propose command.

        Creates a new proposal and optionally runs it through CYNIC judgment.

        Expected args:
        - title: str (required)
        - description: str (optional)
        - category: str (optional, e.g., "general", "treasury", "governance")

        Returns:
            BotResponse with proposal_id and status in data
        """
        try:
            title = command.args.get("title", "").strip()
            description = command.args.get("description", "").strip()
            category = command.args.get("category", "general")

            # Validate required fields
            if not title:
                return BotResponse(
                    success=False,
                    message="Proposal title is required",
                    ephemeral=True,
                )

            # Create proposal ID
            proposal_id = f"prop-{uuid.uuid4().hex[:8]}"

            # In a full implementation, would:
            # 1. Store proposal in database
            # 2. Run through CYNIC judgment
            # 3. Return verdict and Q-score

            logger.info(f"Proposal created: {proposal_id} by user {command.user_id}")

            return BotResponse(
                success=True,
                message=f"Proposal created: **{title}**",
                data={
                    "proposal_id": proposal_id,
                    "title": title,
                    "description": description,
                    "category": category,
                    "creator": command.user_id,
                    "status": "pending",
                },
            )

        except Exception as e:
            logger.error(f"Error in _handle_propose: {e}", exc_info=True)
            return BotResponse(
                success=False,
                message="Failed to create proposal",
                error=str(e),
                ephemeral=True,
            )

    async def _handle_proposals(self, command: BotCommand) -> BotResponse:
        """
        Handle /proposals command.

        Returns paginated list of proposals.

        Expected args:
        - page: int (optional, default 1)
        - limit: int (optional, default 5)

        Returns:
            BotResponse with proposals list and pagination info in data
        """
        try:
            page = max(1, command.args.get("page", 1))
            limit = command.args.get("limit", 5)

            # In a full implementation, would:
            # 1. Query proposals from database
            # 2. Apply pagination
            # 3. Return paginated results

            proposals = []  # Would be fetched from DB
            total_pages = (len(proposals) + limit - 1) // limit if limit > 0 else 1

            logger.info(f"Proposals list requested by user {command.user_id}, page={page}")

            return BotResponse(
                success=True,
                message=f"Proposals (page {page}/{total_pages})",
                data={
                    "proposals": proposals,
                    "page": page,
                    "limit": limit,
                    "total": len(proposals),
                    "total_pages": total_pages,
                },
            )

        except Exception as e:
            logger.error(f"Error in _handle_proposals: {e}", exc_info=True)
            return BotResponse(
                success=False,
                message="Failed to fetch proposals",
                error=str(e),
                ephemeral=True,
            )

    async def _handle_vote(self, command: BotCommand) -> BotResponse:
        """
        Handle /vote command.

        Records a vote on a proposal.

        Expected args:
        - proposal_id: str (required)
        - vote: str (required, one of: "yes", "no", "abstain")

        Returns:
            BotResponse with vote recorded confirmation in data
        """
        try:
            proposal_id = command.args.get("proposal_id", "").strip()
            vote = command.args.get("vote", "").strip().lower()

            # Validate required fields
            if not proposal_id:
                return BotResponse(
                    success=False,
                    message="Proposal ID is required",
                    ephemeral=True,
                )

            if vote not in {"yes", "no", "abstain"}:
                return BotResponse(
                    success=False,
                    message="Vote must be 'yes', 'no', or 'abstain'",
                    ephemeral=True,
                )

            # In a full implementation, would:
            # 1. Validate proposal exists
            # 2. Check voting is active
            # 3. Check user hasn't already voted (or allow changes)
            # 4. Record vote in database
            # 5. Emit voting event

            logger.info(
                f"Vote recorded: proposal={proposal_id}, user={command.user_id}, "
                f"vote={vote}"
            )

            return BotResponse(
                success=True,
                message=f"Your vote of **{vote}** has been recorded",
                data={
                    "proposal_id": proposal_id,
                    "user_id": command.user_id,
                    "vote": vote,
                    "status": "recorded",
                },
                ephemeral=True,  # Voting responses are ephemeral
            )

        except Exception as e:
            logger.error(f"Error in _handle_vote: {e}", exc_info=True)
            return BotResponse(
                success=False,
                message="Failed to record vote",
                error=str(e),
                ephemeral=True,
            )

    async def _handle_status(self, command: BotCommand) -> BotResponse:
        """
        Handle /judgment_status command.

        Returns status of CYNIC judgment system.

        Expected args: none

        Returns:
            BotResponse with judge status, total judgments, consensus in data
        """
        try:
            # Query consciousness state
            total_judgments = self.conscious_state.total_judgments
            recent_judgments = self.conscious_state.get_recent_judgments(5)
            consensus_score = self.conscious_state.get_consensus_score()

            logger.info(f"Status requested by user {command.user_id}")

            return BotResponse(
                success=True,
                message="CYNIC Judgment System Status",
                data={
                    "total_judgments": total_judgments,
                    "recent_judgments": len(recent_judgments),
                    "consensus_score": consensus_score,
                    "dog_agreement_scores": self.conscious_state.dog_agreement_scores,
                },
            )

        except Exception as e:
            logger.error(f"Error in _handle_status: {e}", exc_info=True)
            return BotResponse(
                success=False,
                message="Failed to fetch status",
                error=str(e),
                ephemeral=True,
            )
