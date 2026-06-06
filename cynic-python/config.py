"""
Unified configuration loader for CYNIC Python tier.

Priority (highest first):
1. Environment variables (set explicitly, e.g., export CYNIC_API_KEY=...)
2. .env file in current working directory
3. ~/.config/cynic/.env
4. ~/.cynic-env (parsed as shell format)
5. Hardcoded defaults (safe defaults only)

This module enables consistent config loading across all Python subsystems.
Prevents R23 violations: subprocess env is explicit, not inherited.

Usage:
    from config import Config
    cfg = Config()
    print(cfg.cynic_api_key)
    print(cfg.cynic_rest_addr)
"""

import os
import re
from pathlib import Path
from typing import Optional


class Config:
    """Unified configuration loader."""

    def __init__(self):
        """Initialize config from all sources."""
        # Hardcoded defaults (safe)
        self._defaults = {
            "cynic_rest_addr": "http://localhost:3030",
            "cynic_api_key": None,
            "cultscreener_api_key": None,
            "helius_api_key": None,
            "gemini_api_key": None,
            "gemini_cli_trust_workspace": "false",
        }

        # Load from all sources in priority order
        self._config = dict(self._defaults)
        self._config.update(self._load_from_home_env())
        self._config.update(self._load_from_config_dir())
        self._config.update(self._load_from_dotenv())
        self._config.update(self._load_from_env_vars())

    def _load_from_env_vars(self) -> dict:
        """Load from environment variables (highest priority)."""
        keys = [
            "cynic_api_key",
            "cynic_rest_addr",
            "cultscreener_api_key",
            "helius_api_key",
            "gemini_api_key",
            "gemini_cli_trust_workspace",
        ]
        result = {}
        for key in keys:
            val = os.getenv(key.upper())
            if val:
                result[key] = val
        return result

    def _load_from_dotenv(self) -> dict:
        """Load from .env file (current directory or ~/.config/cynic/.env)."""
        result = {}
        # Try current directory first
        for path in [Path(".env"), Path.home() / ".config" / "cynic" / ".env"]:
            if path.exists():
                try:
                    with open(path) as f:
                        for line in f:
                            line = line.strip()
                            if not line or line.startswith("#"):
                                continue
                            if "=" in line:
                                key, val = line.split("=", 1)
                                result[key.lower()] = val.strip('"\'')
                except Exception:
                    pass  # Silently skip unreadable files
        return result

    def _load_from_home_env(self) -> dict:
        """Load from ~/.cynic-env (shell export format)."""
        result = {}
        path = Path.home() / ".cynic-env"
        if path.exists():
            try:
                with open(path) as f:
                    for line in f:
                        line = line.strip()
                        if not line or line.startswith("#"):
                            continue
                        # Remove leading 'export ' if present
                        if line.startswith("export "):
                            line = line[7:]
                        if "=" in line:
                            key, val = line.split("=", 1)
                            result[key.lower()] = val.strip('"\'')
            except Exception:
                pass  # Silently skip unreadable files
        return result

    def _load_from_config_dir(self) -> dict:
        """Load from ~/.config/cynic/{llama-server,surrealdb}.env."""
        result = {}
        config_dir = Path.home() / ".config" / "cynic"
        for filename in ["llama-server.env", "surrealdb.env"]:
            path = config_dir / filename
            if path.exists():
                try:
                    with open(path) as f:
                        for line in f:
                            line = line.strip()
                            if not line or line.startswith("#"):
                                continue
                            if "=" in line:
                                key, val = line.split("=", 1)
                                result[key.lower()] = val.strip('"\'')
                except Exception:
                    pass
        return result

    @property
    def cynic_api_key(self) -> Optional[str]:
        """CYNIC kernel API key."""
        return self._config.get("cynic_api_key")

    @property
    def cynic_rest_addr(self) -> str:
        """CYNIC kernel REST address (default: localhost:3030)."""
        return self._config.get("cynic_rest_addr", "http://localhost:3030")

    @property
    def cultscreener_api_key(self) -> Optional[str]:
        """CultScreener API key for twitter signal analysis."""
        return self._config.get("cultscreener_api_key")

    @property
    def helius_api_key(self) -> Optional[str]:
        """Helius API key for on-chain data."""
        return os.getenv("HELIUS_API_KEY")

    @property
    def gemini_api_key(self) -> Optional[str]:
        """Gemini API key for inference."""
        return self._config.get("gemini_api_key")

    @property
    def gemini_cli_trust_workspace(self) -> bool:
        """Gemini CLI trust workspace setting."""
        val = self._config.get("gemini_cli_trust_workspace", "false").lower()
        return val in ("true", "1", "yes")

    def __repr__(self) -> str:
        """Repr showing loaded config (secrets masked)."""
        safe = {
            k: "***" if "key" in k or "token" in k else v
            for k, v in self._config.items()
        }
        return f"Config({safe})"


# Singleton instance
_instance = None


def get_config() -> Config:
    """Get or create the singleton config instance."""
    global _instance
    if _instance is None:
        _instance = Config()
    return _instance
