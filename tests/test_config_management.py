"""
Tests for Configuration Management

Verifies that:
- Configuration validation works correctly
- Environment-specific settings load properly
- Invalid configurations are rejected
- Missing required settings are detected
- Type validation works
- Configuration API provides safe access
"""

import pytest
import tempfile
from pathlib import Path
from pydantic import ValidationError

import sys
sys.path.insert(0, str(Path(__file__).parent.parent / "governance_bot"))

from config import (
    Config,
    DiscordSettings,
    CYNICSettings,
    DatabaseSettings,
    CommunityDefaults,
    Features,
    LoggingSettings,
    BotSettings,
)


class TestDiscordSettings:
    """Test Discord configuration"""

    def test_discord_token_required(self):
        """Discord token must be provided"""
        with pytest.raises(ValidationError):
            DiscordSettings(token=None)

    def test_discord_token_accepted(self):
        """Discord token is accepted when provided"""
        settings = DiscordSettings(token="valid_token_12345")
        assert settings.token == "valid_token_12345"

    def test_discord_prefix_default(self):
        """Discord prefix defaults to /"""
        settings = DiscordSettings(token="test")
        assert settings.prefix == "/"

    def test_discord_prefix_custom(self):
        """Custom Discord prefix is accepted"""
        settings = DiscordSettings(token="test", prefix="!")
        assert settings.prefix == "!"


class TestCYNICSettings:
    """Test CYNIC integration configuration"""

    def test_cynic_url_default(self):
        """CYNIC URL has sensible default"""
        settings = CYNICSettings()
        assert "127.0.0.1:8765" in str(settings.url)

    def test_cynic_mcp_enabled_default(self):
        """CYNIC MCP is enabled by default"""
        settings = CYNICSettings()
        assert settings.mcp_enabled is True

    def test_cynic_timeout_validation(self):
        """CYNIC timeout must be between 1 and 300 seconds"""
        with pytest.raises(ValidationError):
            CYNICSettings(timeout_seconds=0)

        with pytest.raises(ValidationError):
            CYNICSettings(timeout_seconds=301)

        settings = CYNICSettings(timeout_seconds=60)
        assert settings.timeout_seconds == 60

    def test_cynic_retry_validation(self):
        """CYNIC retry count must be between 0 and 10"""
        with pytest.raises(ValidationError):
            CYNICSettings(retry_count=-1)

        with pytest.raises(ValidationError):
            CYNICSettings(retry_count=11)

        settings = CYNICSettings(retry_count=5)
        assert settings.retry_count == 5


class TestDatabaseSettings:
    """Test database configuration"""

    def test_database_url_default(self):
        """Database URL has SQLite default"""
        settings = DatabaseSettings()
        assert "sqlite" in settings.url

    def test_database_pool_size_validation(self):
        """Pool size must be between 1 and 100"""
        with pytest.raises(ValidationError):
            DatabaseSettings(pool_size=0)

        with pytest.raises(ValidationError):
            DatabaseSettings(pool_size=101)

        settings = DatabaseSettings(pool_size=10)
        assert settings.pool_size == 10

    def test_database_max_overflow_validation(self):
        """Max overflow must be between 0 and 100"""
        with pytest.raises(ValidationError):
            DatabaseSettings(max_overflow=-1)

        with pytest.raises(ValidationError):
            DatabaseSettings(max_overflow=101)

        settings = DatabaseSettings(max_overflow=20)
        assert settings.max_overflow == 20

    def test_database_backup_enabled_default(self):
        """Database backup is enabled by default"""
        settings = DatabaseSettings()
        assert settings.backup_enabled is True


class TestCommunityDefaults:
    """Test community governance defaults"""

    def test_voting_period_default(self):
        """Voting period defaults to 72 hours"""
        settings = CommunityDefaults()
        assert settings.voting_period_hours == 72

    def test_voting_period_validation(self):
        """Voting period must be 1-720 hours"""
        with pytest.raises(ValidationError):
            CommunityDefaults(voting_period_hours=0)

        with pytest.raises(ValidationError):
            CommunityDefaults(voting_period_hours=721)

        settings = CommunityDefaults(voting_period_hours=48)
        assert settings.voting_period_hours == 48

    def test_quorum_validation(self):
        """Quorum percentage must be 1-100"""
        with pytest.raises(ValidationError):
            CommunityDefaults(quorum_percentage=0)

        with pytest.raises(ValidationError):
            CommunityDefaults(quorum_percentage=101)

    def test_approval_threshold_validation(self):
        """Approval threshold must be 1-100"""
        with pytest.raises(ValidationError):
            CommunityDefaults(approval_threshold_percentage=0)

        with pytest.raises(ValidationError):
            CommunityDefaults(approval_threshold_percentage=101)

    def test_min_dogs_validation(self):
        """Min dogs must be 1-11"""
        with pytest.raises(ValidationError):
            CommunityDefaults(min_dogs_consensus=0)

        with pytest.raises(ValidationError):
            CommunityDefaults(min_dogs_consensus=12)


class TestFeatures:
    """Test feature flag configuration"""

    def test_gasdf_default(self):
        """GASdf is enabled by default"""
        settings = Features()
        assert settings.gasdf_enabled is True

    def test_near_execution_default(self):
        """NEAR execution is disabled by default"""
        settings = Features()
        assert settings.near_execution_enabled is False

    def test_learning_loop_default(self):
        """Learning loop is enabled by default"""
        settings = Features()
        assert settings.learning_loop_enabled is True

    def test_agent_members_default(self):
        """Agent members are disabled by default"""
        settings = Features()
        assert settings.agent_members_enabled is False


class TestLoggingSettings:
    """Test logging configuration"""

    def test_logging_level_default(self):
        """Logging level defaults to INFO"""
        settings = LoggingSettings()
        assert settings.level == "INFO"

    def test_logging_level_validation(self):
        """Logging level must be valid"""
        valid_levels = ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]

        for level in valid_levels:
            settings = LoggingSettings(level=level)
            assert settings.level == level

        with pytest.raises(ValidationError):
            LoggingSettings(level="INVALID")

    def test_logging_file_enabled_default(self):
        """File logging is enabled by default"""
        settings = LoggingSettings()
        assert settings.file_enabled is True


class TestBotSettings:
    """Test bot runtime configuration"""

    def test_environment_default(self):
        """Environment defaults to development"""
        settings = BotSettings()
        assert settings.environment == "development"

    def test_environment_validation(self):
        """Environment must be valid"""
        valid_envs = ["development", "staging", "production"]

        for env in valid_envs:
            settings = BotSettings(environment=env)
            assert settings.environment == env

        with pytest.raises(ValidationError):
            BotSettings(environment="invalid")

    def test_status_default(self):
        """Bot status defaults to ACTIVE"""
        settings = BotSettings()
        assert settings.status == "ACTIVE"


class TestMasterConfig:
    """Test master configuration class"""

    def test_config_initialization(self):
        """Config initializes with defaults"""
        import os

        # Set required env vars
        os.environ["DISCORD_TOKEN"] = "test_token"

        config = Config()

        assert config.discord.token == "test_token"
        assert config.database.url
        assert config.bot.environment

    def test_config_validate_critical_settings_missing_token(self):
        """Validation fails if Discord token missing"""
        import os

        # Clear token
        if "DISCORD_TOKEN" in os.environ:
            del os.environ["DISCORD_TOKEN"]

        config = Config(
            discord=DiscordSettings(token="YOUR_DISCORD_TOKEN_HERE")
        )

        with pytest.raises(ValueError) as exc_info:
            config.validate_critical_settings()

        assert "DISCORD_TOKEN" in str(exc_info.value)

    def test_config_validate_critical_settings_valid(self):
        """Validation passes with valid settings"""
        config = Config(
            discord=DiscordSettings(token="valid_token_12345"),
            database=DatabaseSettings(),
            cynic=CYNICSettings(mcp_enabled=False),
        )

        result = config.validate_critical_settings()
        assert result["status"] == "VALID"
        assert result["issues"] == 0

    def test_config_get_environment_info(self):
        """Get environment info returns expected structure"""
        config = Config(
            discord=DiscordSettings(token="test"),
            database=DatabaseSettings(),
        )

        info = config.get_environment_info()

        assert "environment" in info
        assert "discord_prefix" in info
        assert "cynic_enabled" in info
        assert "features" in info
        assert "database" in info
        assert "logging" in info

    def test_config_is_production(self):
        """is_production() returns correct value"""
        config_dev = Config(
            discord=DiscordSettings(token="test"),
            bot=BotSettings(environment="development"),
        )
        assert config_dev.is_production() is False

        config_prod = Config(
            discord=DiscordSettings(token="test"),
            bot=BotSettings(environment="production"),
        )
        assert config_prod.is_production() is True

    def test_config_is_testnet(self):
        """is_testnet() checks for testnet configuration"""
        config_dev = Config(
            discord=DiscordSettings(token="test"),
            database=DatabaseSettings(url="sqlite:///dev.db"),
            bot=BotSettings(environment="development"),
        )
        assert config_dev.is_testnet() is False

        config_testnet = Config(
            discord=DiscordSettings(token="test"),
            database=DatabaseSettings(
                url="postgresql://user:pass@localhost/testnet_db"
            ),
            bot=BotSettings(environment="staging"),
        )
        assert config_testnet.is_testnet() is True


class TestConfigurationValidation:
    """Test comprehensive configuration validation"""

    def test_quorum_less_than_approval_threshold(self):
        """Quorum should be less than or equal to approval threshold"""
        # This is a logical constraint, though not enforced in current implementation
        config = Config(
            discord=DiscordSettings(token="test"),
            community=CommunityDefaults(
                quorum_percentage=25,
                approval_threshold_percentage=50,
            ),
        )

        # Should pass - quorum < approval
        result = config.validate_critical_settings()
        assert result["status"] == "VALID"

    def test_voting_period_reasonable(self):
        """Voting period should be reasonable"""
        config = Config(
            discord=DiscordSettings(token="test"),
            community=CommunityDefaults(voting_period_hours=168),  # 1 week
        )

        result = config.validate_critical_settings()
        assert result["status"] == "VALID"


class TestConfigurationDocumentation:
    """Test that configuration is properly documented"""

    def test_config_class_has_docstring(self):
        """Config class has comprehensive docstring"""
        assert Config.__doc__ is not None
        assert "type-safe" in Config.__doc__.lower()
        assert "environment" in Config.__doc__.lower()

    def test_validate_critical_settings_documented(self):
        """validate_critical_settings method is documented"""
        assert Config.validate_critical_settings.__doc__ is not None

    def test_environment_helpers_documented(self):
        """Environment helper methods are documented"""
        assert Config.is_production.__doc__ is not None
        assert Config.is_testnet.__doc__ is not None


class TestConfigurationBackwardCompatibility:
    """Test backward compatibility with old config format"""

    def test_old_config_exports_still_available(self):
        """Old-style config exports still work"""
        # These exports should exist for backward compatibility
        from config import (
            DISCORD_TOKEN,
            DATABASE_URL,
            DEFAULT_VOTING_PERIOD_HOURS,
            ENABLE_GASDF,
        )

        assert DISCORD_TOKEN is not None  # Should exist
        assert isinstance(DISCORD_TOKEN, str)  # Should be string
        assert DATABASE_URL is not None
        assert isinstance(DEFAULT_VOTING_PERIOD_HOURS, int)
        assert isinstance(ENABLE_GASDF, bool)
