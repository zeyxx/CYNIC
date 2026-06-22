#!/usr/bin/env python3
"""Reconcile Discord organ desired state into runtime/generated state.

Runbook order: read runbook -> read desired state -> run this controller -> verify.
This controller is idempotent and does not require the CYNIC kernel.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
WORKSPACE = ROOT.parent
DEFAULT_DESIRED = ROOT / "organs/discord/desired/discord-organ.json"
RUNTIME_CONFIG = Path.home() / ".cynic/organs/discord/config.yaml"
CHROME_POLICY = Path("/etc/opt/chrome/policies/managed/cynic-discord-proxy.json")
TOPOLOGY = ROOT / "organs/discord/state/topology.json"
WATCH = ROOT / "organs/discord/state/percolator-watch.json"
SSOT = ROOT / "organs/discord/state/SSOT.json"
AUTONOMY = ROOT / "organs/discord/state/autonomy.json"


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write(path: Path, content: str, dry_run: bool) -> bool:
    path.parent.mkdir(parents=True, exist_ok=True)
    current = path.read_text(encoding="utf-8") if path.exists() else None
    if current == content:
        return False
    if not dry_run:
        path.write_text(content, encoding="utf-8")
    return True


def runtime_yaml(desired: dict[str, Any]) -> str:
    runtime = desired["runtime"]
    channels = desired["surfaces"]["channels"]
    allowed = [item["discord_channel_id"] for item in channels if item.get("capture_policy") == "messages_allowlisted"]
    lines = [
        f"db_path: {runtime['db_path']}",
        f"heartbeat_interval_seconds: {runtime['heartbeat_interval_seconds']}",
        "allowed_channel_ids:",
    ]
    lines.extend([f"  - {channel_id}" for channel_id in allowed])
    lines.extend(
        [
            f"capture_all_discord_metadata: {str(runtime['capture_all_discord_metadata']).lower()}",
            f"store_raw_events: {str(runtime['store_raw_events']).lower()}",
            "raw_event_allowlist: []" if not runtime.get("raw_event_allowlist") else "raw_event_allowlist:",
            "retention:",
            f"  raw_days: {runtime['retention']['raw_days']}",
        ]
    )
    return "\n".join(lines) + "\n"


def topology_json(desired: dict[str, Any]) -> str:
    channels = desired["surfaces"]["channels"]
    servers = desired["surfaces"]["servers"]
    payload = {
        "schema_version": "1.0.0",
        "organ": "discord",
        "status": "generated_from_desired_state",
        "source_of_truth": desired["source_of_truth"],
        "runtime_database": desired["runtime"]["db_path"],
        "capture_mode": "passive_proxy_allowlist_only",
        "identities": [],
        "servers": servers,
        "groups": [],
        "direct_contacts": [],
        "channels": channels,
        "watch_profiles": desired["watch_profiles"],
        "policies": {
            "default_server_capture_policy": "disabled",
            "default_group_capture_policy": "disabled",
            "default_direct_contact_capture_policy": "disabled",
            "message_capture_requires_explicit_channel_allowlist": True,
            "private_content_export_policy": "local_only"
        },
        "mapping": {
            "discord_guild_id_to_server_id": {server["discord_guild_id"]: server["server_id"] for server in servers},
            "discord_channel_id_to_channel_id": {channel["discord_channel_id"]: channel["channel_id"] for channel in channels},
            "discord_user_id_to_identity_id": {}
        }
    }
    return json.dumps(payload, indent=2, ensure_ascii=True) + "\n"


def watch_json(desired: dict[str, Any]) -> str:
    profile = desired["watch_profiles"][0]
    payload = {
        "schema_version": "1.0.0",
        "watch_profile_id": profile["watch_profile_id"],
        "purpose": "Map Discord social graph surfaces to Percolator bounty anti-duplicate intelligence.",
        "inputs": {
            "desired_state": str(DEFAULT_DESIRED.relative_to(WORKSPACE)),
            "messages_db": desired["runtime"]["db_path"],
            "server_ids": [server["discord_guild_id"] for server in desired["surfaces"]["servers"]],
            "channel_ids": [channel["discord_channel_id"] for channel in desired["surfaces"]["channels"]]
        },
        "outputs": {
            "researcher_feed": profile["feed"],
            "finding_board": "percolator/ops/finding-board.json",
            "inference_queue": profile["queue"]
        },
        "dedupe_keys": ["discord_channel_id", "discord_message_id", "content_hash", "author_identity_id", "risk_class"]
    }
    return json.dumps(payload, indent=2, ensure_ascii=True) + "\n"


def ssot_json(desired: dict[str, Any]) -> str:
    payload = {
        "schema_version": "1.0.0",
        "organ": "discord",
        "source_of_truth": "CYNIC/organs/discord/desired/discord-organ.json",
        "runbook": "CYNIC/organs/discord/runbooks/discord-ops.json",
        "controller": "CYNIC/organs/discord/controllers/reconcile_discord.py",
        "generated_views": [
            "CYNIC/organs/discord/state/topology.json",
            "CYNIC/organs/discord/state/percolator-watch.json",
            "CYNIC/organs/discord/state/proxy-lifecycle.json",
            "CYNIC/organs/discord/state/autonomy.json"
        ],
        "runtime_outputs": [
            desired["runtime"]["db_path"],
            "percolator/ops/researcher-feed.json",
            "percolator/ops/infer-queue.jsonl",
            "percolator/ops/infer-queue-state.json"
        ],
        "storage_contract": desired.get("organ_contract", {}).get("storage_contract", {}),
        "rule": "Change desired state first, then run the controller. Do not manually edit generated views unless promoting the change back into desired state."
    }
    return json.dumps(payload, indent=2, ensure_ascii=True) + "\n"


def autonomy_json(desired: dict[str, Any]) -> str:
    contract = desired.get("organ_contract", {})
    usecases = desired.get("usecases", [])
    payload = {
        "schema_version": "1.0.0",
        "organ": "discord",
        "mode": contract.get("authoritative_runtime", "edge_capture_without_kernel_dependency"),
        "principle": "The personal PC captures and structures low-cost data only. Heavy inference work is queued durably and consumed by an always-on worker elsewhere.",
        "local_pc_responsibilities": [
            "passive_discord_capture",
            "sqlite_entity_storage",
            "deterministic_feed_export",
            "durable_inference_task_queue"
        ],
        "not_local_pc_responsibilities": [
            "always_on_llm_inference",
            "night_long_gpu_or_cpu_jobs",
            "bright_screen_dependent_monitoring",
            "manual_discord_refresh_loops"
        ],
        "failure_policy": contract.get("failure_policy", {}),
        "queues": {
            usecase["usecase_id"]: usecase.get("artifact_roots", {}).get("inference_queue")
            for usecase in usecases
            if isinstance(usecase, dict) and usecase.get("usecase_id")
        },
        "consumer_options": [
            "remote_cynic_kernel",
            "headless_home_server",
            "vps_worker",
            "manual_batch_when_available"
        ],
        "sleep_rule": {
            "personal_pc_kernel_required": False,
            "personal_pc_screen_required": False,
            "safe_to_sleep_after_capture": True
        }
    }
    return json.dumps(payload, indent=2, ensure_ascii=True) + "\n"


def chrome_policy_json(desired: dict[str, Any]) -> str:
    payload = {
        "ProxyMode": "pac_script",
        "ProxyPacUrl": desired["proxy"]["pac_url"]
    }
    return json.dumps(payload, indent=2, ensure_ascii=True) + "\n"


def verify(desired: dict[str, Any]) -> dict[str, Any]:
    checks: dict[str, Any] = {}
    expected_channels = sorted(channel["discord_channel_id"] for channel in desired["surfaces"]["channels"])
    checks["runtime_config_exists"] = RUNTIME_CONFIG.exists()
    if RUNTIME_CONFIG.exists():
        text = RUNTIME_CONFIG.read_text(encoding="utf-8")
        checks["runtime_config_channel_ids_present"] = all(channel_id in text for channel_id in expected_channels)
    else:
        checks["runtime_config_channel_ids_present"] = False
    checks["topology_exists"] = TOPOLOGY.exists()
    checks["watch_profile_exists"] = WATCH.exists()
    checks["ssot_exists"] = SSOT.exists()
    checks["autonomy_exists"] = AUTONOMY.exists()
    checks["chrome_policy_exists"] = CHROME_POLICY.exists()
    checks["all_passed"] = all(bool(value) for value in checks.values())
    return checks


def maybe_reload_systemd(dry_run: bool) -> None:
    if dry_run:
        return
    subprocess.run(["systemctl", "--user", "daemon-reload"], check=False)


def main() -> int:
    parser = argparse.ArgumentParser(description="Reconcile Discord organ desired state")
    parser.add_argument("--desired", default=str(DEFAULT_DESIRED))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()

    desired = load_json(Path(args.desired))
    changed = {
        "runtime_config": write(RUNTIME_CONFIG, runtime_yaml(desired), args.dry_run),
        "topology": write(TOPOLOGY, topology_json(desired), args.dry_run),
        "watch_profile": write(WATCH, watch_json(desired), args.dry_run),
        "ssot": write(SSOT, ssot_json(desired), args.dry_run),
        "autonomy": write(AUTONOMY, autonomy_json(desired), args.dry_run),
        "chrome_policy": write(CHROME_POLICY, chrome_policy_json(desired), args.dry_run),
    }
    maybe_reload_systemd(args.dry_run)
    result = {
        "changed": changed,
        "verify": verify(desired) if args.verify else None,
    }
    print(json.dumps(result, indent=2, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
