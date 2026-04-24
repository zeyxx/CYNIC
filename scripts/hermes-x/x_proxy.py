"""
CYNIC X Proxy — mitmproxy addon that captures X/Twitter API responses.

Sits between browser (Playwright/camofox) and X. Passively captures
GraphQL responses for monitored operations. Extracts structured signals
and forwards to the CYNIC kernel via POST /judge.

Usage:
    mitmdump -s x_proxy.py --listen-host 127.0.0.1 -p 8888 --set stream_large_bodies=1m

Then configure browser proxy to localhost:8888.

SECURITY: Always use --listen-host 127.0.0.1 to prevent LAN exposure (KC4).
Without it, mitmdump binds 0.0.0.0 — any device on the LAN can route traffic through it.
"""

import json
import os
import logging
from datetime import datetime, timezone
from pathlib import Path

from mitmproxy import http

logger = logging.getLogger("x-proxy")

# Operations worth capturing — filter by name, not hash (hashes rotate on X deploys)
CAPTURE_OPS = {
    "SearchTimeline",
    "UserTweets",
    "TweetDetail",
    "HomeTimeline",
    "HomeLatestTimeline",
    "ListLatestTweetsTimeline",
}

# Where to dump raw captures for analysis + signal extraction
CAPTURE_DIR = Path(os.environ.get(
    "X_CAPTURE_DIR",
    Path(__file__).parent / "captures"
))


class XProxy:
    """mitmproxy addon: intercepts X GraphQL responses."""

    def __init__(self):
        CAPTURE_DIR.mkdir(parents=True, exist_ok=True)
        logger.info("x-proxy loaded — capturing: %s", ", ".join(sorted(CAPTURE_OPS)))

    def response(self, flow: http.HTTPFlow) -> None:
        url = flow.request.url
        if "/i/api/graphql/" not in url:
            return

        op_name = self._extract_op_name(url)
        if not op_name or op_name not in CAPTURE_OPS:
            return

        if flow.response is None or flow.response.status_code != 200:
            logger.warning("x-proxy: %s returned %s", op_name,
                           flow.response.status_code if flow.response else "no response")
            return

        try:
            data = json.loads(flow.response.content)
        except (json.JSONDecodeError, TypeError):
            logger.warning("x-proxy: %s — invalid JSON", op_name)
            return

        # Extract query context from URL variables
        query_ctx = self._extract_variables(url)
        ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")

        # Dump raw response for structure analysis
        capture_path = CAPTURE_DIR / f"{ts}_{op_name}.json"
        capture_path.write_text(json.dumps({
            "operation": op_name,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "variables": query_ctx,
            "response": data,
        }, indent=2, ensure_ascii=False))

        logger.info("x-proxy: captured %s → %s (%d bytes)",
                     op_name, capture_path.name, len(flow.response.content))

    @staticmethod
    def _extract_op_name(url: str) -> str | None:
        """Extract GraphQL operation name from URL.

        URL format: /i/api/graphql/{hash}/{OperationName}?variables=...
        We filter by OperationName (stable) not hash (rotates on X deploys).
        """
        try:
            after_graphql = url.split("/i/api/graphql/")[1]
            path_part = after_graphql.split("?")[0]
            segments = path_part.split("/")
            return segments[1] if len(segments) >= 2 else None
        except (IndexError, AttributeError):
            return None

    @staticmethod
    def _extract_variables(url: str) -> dict:
        """Extract the variables JSON from the URL query string."""
        try:
            from urllib.parse import urlparse, parse_qs
            params = parse_qs(urlparse(url).query)
            raw = params.get("variables", ["{}"])[0]
            return json.loads(raw)
        except (json.JSONDecodeError, KeyError):
            return {}


addons = [XProxy()]
