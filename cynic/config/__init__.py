"""CYNIC unified configuration system.

Single source of truth for all application settings.
Consolidates scattered configuration into environment-based Config dataclass.

Usage:
    from cynic.config import get_config
    config = get_config()
    print(config.discord_token)
    print(config.num_dogs)
"""
from __future__ import annotations

from .config import Config, get_config, reset_config

__all__ = ["Config", "get_config", "reset_config"]
