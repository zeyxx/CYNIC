"""
CYNIC CynicConfig Tests — Phase 2

Tests that CynicConfig correctly loads from env vars and validates.
"""
from __future__ import annotations

import os
from unittest.mock import patch

import pytest

from cynic.core.config import CynicConfig


class TestCynicConfig:
    """CynicConfig loads env vars and validates."""

    def test_defaults(self):
        """Default config should have sane values."""
        config = CynicConfig()
        assert config.surreal_url is None
        assert config.surreal_user == "root"
        assert config.surreal_pass == "local_dev_only"
        assert config.ollama_url == "http://localhost:11434"
        assert config.port == 8765
        assert config.max_confidence == 0.618

    def test_from_env_reads_surreal(self):
        """from_env() should read SURREAL_URL."""
        env = {"SURREAL_URL": "ws://db:8080/rpc", "SURREAL_PASS": "secret123"}
        with patch.dict(os.environ, env, clear=False):
            config = CynicConfig.from_env()
            assert config.surreal_url == "ws://db:8080/rpc"
            assert config.surreal_pass == "secret123"

    def test_from_env_reads_port(self):
        """from_env() should read PORT."""
        with patch.dict(os.environ, {"PORT": "9999"}, clear=False):
            config = CynicConfig.from_env()
            assert config.port == 9999

    def test_from_env_reads_llama_config(self):
        """from_env() should read LLAMA_CPP_* vars."""
        env = {
            "CYNIC_MODELS_DIR": "/models",
            "LLAMA_CPP_GPU_LAYERS": "32",
            "LLAMA_CPP_THREADS": "16",
        }
        with patch.dict(os.environ, env, clear=False):
            config = CynicConfig.from_env()
            assert config.models_dir == "/models"
            assert config.llama_gpu_layers == 32
            assert config.llama_threads == 16

    def test_from_env_database_url_fallback(self):
        """DATABASE_URL should be read if CYNIC_DATABASE_URL is not set."""
        with patch.dict(os.environ, {"DATABASE_URL": "postgres://x"}, clear=False):
            config = CynicConfig.from_env()
            assert config.database_url == "postgres://x"

    def test_from_env_cynic_database_url_priority(self):
        """CYNIC_DATABASE_URL takes priority over DATABASE_URL."""
        env = {
            "CYNIC_DATABASE_URL": "postgres://primary",
            "DATABASE_URL": "postgres://fallback",
        }
        with patch.dict(os.environ, env, clear=False):
            config = CynicConfig.from_env()
            assert config.database_url == "postgres://primary"

    def test_validate_no_storage(self):
        """Validate should warn when no storage backend is configured."""
        config = CynicConfig()
        issues = config.validate()
        assert any("No storage backend" in i for i in issues)

    def test_validate_insecure_password_on_remote(self):
        """Default password on non-localhost should be flagged."""
        config = CynicConfig(
            surreal_url="ws://production-db.example.com:8080/rpc",
            surreal_pass="local_dev_only",
        )
        issues = config.validate()
        assert any("CRITICAL" in i for i in issues)

    def test_validate_local_password_ok(self):
        """Default password on localhost is fine for development."""
        config = CynicConfig(
            surreal_url="ws://localhost:8080/rpc",
            surreal_pass="local_dev_only",
        )
        issues = config.validate()
        assert not any("CRITICAL" in i for i in issues)

    def test_has_surreal(self):
        assert CynicConfig().has_surreal is False
        assert CynicConfig(surreal_url="ws://x").has_surreal is True

    def test_has_postgres(self):
        assert CynicConfig().has_postgres is False
        assert CynicConfig(database_url="postgres://x").has_postgres is True

    def test_has_any_storage(self):
        assert CynicConfig().has_any_storage is False
        assert CynicConfig(surreal_url="ws://x").has_any_storage is True
        assert CynicConfig(database_url="pg://x").has_any_storage is True

    def test_frozen(self):
        """Config should be immutable."""
        config = CynicConfig()
        with pytest.raises(AttributeError):
            config.port = 1234  # type: ignore[misc]
