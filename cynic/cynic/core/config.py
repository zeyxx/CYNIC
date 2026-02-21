"""
CYNIC Configuration — Single Source of Truth

ALL environment variables read here. No os.getenv() anywhere else.
Validation catches insecure defaults and missing backends.

φ-Law: VERIFY — one truth, no scattered defaults.
"""
from __future__ import annotations

import logging
import os
from dataclasses import dataclass, field
from typing import Optional

logger = logging.getLogger("cynic.config")


def _opt_int(val: Optional[str]) -> Optional[int]:
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

    # ── Storage: SurrealDB (primary) ──────────────────────────────────────
    surreal_url: Optional[str] = None
    surreal_user: str = "root"
    surreal_pass: str = "local_dev_only"
    surreal_ns: str = "cynic"
    surreal_db: str = "cynic"

    # ── Storage: PostgreSQL (fallback) ────────────────────────────────────
    database_url: Optional[str] = None

    # ── LLM: Ollama (primary — local, free) ──────────────────────────────
    ollama_url: str = "http://localhost:11434"

    # ── LLM: Claude API (MACRO cycle reasoning) ─────────────────────────
    anthropic_api_key: Optional[str] = None

    # ── LLM: Gemini API (free tier alternative) ─────────────────────────
    google_api_key: Optional[str] = None

    # ── LLM: Local inference (llama.cpp) ─────────────────────────────────
    models_dir: Optional[str] = None
    llama_gpu_layers: int = -1
    llama_threads: int = 8

    # ── LLM: Ollama tuning ────────────────────────────────────────────────
    ollama_num_parallel: Optional[int] = None

    # ── Runtime ──────────────────────────────────────────────────────────
    port: int = 8765
    log_level: str = "INFO"

    # ── Thresholds (φ-derived, should rarely change) ─────────────────────
    max_confidence: float = 0.618
    residual_threshold: float = 0.382

    @classmethod
    def from_env(cls) -> CynicConfig:
        """Load from environment variables with type coercion."""
        return cls(
            # Storage
            surreal_url=os.getenv("SURREAL_URL"),
            surreal_user=os.getenv("SURREAL_USER", "root"),
            surreal_pass=os.getenv("SURREAL_PASS", "local_dev_only"),
            surreal_ns=os.getenv("SURREAL_NS", "cynic"),
            surreal_db=os.getenv("SURREAL_DB", "cynic"),
            database_url=(
                os.getenv("CYNIC_DATABASE_URL")
                or os.getenv("DATABASE_URL")
            ),
            # LLM
            ollama_url=os.getenv("OLLAMA_URL", "http://localhost:11434"),
            anthropic_api_key=os.getenv("ANTHROPIC_API_KEY"),
            google_api_key=os.getenv("GOOGLE_API_KEY"),
            models_dir=os.getenv("CYNIC_MODELS_DIR"),
            llama_gpu_layers=int(os.getenv("LLAMA_CPP_GPU_LAYERS", "-1")),
            llama_threads=int(os.getenv("LLAMA_CPP_THREADS", "8")),
            # Ollama tuning
            ollama_num_parallel=_opt_int(os.getenv("OLLAMA_NUM_PARALLEL")),
            # Runtime
            port=int(os.getenv("PORT", "8765")),
            log_level=os.getenv("LOG_LEVEL", "INFO"),
        )

    def validate(self) -> list[str]:
        """Return list of warnings/errors about this configuration."""
        issues: list[str] = []

        # Storage warnings
        if self.surreal_url is None and self.database_url is None:
            issues.append(
                "WARN: No storage backend configured "
                "(set SURREAL_URL or DATABASE_URL)"
            )

        if self.surreal_pass in ("root", "local_dev_only", "cynic_phi_618"):
            if self.surreal_url and "localhost" not in self.surreal_url:
                issues.append(
                    "CRITICAL: Using default SurrealDB password on non-local URL"
                )

        # LLM warnings
        has_any_llm = bool(
            self.anthropic_api_key
            or self.google_api_key
            or self.models_dir
        )
        if not has_any_llm:
            issues.append(
                "INFO: No API keys or local models configured — "
                "Ollama-only mode (will use heuristic fallback if Ollama is down)"
            )

        # Port
        if self.port < 1024:
            issues.append(
                f"WARN: Port {self.port} requires root/admin privileges"
            )

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
