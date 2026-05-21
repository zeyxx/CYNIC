"""Tests for config loading — YAML + env vars."""
import tempfile
from pathlib import Path
from unittest.mock import patch

from organs.telegram.config import load_config


def test_load_config_from_yaml() -> None:
    """Config loads non-secret values from YAML file."""
    with tempfile.TemporaryDirectory() as tmp:
        cfg_path = Path(tmp) / "config.yaml"
        cfg_path.write_text(
            "session_path: /tmp/test.session\n"
            "db_path: /tmp/test.db\n"
            "media_dir: /tmp/media/\n"
            "buffer_window_seconds: 30\n"
            "heartbeat_interval_seconds: 120\n"
        )
        cfg = load_config(str(cfg_path))
        assert cfg.buffer_window_seconds == 30
        assert cfg.heartbeat_interval_seconds == 120
        assert cfg.db_path == "/tmp/test.db"


def test_load_config_defaults() -> None:
    """Config uses defaults when no YAML file exists."""
    cfg = load_config("/nonexistent/path.yaml")
    assert cfg.buffer_window_seconds == 60
    assert cfg.heartbeat_interval_seconds == 300


def test_env_vars_for_secrets() -> None:
    """Secrets come from env vars, not YAML."""
    env = {
        "CYNIC_REST_ADDR": "http://localhost:3030",
        "CYNIC_API_KEY": "test-key",
        "TELEGRAM_API_ID": "12345",
        "TELEGRAM_API_HASH": "abcdef",
    }
    with patch.dict("os.environ", env, clear=False):
        cfg = load_config("/nonexistent/path.yaml")
        assert cfg.kernel_url == "http://localhost:3030"
        assert cfg.api_key == "test-key"
        assert cfg.telegram_api_id == 12345
        assert cfg.telegram_api_hash == "abcdef"


def test_retention_defaults() -> None:
    """Retention config has defaults."""
    cfg = load_config("/nonexistent/path.yaml")
    assert cfg.retention.raw_days == 90
    assert cfg.retention.media_days == 30


def test_retention_from_yaml() -> None:
    """Retention values load from YAML."""
    with tempfile.TemporaryDirectory() as tmp:
        cfg_path = Path(tmp) / "config.yaml"
        cfg_path.write_text(
            "retention:\n"
            "  raw_days: 14\n"
            "  media_days: 7\n"
        )
        cfg = load_config(str(cfg_path))
        assert cfg.retention.raw_days == 14
        assert cfg.retention.media_days == 7
