"""Browser Hub status adapter shared by diagnostics and organism observation."""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class BrowserHubStatus:
    url: str
    http_ok: bool
    observed: str
    payload: dict[str, Any]

    @property
    def chrome_reachable(self) -> bool:
        return self.payload.get("chrome") == "reachable"

    @property
    def chrome_detail(self) -> str:
        blockers = self.payload.get("blockers")
        if isinstance(blockers, list) and blockers:
            return str(blockers)
        return str(self.payload.get("chrome") or "unknown")


def read_browser_hub_status(hub_url: str, *, timeout: float = 2) -> BrowserHubStatus:
    base = hub_url.rstrip("/")
    status_url = f"{base}/status"
    try:
        with urllib.request.urlopen(status_url, timeout=timeout) as response:
            body = response.read().decode("utf-8", errors="replace")
            if 200 <= int(response.status) < 300:
                payload = json.loads(body) if body.strip() else {}
                payload = payload if isinstance(payload, dict) else {}
                return BrowserHubStatus(
                    url=base,
                    http_ok=True,
                    observed=f"{status_url} reachable",
                    payload=payload,
                )
            return BrowserHubStatus(
                url=base,
                http_ok=False,
                observed=f"{status_url} returned HTTP {response.status}",
                payload={},
            )
    except urllib.error.HTTPError as exc:
        return BrowserHubStatus(url=base, http_ok=False, observed=f"{status_url} returned HTTP {exc.code}", payload={})
    except json.JSONDecodeError as exc:
        return BrowserHubStatus(url=base, http_ok=False, observed=f"{status_url} returned invalid JSON: {exc}", payload={})
    except Exception as exc:
        return BrowserHubStatus(url=base, http_ok=False, observed=f"{status_url} unreachable: {exc}", payload={})
