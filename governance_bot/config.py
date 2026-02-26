"""
Governance Bot Configuration Management

Provides:
- Validated configuration with pydantic
- Environment-specific settings (testnet/mainnet)
- Configuration validation on startup
- Type-safe access to all settings
- Secrets management support
"""

import os
import logging
from pathlib import Path
from typing import Optional, Literal

try:
    from pydantic_settings import BaseSettings
except ImportError:
    from pydantic import BaseSettings

from pydantic import Field, validator, AnyUrl
from dotenv import load_dotenv

# Load .env file if it exists
env_file = Path(__file__).parent.parent / ".env"
if env_file.exists():
    load_dotenv(env_file)

logger = logging.getLogger(__name__)


class DiscordSettings(BaseSettings):
    """Discord Bot Configuration"""

    token: str = Field(..., description="Discord bot token (required)")
    prefix: str = Field(default="/", description="Command prefix")

    class Config:
        env_prefix = "DISCORD_"


class CYNICSettings(BaseSettings):
    """CYNIC Integration Configuration"""

    url: AnyUrl = Field(
        default="http://127.0.0.1:8765",
        description="CYNIC organism URL"
    )
    mcp_enabled: bool = Field(
        default=True,
        description="Enable CYNIC MCP integration"
    )
    mcp_url: AnyUrl = Field(
        default="http://127.0.0.1:8766",
        description="CYNIC MCP server URL"
    )
    timeout_seconds: int = Field(
        default=30,
        description="Timeout for CYNIC requests",
        ge=1,
        le=300
    )
    retry_count: int = Field(
        default=3,
        description="Number of retries for failed CYNIC calls",
        ge=0,
        le=10
    )

    class Config:
        env_prefix = "CYNIC_"


class DatabaseSettings(BaseSettings):
    """Database Configuration"""

    url: str = Field(
        default="sqlite:///governance_bot.db",
        description="Database URL"
    )
    pool_size: int = Field(
        default=5,
        description="Connection pool size",
        ge=1,
        le=100
    )
    max_overflow: int = Field(
        default=10,
        description="Max overflow connections",
        ge=0,
        le=100
    )
    pool_recycle_seconds: int = Field(
        default=3600,
        description="Recycle connections after N seconds",
        ge=60,
        le=86400
    )
    backup_enabled: bool = Field(
        default=True,
        description="Enable automatic backups"
    )
    backup_dir: str = Field(
        default="./backups",
        description="Directory for database backups"
    )

    class Config:
        env_prefix = "DATABASE_"


class CommunityDefaults(BaseSettings):
    """Default Community Governance Settings"""

    voting_period_hours: int = Field(
        default=72,
        description="Default voting period",
        ge=1,
        le=720
    )
    execution_delay_hours: int = Field(
        default=24,
        description="Delay before executing proposals",
        ge=0,
        le=720
    )
    quorum_percentage: int = Field(
        default=25,
        description="Minimum quorum percentage",
        ge=1,
        le=100
    )
    approval_threshold_percentage: int = Field(
        default=50,
        description="Approval threshold percentage",
        ge=1,
        le=100
    )
    proposal_submission_fee: float = Field(
        default=100,
        description="Fee to submit proposal",
        ge=0
    )
    min_dogs_consensus: int = Field(
        default=6,
        description="Minimum Dogs for consensus",
        ge=1,
        le=11
    )

    class Config:
        env_prefix = "COMMUNITY_"


class Features(BaseSettings):
    """Feature Flags"""

    gasdf_enabled: bool = Field(
        default=True,
        description="Enable GASdf integration"
    )
    near_execution_enabled: bool = Field(
        default=False,
        description="Enable NEAR transaction execution"
    )
    learning_loop_enabled: bool = Field(
        default=True,
        description="Enable CYNIC learning loop"
    )
    agent_members_enabled: bool = Field(
        default=False,
        description="Enable agent member participation"
    )

    class Config:
        env_prefix = "FEATURES_"


class LoggingSettings(BaseSettings):
    """Logging Configuration"""

    level: Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] = Field(
        default="INFO",
        description="Logging level"
    )
    file: str = Field(
        default="governance_bot.log",
        description="Log file path"
    )
    file_enabled: bool = Field(
        default=True,
        description="Enable file logging"
    )
    max_file_size_mb: int = Field(
        default=10,
        description="Max log file size in MB",
        ge=1
    )
    backup_count: int = Field(
        default=5,
        description="Number of backup log files",
        ge=1
    )

    class Config:
        env_prefix = "LOGGING_"


class BotSettings(BaseSettings):
    """Bot Runtime Configuration"""

    activity: str = Field(
        default="governance decisions",
        description="Bot activity status"
    )
    status: Literal["ACTIVE", "IDLE", "DND", "INVISIBLE"] = Field(
        default="ACTIVE",
        description="Bot status"
    )
    environment: Literal["development", "staging", "production"] = Field(
        default="development",
        description="Deployment environment"
    )

    class Config:
        env_prefix = "BOT_"


class NearSettings(BaseSettings):
    """NEAR blockchain settings"""

    account_id: str = Field(
        default="",
        description="NEAR account ID (e.g., governance.testnet)"
    )
    network: str = Field(
        default="testnet",
        description="NEAR network (testnet or mainnet)"
    )
    rpc_url: str = Field(
        default="https://rpc.testnet.near.org",
        description="NEAR RPC endpoint"
    )
    contract_id: str = Field(
        default="",
        description="Governance contract ID"
    )
    private_key: str = Field(
        default="",
        description="ed25519 private key (base64 encoded)"
    )

    class Config:
        env_prefix = "NEAR_"
        extra = "ignore"


class AuthSettings(BaseSettings):
    """Authentication and authorization settings"""

    jwt_secret: str = Field(
        default="",
        description="JWT secret key for signing tokens"
    )
    jwt_algorithm: str = Field(
        default="HS256",
        description="JWT algorithm for token signing"
    )
    jwt_expiry_hours: int = Field(
        default=24,
        description="JWT token expiry time in hours",
        ge=1,
        le=720
    )
    enable_auth: bool = Field(
        default=False,
        description="Enable JWT authentication"
    )

    class Config:
        env_prefix = "AUTH_"
        extra = "ignore"


class Config(BaseSettings):
    """
    Master Configuration Class

    Provides validated, type-safe access to all bot settings.
    Supports environment-specific configuration via BOT_ENVIRONMENT.

    Usage:
        from config import config

        # Access settings
        token = config.discord.token
        db_url = config.database.url

        # Validate configuration
        config.validate_critical_settings()
    """

    # Sub-configurations
    discord: DiscordSettings = Field(default_factory=DiscordSettings)
    cynic: CYNICSettings = Field(default_factory=CYNICSettings)
    database: DatabaseSettings = Field(default_factory=DatabaseSettings)
    community: CommunityDefaults = Field(default_factory=CommunityDefaults)
    features: Features = Field(default_factory=Features)
    logging: LoggingSettings = Field(default_factory=LoggingSettings)
    bot: BotSettings = Field(default_factory=BotSettings)
    near: NearSettings = Field(default_factory=NearSettings)
    auth: AuthSettings = Field(default_factory=AuthSettings)

    class Config:
        env_file = ".env"
        case_sensitive = False
        extra = "ignore"  # Ignore extra environment variables

    def validate_critical_settings(self) -> dict:
        """
        Validate that all critical settings are present and valid.

        Returns:
            Dictionary with validation results

        Raises:
            ValueError: If critical settings are missing
        """
        issues = []

        # Check Discord token
        if not self.discord.token or self.discord.token == "YOUR_DISCORD_TOKEN_HERE":
            issues.append("DISCORD_TOKEN is not configured (required)")

        # Check database URL
        if not self.database.url:
            issues.append("DATABASE_URL is not configured (required)")

        # Check CYNIC settings if enabled
        if self.cynic.mcp_enabled:
            if not self.cynic.url:
                issues.append("CYNIC_URL is not configured (required when MCP enabled)")
            if not self.cynic.mcp_url:
                issues.append("CYNIC_MCP_URL is not configured (required when MCP enabled)")

        # Check community defaults
        if self.community.quorum_percentage > self.community.approval_threshold_percentage:
            issues.append(
                f"Quorum ({self.community.quorum_percentage}%) > "
                f"Approval Threshold ({self.community.approval_threshold_percentage}%)"
            )

        if issues:
            error_msg = "Configuration validation failed:\n" + "\n".join(f"  - {issue}" for issue in issues)
            raise ValueError(error_msg)

        return {"status": "VALID", "issues": 0}

    def validate_near_config(self) -> bool:
        """Validate NEAR configuration if contract is enabled"""
        if not self.features.near_execution_enabled:
            return True

        if not self.near.account_id:
            raise ValueError("NEAR_ACCOUNT_ID required when NEAR execution enabled")
        if not self.near.private_key:
            raise ValueError("NEAR_PRIVATE_KEY required when NEAR execution enabled")
        if not self.near.contract_id:
            raise ValueError("NEAR_CONTRACT_ID required when NEAR execution enabled")

        return True

    def get_environment_info(self) -> dict:
        """Get current environment configuration"""
        return {
            "environment": self.bot.environment,
            "discord_prefix": self.discord.prefix,
            "cynic_enabled": self.cynic.mcp_enabled,
            "features": {
                "gasdf": self.features.gasdf_enabled,
                "near_execution": self.features.near_execution_enabled,
                "learning_loop": self.features.learning_loop_enabled,
                "agent_members": self.features.agent_members_enabled,
            },
            "database": {
                "type": "sqlite" if "sqlite" in self.database.url else "postgresql" if "postgres" in self.database.url else "unknown",
                "pool_size": self.database.pool_size,
            },
            "logging": {
                "level": self.logging.level,
                "file_enabled": self.logging.file_enabled,
            },
        }

    def is_production(self) -> bool:
        """Check if running in production"""
        return self.bot.environment == "production"

    def is_testnet(self) -> bool:
        """Check if configured for testnet"""
        return "testnet" in self.database.url.lower() or self.bot.environment == "staging"

    def log_configuration(self) -> None:
        """Log configuration summary (without secrets)"""
        logger.info("=== Configuration Summary ===")
        logger.info(f"Environment: {self.bot.environment}")
        logger.info(f"Logging Level: {self.logging.level}")
        logger.info(f"Database: {self.database.url.split('/')[-1]}")
        logger.info(f"CYNIC MCP: {'Enabled' if self.cynic.mcp_enabled else 'Disabled'}")
        logger.info(f"NEAR Execution: {'Enabled' if self.features.near_execution_enabled else 'Disabled'}")
        logger.info(f"Learning Loop: {'Enabled' if self.features.learning_loop_enabled else 'Disabled'}")
        logger.info(f"Agent Members: {'Enabled' if self.features.agent_members_enabled else 'Disabled'}")
        logger.info("=" * 30)


# Create global config instance
try:
    config = Config()
    config.validate_critical_settings()
    config.log_configuration()
except ValueError as e:
    logger.error(f"Configuration error: {e}")
    raise

# Export common settings for backward compatibility
DISCORD_TOKEN = config.discord.token
DISCORD_PREFIX = config.discord.prefix
CYNIC_URL = config.cynic.url
CYNIC_MCP_ENABLED = config.cynic.mcp_enabled
CYNIC_MCP_URL = config.cynic.mcp_url
DATABASE_URL = config.database.url
DEFAULT_VOTING_PERIOD_HOURS = config.community.voting_period_hours
DEFAULT_EXECUTION_DELAY_HOURS = config.community.execution_delay_hours
DEFAULT_QUORUM_PERCENTAGE = config.community.quorum_percentage
DEFAULT_APPROVAL_THRESHOLD_PERCENTAGE = config.community.approval_threshold_percentage
DEFAULT_PROPOSAL_SUBMISSION_FEE = config.community.proposal_submission_fee
DEFAULT_MIN_DOGS_CONSENSUS = config.community.min_dogs_consensus
ENABLE_GASDF = config.features.gasdf_enabled
ENABLE_NEAR_EXECUTION = config.features.near_execution_enabled
ENABLE_LEARNING_LOOP = config.features.learning_loop_enabled
LOG_LEVEL = config.logging.level
LOG_FILE = config.logging.file
BOT_ACTIVITY = config.bot.activity
BOT_STATUS = config.bot.status
