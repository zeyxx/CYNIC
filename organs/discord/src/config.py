"""Configuration loader for organ-discord."""
from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path

import yaml

_DEFAULT_ORGAN_DIR = Path.home() / ".cynic" / "organs" / "discord"


@dataclass
class RetentionConfig:
    """Retention policy for locally stored Discord messages."""

    raw_days: int = 90


@dataclass
class DiscordConfig:
    """Discord organ configuration."""

    db_path: str = str(_DEFAULT_ORGAN_DIR / "messages.db")
    heartbeat_interval_seconds: int = 300
    allowed_channel_ids: list[int] = field(default_factory=list)
    capture_all_discord_metadata: bool = True
    store_raw_events: bool = False
    raw_event_allowlist: list[str] = field(default_factory=list)
    retention: RetentionConfig = field(default_factory=RetentionConfig)

    kernel_url: str = ""
    auth_key: str = ""


def _ids(value: object) -> list[int]:
    if not isinstance(value, list):
        return []
    out: list[int] = []
    for item in value:
        try:
            out.append(int(item))
        except (TypeError, ValueError):
            continue
    return out


def ensure_runtime_dirs(cfg: DiscordConfig) -> None:
    """Create local runtime directories."""
    Path(cfg.db_path).expanduser().parent.mkdir(parents=True, exist_ok=True)


def validate_allowlist(cfg: DiscordConfig) -> None:
    """Avoid accidental whole-server ingestion."""
    if not cfg.allowed_channel_ids:
        raise ValueError("organ-discord requires allowed_channel_ids in config")


def load_config(yaml_path: str) -> DiscordConfig:
    """Load YAML config and env secrets."""
    cfg = DiscordConfig()
    path = Path(yaml_path).expanduser()
    if path.exists():
        with path.open() as f:
            data = yaml.safe_load(f) or {}
        if "db_path" in data:
            cfg.db_path = str(Path(data["db_path"]).expanduser())
        if "heartbeat_interval_seconds" in data:
            cfg.heartbeat_interval_seconds = int(data["heartbeat_interval_seconds"])
        cfg.allowed_channel_ids = _ids(data.get("allowed_channel_ids", []))
        cfg.capture_all_discord_metadata = bool(
            data.get("capture_all_discord_metadata", cfg.capture_all_discord_metadata)
        )
        cfg.store_raw_events = bool(data.get("store_raw_events", cfg.store_raw_events))
        if isinstance(data.get("raw_event_allowlist"), list):
            cfg.raw_event_allowlist = [str(item) for item in data["raw_event_allowlist"]]
        if "retention" in data and isinstance(data["retention"], dict):
            cfg.retention = RetentionConfig(
                raw_days=int(data["retention"].get("raw_days", 90))
            )

    cfg.kernel_url = os.environ.get("CYNIC_REST_ADDR", "")
    if cfg.kernel_url and not cfg.kernel_url.startswith("http"):
        cfg.kernel_url = f"http://{cfg.kernel_url}"
    cfg.auth_key = os.environ.get("CYNIC_API_KEY", "")
    return cfg
