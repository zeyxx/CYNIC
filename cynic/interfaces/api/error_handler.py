"""
User-friendly error formatting and recovery guidance.

Converts raw Python exceptions into actionable, user-friendly messages.
No internal details, paths, or secrets exposed.

Categories:
  - DATABASE: Connection, SQL, transaction errors
  - TIMEOUT: Request timeouts, slow operations
  - VALIDATION: Input validation, field constraints
  - AUTH: Authentication, JWT, session errors
  - NETWORK: Network connectivity, unreachable hosts
  - INTERNAL: Unexpected critical errors
  - UNKNOWN: Unclassified errors
"""

import hashlib
import logging
from enum import Enum
from typing import Dict

logger = logging.getLogger(__name__)


class ErrorCategory(str, Enum):
    """Error categories for routing to friendly messages."""

    DATABASE = "database"
    TIMEOUT = "timeout"
    VALIDATION = "validation"
    AUTH = "auth"
    NETWORK = "network"
    INTERNAL = "internal"
    UNKNOWN = "unknown"


# Friendly error messages by category
ERROR_MESSAGES: Dict[ErrorCategory, Dict[str, str]] = {
    ErrorCategory.DATABASE: {
        "friendly": "Database connection issue. This is usually temporary.",
        "action": "Try again in a moment. If it persists, we'll investigate.",
    },
    ErrorCategory.TIMEOUT: {
        "friendly": "That request took too long. The system might be busy.",
        "action": "Try again, or reduce the scope of your request.",
    },
    ErrorCategory.VALIDATION: {
        "friendly": "Invalid input. Check your command parameters.",
        "action": "Review the command syntax and try again.",
    },
    ErrorCategory.AUTH: {
        "friendly": "Your session expired or you're not authenticated.",
        "action": "Please log in again or refresh the page.",
    },
    ErrorCategory.NETWORK: {
        "friendly": "Network connection issue. Check your internet.",
        "action": "Retry when your connection is stable.",
    },
    ErrorCategory.INTERNAL: {
        "friendly": "Something unexpected happened.",
        "action": "We've logged this error. If it continues, contact support.",
    },
    ErrorCategory.UNKNOWN: {
        "friendly": "An error occurred.",
        "action": "Try again or contact support if the problem persists.",
    },
}


def categorize_error(error_msg: str) -> ErrorCategory:
    """Detect error category from exception message."""
    msg_lower = error_msg.lower()

    if (
        "database" in msg_lower
        or "asyncpg" in msg_lower
        or "sql" in msg_lower
        or "table" in msg_lower
        or "constraint" in msg_lower
    ):
        return ErrorCategory.DATABASE
    elif "timeout" in msg_lower or "took too long" in msg_lower:
        return ErrorCategory.TIMEOUT
    elif "validation" in msg_lower or "invalid" in msg_lower:
        return ErrorCategory.VALIDATION
    elif (
        "auth" in msg_lower
        or "token" in msg_lower
        or "session" in msg_lower
        or "jwt" in msg_lower
    ):
        return ErrorCategory.AUTH
    elif (
        "network" in msg_lower
        or "connection" in msg_lower
        or "unreachable" in msg_lower
    ):
        return ErrorCategory.NETWORK
    elif "critical" in msg_lower or "internal" in msg_lower:
        return ErrorCategory.INTERNAL
    else:
        return ErrorCategory.UNKNOWN


def generate_error_code(error_msg: str) -> str:
    """Generate short error code for support reference."""
    hash_obj = hashlib.md5(error_msg.encode())
    hash_hex = hash_obj.hexdigest()[:4].upper()
    return f"#{hash_hex}"


def is_safe_to_show_user(error_msg: str) -> bool:
    """Check if error message is safe to show user (no secrets/paths)."""
    dangerous_patterns = [
        "password",
        "postgresql://",
        "/app/",
        "apikey",
        "secret",
        "token=",
    ]

    msg_lower = error_msg.lower()
    return not any(pattern in msg_lower for pattern in dangerous_patterns)


def format_error_for_user(error_msg: str) -> str:
    """
    Convert technical error to user-friendly message.

    Returns a message that:
      - Explains what happened in simple terms
      - Suggests an action (retry, verify input, contact support)
      - Includes error code for support reference
      - Never exposes internal details
    """

    # Categorize the error
    category = categorize_error(error_msg)
    msg = ERROR_MESSAGES[category]

    # Generate error code (for support reference)
    error_code = generate_error_code(error_msg)

    # Log the actual error for debugging
    logger.error(f"[{category.value}] {error_msg} ({error_code})")

    # Build user-friendly message
    friendly = msg["friendly"]
    action = msg["action"]

    # For validation errors, try to extract constraint info
    enhanced_friendly = friendly
    if category == ErrorCategory.VALIDATION and "must be" in error_msg:
        # Extract range like "must be 0-100"
        parts = error_msg.split("must be ")
        if len(parts) > 1:
            constraint = parts[1].split(",")[0].strip()
            enhanced_friendly = f"{friendly} (Valid range: {constraint})"

    return f"{enhanced_friendly}\n\nℹ️ {action}\n\nError code: {error_code}"
