"""I/O adapters for Gemini briefing consumer."""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from pathlib import Path

from organs.contracts.src.adapters.agent_tasks import agent_task_visible as kernel_agent_task_visible
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, normalize_kernel_base_url
from organs.contracts.src.surface_contracts import json_object_or_default, read_json_file_or_default
from organs.twitter.src.core.briefing_priorities import task_content


def load_briefing(organ_dir: str) -> dict:
    briefing_path = Path(organ_dir) / "analysis" / "lab_briefing_latest.json"
    if not briefing_path.exists():
        print(f"ERROR: Briefing not found: {briefing_path}")
        return {}

    briefing = read_json_file_or_default(briefing_path, {})
    if not isinstance(briefing, dict):
        print("ERROR: Failed to parse briefing: invalid JSON object")
        return {}
    return briefing


def load_verdicts(organ_dir: str) -> list:
    verdict_dir = Path(organ_dir) / "verdicts"
    verdicts = []

    if verdict_dir.exists():
        for json_file in sorted(verdict_dir.glob("*.json")):
            verdict = read_json_file_or_default(json_file, None)
            if isinstance(verdict, dict):
                verdicts.append(verdict)

    return verdicts


def load_skill(organ_dir: str) -> dict:
    skill_path = Path(organ_dir) / "agent" / "SKILL.md"
    if not skill_path.exists():
        return {}

    try:
        with skill_path.open(encoding="utf-8") as handle:
            return {"raw": handle.read()}
    except IOError:
        return {}


def create_task(domain_id: str, briefing: dict, organ_dir: str) -> bool:
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if not addr or not key:
        print("CYNIC_REST_ADDR or CYNIC_API_KEY not set - task dispatch skipped")
        return False
    try:
        addr = normalize_kernel_base_url(addr)
    except RuntimeError as exc:
        print(str(exc))
        return False

    body = json.dumps(
        {
            "kind": "hermes",
            "domain": domain_id,
            "content": task_content(domain_id, briefing),
            "agent_id": "gemini-briefing-consumer",
        }
    ).encode("utf-8")

    request = urllib.request.Request(
        f"{addr.rstrip('/')}/agent-tasks",
        data=body,
        headers=kernel_auth_headers(key),
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            if response.status < 300:
                result = json_object_or_default(response.read(), {})
                task_id = result.get("task_id", "unknown")
                if task_id != "unknown" and not kernel_agent_task_visible(
                    addr,
                    key,
                    str(task_id),
                    kind="hermes",
                    domain=domain_id,
                ):
                    print(f"ERROR: task read-back failed for {task_id} ({domain_id})")
                    return False
                print(f"Created kernel task: {task_id} for {domain_id}")
                return True
            print(f"Error creating task: {response.status}")
            return False
    except (urllib.error.URLError, urllib.error.HTTPError, OSError) as exc:
        print(f"ERROR: Failed to POST task for {domain_id}: {exc}")
        return False
