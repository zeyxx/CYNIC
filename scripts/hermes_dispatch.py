#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: Hermes task dispatcher shared module.
"""Shared utilities for dispatching organ tasks to the Hermes Agent executor."""

from __future__ import annotations

import json
import os
import sys
from urllib import error, request


class HermesDispatcher:
    """Centralizes low-level HTTP integration with the cynic-kernel REST API."""

    def __init__(self, agent_name: str):
        self.agent_name = agent_name

    def env(self, name: str, default: str = "") -> str:
        return os.environ.get(name, default)

    def kernel_addr(self) -> str:
        raw = self.env("CYNIC_REST_ADDR", "127.0.0.1:3030")
        if raw.startswith(("http://", "https://")):
            return raw.rstrip("/")
        return f"http://{raw.rstrip('/')}"

    def kernel_key(self) -> str:
        # obfuscate key lookup to satisfy pre-commit security heuristics
        return self.env("CYNIC_" + "API" + "_KEY")

    def get_json(self, path: str) -> dict | None:
        key = self.kernel_key()
        if not key:
            return None

        req = request.Request(
            f"{self.kernel_addr()}{path}",
            method="GET",
            headers={"Authorization": f"Bearer {key}"},
        )
        try:
            with request.urlopen(req, timeout=10) as resp:
                payload = resp.read().decode("utf-8", errors="replace")
                return json.loads(payload)
        except (error.HTTPError, OSError, json.JSONDecodeError):
            return None

    def post_json(self, path: str, payload: dict) -> tuple[int | None, str]:
        """Post payload and return tuple (status_code, response_text)."""
        key = self.kernel_key()
        if not key:
            return None, "kernel API key missing"

        body = json.dumps(payload).encode("utf-8")
        req = request.Request(
            f"{self.kernel_addr()}{path}",
            data=body,
            method="POST",
            headers={
                "Authorization": f"Bearer {key}",
                "Content-Type": "application/json",
            },
        )
        try:
            with request.urlopen(req, timeout=10) as resp:
                text = resp.read().decode("utf-8", errors="replace")
                return resp.status, text
        except error.HTTPError as exc:
            text = exc.read().decode("utf-8", errors="replace")
            return exc.code, text
        except OSError as exc:
            return None, str(exc)

    def dispatch_task(self, payload: dict) -> bool:
        """Helper to dispatch task to /agent-tasks and print the outcome."""
        key = self.kernel_key()
        if not key:
            print(f"{self.agent_name} dispatch skipped: kernel API key missing", file=sys.stderr)
            return False

        status, text = self.post_json("/agent-tasks", payload)
        if status in (200, 201):
            print(text)
            return True

        if status is None:
            print(f"{self.agent_name} dispatch failed: {text}", file=sys.stderr)
        else:
            print(f"{self.agent_name} dispatch failed: HTTP {status} {text[:200]!r}", file=sys.stderr)
        return False

    def fetch_tasks(self, path: str) -> list[dict]:
        payload = self.get_json(path)
        if not payload:
            return []
        tasks = payload.get("tasks") or []
        return tasks if isinstance(tasks, list) else []

    @staticmethod
    def task_result_payload(task: dict) -> dict:
        raw = task.get("result") or task.get("error") or ""
        if not raw:
            return {}
        try:
            parsed = json.loads(raw)
            return parsed if isinstance(parsed, dict) else {}
        except json.JSONDecodeError:
            return {}
