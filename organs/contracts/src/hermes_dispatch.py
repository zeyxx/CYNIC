#!/usr/bin/env python3
"""Shared utilities for dispatching organ tasks to the Hermes Agent executor."""

from __future__ import annotations

import json
import os
import sys

from organs.contracts.src.adapters.agent_tasks import (
    agent_task_collection_path,
    agent_task_items,
    agent_task_listing_query,
    agent_task_visible as kernel_agent_task_visible,
)
from organs.contracts.src.adapters.kernel_http import kernel_json_request, normalize_kernel_base_url


class HermesDispatcher:
    """Centralizes low-level HTTP integration with the cynic-kernel REST API."""

    def __init__(self, agent_name: str):
        self.agent_name = agent_name

    def env(self, name: str, default: str = "") -> str:
        return os.environ.get(name, default)

    def kernel_addr(self) -> str:
        raw = self.env("CYNIC_REST_ADDR", "").strip()
        if not raw:
            raise RuntimeError("CYNIC_REST_ADDR is required (SSOT kernel endpoint).")

        return normalize_kernel_base_url(raw)

    def kernel_key(self) -> str:
        return self.env("CYNIC_" + "API" + "_KEY")

    def get_json(self, path: str) -> dict | None:
        key = self.kernel_key()
        if not key:
            return None
        try:
            addr = self.kernel_addr()
        except RuntimeError as exc:
            print(f"{self.agent_name} get_json skipped: {exc}", file=sys.stderr)
            return None
        _, _, payload = kernel_json_request("GET", addr, path, key, timeout=10)
        return payload if isinstance(payload, dict) else {}

    def post_json(self, path: str, payload: dict) -> tuple[int | None, str]:
        """Post payload and return tuple (status_code, response_text)."""
        key = self.kernel_key()
        if not key:
            return None, "kernel API key missing"
        try:
            addr = self.kernel_addr()
        except RuntimeError as exc:
            return None, str(exc)
        status, text, _ = kernel_json_request("POST", addr, path, key, payload=payload, timeout=10)
        return status, text

    def dispatch_task(self, payload: dict) -> bool:
        """Dispatch task to /agent-tasks and require read-back when a task id is returned."""
        key = self.kernel_key()
        if not key:
            print(f"{self.agent_name} dispatch skipped: kernel API key missing", file=sys.stderr)
            return False

        status, text = self.post_json(agent_task_collection_path(False), payload)
        if status in (200, 201):
            print(text)
            task_id = self._response_task_id(text)
            if not task_id:
                return True
            kind = str(payload.get("kind") or "")
            domain = str(payload.get("domain") or "")
            if self.agent_task_visible(task_id, kind=kind, domain=domain):
                return True
            print(
                f"{self.agent_name} dispatch read-back failed: task_id={task_id} kind={kind} domain={domain}",
                file=sys.stderr,
            )
            return False

        if status is None:
            print(f"{self.agent_name} dispatch failed: {text}", file=sys.stderr)
        else:
            print(f"{self.agent_name} dispatch failed: HTTP {status} {text[:200]!r}", file=sys.stderr)
        return False

    def fetch_tasks(self, path: str) -> list[dict]:
        payload = self.get_json(path)
        return agent_task_items(payload)

    def list_agent_tasks(self, *, kind: str, domain: str, limit: int = 50) -> list[dict]:
        query = agent_task_listing_query(kind, domain, limit)
        return self.fetch_tasks(f"{agent_task_collection_path(False)}?{query}")

    def agent_task_visible(self, task_id: str, *, kind: str, domain: str) -> bool:
        if not task_id or not kind or not domain:
            return False
        try:
            addr = self.kernel_addr()
        except RuntimeError:
            return False
        return kernel_agent_task_visible(addr, self.kernel_key(), task_id, kind=kind, domain=domain)

    @staticmethod
    def _response_task_id(text: str) -> str | None:
        try:
            payload = json.loads(text) if text else {}
        except json.JSONDecodeError:
            return None
        if not isinstance(payload, dict):
            return None
        value = payload.get("task_id") or payload.get("id")
        return str(value) if value else None

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
