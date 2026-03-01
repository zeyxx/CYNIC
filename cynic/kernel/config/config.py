"""
CYNIC Unified Configuration System

Consolidates scattered configuration into a single source of truth.
All environment variables are read hereâ€”no os.getenv() elsewhere.

Ï†-Law: VERIFY â€” one truth, no scattered defaults.
"""

from __future__ import annotations

import os
from dataclasses import dataclass


@dataclass(frozen=True)
class Config:
    """Immutable configuration loaded from environment variables.

    Single source of truth for all application settings.
    Validates ranges and required fields on initialization.

    Usage:
        config = get_config()  # Singleton access
        print(config.num_dogs)
        print(config.discord_token)
    """

    # â”€â”€ Discord Bot Configuration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    discord_token: str
    discord_guild_id: str | None = None

    # â”€â”€ Telegram Bot Configuration (Optional) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    telegram_token: str | None = None

    # â”€â”€ Database Configuration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    database_url: str = "sqlite:///cynic.db"

    # â”€â”€ Judgment & Governance Settings â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    max_judgments_batch: int = 10
    judgment_timeout_seconds: float = 30.0
    num_dogs: int = 11

    # â”€â”€ Learning & Q-Table Settings â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    learning_rate: float = 0.1
    discount_factor: float = 0.99

    # â”€â”€ Application Settings â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    environment: str = "development"
    log_level: str = "INFO"
    log_file: str | None = None
    debug: bool = False

    # â”€â”€ Advanced Settings â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    consensus_timeout_ms: int = 5000
    e_score_decay: float = 0.95
    judgment_buffer_max: int = 89

    def __post_init__(self) -> None:
        """Validate configuration ranges and required fields.

        Raises:
            ValueError: If any configuration value is outside valid range
        """
        # Validate num_dogs in range [1, 11]
        if not (1 <= self.num_dogs <= 11):
            raise ValueError(f"num_dogs must be [1, 11], got {self.num_dogs}")

        # Validate learning_rate in [0, 1]
        if not (0.0 <= self.learning_rate <= 1.0):
            raise ValueError(f"learning_rate must be [0, 1], got {self.learning_rate}")

        # Validate discount_factor in [0, 1]
        if not (0.0 <= self.discount_factor <= 1.0):
            raise ValueError(f"discount_factor must be [0, 1], got {self.discount_factor}")

        # Validate e_score_decay in [0, 1]
        if not (0.0 <= self.e_score_decay <= 1.0):
            raise ValueError(f"e_score_decay must be [0, 1], got {self.e_score_decay}")

        # Validate environment
        valid_envs = {"development", "staging", "production"}
        if self.environment not in valid_envs:
            raise ValueError(f"environment must be in {valid_envs}, got {self.environment}")

        # Validate log_level
        valid_levels = {"DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}
        if self.log_level not in valid_levels:
            raise ValueError(f"log_level must be in {valid_levels}, got {self.log_level}")

        # Validate timeout values > 0
        if self.judgment_timeout_seconds <= 0:
            raise ValueError(
                f"judgment_timeout_seconds must be > 0, got {self.judgment_timeout_seconds}"
            )

        if self.consensus_timeout_ms <= 0:
            raise ValueError(f"consensus_timeout_ms must be > 0, got {self.consensus_timeout_ms}")


# â”€â”€ Singleton pattern for global configuration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
_GLOBAL_CONFIG: Config | None = None


def get_config() -> Config:
    """Get global configuration instance (singleton).

    Ensures single source of truth throughout application.
    Loads from environment on first call, cached thereafter.

    Returns:
        Config: Global configuration instance

    Example:
        config = get_config()
        print(config.discord_token)
        # Second call returns same instance
        assert get_config() is config
    """
    global _GLOBAL_CONFIG
    if _GLOBAL_CONFIG is None:
        _GLOBAL_CONFIG = Config(
            discord_token=os.environ.get("CYNIC_DISCORD_TOKEN", ""),
            discord_guild_id=os.environ.get("CYNIC_DISCORD_GUILD_ID"),
            telegram_token=os.environ.get("CYNIC_TELEGRAM_TOKEN"),
            database_url=os.environ.get("CYNIC_DATABASE_URL", "sqlite:///cynic.db"),
            max_judgments_batch=int(os.environ.get("CYNIC_MAX_JUDGMENTS_BATCH", "10")),
            judgment_timeout_seconds=float(os.environ.get("CYNIC_JUDGMENT_TIMEOUT_SECONDS", "30")),
            num_dogs=int(os.environ.get("CYNIC_NUM_DOGS", "11")),
            learning_rate=float(os.environ.get("CYNIC_LEARNING_RATE", "0.1")),
            discount_factor=float(os.environ.get("CYNIC_DISCOUNT_FACTOR", "0.99")),
            environment=os.environ.get("CYNIC_ENVIRONMENT", "development"),
            log_level=os.environ.get("CYNIC_LOG_LEVEL", "INFO"),
            log_file=os.environ.get("CYNIC_LOG_FILE"),
            debug=os.environ.get("CYNIC_DEBUG", "false").lower() == "true",
            consensus_timeout_ms=int(os.environ.get("CYNIC_CONSENSUS_TIMEOUT_MS", "5000")),
            e_score_decay=float(os.environ.get("CYNIC_E_SCORE_DECAY", "0.95")),
            judgment_buffer_max=int(os.environ.get("CYNIC_JUDGMENT_BUFFER_MAX", "89")),
        )
    return _GLOBAL_CONFIG


def reset_config() -> None:
    """Reset global configuration (for testing).

    This allows tests to change environment variables and reload config.
    Should NOT be used in production code.

    Example:
        monkeypatch.setenv("CYNIC_DEBUG", "true")
        reset_config()  # Will reload from env on next get_config() call
        config = get_config()
        assert config.debug is True
    """
    global _GLOBAL_CONFIG
    _GLOBAL_CONFIG = None
