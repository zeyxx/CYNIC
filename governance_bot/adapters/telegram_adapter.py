"""
Telegram Adapter implementing unified BotInterface.

This module bridges Telegram interactions to the unified CYNIC consciousness system.
It translates Telegram message events into platform-agnostic BotCommand objects and
converts BotResponse objects back to Telegram markdown messages.

Architecture:
┌─────────────────────────────────────────────────────────┐
│          TELEGRAM ADAPTER FLOW                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Telegram Message (text with slash command)             │
│  ↓ (Telegram adapter translates)                        │
│  BotCommand(name, args, user_id, platform="telegram")  │
│  ↓ (Core logic routes)                                  │
│  UnifiedConsciousState.judge/record/query              │
│  ↓ (Core returns)                                       │
│  BotResponse(success, message, data, ephemeral)        │
│  ↓ (Telegram adapter converts)                         │
│  Telegram Markdown Message (bold, code blocks)         │
│  ↓ (Handle pagination for long messages)               │
│  Telegram API send                                      │
│                                                         │
└─────────────────────────────────────────────────────────┘

Key principles:
1. TelegramAdapter implements BotInterface contract
2. All commands route through handle_command()
3. All responses are BotResponse objects
4. Error handling is graceful (no exceptions escape)
5. Slash commands parsed from message text (/propose, /vote, etc)
6. Telegram markdown formatting (bold: *text*, code: `text`)
7. Message pagination for responses > 4000 chars
8. Async throughout
"""

from __future__ import annotations

import logging
import re
from typing import Dict, Any, Optional, Tuple, List
import uuid

from cynic.bots.bot_interface import BotInterface, BotCommand, BotResponse
from cynic.core.unified_state import UnifiedConsciousState

# Try importing telegram, but don't fail if not available (for testing)
try:
    from telegram import Bot
except ImportError:
    Bot = None


logger = logging.getLogger(__name__)

# Telegram message character limits
TELEGRAM_MAX_LENGTH = 4096
TELEGRAM_PAGINATION_BUFFER = 100  # Safety margin


class TelegramAdapter(BotInterface):
    """
    Telegram adapter implementing unified BotInterface.

    Bridges Telegram interactions to CYNIC consciousness system. Translates
    Telegram messages ↔ BotCommand/BotResponse and handles platform-specific
    formatting (markdown, message pagination).

    Attributes:
        client: Telegram Bot instance
        conscious_state: UnifiedConsciousState instance (CYNIC consciousness)
    """

    def __init__(self, client: Any, conscious_state: UnifiedConsciousState):
        """
        Initialize Telegram adapter.

        Args:
            client: Telegram Bot instance
            conscious_state: UnifiedConsciousState (CYNIC organism consciousness)

        Raises:
            TypeError: If conscious_state is not UnifiedConsciousState
        """
        if not isinstance(conscious_state, UnifiedConsciousState):
            raise TypeError(
                f"conscious_state must be UnifiedConsciousState, "
                f"got {type(conscious_state).__name__}"
            )

        self.client = client
        self.conscious_state = conscious_state

        logger.debug(
            f"TelegramAdapter initialized with client={type(client).__name__}, "
            f"conscious_state=UnifiedConsciousState"
        )

    async def start(self) -> None:
        """
        Start the Telegram adapter.

        Called during bot initialization. In practice, the Telegram Bot
        is started separately, but this method provides the BotInterface contract.

        In a full implementation, this would:
        - Set up message handlers
        - Start polling or webhook listener
        - Register command handlers
        """
        logger.info("Telegram adapter starting")
        # Telegram client start is handled by bot main loop
        # This method exists to satisfy BotInterface contract

    async def stop(self) -> None:
        """
        Stop the Telegram adapter cleanly.

        Called during shutdown. Cleans up resources and closes client connection.

        In practice, shutdown is handled by Telegram Bot, but this method
        provides the BotInterface contract.
        """
        logger.info("Telegram adapter stopping")
        # Cleanup handled by Telegram Bot shutdown
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

        # Validate platform is telegram
        if command.platform != "telegram":
            logger.warning(f"Command for wrong platform: {command.platform}")
            return BotResponse(
                success=False,
                message=f"This adapter only handles telegram platform, got {command.platform}",
                error="Platform mismatch",
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
    # MESSAGE PARSING
    # ════════════════════════════════════════════════════════════════════════════

    def _parse_telegram_message(self, text: str) -> Optional[Tuple[str, Dict[str, Any]]]:
        """
        Parse Telegram message text for slash command and arguments.

        Parses format: /command arg1 arg2 arg3
        Returns (command_name, args_dict)

        Args:
            text: Message text from Telegram

        Returns:
            Tuple of (command_name, args_dict) or None if not a valid command

        Examples:
            "/propose My Title" → ("propose", {"title": "My Title", ...})
            "/vote prop-123 yes" → ("vote", {"proposal_id": "prop-123", "vote": "yes"})
            "/proposals 2" → ("proposals", {"page": 2})
        """
        if not text or not text.startswith("/"):
            return None

        # Extract command and arguments
        parts = text.split(maxsplit=1)
        command = parts[0][1:].lower()  # Remove leading / and lowercase

        # Parse arguments based on command type
        args_text = parts[1] if len(parts) > 1 else ""
        args = self._parse_command_args(command, args_text)

        logger.debug(f"Parsed telegram message: command={command}, args={args}")
        return (command, args)

    def _parse_command_args(self, command: str, args_text: str) -> Dict[str, Any]:
        """
        Parse command-specific arguments from text.

        Args:
            command: Command name (propose, vote, proposals, judgment_status)
            args_text: Remaining text after command

        Returns:
            Dictionary of parsed arguments
        """
        args = {}

        if command == "propose":
            # Format: /propose Title Description [Category]
            # Title is first word, Description is rest, Category optional
            parts = args_text.split(maxsplit=1)
            if parts:
                args["title"] = parts[0]
                if len(parts) > 1:
                    # Split description and potential category
                    rest = parts[1].split(maxsplit=1)
                    args["description"] = rest[0]
                    if len(rest) > 1:
                        args["category"] = rest[1]
                    else:
                        args["category"] = "general"
                else:
                    args["description"] = ""
                    args["category"] = "general"

        elif command == "proposals":
            # Format: /proposals [page]
            parts = args_text.split()
            if parts:
                try:
                    args["page"] = max(1, int(parts[0]))
                except ValueError:
                    args["page"] = 1
            else:
                args["page"] = 1
            args["limit"] = 5  # Default limit

        elif command == "vote":
            # Format: /vote proposal_id yes|no|abstain
            parts = args_text.split()
            if len(parts) >= 2:
                args["proposal_id"] = parts[0]
                args["vote"] = parts[1].lower()
            elif len(parts) == 1:
                args["proposal_id"] = parts[0]
                # vote will be required later

        elif command == "judgment_status":
            # Format: /judgment_status (no args)
            pass

        return args

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
                    message="Proposal title is required\n\nUsage: /propose Title [Description] [Category]",
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
                message=f"Proposal created: *{title}*",
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
                    message="Proposal ID is required\n\nUsage: /vote proposal_id yes|no|abstain",
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
                message=f"Your vote of *{vote}* has been recorded",
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

    # ════════════════════════════════════════════════════════════════════════════
    # RESPONSE FORMATTING
    # ════════════════════════════════════════════════════════════════════════════

    def _format_response_for_telegram(self, response: BotResponse) -> str:
        """
        Convert BotResponse to Telegram markdown format.

        Converts to Telegram markdown (bold: *text*, code: `text`) and handles
        message pagination for responses > 4000 chars.

        Args:
            response: BotResponse object

        Returns:
            Formatted message string (single message or paginated)
        """
        # Build message parts
        message_parts = []

        # Status indicator
        if response.success:
            status_emoji = "✅"
        else:
            status_emoji = "❌"

        message_parts.append(f"{status_emoji} {response.message}")

        # Add data summary if present
        if response.data:
            message_parts.append("\n*Details:*")
            for key, value in response.data.items():
                if key not in {"proposals", "items", "recent_judgments"}:
                    # Format simple values
                    formatted_value = str(value)
                    if isinstance(value, bool):
                        formatted_value = "Yes" if value else "No"
                    elif isinstance(value, float):
                        formatted_value = f"{value:.2f}"
                    message_parts.append(f"  `{key}`: {formatted_value}")

        # Add error if present
        if response.error:
            message_parts.append(f"\n*Error:* `{response.error}`")

        full_message = "\n".join(message_parts)

        # Handle pagination if needed
        pages = self._paginate_message(full_message, TELEGRAM_MAX_LENGTH)

        # Return first page (in production, would send all pages)
        return pages[0] if pages else full_message

    def _paginate_message(self, text: str, max_length: int = TELEGRAM_MAX_LENGTH) -> List[str]:
        """
        Split long message into pages for Telegram.

        Telegram has a 4096 character limit per message. This method splits
        long messages intelligently, breaking on paragraph boundaries when possible.

        Args:
            text: Message text to paginate
            max_length: Maximum length per page (default 4096)

        Returns:
            List of message pages
        """
        if len(text) <= max_length:
            return [text]

        pages = []
        current_page = ""

        # Try to split on double newlines first (paragraphs)
        paragraphs = text.split("\n\n")

        for paragraph in paragraphs:
            # If single paragraph is > max_length, break it by lines
            if len(paragraph) > max_length:
                lines = paragraph.split("\n")
                for line in lines:
                    # Check if adding this line would exceed limit
                    separator = "\n" if current_page else ""
                    test_length = len(current_page) + len(separator) + len(line)

                    if test_length > max_length:
                        # Current page is full, save it and start new one
                        if current_page:
                            pages.append(current_page)
                        current_page = line
                    else:
                        # Add line to current page
                        current_page = current_page + separator + line if current_page else line
            else:
                # Try to add paragraph to current page
                separator = "\n\n" if current_page else ""
                test_length = len(current_page) + len(separator) + len(paragraph)

                if test_length <= max_length:
                    # Fits in current page
                    current_page = current_page + separator + paragraph if current_page else paragraph
                else:
                    # Doesn't fit, start new page
                    if current_page:
                        pages.append(current_page)
                    current_page = paragraph

        # Add remaining content
        if current_page:
            pages.append(current_page)

        return pages if pages else [text]
