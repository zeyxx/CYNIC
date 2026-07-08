"""Command handlers for runtime principals."""

from __future__ import annotations

import argparse
import json
import subprocess

from organs.supervisor.src.controllers.actions.dependencies import (
    update_dry_run,
    update_primary_dependencies,
)
from organs.supervisor.src.controllers.actions.lifecycle_activation import (
    activate_lifecycle,
    activation_dry_run as _delegate_activation_dry_run,
    lifecycle_profile_dry_run,
    request_lifecycle_profile as _delegate_request_lifecycle_profile,
)
from organs.supervisor.src.controllers.actions.system_setup import apply, install_defaults
from organs.supervisor.src.controllers.actions.workspace_repos import (
    repos_add,
    repos_remove,
    repos_status,
    repos_sync,
)
from organs.supervisor.src.core.runtime_principals_command_contracts import COMMAND_HELP
from organs.supervisor.src.core.runtime_principals_command_contracts import (
    activation_dry_run_contract as _activation_dry_run_contract,
)
from organs.supervisor.src.core.runtime_principals_command_contracts import (
    native_update_contract as _native_update_contract,
)
from organs.supervisor.src.core.runtime_principals_dispatch import (
    RuntimePrincipalsCommandDispatcher,
    RuntimePrincipalsCommandSpec,
)
from organs.contracts.src.surface_contracts import json_stdout_or_envelope
from organs.supervisor.src.entrypoints.runtime_principals_views import HOME_DIR, LIFECYCLE_PROFILE_COMMANDS
from organs.supervisor.src.entrypoints.runtime_principals_views_organism import (
    activation_dispatch_text,
    organism_actions_report,
    organism_actions_text,
    organism_convergence_dispatch_report,
    organism_convergence_dispatch_text,
    organism_convergence_report,
    organism_convergence_text,
    organism_report,
    organism_text,
    lifecycle_profile_request_text,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime import (
    cynic_cli_body,
    launcher_body,
    profile_body,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime_reports import (
    doctor_text,
    lifecycle_activation_report,
    plan,
    status,
    _run_capture,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_surface import (
    fleet_status,
    infra_report_text,
    lxc_exec_bridge_plan,
    lxc_exec_bridge_status,
    mesh_status,
    storage_primary_bundle,
    storage_primary_plan,
    storage_primary_status,
)


def _wants_json(args: list[str]) -> tuple[bool, list[str]]:
    return "--json" in args, [arg for arg in args if arg != "--json"]


def _emit_json(payload: object) -> int:
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def _emit_text(body: str) -> int:
    print(body, end="")
    return 0


def _runtime_update_contract() -> dict[str, object]:
    execution_posture = status().get("execution_posture", {})
    if not isinstance(execution_posture, dict):
        execution_posture = {}
    return _native_update_contract(execution_posture)


def _handle_command_help(command: str, extra: list[str]) -> bool:
    if not any(arg in {"-h", "--help"} for arg in extra):
        return False
    print(COMMAND_HELP.get(command, f"usage: cynic {command}\n"), end="")
    return True


def update_dry_run() -> dict[str, object]:
    return _runtime_update_contract()


def activation_dry_run() -> dict[str, object]:
    return _activation_dry_run_contract(lifecycle_activation_report())


def _extract_lifecycle_task_ids(payload: dict[str, object]) -> list[str]:
    task_ids: list[str] = []

    direct_task_ids = payload.get("task_ids")
    if isinstance(direct_task_ids, list):
        task_ids.extend(str(task_id) for task_id in direct_task_ids if task_id)

    results = payload.get("results")
    if not isinstance(results, list):
        return task_ids

    for item in results:
        result_payload: object = item
        if isinstance(item, dict) and "result" in item:
            result_payload = item.get("result")
        if isinstance(result_payload, str):
            try:
                result_payload = json.loads(result_payload)
            except json.JSONDecodeError:
                continue
        if isinstance(result_payload, dict):
            task_id = result_payload.get("task_id")
            if task_id:
                task_ids.append(str(task_id))
            nested_task_ids = result_payload.get("task_ids")
            if isinstance(nested_task_ids, list):
                task_ids.extend(str(task_id) for task_id in nested_task_ids if task_id)

    return list(dict.fromkeys(task_ids))


def request_lifecycle_profile(profile: str) -> dict[str, object]:
    if profile not in LIFECYCLE_PROFILE_COMMANDS:
        raise SystemExit(f"ERROR: unknown lifecycle profile: {profile}")
    registry_profile = LIFECYCLE_PROFILE_COMMANDS[profile]
    command = [
        "python3",
        "organs/forge/src/entrypoints/cli.py",
        "delegate-profile",
        registry_profile,
        "--json",
    ]
    timeout_seconds = 30
    result = _run_capture(
        command,
        timeout=timeout_seconds,
        stdout_limit=None,
        stderr_limit=None,
    )
    stderr_payload: dict[str, object] = {}
    if result["status"] != "ok":
        try:
            parsed_stderr = json.loads(str(result.get("stderr", "") or "") or "null")
        except json.JSONDecodeError:
            parsed_stderr = None
        if isinstance(parsed_stderr, dict):
            stderr_payload = parsed_stderr
    payload = (
        stderr_payload
        if stderr_payload
        else json_stdout_or_envelope(
            result,
            schema=f"cynic.lifecycle.profile_dry_run.{profile}.v1",
            stdout_transform=str,
        )
    )
    if not isinstance(payload, dict):
        payload = {}
    payload.setdefault("status", result["status"])
    payload["profile"] = registry_profile
    payload["operator_command"] = profile
    payload["command"] = f"cynic {profile}"
    payload["inspect_command"] = f"cynic {profile} --dry-run"
    payload["dispatch_json_command"] = f"cynic {profile} --json"
    payload["operator_sequence"] = [
        {"step": "inspect", "command": f"cynic {profile} --dry-run", "mutates_mempool": False},
        {"step": "dispatch", "command": f"cynic {profile}", "mutates_mempool": True},
        {"step": "dispatch_json", "command": f"cynic {profile} --json", "mutates_mempool": True},
        {"step": "observe", "command": "cynic organism", "mutates_mempool": False},
    ]
    payload["proof_commands"] = ["cynic actions", "cynic organism"]
    payload["read_back"] = "dispatch_json_command must return task ids or duplicate pending/completed Kernel task visibility"
    payload["fallback_command"] = " ".join(command)
    payload["mutates_mempool"] = True
    payload["task_ids"] = _extract_lifecycle_task_ids(payload)
    if result["status"] == "timeout" and not stderr_payload:
        payload.setdefault(
            "error",
            "delegate-profile timed out before returning Kernel read-back",
        )
        payload.setdefault("error_kind", "dispatch_timeout")
        payload.setdefault(
            "operator_action",
            "inspect cynic organism --json and cynic actions --json before retrying the lifecycle dispatch",
        )
        payload["timeout_seconds"] = timeout_seconds
    return payload


def print_launchers() -> str:
    return (
        f"--- {HOME_DIR}/bin/codex-yolo ---\n"
        f"{launcher_body('codex')}"
        f"--- {HOME_DIR}/bin/antigravity-yolo ---\n"
        f"{launcher_body('agy')}"
        f"--- {HOME_DIR}/bin/hermes-yolo ---\n"
        f"{launcher_body('hermes')}"
    )


def _handle_status(extra: list[str]) -> int:
    del extra
    return _emit_json(status())


def _handle_doctor(extra: list[str]) -> int:
    del extra
    return _emit_text(doctor_text())


def _handle_plan(extra: list[str]) -> int:
    del extra
    return _emit_json(plan())


def _handle_update(extra: list[str]) -> int:
    if "--dry-run" in extra:
        return _emit_json(update_dry_run())
    if "--bootstrap" in extra:
        bootstrap_extra = [arg for arg in extra if arg != "--bootstrap"]
        return _handle_bootstrap(bootstrap_extra)
    return _emit_json(update_primary_dependencies())


def _bootstrap_dry_run() -> dict[str, object]:
    return {
        "schema": "cynic.runtime_principals.bootstrap_plan.v1",
        "command": "cynic bootstrap --dry-run",
        "dispatch_command": "cynic bootstrap",
        "dispatch_json_command": "cynic bootstrap --json",
        "mutates": False,
        "operator_sequence": [
            {"step": "inspect", "command": "cynic update --dry-run", "mutates_process": False},
            {"step": "update", "command": "cynic update", "mutates_process": True},
            {"step": "apply", "command": "cynic apply", "mutates_host": True},
            {"step": "install_defaults", "command": "cynic install-defaults", "mutates_host": True},
            {"step": "prove", "command": "sudo cynic status --json", "mutates_host": False},
        ],
        "proof_commands": ["sudo cynic status --json"],
        "done_when": [
            "cynic update succeeds for every declared primary CLI",
            "cynic apply updates the dedicated cynic runtime projection",
            "cynic install-defaults installs profile aliases",
            "sudo cynic status --json reports runtime_posture.native_default = true",
        ],
        "fallback_command": "python3 organs/supervisor/entrypoints/runtime_principals.py bootstrap",
    }


def _handle_bootstrap(extra: list[str]) -> int:
    if "--dry-run" in extra:
        return _emit_json(_bootstrap_dry_run())
    results = {
        "schema": "cynic.runtime_principals.bootstrap_result.v1",
        "command": "cynic bootstrap",
        "dispatch_command": "cynic bootstrap",
        "dispatch_json_command": "cynic bootstrap --json",
        "mutates": True,
        "operator_sequence": [
            {"step": "update", "command": "cynic update", "mutates_process": True},
            {"step": "apply", "command": "cynic apply", "mutates_host": True},
            {"step": "install_defaults", "command": "cynic install-defaults", "mutates_host": True},
            {"step": "prove", "command": "sudo cynic status --json", "mutates_host": False},
        ],
        "proof_commands": ["sudo cynic status --json"],
        "update": update_primary_dependencies(),
        "apply": apply(),
        "install_defaults": install_defaults(),
    }
    results["fallback_command"] = "python3 organs/supervisor/entrypoints/runtime_principals.py bootstrap --dry-run"
    return _emit_json(results)


def _handle_organism(extra: list[str]) -> int:
    wants_json, _ = _wants_json(extra)
    if "--summary" in extra:
        return _emit_json(_organism_summary(organism_report()))
    if wants_json:
        return _emit_json(organism_report())
    return _emit_text(organism_text())


def _handle_actions(extra: list[str]) -> int:
    wants_json, _ = _wants_json(extra)
    report = organism_actions_report()
    if wants_json:
        return _emit_json(report)
    return _emit_text(organism_actions_text(report))


def _compact_blocker(blocker: object) -> dict[str, object]:
    if not isinstance(blocker, dict):
        return {"status": "unknown", "detail": str(blocker)}
    compact: dict[str, object] = {}
    for key in ("surface", "entity", "kind", "status", "health", "reason", "check", "next_action"):
        if key in blocker and blocker.get(key) is not None:
            compact[key] = blocker[key]
    missing = blocker.get("missing")
    if isinstance(missing, list) and missing:
        compact["missing"] = missing[:5]
    next_checks = blocker.get("next_checks")
    if isinstance(next_checks, list) and next_checks:
        compact["next_checks"] = [item for item in next_checks[:5] if item]
    return compact


def _compact_action(action: object) -> dict[str, object]:
    if not isinstance(action, dict):
        return {"action": str(action)}
    compact: dict[str, object] = {}
    for key in ("action", "entity", "node", "role", "consumer", "priority", "next_action"):
        if key in action and action.get(key) is not None:
            compact[key] = action[key]
    return compact


def _organism_summary(report: dict[str, object]) -> dict[str, object]:
    actions_report = report.get("actions")
    action_items: list[object] = []
    if isinstance(actions_report, dict):
        raw_actions = actions_report.get("actions", [])
        if isinstance(raw_actions, list):
            action_items.extend(raw_actions)

    stack = report.get("stack")
    if isinstance(stack, dict):
        raw_stack_actions = stack.get("actions", [])
        if isinstance(raw_stack_actions, list):
            action_items.extend(raw_stack_actions)

    blockers: list[object] = []
    raw_blockers = report.get("blockers", [])
    if isinstance(raw_blockers, list):
        blockers.extend(raw_blockers)
    if isinstance(stack, dict):
        raw_stack_blockers = stack.get("blockers", [])
        if isinstance(raw_stack_blockers, list):
            blockers.extend(raw_stack_blockers)

    raw_next_checks = report.get("next_checks", [])
    next_checks = [str(item) for item in raw_next_checks if item and item != "None"] if isinstance(raw_next_checks, list) else []

    status = report.get("status")
    if not status and isinstance(stack, dict):
        status = stack.get("status")
    if not status:
        status = "blocked" if blockers else "ok"

    return {
        "schema": "cynic.organism.summary.v1",
        "status": status,
        "action_count": len(action_items),
        "blocker_count": len(blockers),
        "actions": [_compact_action(item) for item in action_items[:5]],
        "blockers": [_compact_blocker(item) for item in blockers[:8]],
        "next_checks": next_checks[:8],
        "machine_output": "cynic organism --json",
    }


def _handle_converge(extra: list[str]) -> int:
    wants_json, _ = _wants_json(extra)
    if "--dispatch" in extra:
        report = organism_convergence_dispatch_report()
        if wants_json:
            return _emit_json(report)
        return _emit_text(organism_convergence_dispatch_text(report))
    report = organism_convergence_report()
    if wants_json or "--dry-run" in extra:
        return _emit_json(report)
    return _emit_text(organism_convergence_text(report))


def _handle_activate(extra: list[str]) -> int:
    if "--dry-run" in extra:
        return _emit_json(activation_dry_run())
    report = activate_lifecycle()
    if "--json" in extra:
        return _emit_json(report)
    return _emit_text(activation_dispatch_text(report))


def _handle_lifecycle_profile(profile: str, extra: list[str]) -> int:
    registry_profile = LIFECYCLE_PROFILE_COMMANDS.get(profile, profile)
    if "--dry-run" in extra:
        report = lifecycle_profile_dry_run(registry_profile)
        if isinstance(report, dict):
            report.setdefault("operator_command", profile)
            report.setdefault("command", f"cynic {profile} --dry-run")
            report.setdefault("dispatch_command", f"cynic {profile}")
            report.setdefault("dispatch_json_command", f"cynic {profile} --json")
        return _emit_json(report)
    report = request_lifecycle_profile(profile)
    if "--json" in extra:
        return _emit_json(report)
    return _emit_text(lifecycle_profile_request_text(report))


def _handle_storage(extra: list[str]) -> int:
    storage_command = extra[0] if extra else "status"
    if storage_command == "plan":
        return _emit_json(storage_primary_plan())
    if storage_command == "bundle":
        return _emit_json(storage_primary_bundle(extra[1:]))
    if storage_command == "status":
        wants_json, passthrough = _wants_json(extra[1:])
        report = storage_primary_status(passthrough)
        if wants_json:
            return _emit_json(report)
        return _emit_text(
            infra_report_text(report, title="CYNIC storage primary status", json_command="cynic storage status --json")
        )
    raise SystemExit("ERROR: usage: cynic storage [plan|bundle|status]")


def _handle_bridge(extra: list[str]) -> int:
    bridge_command = extra[0] if extra else "status"
    if bridge_command == "plan":
        return _emit_json(lxc_exec_bridge_plan())
    if bridge_command == "status":
        wants_json, passthrough = _wants_json(extra[1:])
        if passthrough:
            raise SystemExit("ERROR: usage: cynic bridge status [--json]")
        report = lxc_exec_bridge_status()
        if wants_json:
            return _emit_json(report)
        return _emit_text(infra_report_text(report, title="CYNIC bridge status", json_command="cynic bridge status --json"))
    raise SystemExit("ERROR: cynic bridge [plan|status]")


def _handle_fleet(extra: list[str]) -> int:
    fleet_command = extra[0] if extra else "status"
    if fleet_command == "status":
        wants_json, passthrough = _wants_json(extra[1:])
        report = fleet_status(passthrough)
        if wants_json:
            return _emit_json(report)
        return _emit_text(infra_report_text(report, title="CYNIC fleet status", json_command="cynic fleet status --json"))
    raise SystemExit("ERROR: usage: cynic fleet status [organ-fleet report args...]")


def _handle_mesh(extra: list[str]) -> int:
    mesh_command = extra[0] if extra else "status"
    if mesh_command == "status":
        wants_json, passthrough = _wants_json(extra[1:])
        report = mesh_status(passthrough)
        if wants_json:
            return _emit_json(report)
        return _emit_text(infra_report_text(report, title="CYNIC mesh status", json_command="cynic mesh status --json"))
    raise SystemExit("ERROR: usage: cynic mesh status [mesh status args...]")


def _handle_infra(extra: list[str]) -> int:
    infra_command = extra[0] if extra else "status"
    if infra_command == "provision":
        if len(extra) < 2:
            raise SystemExit("ERROR: usage: cynic infra provision <target>")
        target = extra[1]
        command = [
            "python3",
            "-m",
            "organs.lifecycle.src.entrypoints.cli",
            "provision",
            "--target",
            target,
            "--registry",
            "infra/registry.json",
        ]
        return subprocess.run(command, check=False).returncode
    raise SystemExit("ERROR: usage: cynic infra [provision <target>]")


def _handle_repos(extra: list[str]) -> int:
    if not extra:
        raise SystemExit("ERROR: usage: cynic repos <status|add|remove|sync>")
    repos_command = extra[0]
    repos_args = extra[1:]
    if repos_command == "status":
        return _emit_json(repos_status(repos_args))
    if repos_command == "add":
        return _emit_json(repos_add(repos_args))
    if repos_command == "remove":
        return _emit_json(repos_remove(repos_args))
    if repos_command == "sync":
        return _emit_json(repos_sync())
    raise SystemExit("ERROR: usage: cynic repos <status|add|remove|sync>")


def _handle_apply(extra: list[str]) -> int:
    del extra
    return _emit_json(apply())


def _handle_install_defaults(extra: list[str]) -> int:
    del extra
    return _emit_json(install_defaults())


def _handle_print_launchers(extra: list[str]) -> int:
    del extra
    return _emit_text(print_launchers())


def _handle_print_profile(extra: list[str]) -> int:
    del extra
    return _emit_text(profile_body())


def _handle_print_cynic_cli(extra: list[str]) -> int:
    del extra
    return _emit_text(cynic_cli_body())


def _handicap_legacy_stub() -> int:
    print("ERROR: This command has been migrated to the cognitive sensor runtime: crates/cynic-cognitive-sensor.")
    print("Physical I/O and cognitive analysis are now decoupled from Python organs.")
    return 1


def _hf_search(args: list[str]) -> int:
    import requests
    if not args:
        print("usage: cynic hf search <models|datasets|spaces> <query>")
        return 1
        
    repo_type = args[0]
    if repo_type not in ("models", "datasets", "spaces"):
        query = " ".join(args)
        repo_type = "models"
    else:
        query = " ".join(args[1:])
        
    if not query.strip():
        print("ERROR: Search query cannot be empty.")
        return 1
        
    url = f"https://huggingface.co/api/{repo_type}?search={query}&limit=10&sort=downloads&direction=-1"
    print(f"Searching Hugging Face {repo_type} for: '{query}'...")
    
    try:
        res = requests.get(url, timeout=15)
        if res.status_code != 200:
            print(f"ERROR: Failed to search HF ({res.status_code}): {res.text}")
            return 1
            
        results = res.json()
        if not results:
            print("No matches found.")
            return 0
            
        print("\n" + "═" * 90)
        print(f"{'Repository ID':<50} | {'Downloads':<15} | {'Likes':<10}")
        print("═" * 90)
        for r in results:
            rid = r.get("id", "N/A")
            if len(rid) > 48:
                rid = rid[:45] + "..."
            downloads = f"{r.get('downloads', 0):,}"
            likes = f"{r.get('likes', 0):,}"
            print(f"{rid:<50} | {downloads:<15} | {likes:<10}")
        print("═" * 90 + "\n")
        return 0
    except Exception as e:
        print(f"ERROR: Failed to query Hugging Face: {e}")
        return 1


def _hf_info(args: list[str]) -> int:
    import requests
    if not args:
        print("usage: cynic hf info <models|datasets|spaces> <repo_id>")
        return 1
        
    repo_type = args[0]
    if repo_type not in ("models", "datasets", "spaces"):
        repo_id = " ".join(args)
        repo_type = "models"
    else:
        repo_id = " ".join(args[1:])
        
    if not repo_id.strip():
        print("ERROR: Repository ID cannot be empty.")
        return 1
        
    url = f"https://huggingface.co/api/{repo_type}/{repo_id}"
    print(f"Fetching Hugging Face {repo_type[:-1]} info for: {repo_id}...")
    
    try:
        res = requests.get(url, timeout=15)
        if res.status_code != 200:
            print(f"ERROR: Failed to fetch info from HF ({res.status_code}): {res.text}")
            return 1
            
        data = res.json()
        print("\n" + "═" * 70)
        print(f"🤗 Hugging Face {repo_type[:-1].capitalize()}: {data.get('id', repo_id)}")
        print("═" * 70)
        print(f"Author:        {data.get('author', 'Unknown')}")
        print(f"Downloads:     {data.get('downloads', 0):,}")
        print(f"Likes:         {data.get('likes', 0):,}")
        if repo_type == "models":
            print(f"Library Name:  {data.get('library_name', 'N/A')}")
            print(f"Pipeline Tag:  {data.get('pipeline_tag', 'N/A')}")
            
        tags = data.get("tags", [])
        if tags:
            print(f"Tags:          {', '.join(tags[:10])}" + ("..." if len(tags) > 10 else ""))
            
        siblings = data.get("siblings", [])
        if siblings:
            print(f"Files:         " + ", ".join([s.get("rfilename") for s in siblings[:5] if s.get("rfilename")]) + ("..." if len(siblings) > 5 else ""))
            
        print("═" * 70 + "\n")
        return 0
    except Exception as e:
        print(f"ERROR: Failed to connect to Hugging Face: {e}")
        return 1


def _hf_download(args: list[str]) -> int:
    import os
    import requests
    from pathlib import Path
    
    if len(args) < 2:
        print("usage: cynic hf download <repo_id> <filename> [--out <dest_path>] [--dataset]")
        return 1
        
    repo_id = args[0]
    filename = args[1]
    
    dest_path = None
    is_dataset = False
    
    remaining = args[2:]
    if "--dataset" in remaining:
        is_dataset = True
        remaining = [r for r in remaining if r != "--dataset"]
        
    for i, arg in enumerate(remaining):
        if arg == "--out" and i + 1 < len(remaining):
            dest_path = remaining[i+1]
            
    if not dest_path:
        dest_path = os.path.basename(filename)
        
    if is_dataset:
        url = f"https://huggingface.co/datasets/{repo_id}/resolve/main/{filename}"
    else:
        url = f"https://huggingface.co/{repo_id}/resolve/main/{filename}"
        
    print(f"Downloading {filename} from {repo_id}...")
    print(f"URL: {url}")
    
    try:
        headers = {}
        hf_token = os.environ.get("HF_API_KEY") or os.environ.get("HF_TOKEN")
        if hf_token:
            headers["Authorization"] = f"Bearer {hf_token}"
            
        response = requests.get(url, stream=True, headers=headers, timeout=30)
        if response.status_code != 200:
            print(f"ERROR: Failed to download file from HF ({response.status_code}): {response.text}")
            return 1
            
        total_size = int(response.headers.get('content-length', 0))
        block_size = 1024 * 1024
        
        dest = Path(dest_path)
        print(f"Saving to: {dest}")
        downloaded = 0
        with open(dest, "wb") as f:
            for chunk in response.iter_content(chunk_size=block_size):
                if chunk:
                    f.write(chunk)
                    downloaded += len(chunk)
                    if total_size:
                        percent = (downloaded / total_size) * 100
                        print(f"Progress: {downloaded / (1024*1024):.1f} MB / {total_size / (1024*1024):.1f} MB ({percent:.1f}%)", end="\r")
                    else:
                        print(f"Progress: {downloaded / (1024*1024):.1f} MB downloaded", end="\r")
        print("\n✓ Download completed successfully!")
        return 0
    except Exception as e:
        print(f"ERROR: Download failed: {e}")
        return 1


def _handle_hf(extra: list[str]) -> int:
    if not extra:
        print("usage: cynic hf [search|info|download] [args...]")
        print("commands:")
        print("  search <models|datasets|spaces> <query>  Search the Hugging Face Hub")
        print("  info <models|datasets|spaces> <id>       Show details of a repository")
        print("  download <repo_id> <filename> [--out <dest>] Download a file from a repository")
        return 1
        
    subcmd = extra[0]
    args = extra[1:]
    
    if subcmd == "search":
        return _hf_search(args)
    elif subcmd == "info":
        return _hf_info(args)
    elif subcmd == "download":
        return _hf_download(args)
    else:
        if "/" in subcmd:
            return _hf_info(["models", subcmd] + args)
        else:
            return _hf_search(["models", subcmd] + args)


def _handle_handicap(extra: list[str]) -> int:
    if not extra:
        print("usage: cynic handicap [hf]")
        return _handicap_legacy_stub()
    
    subcmd = extra[0]
    args = extra[1:]
    
    if subcmd == "hf":
        return _handle_hf(args)
    else:
        return _handicap_legacy_stub()





def _build_command_specs() -> list[RuntimePrincipalsCommandSpec]:
    specs: list[RuntimePrincipalsCommandSpec] = [
        RuntimePrincipalsCommandSpec("status", _handle_status, mutates=False, json_output=True, usage="status"),
        RuntimePrincipalsCommandSpec("doctor", _handle_doctor, mutates=False, json_output=False, usage="doctor"),
        RuntimePrincipalsCommandSpec("plan", _handle_plan, mutates=False, json_output=True, usage="plan"),
        RuntimePrincipalsCommandSpec("update", _handle_update, mutates=True, json_output=True, usage="update [--dry-run] [--bootstrap] [--json]"),
        RuntimePrincipalsCommandSpec("organism", _handle_organism, mutates=False, json_output=True, usage="organism [--json]"),
        RuntimePrincipalsCommandSpec("actions", _handle_actions, mutates=False, json_output=True, usage="actions [--json]"),
        RuntimePrincipalsCommandSpec("converge", _handle_converge, mutates=True, json_output=True, usage="converge [--dry-run] [--dispatch] [--json]"),
        RuntimePrincipalsCommandSpec("activate", _handle_activate, mutates=True, json_output=True, usage="activate [--dry-run] [--json]"),
        RuntimePrincipalsCommandSpec("bootstrap", _handle_bootstrap, mutates=True, json_output=True, usage="bootstrap [--dry-run] [--json]"),
        RuntimePrincipalsCommandSpec("storage", _handle_storage, mutates=False, json_output=True, usage="storage [plan|bundle|status]"),
        RuntimePrincipalsCommandSpec("bridge", _handle_bridge, mutates=False, json_output=True, usage="bridge [plan|status]"),
        RuntimePrincipalsCommandSpec("fleet", _handle_fleet, mutates=False, json_output=True, usage="fleet [status]"),
        RuntimePrincipalsCommandSpec("mesh", _handle_mesh, mutates=False, json_output=True, usage="mesh [status]"),
        RuntimePrincipalsCommandSpec("infra", _handle_infra, mutates=False, json_output=False, usage="infra provision <target>"),
        RuntimePrincipalsCommandSpec("repos", _handle_repos, mutates=True, json_output=True, usage="repos <status|add|remove|sync>"),
        RuntimePrincipalsCommandSpec("apply", _handle_apply, mutates=True, json_output=True, usage="apply"),
        RuntimePrincipalsCommandSpec("install-defaults", _handle_install_defaults, mutates=True, json_output=True, usage="install-defaults"),
        RuntimePrincipalsCommandSpec("print-launchers", _handle_print_launchers, mutates=False, json_output=False, usage="print-launchers"),
        RuntimePrincipalsCommandSpec("print-profile", _handle_print_profile, mutates=False, json_output=False, usage="print-profile"),
        RuntimePrincipalsCommandSpec("print-cynic-cli", _handle_print_cynic_cli, mutates=False, json_output=False, usage="print-cynic-cli"),
        RuntimePrincipalsCommandSpec("handicap", _handle_handicap, mutates=True, json_output=False, usage="handicap [setup|voice|telegram-bot|autopilot]"),
        RuntimePrincipalsCommandSpec("hf", _handle_hf, mutates=True, json_output=False, usage="hf [search|info|download]"),
    ]
    for command in LIFECYCLE_PROFILE_COMMANDS:
        specs.append(
            RuntimePrincipalsCommandSpec(
                command,
                lambda args, profile=command: _handle_lifecycle_profile(profile, args),
                mutates=True,
                json_output=True,
                usage=f"{command} [--dry-run] [--json]",
            )
        )
    return specs


def _command_dispatcher() -> RuntimePrincipalsCommandDispatcher:
    return RuntimePrincipalsCommandDispatcher(_build_command_specs())


def command_manifest() -> dict[str, object]:
    """Return machine-readable command metadata for cortex operators."""
    return _command_dispatcher().manifest()


def main() -> int:
    command_dispatcher = _command_dispatcher()
    parser = argparse.ArgumentParser(description="CYNIC runtime principal activation.")
    parser.add_argument("command", choices=sorted(set(command_dispatcher.command_names)))
    parser.add_argument("extra", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if _handle_command_help(args.command, args.extra):
        return 0
    return command_dispatcher.execute(args.command, args.extra)


if __name__ == "__main__":
    raise SystemExit(main())
