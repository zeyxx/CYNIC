"""
CYNIC Configuration - Single Source of Truth

ALL environment variables read here. No os.getenv() anywhere else.
Validation catches insecure defaults and missing backends.

-Law: VERIFY - one truth, no scatter
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass
from typing import Any, Optional

logger = logging.getLogger("cynic.kernel.config")


def _opt_int(v: str | None) -> int | None:
    return int(v) if v is not None else None


@dataclass
class CynicConfig:
    # -- Storage: SurrealDB (Primary) -----------------------------------------
    surreal_url: str | None = None
    surreal_user: str = "root"
    surreal_pass: str = "local_dev_only"
    surreal_ns: str = "cynic"
    surreal_db: str = "cynic"

    # -- Network: Redis (Nervous System backbone) -----------------------------
    redis_url: str = "redis://localhost:6379/0"

    # -- Storage: PostgreSQL (fallback) ----------------------------------------
    database_url: str | None = "sqlite:///cynic.db"

    # -- Symbiosis Routing (Discovered by default) ----------------------------
    llm_primary_model: str | None = None
    llm_fast_model: str | None = None
    llm_local_model: str | None = None
    force_slow_mode: bool = False

    # -- LLM: API Keys --------------------------------------------------------
    anthropic_api_key: str | None = None
    google_api_key: str | None = None

    # -- Vault: Secret Management ---------------------------------------------
    vault_addr: str | None = None
    vault_token: str | None = None
    vault_namespace: str | None = None

    # -- LLM: Local inference (llama.cpp / Ollama) ----------------------------
    ollama_url: str = "http://localhost:11434"
    models_dir: str | None = None
    llama_gpu_layers: int = -1
    llama_threads: int = 8
    ollama_num_parallel: int | None = None
    dialogue_llm_provider: str = "auto"

    # -- Bots: Discord & Telegram ----------------------------------------------
    discord_token: str | None = None
    discord_guild_id: str | None = None
    telegram_token: str | None = None
    telegram_chat_id: str | None = None

    # -- Integrations: Web3 & Market Data --------------------------------------
    helius_api_key: str | None = None

    # -- GASdf: Governance (On-chain execution) -------------------------------
    gasdf_url: str = "http://localhost:8766"
    gasdf_enabled: bool = False

    # -- MCP Server Configuration -----------------------------------------------
    mcp_stdio_only: bool = True  # If True, only start stdio MCP (not HTTP)
    mcp_token: str | None = None

    # -- Judgment & Consensus --------------------------------------------------
    num_dogs: int = 11
    max_judgments_batch: int = 10
    judgment_timeout_seconds: float = 30.0
    judgment_buffer_max: int = 89

    # -- Learning -------------------------------------------------------------
    learning_rate: float = 0.1
    discount_factor: float = 0.99

    # -- Runtime --------------------------------------------------------------
    port: int = 8765  # Main API port
    log_level: str = "INFO"
    log_file: str | None = None
    debug: bool = False
    knet_host: str = "::"
    knet_port: int = 0  # 0 = OS selects free port (Standard SRE)
    environment: str = "development"

    # -- Scoring & Decay ------------------------------------------------------
    e_score_decay: float = 0.95

    # -- Thresholds -----------------------------------------------------------
    max_confidence: float = 0.618
    residual_threshold: float = 0.382

    # -- Consensus ------------------------------------------------------------
    consensus_timeout_ms: int = 5000

    def __post_init__(self) -> None:
        """Validate configuration after initialization."""
        if self.num_dogs < 1 or self.num_dogs > 11:
            raise ValueError("num_dogs must be between 1 and 11")
        if not (0 <= self.learning_rate <= 1):
            raise ValueError("learning_rate must be between 0 and 1")
        if not (0 <= self.discount_factor <= 1):
            raise ValueError("discount_factor must be between 0 and 1")
        if not (0 <= self.e_score_decay <= 1):
            raise ValueError("e_score_decay must be between 0 and 1")
        if self.judgment_timeout_seconds <= 0:
            raise ValueError("judgment_timeout_seconds must be > 0")
        if self.consensus_timeout_ms <= 0:
            raise ValueError("consensus_timeout_ms must be > 0")

        valid_envs = ("development", "test", "local", "production", "staging")
        if self.environment not in valid_envs:
            raise ValueError(f"environment must be in {valid_envs}")

        valid_log_levels = ("DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL")
        if self.log_level.upper() not in valid_log_levels:
            raise ValueError(f"log_level must be in {valid_log_levels}")

    @classmethod
    def from_env(cls) -> CynicConfig:
        """Load from environment variables with type coercion."""
        try:
            from dotenv import load_dotenv

            load_dotenv()
        except ImportError:
            pass

        default_models_dir = os.path.join(os.path.expanduser("~"), ".cynic", "models")

        # Helper for prefixed env vars
        def get_env(key: str, default: Any = None) -> Any:
            return os.getenv(f"CYNIC_{key}") or os.getenv(key) or default

        return cls(
            # Storage
            surreal_url=get_env("SURREAL_URL"),
            surreal_user=get_env("SURREAL_USER", "root"),
            surreal_pass=get_env("SURREAL_PASS", "local_dev_only"),
            surreal_ns=get_env("SURREAL_NS", "cynic"),
            surreal_db=get_env("SURREAL_DB", "cynic"),
            redis_url=get_env("REDIS_URL", "redis://localhost:6379/0"),
            database_url=get_env("DATABASE_URL", "sqlite:///cynic.db"),
            # Symbiosis Routing
            llm_primary_model=get_env("PRIMARY_MODEL"),
            llm_fast_model=get_env("FAST_MODEL"),
            llm_local_model=get_env("LOCAL_MODEL"),
            force_slow_mode=get_env("FORCE_SLOW_MODE", "0") == "1",
            # LLM Keys
            anthropic_api_key=get_env("ANTHROPIC_API_KEY"),
            google_api_key=get_env("GOOGLE_API_KEY"),
            # Vault
            vault_addr=get_env("VAULT_ADDR"),
            vault_token=get_env("VAULT_TOKEN"),
            vault_namespace=get_env("VAULT_NAMESPACE"),
            # Local Inference
            ollama_url=get_env("OLLAMA_URL", "http://localhost:11434"),
            models_dir=get_env("MODELS_DIR", default_models_dir),
            llama_gpu_layers=int(get_env("LLAMA_CPP_GPU_LAYERS", "-1")),
            llama_threads=int(get_env("LLAMA_CPP_THREADS", "8")),
            ollama_num_parallel=_opt_int(get_env("OLLAMA_NUM_PARALLEL")),
            # Dialogue LLM Provider
            dialogue_llm_provider=get_env("DIALOGUE_LLM_PROVIDER", "auto"),
            # Bots
            discord_token=get_env("DISCORD_TOKEN"),
            discord_guild_id=get_env("DISCORD_GUILD_ID"),
            telegram_token=get_env("TELEGRAM_BOT_TOKEN") or get_env("TELEGRAM_TOKEN"),
            telegram_chat_id=get_env("TELEGRAM_CHAT_ID"),
            # Integrations
            helius_api_key=get_env("HELIUS_API_KEY"),
            # GASdf
            gasdf_url=get_env("GASDF_URL", "http://localhost:8766"),
            gasdf_enabled=get_env("GASDF_ENABLED") == "1",
            # MCP Server
            mcp_stdio_only=get_env("MCP_STDIO_ONLY", "1") == "1",
            mcp_token=get_env("MCP_TOKEN"),
            # Judgment
            num_dogs=int(get_env("NUM_DOGS", "11")),
            max_judgments_batch=int(get_env("MAX_JUDGMENTS_BATCH", "10")),
            judgment_timeout_seconds=float(
                get_env(
                    "JUDGMENT_TIMEOUT_S", get_env("JUDGMENT_TIMEOUT_SECONDS", "30.0")
                )
            ),
            judgment_buffer_max=int(get_env("JUDGMENT_BUFFER_MAX", "89")),
            # Learning
            learning_rate=float(get_env("LEARNING_RATE", "0.1")),
            discount_factor=float(get_env("DISCOUNT_FACTOR", "0.99")),
            e_score_decay=float(get_env("E_SCORE_DECAY", "0.95")),
            # Runtime
            port=int(get_env("PORT", "8765")),
            log_level=get_env("LOG_LEVEL", "INFO"),
            log_file=get_env("LOG_FILE"),
            debug=get_env("DEBUG", "false").lower() == "true",
            knet_host=get_env("KNET_HOST", "::"),
            knet_port=int(get_env("KNET_PORT", "0")),
            environment=get_env("ENVIRONMENT", "development"),
            # Consensus
            consensus_timeout_ms=int(get_env("CONSENSUS_TIMEOUT_MS", "5000")),
        )

    @property
    def has_surreal(self) -> bool:
        return self.surreal_url is not None

    @property
    def has_postgres(self) -> bool:
        return self.database_url is not None

    @property
    def has_any_storage(self) -> bool:
        return self.has_surreal or self.has_postgres


# -- Singleton & Backward Compatibility ---------------------------------------
Config = CynicConfig
_GLOBAL_CONFIG: Optional[CynicConfig] = None


def get_config() -> CynicConfig:
    """Get the global configuration singleton (Rule 3: Single Source of Truth)."""
    global _GLOBAL_CONFIG
    if _GLOBAL_CONFIG is None:
        _GLOBAL_CONFIG = CynicConfig.from_env()
    return _GLOBAL_CONFIG


def reset_config() -> None:
    """Reset the global configuration singleton (forces reload on next get_config)."""
    global _GLOBAL_CONFIG
    _GLOBAL_CONFIG = None
