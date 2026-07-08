"""Static command contracts for runtime principal actions."""

from __future__ import annotations


COMMAND_HELP = {
    "update": """usage: cynic update [--dry-run] [--json] [--bootstrap]

Update primary cortex CLI dependencies with native CLI update commands only.
  --dry-run    Inspect planned update commands without running them.
  --bootstrap  Refresh cynic runtime principal projection (apply + install-defaults) after updates.
  --json       Emit machine-readable output.
""",
    "bootstrap": """usage: cynic bootstrap [--dry-run] [--json]

Refresh the cynic runtime principal projection in one pass.
  --dry-run  Inspect planned update/apply/install-defaults sequence without running it.
  --json     Emit machine-readable output.
""",
    "activate": """usage: cynic activate [--dry-run] [--json]

Dispatch registry-derived lifecycle activation requests through organ-forge.
  --dry-run  Inspect activation requests and blockers without mutating the mempool.
  --json     Emit machine-readable output.
""",
    "converge": """usage: cynic converge [--dry-run] [--dispatch] [--json]

Inspect L0-L7 convergence actions, fragmentation patterns and Kernel-dispatchable requests.
Default and --dry-run do not mutate the mempool. --dispatch sends convergence requests to the Kernel and requires read-back.
  --dry-run  Emit the machine convergence contract.
  --dispatch Dispatch Kernel convergence requests with duplicate suppression by idempotency_key.
  --json     Emit machine-readable output.
""",
    "organism": """usage: cynic organism [--summary] [--json]

Print organism signs of life. Default text is human-facing; --json keeps the full machine contract.
  --summary  Emit compact machine-readable status without deep fleet/mesh/stack payloads.
  --json     Emit machine-readable output.
""",
    "storage": "usage: cynic storage [plan|bundle|status] [args...] [--json]\n",
    "bridge": "usage: cynic bridge [plan|status] [--json]\n",
    "fleet": "usage: cynic fleet status [organ-fleet report args...] [--json]\n",
    "mesh": "usage: cynic mesh status [mesh status args...] [--json]\n",
    "repos": """usage: cynic repos <status|add|remove|sync> [args...]

  status                   Show registered repositories and access evidence.
  add <path...>            Add repositories to runtime-access registry.
  remove <path...>         Remove repositories from runtime-access registry.
  sync                     Re-sync canonical and discovered repos.

Common flags:
  --discovered             Mark repositories as automatically discovered.
  --source=<value>         Record action source metadata.
  --added-by=<value>       Record actor metadata.
  --no-grant               Skip filesystem grant operations.
  --json                   Emit machine-readable output.
""",
    "infra": "usage: cynic infra [provision <target>] [--json]\n",
}


def native_update_contract(execution_posture: dict[str, object]) -> dict[str, object]:
    native_default = str(execution_posture.get("native_default", "unknown")).lower()
    expected_native_default = "true"
    return {
        "schema": "cynic.runtime_principals.update_plan.v1",
        "command": "cynic update --dry-run",
        "dispatch_command": "cynic update",
        "dispatch_json_command": "cynic update --json",
        "mutates": False,
        "updates": {
            "codex": {"command": ["codex", "update"], "mutates_process": True},
            "agy": {"command": ["agy", "update"], "mutates_process": True},
            "hermes": {"command": ["make", "hermes-update"], "mutates_process": True},
        },
        "forbidden_patterns": [
            "curl | sh",
            "curl -fsSL * | sh",
            "cargo build -p cynic-mcp",
            "install -m 0755 target/release/cynic-mcp",
            "cp target/release/cynic-kernel",
        ],
        "proof_commands": ["sudo cynic status --json"],
        "operator_sequence": [
            {"step": "inspect", "command": "cynic update --dry-run", "mutates_process": False},
            {"step": "update_dependency", "command": "codex update", "mutates_process": True},
            {"step": "update_dependency", "command": "agy update", "mutates_process": True},
            {"step": "update_dependency", "command": "make hermes-update", "mutates_process": True},
            {"step": "apply", "command": "cynic bootstrap", "mutates_host": True},
            {"step": "prove", "command": "sudo cynic status --json", "mutates_host": False},
        ],
        "current_read_back": f"execution_posture.native_default={native_default}",
        "read_back": f"execution_posture.native_default={expected_native_default}",
        "done_when": [
            "native update commands are the only allowed update paths",
            "forbidden update patterns cover non-native update vectors",
            "runtime-principals apply refreshes bridges after updates",
            f"execution_posture.native_default={expected_native_default}",
        ],
    }


def activation_dry_run_contract(activation: dict[str, object]) -> dict[str, object]:
    profile_contract = {
        "inspect_command": "cynic activate --dry-run",
        "dispatch_command": "cynic activate",
        "dispatch_json_command": "cynic activate --json",
        "operator_sequence": [
            {"step": "inspect", "command": "cynic activate --dry-run", "mutates_mempool": False},
            {"step": "dispatch", "command": "cynic activate", "mutates_mempool": True},
            {"step": "dispatch_json", "command": "cynic activate --json", "mutates_mempool": True},
            {"step": "observe", "command": "cynic organism", "mutates_mempool": False},
        ],
        "proof_commands": ["cynic actions", "cynic organism"],
        "done_when": [
            "cynic activate --dry-run resolves activation blockers without mutating the mempool",
            "cynic activate or cynic activate --json reports task ids or existing pending duplicates with Kernel read-back",
            "cynic actions and cynic organism show updated activation pressure after organ-forge consumes tasks",
        ],
    }
    payload = dict(activation)
    payload.update(profile_contract)
    payload["command"] = "cynic activate --dry-run"
    payload["dispatch_command"] = "cynic activate"
    payload["dispatch_json_command"] = "cynic activate --json"
    payload["inspect_command"] = "cynic activate --dry-run"
    payload["fallback_command"] = "python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers"
    payload["mutates_mempool"] = False
    return payload
