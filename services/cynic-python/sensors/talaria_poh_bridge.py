#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Talaria event ingestion bridge.
Bridge B&C PoH/chess events into typed Talaria observations.

This bridge records raw evidence for Talaria review. It does not decide that a
user is human, trusted, or eligible for governance by itself.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

import requests

try:
    from .talaria_events import TalariaEvent, evidence_hash
except ImportError:  # pragma: no cover - direct script execution
    from talaria_events import TalariaEvent, evidence_hash

AGENT_ID = "talaria-poh-bridge"
POH_KINDS = {
    "bnc.poh.game_completed",
    "bnc.poh.evidence_sealed",
    "bnc.poh.wallet_binding_pending",
    "bnc.poh.wallet_binding_verified",
    "bnc.poh.profile_ready",
    "bnc.poh.cynic_judgment_requested",
}
CHESS_KINDS = {
    "bnc.chess.game_completed",
    "bnc.chess.tournament_joined",
    "bnc.chess.tournament_completed",
    "bnc.chess.rating_updated",
    "bnc.chess.fair_play_signal",
    "bnc.chess.community_signal",
}


def _normalize_addr(raw: str) -> str:
    if raw and not raw.startswith("http"):
        return f"http://{raw}"
    return raw.rstrip("/")


def load_kernel_env() -> tuple[str, str]:
    """Load kernel REST address and API key from env only."""
    return _normalize_addr(os.environ.get("CYNIC_REST_ADDR", "")), os.environ.get("CYNIC_API_KEY", "")


def load_event(path: str | None) -> dict[str, Any]:
    """Load one B&C event from a file path or stdin."""
    text = Path(path).read_text() if path else sys.stdin.read()
    data = json.loads(text)
    if not isinstance(data, dict):
        raise ValueError("B&C event must be a JSON object")
    return data


def to_talaria_event(raw: dict[str, Any]) -> TalariaEvent:
    """Convert a B&C event to a canonical Talaria event envelope."""
    kind = str(raw.get("kind") or raw.get("event") or "").strip()
    if kind not in POH_KINDS and kind not in CHESS_KINDS:
        raise ValueError(f"unsupported B&C event kind: {kind or '<missing>'}")

    value = raw.get("value") if isinstance(raw.get("value"), dict) else dict(raw)
    subject_id = str(raw.get("subject_id") or raw.get("user_id") or raw.get("player_id") or "")
    wallet = raw.get("wallet_address") or raw.get("wallet")
    wallet_address = str(wallet) if wallet else None
    actor = subject_id or wallet_address or "unknown-bnc-user"
    scope = "talaria.poh.user" if kind in POH_KINDS else "talaria.chess.signal"
    source = str(raw.get("source") or "blitz-and-chill")
    source_url = raw.get("source_url") or raw.get("url")
    depends = raw.get("depends_on") if isinstance(raw.get("depends_on"), list) else []

    return TalariaEvent(
        scope=scope,  # type: ignore[arg-type]
        actor=actor,
        actor_kind="app_user",
        kind=kind,
        title=str(raw.get("title") or kind.replace(".", " ")),
        summary=str(raw.get("summary") or _summary(kind, subject_id, wallet_address)),
        source=source,
        source_url=str(source_url) if source_url else None,
        evidence_hash=str(raw.get("evidence_hash") or evidence_hash(value)),
        wallet_address=wallet_address,
        subject_id=subject_id or None,
        confidence="observed",
        visibility=str(raw.get("visibility") or "internal"),  # type: ignore[arg-type]
        value=value,
        depends_on=[str(v) for v in depends],
    )


def _summary(kind: str, subject_id: str, wallet: str | None) -> str:
    subject = subject_id or wallet or "unknown subject"
    return f"B&C emitted {kind} for {subject}; Talaria records raw evidence for review."


def post_observation(addr: str, api_key: str, payload: dict[str, Any], timeout: int) -> None:
    """Send an observe payload to the CYNIC kernel."""
    if not addr or not api_key:
        raise ValueError("CYNIC_REST_ADDR and CYNIC_API_KEY must be set")
    resp = requests.post(
        f"{addr}/observe",
        json=payload,
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        timeout=timeout,
    )
    if resp.status_code != 200:
        raise RuntimeError(f"/observe failed: HTTP {resp.status_code} {resp.text[:200]}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Bridge B&C PoH/chess events into Talaria /observe")
    parser.add_argument("event", nargs="?", help="JSON event file; stdin when omitted")
    parser.add_argument("--dry-run", action="store_true", help="Print observe payload instead of POSTing")
    parser.add_argument("--timeout", type=int, default=10)
    args = parser.parse_args()

    try:
        raw = load_event(args.event)
        event = to_talaria_event(raw)
        payload = event.observe_payload(agent_id=AGENT_ID)
        if args.dry_run:
            print(json.dumps(payload, sort_keys=True, indent=2))
            return 0
        addr, key = load_kernel_env()
        post_observation(addr, key, payload, args.timeout)
        print(json.dumps({"status": "observed", "id": event.envelope()["id"]}))
        return 0
    except Exception as exc:
        print(f"talaria-poh-bridge: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
