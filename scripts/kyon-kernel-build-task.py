#!/usr/bin/env python3
"""Dispatch a Kyon Android build task to kernel /agent-tasks and wait for completion.

This helper is intentionally minimal and does not run any Android tooling directly.
It posts a structured task payload, then polls /agent-tasks/completed for result.
"""

from __future__ import annotations

import argparse
import json
import time
import urllib.error
import urllib.request


def _normalize_url(raw: str) -> str:
    raw = raw.strip()
    if not raw:
        raise ValueError("kernel rest address is required")
    if raw.startswith(("http://", "https://")):
        return raw.rstrip("/")
    return f"http://{raw.rstrip('/')}"


def _request(url: str, api_key: str, payload: dict | None = None, timeout: int = 30):
    body = b""
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(
        url,
        data=body,
        method="POST",
        headers=headers,
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        text = resp.read().decode("utf-8", errors="replace")
        if not text:
            return {}
        return json.loads(text)


def _get(url: str, api_key: str, timeout: int = 30):
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8", errors="replace"))


def _poll_completed(base_url: str, api_key: str, kind: str, domain: str, task_id: str, timeout: int):
    deadline = time.time() + timeout
    poll_url = f"{base_url}/agent-tasks/completed?kind={kind}&domain={domain}&limit=50"

    while time.time() < deadline:
        payload = _get(poll_url, api_key)
        tasks = payload.get("tasks") if isinstance(payload, dict) else []
        if not isinstance(tasks, list):
            tasks = []

        match = None
        for task in tasks:
            if task.get("id") == task_id:
                match = task
                break

        if match:
            return match

        time.sleep(2)

    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel-url", default="127.0.0.1:3030")
    parser.add_argument("--api-key", required=True)
    parser.add_argument("--project-root", required=True)
    parser.add_argument("--command", required=True)
    parser.add_argument("--kind", default="kyon")
    parser.add_argument("--domain", default="kyon-mobile")
    parser.add_argument("--artifact-path", default="app/build/outputs/apk/debug/app-debug.apk")
    parser.add_argument("--timeout", default=900, type=int)

    args = parser.parse_args()

    base_url = _normalize_url(args.kernel_url)

    content = {
        "task": "kyon-android-build",
        "command": args.command,
        "project_root": args.project_root,
        "artifact_path": args.artifact_path,
        "destination_path": args.artifact_path,
    }

    dispatch_payload = {
        "kind": args.kind,
        "domain": args.domain,
        "content": json.dumps(content),
        "agent_id": "kyon-mobile-build",
    }

    response = _request(
        f"{base_url}/agent-tasks",
        args.api_key,
        dispatch_payload,
        timeout=max(10, min(60, args.timeout)),
    )

    task_id = response.get("task_id") if isinstance(response, dict) else None
    if not task_id and isinstance(response, dict):
        task_id = response.get("id")
    if not task_id:
        raise SystemExit(f"No task_id returned from kernel: {response}")

    task = _poll_completed(
        base_url=base_url,
        api_key=args.api_key,
        kind=args.kind,
        domain=args.domain,
        task_id=task_id,
        timeout=args.timeout,
    )
    if task is None:
        raise SystemExit(f"Kernel task {task_id} not completed in {args.timeout}s")

    result_payload = task.get("result")
    if isinstance(result_payload, str):
        try:
            result_payload = json.loads(result_payload)
        except json.JSONDecodeError:
            result_payload = {"raw_result": result_payload}

    status = "failed"
    if isinstance(result_payload, dict):
        status = str(result_payload.get("status", "failed"))

    if isinstance(result_payload, dict) and isinstance(result_payload.get("status"), bool):
        status = "success" if result_payload.get("status") else "failed"

    if status == "success":
        print(task_id)
        return 0

    raise SystemExit(f"Kernel task {task_id} failed: {result_payload}")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except urllib.error.HTTPError as exc:
        raise SystemExit(f"Kernel task dispatch failed (HTTP {exc.code}): {exc.read().decode(errors='replace')}")
    except urllib.error.URLError as exc:
        raise SystemExit(f"Kernel task dispatch failed (network): {exc}")
