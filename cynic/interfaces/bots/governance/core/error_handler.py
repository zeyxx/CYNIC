"""
Comprehensive Error Handling for CYNIC Governance Bot

Provides:
- Centralized error logging
- Graceful error recovery
- Discord notification of errors
- Circuit breaker for CYNIC unavailability
- Automatic retry with exponential backoff
"""

import asyncio
import logging
import traceback
from collections.abc import Callable
from datetime import datetime
from functools import wraps
from typing import Any

import discord

logger = logging.getLogger(__name__)


class CircuitBreaker:
    """Circuit breaker pattern for CYNIC unavailability."""

    def __init__(self, failure_threshold: int = 5, recovery_timeout: int = 300):
        """Initialize circuit breaker.

        Args:
            failure_threshold: Number of failures before opening circuit
            recovery_timeout: Seconds before attempting to close circuit
        """
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.failure_count = 0
        self.last_failure_time = None
        self.state = "CLOSED"  # CLOSED | OPEN | HALF_OPEN

    def record_failure(self):
        """Record a failure."""
        self.failure_count += 1
        self.last_failure_time = datetime.utcnow()

        if self.failure_count >= self.failure_threshold:
            self.state = "OPEN"
            logger.error(
                f"Circuit breaker OPENED after {self.failure_count} failures. "
                f"CYNIC will be unavailable for {self.recovery_timeout}s"
            )

    def record_success(self):
        """Record a success."""
        self.failure_count = 0
        self.state = "CLOSED"

    def is_available(self) -> bool:
        """Check if circuit allows requests."""

        if self.state == "CLOSED":
            return True

        if self.state == "OPEN":
            # Check if recovery timeout elapsed
            if self.last_failure_time:
                elapsed = (datetime.utcnow() - self.last_failure_time).total_seconds()
                if elapsed > self.recovery_timeout:
                    self.state = "HALF_OPEN"
                    logger.info("Circuit breaker HALF_OPEN, attempting recovery")
                    return True
            return False

        # HALF_OPEN: allow one request to test recovery
        return True

    def get_status(self) -> str:
        """Get circuit breaker status for monitoring."""
        return f"CircuitBreaker({self.state}, failures={self.failure_count}/{self.failure_threshold})"


# Global circuit breaker for CYNIC
cynic_circuit_breaker = CircuitBreaker(failure_threshold=5, recovery_timeout=300)


class GovernanceError(Exception):
    """Base exception for governance errors."""

    def __init__(self, message: str, error_type: str = "UNKNOWN"):
        self.message = message
        self.error_type = error_type
        super().__init__(message)


class CYNICUnavailableError(GovernanceError):
    """CYNIC orchestrator is unavailable."""

    def __init__(self, message: str = "CYNIC is currently unavailable"):
        super().__init__(message, "CYNIC_UNAVAILABLE")


class DatabaseError(GovernanceError):
    """Database operation failed."""

    def __init__(self, message: str, operation: str = "unknown"):
        super().__init__(f"Database {operation} failed: {message}", "DATABASE_ERROR")


class DiscordError(GovernanceError):
    """Discord API error."""

    def __init__(self, message: str):
        super().__init__(message, "DISCORD_ERROR")


async def handle_error(
    error: Exception,
    context: str = "Unknown",
    interaction: discord.Interaction | None = None,
) -> str:
    """Handle an error gracefully.

    Args:
        error: The exception that occurred
        context: Context describing what operation failed
        interaction: Discord interaction if available (for user feedback)

    Returns:
        Error message to show user
    """

    error_type = type(error).__name__
    error_message = str(error)

    logger.error(
        f"Error in {context}: {error_type}: {error_message}\n{traceback.format_exc()}"
    )

    # Record failure for circuit breaker if CYNIC-related
    if isinstance(error, CYNICUnavailableError):
        cynic_circuit_breaker.record_failure()

    # Categorize error for user message
    if isinstance(error, CYNICUnavailableError):
        user_message = (
            " CYNIC is currently unavailable. Governance is paused. "
            "Try again in a few minutes."
        )

    elif isinstance(error, DatabaseError):
        user_message = (
            " Database error occurred. Please try again. "
            "If this persists, notify the governance administrator."
        )

    elif isinstance(error, discord.Forbidden):
        user_message = (
            " I don't have permission to perform this action. "
            "Please check my Discord permissions."
        )

    elif isinstance(error, asyncio.TimeoutError):
        user_message = (
            " Operation timed out. Please try again."
        )

    else:
        user_message = (
            f" An error occurred: {error_message[:100]}. "
            "Please try again or contact the administrator."
        )

    # Send feedback to user if interaction available
    if interaction:
        try:
            await interaction.response.send_message(user_message, ephemeral=True)
        except Exception as e:
            logger.error(f"Failed to send error message to user: {e}")

    return user_message


async def retry_with_backoff(
    func: Callable,
    *args,
    max_retries: int = 3,
    initial_delay: float = 1.0,
    backoff_factor: float = 2.0,
    **kwargs,
) -> Any:
    """Retry a function with exponential backoff.

    Args:
        func: Async function to retry
        max_retries: Maximum number of retries
        initial_delay: Initial delay in seconds
        backoff_factor: Multiplier for delay between retries
        *args: Arguments for func
        **kwargs: Keyword arguments for func

    Returns:
        Result of func call

    Raises:
        Exception: If all retries exhausted
    """

    delay = initial_delay

    for attempt in range(max_retries + 1):
        try:
            return await func(*args, **kwargs)

        except (TimeoutError, ConnectionError) as e:
            if attempt == max_retries:
                raise

            logger.warning(
                f"Attempt {attempt + 1}/{max_retries + 1} failed for {func.__name__}: {e}. "
                f"Retrying in {delay}s..."
            )

            await asyncio.sleep(delay)
            delay *= backoff_factor

        except Exception:
            # Don't retry on other exceptions
            raise


def with_error_handling(context: str = "Unknown Operation"):
    """Decorator for error handling in async command handlers.

    Usage:
        @with_error_handling("create proposal")
        async def my_command(interaction):
            ...
    """

    def decorator(func: Callable) -> Callable:
        @wraps(func)
        async def wrapper(interaction: discord.Interaction, *args, **kwargs):
            try:
                return await func(interaction, *args, **kwargs)

            except Exception as e:
                user_message = await handle_error(e, context, interaction)
                logger.error(f"Command {func.__name__} failed in {context}: {e}")

                # If response already deferred, edit it
                if interaction.response.is_done():
                    try:
                        await interaction.followup.send(user_message, ephemeral=True)
                    except Exception as send_err:
                        logger.error(f"Failed to send followup error: {send_err}")

        return wrapper

    return decorator


async def log_error_to_discord(
    bot: discord.Client,
    error_message: str,
    error_type: str = "Unknown",
    severity: str = "WARNING",  # INFO | WARNING | ERROR | CRITICAL
):
    """Log error to a Discord channel for monitoring.

    Args:
        bot: Discord bot instance
        error_message: Message to log
        error_type: Type of error
        severity: Severity level
    """

    try:
        # Find error logging channel (create if needed)
        for guild in bot.guilds:
            for channel in guild.text_channels:
                if channel.name == "governance-errors":
                    embed = discord.Embed(
                        title=f"[{severity}] {error_type}",
                        description=error_message[:2000],
                        color=discord.Color.red() if severity == "ERROR" else discord.Color.yellow(),
                        timestamp=datetime.utcnow(),
                    )
                    await channel.send(embed=embed)
                    return

    except Exception as e:
            logger.error(f"Failed to log error to Discord: {e}")


class ErrorMetrics:
    """Track error metrics for monitoring."""

    def __init__(self):
        self.total_errors = 0
        self.errors_by_type: dict[str, int] = {}
        self.last_error_time = None
        self.cynic_downtime_seconds = 0

    def record_error(self, error_type: str):
        """Record an error occurrence."""
        self.total_errors += 1
        self.errors_by_type[error_type] = self.errors_by_type.get(error_type, 0) + 1
        self.last_error_time = datetime.utcnow()

    def get_metrics(self) -> dict:
        """Get current error metrics."""
        return {
            "total_errors": self.total_errors,
            "errors_by_type": self.errors_by_type,
            "last_error": self.last_error_time,
            "circuit_breaker": cynic_circuit_breaker.get_status(),
        }


# Global error metrics
error_metrics = ErrorMetrics()


async def health_check(bot: discord.Client) -> dict:
    """Perform health check of bot and dependencies.

    Returns:
        Health status dict
    """

    health = {
        "bot_status": "OK" if bot.is_ready() else "NOT_READY",
        "discord_latency_ms": round(bot.latency * 1000),
        "circuit_breaker": cynic_circuit_breaker.get_status(),
        "error_metrics": error_metrics.get_metrics(),
        "timestamp": datetime.utcnow().isoformat(),
    }

    return health
