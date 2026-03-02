"""
CYNIC Configuration — Single Source of Truth

ALL environment variables read here. No os.getenv() anywhere else.
Validation catches insecure defaults and missing backends.

φ-Law: VERIFY — one truth, no scattered defaults.
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass

logger = logging.getLogger("cynic.kernel.config")


def _opt_int(val: str | None) -> int | None:
    """Parse optional int from env var string."""
    return int(val) if val is not None else None


@dataclass(frozen=True)
class CynicConfig:
    """
    Single source of truth for ALL CYNIC configuration.

    Usage:
        config = CynicConfig.from_env()
        issues = config.validate()
        if issues:
            for issue in issues:
                logger.warning(issue)
    """

    # —— Storage: SurrealDB (primary) ──────────────────────────────────────────
    surreal_url: str | None = None
    surreal_user: str = "root"
    surreal_pass: str = "local_dev_only"
    surreal_ns: str = "cynic"
    surreal_db: str = "cynic"

    # —— Network: Redis (Nervous System backbone) ─────────────────────────────
    redis_url: str = "redis://localhost:6379/0"

    # —— Storage: PostgreSQL (fallback) ────────────────────────────────────────
    database_url: str | None = None

    # —— LLM: Multi-Model Symbiosis (Dynamic Routing) ──────────────────────────
    # CYNIC discovers its capabilities purely from the environment.
    # No hardcoded names allowed.
    llm_primary_model: str | None = None
    llm_fast_model: str | None = None
    llm_local_model: str | None = None
    
    # —— LLM: Operator Overrides ──────────────────────────────────────────────
    force_slow_mode: bool = False # If True, always use primary model regardless of cost/speed
    
    # —— LLM: API Keys ────────────────────────────────────────────────────────
    anthropic_api_key: str | None = None
    google_api_key: str | None = None

    # —— LLM: Local inference (llama.cpp / Ollama) ────────────────────────────
    ollama_url: str = "http://localhost:11434"
    models_dir: str | None = None
    llama_gpu_layers: int = -1
    llama_threads: int = 8
    ollama_num_parallel: int | None = None

    # —— Bots: Discord & Telegram ──────────────────────────────────────────────
    discord_token: str | None = None
    discord_guild_id: str | None = None
    telegram_token: str | None = None

    # —— GASdf: Governance (On-chain execution) ───────────────────────────────
    gasdf_url: str = "http://localhost:8766"
    gasdf_enabled: bool = False

    # —— Judgment & Consensus ──────────────────────────────────────────────────
    num_dogs: int = 11
    max_judgments_batch: int = 10
    judgment_timeout_seconds: float = 30.0

    # —— Learning & Q-Learning ─────────────────────────────────────────────────
    learning_rate: float = 0.1
    discount_factor: float = 0.99

    # —— Runtime ──────────────────────────────────────────────────────────────
    port: int = 8765  # Main API port
    log_level: str = "INFO"
    knet_host: str = "::"
    knet_port: int = 0  # 0 = OS selects free port (Standard SRE)
    environment: str = "development"

    # —— Thresholds (φ-derived, should rarely change) ─────────────────────────
    max_confidence: float = 0.618
    residual_threshold: float = 0.382

    @classmethod
    def from_env(cls) -> CynicConfig:
        """Load from environment variables with type coercion."""
        default_models_dir = os.path.join(os.path.expanduser("~"), ".cynic", "models")
        return cls(
            # Storage
            surreal_url=os.getenv("SURREAL_URL"),
            surreal_user=os.getenv("SURREAL_USER", "root"),
            surreal_pass=os.getenv("SURREAL_PASS", "local_dev_only"),
            surreal_ns=os.getenv("SURREAL_NS", "cynic"),
            surreal_db=os.getenv("SURREAL_DB", "cynic"),
            redis_url=os.getenv("REDIS_URL", "redis://localhost:6379/0"),
            database_url=(os.getenv("CYNIC_DATABASE_URL") or os.getenv("DATABASE_URL")),
            
            # Symbiosis Routing (Discovered by default)
            llm_primary_model=os.getenv("CYNIC_PRIMARY_MODEL"),
            llm_fast_model=os.getenv("CYNIC_FAST_MODEL"),
            llm_local_model=os.getenv("CYNIC_LOCAL_MODEL"),
            force_slow_mode=os.getenv("CYNIC_FORCE_SLOW_MODE", "0") == "1",
            
            # LLM Keys
            anthropic_api_key=os.getenv("ANTHROPIC_API_KEY"),
            google_api_key=os.getenv("GOOGLE_API_KEY"),

            # Local Inference
            ollama_url=os.getenv("OLLAMA_URL", "http://localhost:11434"),
            models_dir=os.getenv("CYNIC_MODELS_DIR", default_models_dir),
            llama_gpu_layers=int(os.getenv("LLAMA_CPP_GPU_LAYERS", "-1")),
            llama_threads=int(os.getenv("LLAMA_CPP_THREADS", "8")),
            ollama_num_parallel=_opt_int(os.getenv("OLLAMA_NUM_PARALLEL")),

            # Bots
            discord_token=os.getenv("DISCORD_TOKEN"),
            discord_guild_id=os.getenv("DISCORD_GUILD_ID"),
            telegram_token=os.getenv("TELEGRAM_BOT_TOKEN"),

            # GASdf
            gasdf_url=os.getenv("GASDF_URL", "http://localhost:8766"),
            gasdf_enabled=os.getenv("GASDF_ENABLED") == "1",

            # Judgment
            num_dogs=int(os.getenv("NUM_DOGS", "11")),
            max_judgments_batch=int(os.getenv("MAX_JUDGMENTS_BATCH", "10")),
            judgment_timeout_seconds=float(os.getenv("JUDGMENT_TIMEOUT_S", "30.0")),

            # Learning
            learning_rate=float(os.getenv("LEARNING_RATE", "0.1")),
            discount_factor=float(os.getenv("DISCOUNT_FACTOR", "0.99")),

            # Runtime
            port=int(os.getenv("PORT", "8765")),
            log_level=os.getenv("LOG_LEVEL", "INFO"),
            knet_host=os.getenv("KNET_HOST", "::"),
            knet_port=int(os.getenv("KNET_PORT", "0")),
            environment=os.getenv("ENVIRONMENT", "development"),
        )

    def validate(self) -> list[str]:
        """Return list of warnings/errors about this configuration."""
        issues: list[str] = []
        is_prod = self.environment not in ("development", "test", "local")

        # Storage warnings
        if self.surreal_url is None and self.database_url is None:
            issues.append(
                "WARN: No storage backend configured (set SURREAL_URL or DATABASE_URL)"
            )

        # Security: Default passwords
        if self.surreal_pass in ("root", "local_dev_only", "cynic_phi_618"):
            if is_prod or (self.surreal_url and "localhost" not in self.surreal_url):
                issues.append(
                    "CRITICAL: Using default SurrealDB password in production. "
                    "Set SURREAL_PASS environment variable."
                )

        # Security: Required credentials in production
        if is_prod:
            if not self.surreal_pass or self.surreal_pass in ("root", "local_dev_only"):
                issues.append("CRITICAL: SURREAL_PASS not set for production")
            if not self.discord_token:
                issues.append("WARN: DISCORD_TOKEN not set (Discord bot disabled)")

        # LLM warnings
        has_any_llm = bool(self.anthropic_api_key or self.google_api_key or self.models_dir)
        if not has_any_llm:
            issues.append(
                "INFO: No API keys or local models configured — "
                "Ollama-only mode (will use heuristic fallback if Ollama is down)"
            )

        # Port
        if self.port < 1024:
            issues.append(f"WARN: Port {self.port} requires root/admin privileges")

        return issues

    @property
    def has_surreal(self) -> bool:
        return self.surreal_url is not None

    @property
    def has_postgres(self) -> bool:
        return self.database_url is not None

    @property
    def has_any_storage(self) -> bool:
        return self.has_surreal or self.has_postgres
