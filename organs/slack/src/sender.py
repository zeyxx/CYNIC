"""
organ-slack — Sender
Thin adapter around the Slack chat.postMessage API.
Never called directly — invoked by dispatch.py after the router resolves
the target channel.

Sovereignty note: this module is the ONLY place in CYNIC that speaks HTTP
to Slack. When we migrate to sovereign messaging, replace only this file.
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request


SLACK_API_URL = "https://slack.com/api/chat.postMessage"


def _bot_token() -> str:
    token = os.environ.get("SLACK_BOT_TOKEN", "")
    if not token:
        raise RuntimeError("organ-slack: SLACK_BOT_TOKEN is not set in environment")
    return token


def send(channel_id: str, text: str, blocks: list | None = None) -> dict:
    """
    Post a message to a Slack channel.

    Args:
        channel_id: Concrete Slack channel ID (e.g. "C0BC326EQHM").
        text:       Fallback plain-text content (always required by Slack API).
        blocks:     Optional Block Kit blocks for rich formatting.

    Returns:
        Slack API response dict.

    Raises:
        RuntimeError on HTTP error or Slack API-level failure (ok=false).
    """
    payload: dict = {"channel": channel_id, "text": text}
    if blocks:
        payload["blocks"] = blocks

    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        SLACK_API_URL,
        data=body,
        headers={
            "Authorization": f"Bearer {_bot_token()}",
            "Content-Type": "application/json; charset=utf-8",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw = resp.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"organ-slack: HTTP {exc.code} from Slack API — {raw}") from exc
    except OSError as exc:
        raise RuntimeError(f"organ-slack: network error — {exc}") from exc

    result = json.loads(raw)
    if not result.get("ok"):
        error_code = result.get("error", "unknown")
        raise RuntimeError(f"organ-slack: Slack API error — {error_code} (channel={channel_id})")

    return result


def format_observation(observation: dict) -> tuple[str, list]:
    """
    Format a CYNIC observation dict into a Slack text + Block Kit payload.
    Keeps it clean and readable in the caplogy channel.
    """
    domain = observation.get("domain", "unknown")
    project = observation.get("project", "")
    context = observation.get("context", "")
    tool = observation.get("tool", "")
    status = observation.get("status", "")
    target = observation.get("target", "")

    # Fallback plain text
    text = f"[CYNIC/{domain.upper()}] {context or tool}"

    # Rich Block Kit
    header = f":satellite: *CYNIC · {domain.upper()}*"
    if project:
        header += f" · `{project}`"

    fields = []
    if context:
        fields.append({"type": "mrkdwn", "text": f"*Message*\n{context}"})
    if tool:
        fields.append({"type": "mrkdwn", "text": f"*Tool*\n`{tool}`"})
    if status:
        fields.append({"type": "mrkdwn", "text": f"*Status*\n`{status}`"})
    if target and target != observation.get("target"):
        fields.append({"type": "mrkdwn", "text": f"*Target*\n`{target}`"})

    blocks = [
        {"type": "section", "text": {"type": "mrkdwn", "text": header}},
    ]
    if fields:
        blocks.append({"type": "section", "fields": fields})
    blocks.append({"type": "divider"})

    return text, blocks
