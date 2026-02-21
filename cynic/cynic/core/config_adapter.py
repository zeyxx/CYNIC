"""
ConfigurationAdaptationEngine — When CYNIC sees something new, it asks

Axiom: CULTURE (adapt to your machine's culture)
       VERIFY (never assume, always ask)

Pattern:
  1. CYNIC discovers a new environment variable or config
  2. Auto-detect what it might be
  3. Check if user already told us how to handle this
  4. If not: ASK the user interactively
  5. Remember their preference
  6. Auto-adapt for future runs

This is how CYNIC becomes smarter over time — learning YOUR machine.
"""

import os
import sys
import json
import asyncio
import logging
import platform
from pathlib import Path
from typing import Any, Optional
from datetime import datetime

logger = logging.getLogger(__name__)


class ConfigurationAdaptationEngine:
    """When CYNIC discovers something new, it asks the user."""

    def __init__(self):
        self.discoveries_log = Path.home() / ".cynic" / "config_discoveries.json"
        self.preferences_log = Path.home() / ".cynic" / "config_preferences.json"

    async def handle_unknown_setting(
        self, key: str, detected_value: Any, description: str = ""
    ) -> Any:
        """
        When CYNIC encounters unknown configuration:
        1. Check if user already told us
        2. Ask user interactively
        3. Remember preference
        """

        # Step 1: Check if we know this already
        if known := self._get_known_preference(key):
            return known

        # Step 2: Log discovery
        self._log_discovery(key, detected_value, description)

        # Step 3: Ask user (interactive)
        print(f"\n{'='*70}")
        print(f"*sniff* CYNIC discovered new configuration")
        print(f"{'='*70}")
        print(f"\nKey:       {key}")
        print(f"Detected:  {detected_value}")
        if description:
            print(f"Context:   {description}")

        print(f"\nHow should I adapt? (press Enter to accept detected value)")
        print(f"or type custom value:")
        print()

        try:
            # Run blocking I/O in executor thread pool (non-blocking in async context)
            loop = asyncio.get_event_loop()
            user_input = await loop.run_in_executor(None, input, "> ")
            user_input = user_input.strip()
        except (EOFError, KeyboardInterrupt):
            # Non-interactive mode (e.g., Docker) — use detected value
            user_input = ""

        # Step 4: Determine final value
        final_value = user_input if user_input else detected_value

        # Step 5: Remember for future
        self._save_preference(key, final_value)

        # Platform-aware output (Windows cp1252 can't render ✓)
        ok_symbol = "[OK]" if platform.system() == "Windows" else "✓"
        print(f"\n{ok_symbol} Adapted: {key} = {final_value}")
        print(f"  (Saved to ~.cynic/config_preferences.json for future runs)")
        print()

        return final_value

    def _get_known_preference(self, key: str) -> Optional[Any]:
        """Check if we've been told how to handle this before."""
        if not self.preferences_log.exists():
            return None

        try:
            prefs = json.loads(self.preferences_log.read_text())
            return prefs.get(key)
        except json.JSONDecodeError:
            return None

    def _log_discovery(self, key: str, value: Any, description: str) -> None:
        """Record what CYNIC discovered."""
        entry = {
            "timestamp": datetime.now().isoformat(),
            "key": key,
            "detected_value": value,
            "description": description,
        }

        # Load existing
        existing = []
        if self.discoveries_log.exists():
            try:
                data = json.loads(self.discoveries_log.read_text())
                existing = data if isinstance(data, list) else []
            except json.JSONDecodeError:
                pass

        # Append and cap at F(11)=89
        existing.append(entry)
        existing = existing[-89:]

        # Save
        self.discoveries_log.parent.mkdir(parents=True, exist_ok=True)
        self.discoveries_log.write_text(json.dumps(existing, indent=2))

    def _save_preference(self, key: str, value: Any) -> None:
        """Remember user's preference (atomic write safe for concurrent access)."""
        # Load existing
        prefs = {}
        if self.preferences_log.exists():
            try:
                prefs = json.loads(self.preferences_log.read_text())
            except (json.JSONDecodeError, OSError):
                pass  # Corrupted file - start fresh

        # Add/update
        prefs[key] = value

        # Atomic write via temp file + rename (prevents corruption on concurrent writes)
        self.preferences_log.parent.mkdir(parents=True, exist_ok=True)
        temp_file = self.preferences_log.parent / f".{self.preferences_log.name}.tmp"

        try:
            temp_file.write_text(json.dumps(prefs, indent=2))
            # Atomic rename (POSIX atomic, Windows replaces)
            temp_file.replace(self.preferences_log)
        except json.JSONDecodeError as e:
            logger.error(f"Failed to save preference: {e}")
            if temp_file.exists():
                temp_file.unlink()

    async def get_or_ask(self, key: str, default: Any, description: str = "") -> Any:
        """
        Get configuration value:
          1. Check env var
          2. Check user preferences
          3. Ask user if new
        """

        # Priority 1: Environment variable
        if env_value := os.getenv(key):
            return env_value

        # Priority 2: User preference (from previous discovery)
        if pref := self._get_known_preference(key):
            return pref

        # Priority 3: Ask user (new discovery)
        return await self.handle_unknown_setting(key, default, description)


# Singleton
_adapter = None


def get_config_adapter() -> ConfigurationAdaptationEngine:
    """Get singleton adapter."""
    global _adapter
    if _adapter is None:
        _adapter = ConfigurationAdaptationEngine()
    return _adapter
