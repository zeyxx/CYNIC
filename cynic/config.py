"""
CYNIC Global Configuration - Unified Source of Truth.
Lentilles: Solutions Architect, SRE, Security.

Consolidates all scattered config into a single, validated entry point.
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass
from typing import Optional

logger = logging.getLogger("cynic.config")


@dataclass
class CynicConfig:
    instance_id: str = "cynic-alpha"

    # -- Storage --------------------------------------------------------------
    surreal_url: str | None = os.getenv("SURREAL_URL")
    redis_url: str = os.getenv("REDIS_URL", "redis://localhost:6379/0")
    database_url: str = os.getenv("DATABASE_URL", "sqlite:///cynic.db")

    # -- AI Infra -------------------------------------------------------------
    ollama_url: str = os.getenv("OLLAMA_URL", "http://localhost:11434")
    google_api_key: str | None = os.getenv("GOOGLE_API_KEY")
    anthropic_api_key: str | None = os.getenv("ANTHROPIC_API_KEY")

    # -- Security -------------------------------------------------------------
    vault_addr: str | None = os.getenv("VAULT_ADDR")
    vault_token: str | None = os.getenv("VAULT_TOKEN")

    # -- Thresholds -----------------------------------------------------------
    num_dogs: int = 11
    max_confidence: float = 0.618
    learning_rate: float = 0.1

    @classmethod
    def from_env(cls) -> CynicConfig:
        return cls()


_CONFIG: Optional[CynicConfig] = None


def get_config() -> CynicConfig:
    global _CONFIG
    if _CONFIG is None:
        _CONFIG = CynicConfig.from_env()
    return _CONFIG
