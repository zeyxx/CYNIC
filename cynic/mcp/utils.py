"""
MCP Utilities — Logging and helpers.
"""
from __future__ import annotations

import logging
import sys
from typing import Optional


def setup_logging(
    logger_name: str = "cynic.mcp",
    level: str = "INFO",
) -> None:
    """
    Configure structured logging for MCP.

    Args:
        logger_name: Root logger name
        level: Logging level (DEBUG, INFO, WARNING, ERROR)
    """
    logger = logging.getLogger(logger_name)
    logger.setLevel(getattr(logging, level))

    # Only add handler if not already present
    if not logger.handlers:
        handler = logging.StreamHandler(sys.stdout)
        formatter = logging.Formatter(
            "[%(asctime)s] %(name)s[%(levelname)s]: %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S",
        )
        handler.setFormatter(formatter)
        logger.addHandler(handler)

    logger.info(f"Logging configured: {logger_name} at {level}")
