"""
Tests for CYNIC unified configuration system.

Tests validate:
- Configuration loading from environment variables
- Singleton pattern for get_config()
- Default values when environment variables are missing
- Input validation for all configuration options
- Immutability (frozen dataclass)
"""
from __future__ import annotations

import os
import pytest

from cynic.kernel.config import Config, get_config, reset_config


class TestConfigLoading:
    """Test configuration loading from environment variables."""

    def test_config_loads_from_environment(self, monkeypatch):
        """Config should load values from environment variables."""
        monkeypatch.setenv("CYNIC_DISCORD_TOKEN", "test-token-123")
        monkeypatch.setenv("CYNIC_NUM_DOGS", "7")
        monkeypatch.setenv("CYNIC_LEARNING_RATE", "0.2")

        # Create config directly (not singleton for this test)
        config = Config(
            discord_token="test-token-123",
            num_dogs=7,
            learning_rate=0.2,
        )

        assert config.discord_token == "test-token-123"
        assert config.num_dogs == 7
        assert config.learning_rate == 0.2

    def test_config_defaults(self):
        """Config should have sensible defaults for optional values."""
        config = Config(discord_token="test-token")

        # Check critical defaults
        assert config.database_url == "sqlite:///cynic.db"
        assert config.max_judgments_batch == 10
        assert config.judgment_timeout_seconds == 30.0
        assert config.num_dogs == 11
        assert config.learning_rate == 0.1
        assert config.discount_factor == 0.99
        assert config.environment == "development"
        assert config.log_level == "INFO"
        assert config.debug is False
        assert config.consensus_timeout_ms == 5000
        assert config.e_score_decay == 0.95
        assert config.judgment_buffer_max == 89

    def test_config_optional_fields(self):
        """Optional fields should be None when not provided."""
        config = Config(discord_token="test-token")

        assert config.discord_guild_id is None
        assert config.telegram_token is None
        assert config.log_file is None


class TestConfigSingleton:
    """Test get_config() singleton pattern."""

    def test_config_singleton_returns_same_instance(self, monkeypatch):
        """get_config() should return the same instance on repeated calls."""
        reset_config()
        monkeypatch.setenv("CYNIC_DISCORD_TOKEN", "test-token")

        config1 = get_config()
        config2 = get_config()

        assert config1 is config2

    def test_singleton_loads_from_environment(self, monkeypatch):
        """Singleton should load configuration from environment variables."""
        reset_config()
        monkeypatch.setenv("CYNIC_DISCORD_TOKEN", "singleton-token")
        monkeypatch.setenv("CYNIC_ENVIRONMENT", "production")
        monkeypatch.setenv("CYNIC_DEBUG", "true")

        config = get_config()

        assert config.discord_token == "singleton-token"
        assert config.environment == "production"
        assert config.debug is True

    def test_singleton_caches_on_first_call(self, monkeypatch):
        """Singleton should cache after first call, ignoring env changes."""
        reset_config()
        monkeypatch.setenv("CYNIC_DISCORD_TOKEN", "first-token")

        config1 = get_config()
        assert config1.discord_token == "first-token"

        # Change environment (should NOT affect cached config)
        monkeypatch.setenv("CYNIC_DISCORD_TOKEN", "second-token")

        config2 = get_config()
        assert config2.discord_token == "first-token"  # Still cached value


class TestConfigValidation:
    """Test configuration validation rules."""

    def test_config_num_dogs_validation_valid_range(self):
        """num_dogs must be in range [1, 11]."""
        # Valid values
        for num_dogs in [1, 5, 11]:
            config = Config(discord_token="test", num_dogs=num_dogs)
            assert config.num_dogs == num_dogs

    def test_config_num_dogs_validation_too_low(self):
        """num_dogs < 1 should raise ValueError."""
        with pytest.raises(ValueError, match="num_dogs must be"):
            Config(discord_token="test", num_dogs=0)

    def test_config_num_dogs_validation_too_high(self):
        """num_dogs > 11 should raise ValueError."""
        with pytest.raises(ValueError, match="num_dogs must be"):
            Config(discord_token="test", num_dogs=12)

    def test_config_learning_rate_validation_valid_range(self):
        """learning_rate must be in range [0, 1]."""
        # Valid values
        for lr in [0.0, 0.5, 1.0]:
            config = Config(discord_token="test", learning_rate=lr)
            assert config.learning_rate == lr

    def test_config_learning_rate_validation_too_low(self):
        """learning_rate < 0 should raise ValueError."""
        with pytest.raises(ValueError, match="learning_rate must be"):
            Config(discord_token="test", learning_rate=-0.1)

    def test_config_learning_rate_validation_too_high(self):
        """learning_rate > 1 should raise ValueError."""
        with pytest.raises(ValueError, match="learning_rate must be"):
            Config(discord_token="test", learning_rate=1.1)

    def test_config_discount_factor_validation_valid_range(self):
        """discount_factor must be in range [0, 1]."""
        # Valid values
        for df in [0.0, 0.5, 1.0]:
            config = Config(discord_token="test", discount_factor=df)
            assert config.discount_factor == df

    def test_config_discount_factor_validation_too_low(self):
        """discount_factor < 0 should raise ValueError."""
        with pytest.raises(ValueError, match="discount_factor must be"):
            Config(discord_token="test", discount_factor=-0.1)

    def test_config_discount_factor_validation_too_high(self):
        """discount_factor > 1 should raise ValueError."""
        with pytest.raises(ValueError, match="discount_factor must be"):
            Config(discord_token="test", discount_factor=1.1)

    def test_config_e_score_decay_validation_valid_range(self):
        """e_score_decay must be in range [0, 1]."""
        # Valid values
        for decay in [0.0, 0.5, 1.0]:
            config = Config(discord_token="test", e_score_decay=decay)
            assert config.e_score_decay == decay

    def test_config_e_score_decay_validation_too_low(self):
        """e_score_decay < 0 should raise ValueError."""
        with pytest.raises(ValueError, match="e_score_decay must be"):
            Config(discord_token="test", e_score_decay=-0.1)

    def test_config_e_score_decay_validation_too_high(self):
        """e_score_decay > 1 should raise ValueError."""
        with pytest.raises(ValueError, match="e_score_decay must be"):
            Config(discord_token="test", e_score_decay=1.1)

    def test_config_environment_validation_valid_values(self):
        """environment must be development, staging, or production."""
        for env in ["development", "staging", "production"]:
            config = Config(discord_token="test", environment=env)
            assert config.environment == env

    def test_config_environment_validation_invalid_value(self):
        """Invalid environment should raise ValueError."""
        with pytest.raises(ValueError, match="environment must be in"):
            Config(discord_token="test", environment="invalid_env")

    def test_config_log_level_validation_valid_values(self):
        """log_level must be valid logging level."""
        for level in ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]:
            config = Config(discord_token="test", log_level=level)
            assert config.log_level == level

    def test_config_log_level_validation_invalid_value(self):
        """Invalid log_level should raise ValueError."""
        with pytest.raises(ValueError, match="log_level must be in"):
            Config(discord_token="test", log_level="INVALID")

    def test_config_judgment_timeout_validation_must_be_positive(self):
        """judgment_timeout_seconds must be > 0."""
        with pytest.raises(ValueError, match="judgment_timeout_seconds must be > 0"):
            Config(discord_token="test", judgment_timeout_seconds=0)

        with pytest.raises(ValueError, match="judgment_timeout_seconds must be > 0"):
            Config(discord_token="test", judgment_timeout_seconds=-1)

    def test_config_consensus_timeout_validation_must_be_positive(self):
        """consensus_timeout_ms must be > 0."""
        with pytest.raises(ValueError, match="consensus_timeout_ms must be > 0"):
            Config(discord_token="test", consensus_timeout_ms=0)

        with pytest.raises(ValueError, match="consensus_timeout_ms must be > 0"):
            Config(discord_token="test", consensus_timeout_ms=-100)

    def test_config_positive_timeouts_valid(self):
        """Positive timeout values should be valid."""
        config = Config(
            discord_token="test",
            judgment_timeout_seconds=30.5,
            consensus_timeout_ms=5000,
        )
        assert config.judgment_timeout_seconds == 30.5
        assert config.consensus_timeout_ms == 5000


class TestConfigImmutability:
    """Test that Config is frozen (immutable)."""

    def test_config_frozen_cannot_modify(self):
        """Config should be frozen and prevent modifications."""
        config = Config(discord_token="test")

        with pytest.raises(AttributeError):
            config.discord_token = "new-token"

        with pytest.raises(AttributeError):
            config.num_dogs = 5

    def test_config_frozen_on_creation(self):
        """Config should be immutable immediately after creation."""
        config = Config(discord_token="test", num_dogs=7)

        # Should not be able to add attributes
        with pytest.raises(AttributeError):
            config.new_attribute = "value"


class TestConfigEnvironmentVariables:
    """Test environment variable parsing and type conversion."""

    def test_parse_integer_environment_variables(self, monkeypatch):
        """Integer env vars should be parsed correctly."""
        monkeypatch.setenv("CYNIC_NUM_DOGS", "5")
        monkeypatch.setenv("CYNIC_MAX_JUDGMENTS_BATCH", "20")
        monkeypatch.setenv("CYNIC_CONSENSUS_TIMEOUT_MS", "10000")

        reset_config()
        config = get_config()

        assert config.num_dogs == 5
        assert config.max_judgments_batch == 20
        assert config.consensus_timeout_ms == 10000

    def test_parse_float_environment_variables(self, monkeypatch):
        """Float env vars should be parsed correctly."""
        monkeypatch.setenv("CYNIC_LEARNING_RATE", "0.25")
        monkeypatch.setenv("CYNIC_DISCOUNT_FACTOR", "0.95")
        monkeypatch.setenv("CYNIC_JUDGMENT_TIMEOUT_SECONDS", "45.5")

        reset_config()
        config = get_config()

        assert config.learning_rate == 0.25
        assert config.discount_factor == 0.95
        assert config.judgment_timeout_seconds == 45.5

    def test_parse_boolean_environment_variable(self, monkeypatch):
        """Boolean env var should parse "true"/"false" correctly."""
        monkeypatch.setenv("CYNIC_DEBUG", "true")
        reset_config()
        config = get_config()
        assert config.debug is True

        monkeypatch.setenv("CYNIC_DEBUG", "false")
        reset_config()
        config = get_config()
        assert config.debug is False

        monkeypatch.setenv("CYNIC_DEBUG", "True")
        reset_config()
        config = get_config()
        assert config.debug is True

        monkeypatch.setenv("CYNIC_DEBUG", "FALSE")
        reset_config()
        config = get_config()
        assert config.debug is False

    def test_missing_optional_environment_variables(self, monkeypatch):
        """Missing optional env vars should use defaults or None."""
        monkeypatch.delenv("CYNIC_TELEGRAM_TOKEN", raising=False)
        monkeypatch.delenv("CYNIC_DISCORD_GUILD_ID", raising=False)
        monkeypatch.delenv("CYNIC_LOG_FILE", raising=False)

        reset_config()
        config = get_config()

        assert config.telegram_token is None
        assert config.discord_guild_id is None
        assert config.log_file is None
