"""Configuration loader — YAML for tuning, env vars for secrets."""
import logging
import os
from dataclasses import dataclass, field
from pathlib import Path

import yaml

logger = logging.getLogger("telegram.config")

_DEFAULT_ORGAN_DIR = Path.home() / ".cynic" / "organs" / "telegram"


@dataclass
class RetentionConfig:
    """Retention policy — organism-level convention."""

    raw_days: int = 90
    media_days: int = 30


@dataclass
class TelegramConfig:
    """Telegram organ configuration."""

    # From YAML (non-secret, tunable)
    session_path: str = str(_DEFAULT_ORGAN_DIR / "session.session")
    db_path: str = str(_DEFAULT_ORGAN_DIR / "messages.db")
    media_dir: str = str(_DEFAULT_ORGAN_DIR / "media")
    buffer_window_seconds: int = 60
    heartbeat_interval_seconds: int = 300
    retention: RetentionConfig = field(default_factory=RetentionConfig)

    # From env vars (secrets)
    kernel_url: str = ""
    api_key: str = ""
    telegram_api_id: int = 0
    telegram_api_hash: str = ""


def ensure_runtime_dirs(cfg: TelegramConfig) -> None:
    """Create local runtime directories needed by Telethon, SQLite, and media."""
    for path in (cfg.session_path, cfg.db_path):
        Path(path).expanduser().parent.mkdir(parents=True, exist_ok=True)
    Path(cfg.media_dir).expanduser().mkdir(parents=True, exist_ok=True)


def validate_telegram_credentials(cfg: TelegramConfig) -> None:
    """Fail early with a clear error when Telegram credentials are absent."""
    if cfg.telegram_api_id <= 0 or not cfg.telegram_api_hash:
        raise ValueError(
            "TELEGRAM_API_ID and TELEGRAM_API_HASH must be set in the runtime environment"
        )


def load_config(yaml_path: str) -> TelegramConfig:
    """Load config from YAML file (if exists) + env vars for secrets."""
    cfg = TelegramConfig()

    p = Path(yaml_path)
    if p.exists():
        with p.open() as f:
            data = yaml.safe_load(f) or {}
        for key in ("session_path", "db_path", "media_dir",
                     "buffer_window_seconds", "heartbeat_interval_seconds"):
            if key in data:
                val = data[key]
                if key in ("session_path", "db_path", "media_dir") and isinstance(val, str):
                    val = str(Path(val).expanduser())
                setattr(cfg, key, val)
        if "retention" in data and isinstance(data["retention"], dict):
            cfg.retention = RetentionConfig(
                raw_days=data["retention"].get("raw_days", 90),
                media_days=data["retention"].get("media_days", 30),
            )
        logger.info("config loaded from %s", yaml_path)
    else:
        logger.info("no config file at %s, using defaults", yaml_path)

    cfg.kernel_url = os.environ.get("CYNIC_REST_ADDR", "")
    cfg.api_key = os.environ.get("CYNIC_API_KEY", "")
    api_id = os.environ.get("TELEGRAM_API_ID", "0")
    cfg.telegram_api_id = int(api_id) if api_id.isdigit() else 0
    cfg.telegram_api_hash = os.environ.get("TELEGRAM_API_HASH", "")

    return cfg
