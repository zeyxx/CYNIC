"""Kernel transport adapter for Twitter/X ingestion.

This module owns REST payload construction and POST mechanics for X ingest.
It is intentionally transport-specific; domain assignment and stimulus shaping
belong to organs.twitter.src.core.ingest_signals.
"""

import time
from dataclasses import dataclass
from typing import Any

import requests

from organs.contracts.src.adapters.kernel_http import (
    kernel_auth_headers,
    kernel_url,
    normalize_kernel_base_url,
)
from organs.contracts.src.adapters.kernel_observe import post_kernel_observe
from organs.twitter.src.core.ingest_signals import build_social_stimulus


def normalize_kernel_addr(raw: str) -> str:
    return normalize_kernel_base_url(raw, allow_empty=True)


def kernel_headers(api_key: str) -> dict:
    return kernel_auth_headers(api_key)


def build_tweet_observe_payload(row: dict, organ_name: str, domain: str, account_id: str) -> dict:
    text = row.get("text", "")[:200]
    author = row.get("author_screen_name", "?")
    score = row.get("signal_score", 0)
    tags = row.get("cashtags", []) + row.get("narratives", [])

    return {
        "tool": organ_name,
        "target": row.get("tweet_id", ""),
        "domain": domain,
        "status": "captured",
        "context": f"@{author} [{score}]: {text}",
        "tags": tags[:10],
        "agent_id": f"hermes-{account_id}",
        "account_id": account_id,
    }


def build_convergence_payload(signal: Any, account_id: str) -> dict:
    import json as _json

    context = _json.dumps({
        "item": signal.cashtag,
        "resolved_mint": signal.resolved_mint,
        "author_count": signal.author_count,
        "authors": signal.authors,
        "tier_distribution": {},
        "coordination_score": signal.coordination_score,
        "sentiment": signal.sentiment,
        "window_hours": signal.window_hours,
        "key_quotes": signal.key_quotes,
        "source_organ": "x",
    })

    target = signal.resolved_mint or f"${signal.cashtag}"
    tags = ["convergence", signal.cashtag, "compound-loop"]
    if signal.resolved_mint:
        tags.append(f"mint:{signal.resolved_mint}")

    return {
        "tool": "x-convergence",
        "target": target,
        "domain": signal.domain,
        "context": context,
        "tags": tags,
        "agent_id": f"convergence-{account_id}",
    }


def build_token_judgment_payload(signal: Any) -> dict:
    return {
        "content": signal.resolved_mint,
        "context": f"Social convergence: ${signal.cashtag} mentioned by "
                   f"{signal.author_count} authors in {signal.window_hours}h",
        "domain": "D1",
        "priority": "hermes",
    }


def build_social_judgment_payload(row: dict, domain: str, account_id: str) -> dict:
    return {
        "content": build_social_stimulus(row, domain),
        "context": f"Passive capture via x-organ [{account_id}]. Account: {account_id}.",
        "domain": domain,
        "account_id": account_id,
        "priority": "hermes",
    }


def build_heartbeat_payload(
    organ_name: str,
    account_id: str,
    status: str,
    context: str,
    failure_reason: str | None = None,
) -> dict:
    payload = {
        "tool": "heartbeat",
        "target": organ_name,
        "domain": "organ-health",
        "status": status,
        "context": context,
        "tags": ["heartbeat", organ_name, f"account-{account_id}"],
        "agent_id": f"hermes-{organ_name}",
        "account_id": account_id,
    }
    if failure_reason:
        payload["failure_reason"] = failure_reason
    if status == "critical":
        payload["consumer"] = f"hermes-recovery@{account_id}"
    return payload


@dataclass
class KernelTransport:
    kernel_addr: str
    api_key: str
    requests_module: Any = requests
    sleep_fn: Any = time.sleep

    def _url(self, path: str) -> str:
        return kernel_url(self.kernel_addr, path, allow_empty=True)

    def _headers(self) -> dict:
        return kernel_headers(self.api_key)

    def post_json(self, path: str, payload: dict, timeout: int):
        return self.requests_module.post(
            self._url(path),
            json=payload,
            headers=self._headers(),
            timeout=timeout,
        )

    def post_observe(self, payload: dict, timeout: int = 5) -> bool:
        result = post_kernel_observe(
            request_post=self.requests_module.post,
            kernel_url=self.kernel_addr,
            auth_key=self.api_key,
            payload=payload,
            timeout=timeout,
        )
        return result.posted

    def post_judge_payload(self, payload: dict, timeout: int = 30) -> dict | None:
        resp = self.post_json("/judge", payload, timeout=timeout)
        if resp.status_code == 200:
            return resp.json()
        return None

    def post_social_judge(self, payload: dict, max_retries: int = 3) -> dict | None:
        for attempt in range(max_retries + 1):
            resp = self.post_json("/judge", payload, timeout=120)
            if resp.status_code == 200:
                return resp.json()
            if resp.status_code == 429:
                self.sleep_fn(2 ** attempt * 5)
                continue
            return None
        return None
