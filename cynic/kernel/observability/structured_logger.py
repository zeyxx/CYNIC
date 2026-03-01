"""
Structured JSON logging for CYNIC observability.

All logs are emitted as valid JSON for machine parsing by monitoring systems.
No raw text output â€” only JSON. This enables:
  - Centralized log aggregation (ELK, Splunk, etc.)
  - Machine parsing without regex parsing
  - Structured querying by log fields
  - Correlation tracking across requests
"""

import json
import logging
from datetime import UTC, datetime
from typing import Any


class JSONFormatter(logging.Formatter):
    """Format logs as JSON for machine parsing"""

    # Standard logging record attributes to exclude from JSON (not user data)
    _STANDARD_FIELDS = {
        "name",
        "msg",
        "args",
        "created",
        "filename",
        "funcName",
        "levelname",
        "levelno",
        "lineno",
        "module",
        "msecs",
        "message",
        "pathname",
        "process",
        "processName",
        "relativeCreated",
        "thread",
        "threadName",
        "exc_info",
        "exc_text",
        "stack_info",
        "extra",
        "asctime",
    }

    def format(self, record: logging.LogRecord) -> str:
        """Convert LogRecord to JSON string"""
        log_obj = {
            "timestamp": datetime.now(UTC).isoformat(),
            "level": record.levelname,
            "logger": record.name,
            "message": record.getMessage(),
            "module": record.module,
        }

        # Add extra fields if present (from record.extra attribute)
        if hasattr(record, "extra") and isinstance(record.extra, dict):
            log_obj.update(record.extra)

        # Also include any custom attributes added via logging.extra parameter
        for key, value in record.__dict__.items():
            if key not in self._STANDARD_FIELDS and not key.startswith("_"):
                log_obj[key] = value

        return json.dumps(log_obj)


class StructuredLogger:
    """Logger that outputs JSON for observability"""

    def __init__(self, name: str):
        """Initialize logger with JSON formatter

        Args:
            name: Logger name (e.g., "cynic.interfaces.api.server")
        """
        self.logger = logging.getLogger(name)
        self.formatter = JSONFormatter()

        # Remove default handlers to avoid duplicate logs
        for handler in self.logger.handlers[:]:
            self.logger.removeHandler(handler)

        # Add JSON handler
        handler = logging.StreamHandler()
        handler.setFormatter(self.formatter)
        self.logger.addHandler(handler)
        self.logger.setLevel(logging.DEBUG)

    def _format_json(self, level: str, message: str, extra: dict[str, Any]) -> str:
        """Format log as JSON (for testing)

        Args:
            level: Log level (e.g., "INFO", "ERROR")
            message: Log message
            extra: Extra fields to include in JSON

        Returns:
            JSON-formatted log string
        """
        log_obj = {
            "timestamp": datetime.now(UTC).isoformat(),
            "level": level,
            "logger": self.logger.name,
            "message": message,
        }
        log_obj.update(extra)
        return json.dumps(log_obj)

    def info(self, message: str, extra: dict[str, Any] | None = None):
        """Log info level

        Args:
            message: Log message
            extra: Extra fields to include in JSON
        """
        self._log(logging.INFO, message, extra or {})

    def error(self, message: str, extra: dict[str, Any] | None = None):
        """Log error level

        Args:
            message: Log message
            extra: Extra fields to include in JSON
        """
        self._log(logging.ERROR, message, extra or {})

    def debug(self, message: str, extra: dict[str, Any] | None = None):
        """Log debug level

        Args:
            message: Log message
            extra: Extra fields to include in JSON
        """
        self._log(logging.DEBUG, message, extra or {})

    def warning(self, message: str, extra: dict[str, Any] | None = None):
        """Log warning level

        Args:
            message: Log message
            extra: Extra fields to include in JSON
        """
        self._log(logging.WARNING, message, extra or {})

    def _log(self, level: int, message: str, extra: dict[str, Any]):
        """Internal method to add extra fields to LogRecord

        Args:
            level: Logging level
            message: Log message
            extra: Extra fields to include
        """
        record = self.logger.makeRecord(
            self.logger.name,
            level,
            "(json)",
            0,
            message,
            (),
            None,
        )
        record.extra = extra
        self.logger.handle(record)
