#!/usr/bin/env python3
"""Persist an Ouroboros run record and repo evals into SurrealDB.

Input: JSON on stdin with this shape:

{
  "run": {...},
  "repo_results": [
    {
      "repo_id": "hermes-agent",
      "full_name": "NousResearch/hermes-agent",
      "track": "agent-runtimes",
      "task_profile": "repo-fingerprint",
      "outcome": "measured",
      "decision": "ADOPT",
      "effort": "S",
      "confidence": 0.59,
      "evidence_count": 3,
      "exact_files_cited": 0,
      "stale_repo": false,
      "elapsed_s": 1.08,
      "notes": "metadata+README+root listing"
    }
  ]
}

The script also accepts the raw scorecard output from `ouroboros_scorecard.py`
and wraps it automatically.

The script writes:
- one `ouroboros_run` row
- one `ouroboros_repo_eval` row per result

Use `--dry-run` to print the generated SurrealQL instead of posting it.
"""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


def load_env_file(path: Path) -> None:
    if not path.exists():
        return
    with path.open() as fh:
        for raw in fh:
            line = raw.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            os.environ.setdefault(key.strip(), value.strip().strip('"').strip("'"))


def load_runtime_env() -> None:
    load_env_file(Path.home() / ".cynic-env")
    load_env_file(Path.home() / ".config" / "cynic" / "env")


def surreal_escape(value: str) -> str:
    return (
        value.replace("\\", "\\\\")
        .replace("'", "\\'")
        .replace("`", "\\`")
        .replace("\0", "")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


def sanitize_record_id(value: str) -> str:
    out: list[str] = []
    for char in value[:256]:
        if char.isalnum() or char == "_":
            out.append(char)
        elif char == "%":
            out.append("%25")
        elif ord(char) < 128:
            out.append(f"%{ord(char):02x}")
        else:
            for byte in char.encode("utf-8"):
                out.append(f"%{byte:02x}")
    return "".join(out)


def sql_string(value: str | None) -> str:
    if value is None:
        return "NONE"
    return f"'{surreal_escape(value)}'"


def sql_bool(value: bool | None) -> str:
    if value is None:
        return "NONE"
    return "true" if value else "false"


def sql_number(value: int | float | None) -> str:
    if value is None:
        return "NONE"
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def sql_datetime(value: str | None) -> str:
    if value is None:
        return "NONE"
    return f"d'{surreal_escape(value)}'"


def exact_files_count(value: object) -> int:
    if value is None:
        return 0
    if isinstance(value, list):
        return len(value)
    if isinstance(value, tuple):
        return len(value)
    if isinstance(value, dict):
        return len(value)
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, (int, float)):
        return int(value)
    return int(value)


def confidence_value(value: object) -> float:
    if value is None:
        return 0.0
    if isinstance(value, bool):
        return float(int(value))
    if isinstance(value, (int, float)):
        return float(value)
    return float(value)


def elapsed_seconds(value: object) -> float:
    if value is None:
        return 0.0
    if isinstance(value, bool):
        return float(int(value))
    if isinstance(value, (int, float)):
        return float(value)
    return float(value)


def normalize_payload(payload: dict) -> dict:
    if "run" in payload:
        return payload

    run = dict(payload)
    repo_results = run.pop("repo_results", None)
    return {
        "run": run,
        "repo_results": repo_results,
    }


def resolve_run(payload: dict) -> dict:
    run = dict(payload["run"])
    run.setdefault("status", "planned")

    started_at = os.environ.get("OUROBOROS_STARTED_AT") or run.get("started_at") or run.get("generated_at")
    finished_at = os.environ.get("OUROBOROS_FINISHED_AT") or run.get("finished_at") or started_at
    duration_s = os.environ.get("OUROBOROS_DURATION_S")
    if duration_s is not None:
        try:
            duration_s = float(duration_s)
        except ValueError:
            duration_s = None
    elif run.get("duration_s") is not None:
        duration_s = run.get("duration_s")
    elif started_at and finished_at:
        try:
            start_dt = dt.datetime.fromisoformat(started_at.replace("Z", "+00:00"))
            finish_dt = dt.datetime.fromisoformat(finished_at.replace("Z", "+00:00"))
            duration_s = round((finish_dt - start_dt).total_seconds(), 3)
        except ValueError:
            duration_s = None

    run["started_at"] = started_at or dt.datetime.now(dt.timezone.utc).isoformat()
    run["finished_at"] = finished_at or run["started_at"]
    if duration_s is not None:
        run["duration_s"] = duration_s

    env_status = os.environ.get("OUROBOROS_STATUS")
    if env_status:
        run["status"] = env_status

    run.setdefault("trigger", "nightly")
    run.setdefault("mode", "landscape")
    run.setdefault("agent_family", os.environ.get("OUROBOROS_AGENT_FAMILY") or infer_agent_family(run))
    run.setdefault("agent_id", os.environ.get("OUROBOROS_AGENT_ID", "hermes"))
    run.setdefault("model", os.environ.get("OUROBOROS_MODEL", "unknown"))
    run.setdefault("backend_id", os.environ.get("OUROBOROS_BACKEND_ID", "unknown"))
    run.setdefault("corpus_version", os.environ.get("OUROBOROS_CORPUS_VERSION", "0.1"))
    return run


def infer_agent_family(run: dict) -> str:
    agent_id = str(run.get("agent_id") or "").strip().lower()
    model = str(run.get("model") or "").strip().lower()
    backend_id = str(run.get("backend_id") or "").strip().lower()
    for candidate in (agent_id, model, backend_id):
        if "openclaude" in candidate:
            return "openclaude"
        if "claude" in candidate:
            return "claude-code"
        if "gemini" in candidate:
            return "gemini-cli"
        if "hermes" in candidate:
            return "hermes"
    return os.environ.get("OUROBOROS_AGENT_FAMILY") or "hermes"


def build_run_sql(payload: dict) -> str:
    run = payload["run"]
    run_id = run["run_id"]
    safe_key = sanitize_record_id(run_id)
    status = run.get("status") or ("completed" if payload.get("repo_results") else "planned")
    summary_json = json.dumps(
        {
            "run": run,
            "repo_results": resolve_repo_results(payload),
        },
        separators=(",", ":"),
        ensure_ascii=True,
    )
    repos_attempted = int(run.get("scorecard", {}).get("reliability", {}).get("repos_attempted", 0))
    repos_completed = int(run.get("scorecard", {}).get("reliability", {}).get("repos_completed", 0))
    tool_failures = int(run.get("scorecard", {}).get("reliability", {}).get("tool_failures", 0))
    prompt_tokens = int(run.get("scorecard", {}).get("efficiency", {}).get("prompt_tokens", 0))
    completion_tokens = int(run.get("scorecard", {}).get("efficiency", {}).get("completion_tokens", 0))
    internal_patch_candidates = int(run.get("scorecard", {}).get("actionability", {}).get("internal_patch_candidates", 0))
    internal_patches_authored = int(run.get("scorecard", {}).get("actionability", {}).get("internal_patches_authored", 0))
    tests_run = int(run.get("scorecard", {}).get("actionability", {}).get("tests_run", 0))
    tests_passed = int(run.get("scorecard", {}).get("actionability", {}).get("tests_passed", 0))
    duration_s = run.get("duration_s")
    duration_s = run.get("duration_s")

    return (
        f"UPSERT ouroboros_run:`{safe_key}` SET "
        f"run_id = {sql_string(run_id)}, "
        f"corpus_version = {sql_string(run.get('corpus_version'))}, "
        f"trigger = {sql_string(run.get('trigger'))}, "
        f"mode = {sql_string(run.get('mode'))}, "
        f"agent_family = {sql_string(run.get('agent_family'))}, "
        f"status = {sql_string(status)}, "
        f"started_at = {sql_datetime(run['started_at'])}, "
        f"finished_at = {sql_datetime(run['finished_at'])}, "
        f"duration_s = {sql_number(duration_s)}, "
        f"agent_id = {sql_string(run.get('agent_id'))}, "
        f"model = {sql_string(run.get('model'))}, "
        f"backend_id = {sql_string(run.get('backend_id'))}, "
        f"repos_attempted = {repos_attempted}, "
        f"repos_completed = {repos_completed}, "
        f"tool_failures = {tool_failures}, "
        f"prompt_tokens = {prompt_tokens}, "
        f"completion_tokens = {completion_tokens}, "
        f"internal_patch_candidates = {internal_patch_candidates}, "
        f"internal_patches_authored = {internal_patches_authored}, "
        f"tests_run = {tests_run}, "
        f"tests_passed = {tests_passed}, "
        f"summary_json = {sql_string(summary_json)};"
    )


def resolve_repo_results(payload: dict) -> list[dict]:
    repo_results = list(payload.get("repo_results") or [])
    if repo_results:
        return repo_results

    run = payload["run"]
    selected = run.get("selected_repos") or []
    planned = []
    for repo in selected:
        planned.append(
            {
                "repo_id": repo["repo_id"],
                "full_name": repo["full_name"],
                "track": repo["track"],
                "task_profile": "repo-fingerprint",
                "outcome": "planned",
                "decision": "PENDING",
                "effort": "S",
                "confidence": 0.0,
                "evidence_count": 0,
                "exact_files_cited": 0,
                "stale_repo": False,
                "elapsed_s": 0.0,
                "notes": "planned nightly slot",
            }
        )
    return planned


def build_repo_sql(payload: dict) -> str:
    run = payload["run"]
    run_id = run["run_id"]
    created_at = dt.datetime.now(dt.timezone.utc).isoformat()
    statements: list[str] = []
    for row in resolve_repo_results(payload):
        key = sanitize_record_id(f"{run_id}__{row['repo_id']}__{row.get('task_profile', 'repo-fingerprint')}")
        statements.append(
            "UPSERT ouroboros_repo_eval:`{key}` SET "
            "run_id = {run_id}, "
            "agent_family = {agent_family}, "
            "repo_id = {repo_id}, "
            "full_name = {full_name}, "
            "track = {track}, "
            "task_profile = {task_profile}, "
            "outcome = {outcome}, "
            "decision = {decision}, "
            "effort = {effort}, "
            "confidence = {confidence}, "
            "evidence_count = {evidence_count}, "
            "exact_files_cited = {exact_files_cited}, "
            "stale_repo = {stale_repo}, "
            "elapsed_s = {elapsed_s}, "
            "created_at = {created_at}, "
            "notes = {notes};".format(
                key=key,
                run_id=sql_string(run_id),
                agent_family=sql_string(run.get("agent_family")),
                repo_id=sql_string(row["repo_id"]),
                full_name=sql_string(row["full_name"]),
                track=sql_string(row["track"]),
                task_profile=sql_string(row.get("task_profile", "repo-fingerprint")),
                outcome=sql_string(row.get("outcome", "measured")),
                decision=sql_string(row.get("decision")),
                effort=sql_string(row.get("effort")),
                confidence=sql_number(confidence_value(row.get("confidence"))),
                evidence_count=int(row.get("evidence_count", 0)),
                exact_files_cited=exact_files_count(row.get("exact_files_cited", 0)),
                stale_repo=sql_bool(row.get("stale_repo")),
                elapsed_s=sql_number(elapsed_seconds(row.get("elapsed_s"))),
                created_at=sql_datetime(row.get("created_at", created_at)),
                notes=sql_string(row.get("notes", "")),
            )
        )
    return "\n".join(statements)


def post_sql(sql: str) -> None:
    base_url = os.environ.get("SURREALDB_URL", "http://127.0.0.1:8000").rstrip("/")
    ns = os.environ.get("SURREALDB_NS", "cynic")
    db = os.environ.get("SURREALDB_DB", "main")
    user = os.environ.get("SURREALDB_USER", "root")
    password = os.environ.get("SURREALDB_PASS")
    if not password:
        raise SystemExit("SURREALDB_PASS must be set")

    token = base64.b64encode(f"{user}:{password}".encode("utf-8")).decode("ascii")
    request = urllib.request.Request(
        f"{base_url}/sql",
        data=sql.encode("utf-8"),
        method="POST",
        headers={
            "Accept": "application/json",
            "Content-Type": "text/plain; charset=utf-8",
            "surreal-ns": ns,
            "surreal-db": db,
            "Authorization": f"Basic {token}",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            body = response.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raise SystemExit(f"SurrealDB write failed: HTTP {exc.code}: {exc.read().decode('utf-8', 'replace')}") from exc
    except urllib.error.URLError as exc:
        raise SystemExit(f"SurrealDB write failed: {exc}") from exc

    print(body)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", help="Path to JSON payload. Defaults to stdin.")
    parser.add_argument("--dry-run", action="store_true", help="Print SQL instead of writing to SurrealDB.")
    args = parser.parse_args()

    load_runtime_env()

    if args.input:
        with Path(args.input).open() as fh:
            payload = json.load(fh)
    else:
        if sys.stdin.isatty():
            raise SystemExit("Expected JSON on stdin or --input PATH")
        payload = json.load(sys.stdin)

    payload = normalize_payload(payload)
    payload["run"] = resolve_run(payload)
    run_sql = build_run_sql(payload)
    repo_sql = build_repo_sql(payload)
    sql = "\n".join(part for part in [run_sql, repo_sql] if part)

    if args.dry_run:
        print(sql)
        return

    post_sql(sql)


if __name__ == "__main__":
    main()
