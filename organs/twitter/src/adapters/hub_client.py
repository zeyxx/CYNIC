"""
Hub Client — thin sync wrapper for Browser Hub HTTP API.

Used by x_proxy.py (attribution), organic_navigator.py (tab management),
and search_executor.py (tab management).

All methods return safe defaults on Hub failure (K14: degraded, not broken).
"""

__version__ = "0.1.0"

import logging
import os
from typing import Optional

import requests

logger = logging.getLogger("hub-client")

_DEFAULT_HUB_URL = "http://127.0.0.1:40770"
_TIMEOUT = 0.5  # 500ms — Hub is local, must be fast


class HubClient:
    def __init__(self, hub_url: str = None):
        self._url = hub_url or os.environ.get("BROWSER_HUB_URL", _DEFAULT_HUB_URL)

    def get_attribution(self, url: str, timestamp: float) -> str:
        """Query Hub for request attribution. Returns 'unknown' on failure."""
        try:
            resp = requests.get(
                f"{self._url}/attribution",
                params={"url": url, "ts": str(timestamp)},
                timeout=_TIMEOUT,
            )
            return resp.json().get("source", "unknown")
        except Exception:
            return "unknown"

    def create_tab(self, owner: str, url: str, window: str = "agent") -> Optional[dict]:
        """Ask Hub to create a tab. Returns tab info dict or None on failure."""
        try:
            resp = requests.post(
                f"{self._url}/tabs",
                json={"owner": owner, "url": url, "window": window},
                timeout=_TIMEOUT,
            )
            if resp.status_code == 201:
                return resp.json()
            logger.warning("Hub create_tab returned %d", resp.status_code)
            return None
        except Exception as e:
            logger.warning("Hub unreachable for create_tab: %s", e)
            return None

    def release_tab(self, tab_id: str) -> bool:
        """Ask Hub to close and unregister a tab. Returns True on success."""
        try:
            resp = requests.delete(f"{self._url}/tabs/{tab_id}", timeout=_TIMEOUT)
            return resp.status_code == 200
        except Exception as e:
            logger.warning("Hub unreachable for release_tab: %s", e)
            return False

    def get_status(self) -> Optional[dict]:
        """Get Hub status. Returns None on failure."""
        try:
            resp = requests.get(f"{self._url}/status", timeout=_TIMEOUT)
            return resp.json()
        except Exception:
            return None
