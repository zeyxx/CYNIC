"""Pure runtime principal content generators."""

from __future__ import annotations

from pathlib import Path
from typing import Any


YOLO_ARGS = {
    "codex": ['--dangerously-bypass-approvals-and-sandbox'],
    "agy": ['--dangerously-skip-permissions'],
    "hermes": [],
}


def launcher_body(binary: str, canonical_project_root: Path) -> str:
    yolo_args = " ".join(f'"{arg}"' for arg in YOLO_ARGS.get(binary, []))
    exec_suffix = f" {yolo_args}" if yolo_args else ""
    return f"""#!/usr/bin/env bash
set -euo pipefail
if [ -f "${{HOME}}/.cynic-env" ]; then
    # shellcheck disable=SC1091
    set -a
    source "${{HOME}}/.cynic-env"
    set +a
fi
if [ -z "${{CYNIC_PROJECT_ROOT:-}}" ]; then
    if git rev-parse --show-toplevel >/dev/null 2>&1; then
        CYNIC_PROJECT_ROOT="$(git rev-parse --show-toplevel)"
        export CYNIC_PROJECT_ROOT
    fi
fi
export PATH="${{CYNIC_PROJECT_ROOT}}/.cynic/python-runtime/bin:${{HOME}}/bin:${{HOME}}/.local/bin:/usr/local/bin:/usr/bin:/bin:${{PATH:-}}"
export GSETTINGS_BACKEND="${{GSETTINGS_BACKEND:-memory}}"
export NO_AT_BRIDGE=1
if [ ! -d "${{CYNIC_PROJECT_ROOT}}" ]; then
    echo "ERROR: CYNIC_PROJECT_ROOT does not exist: ${{CYNIC_PROJECT_ROOT}}" >&2
    echo "Run from the source checkout: cynic bootstrap" >&2
    exit 1
fi
if ! command -v {binary} >/dev/null 2>&1; then
    echo "ERROR: required CLI not found in cynic PATH: {binary}" >&2
    echo "Install it for the cynic user or expose it under the dedicated user's bin directory." >&2
    exit 127
fi
cd "${{CYNIC_PROJECT_ROOT}}"
exec {binary}{exec_suffix} "$@"
"""


def cynic_cli_body(user_name: str, project_root: Path, canonical_project_root: Path) -> str:
    _ = project_root, canonical_project_root
    return f"""#!/usr/bin/env bash
set -euo pipefail

register_repo_for_cynic() {{
    local repo_path="$1"
    if [ -n "${{CYNIC_AUTO_REPO_ACCESS:-}}" ] && [ "${{CYNIC_AUTO_REPO_ACCESS}}" != "1" ]; then
        return 0
    fi
    if [ -z "$repo_path" ] || [ ! -d "$repo_path" ]; then
        return 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        return 0
    fi
    python3 "$ENTRYPOINT" repos add --no-grant --discovered --source=auto "$repo_path" >/dev/null 2>&1 || true
}}

register_repo_args_for_cynic() {{
    for arg in "$@"; do
        if [ "${{arg#-}}" != "$arg" ]; then
            continue
        fi
        if [ "$arg" = "--" ]; then
            break
        fi
        if [ -d "$arg" ]; then
            register_repo_for_cynic "$arg"
        fi
    done
}}

if [ -f "${{HOME}}/.cynic-env" ]; then
    # shellcheck disable=SC1091
    set -a
    source "${{HOME}}/.cynic-env"
    set +a
fi

resolve_project_root() {{
    if [ -n "${{CYNIC_PROJECT_ROOT:-}}" ] && [ -d "$CYNIC_PROJECT_ROOT" ] && [ -f "$CYNIC_PROJECT_ROOT/organs/supervisor/entrypoints/runtime_principals.py" ]; then
        printf '%s\\n' "$CYNIC_PROJECT_ROOT"
        return 0
    fi
    if [ -n "${{CANONICAL_PROJECT_ROOT:-}}" ] && [ -d "$CANONICAL_PROJECT_ROOT" ] && [ -f "$CANONICAL_PROJECT_ROOT/organs/supervisor/entrypoints/runtime_principals.py" ]; then
        printf '%s\\n' "$CANONICAL_PROJECT_ROOT"
        return 0
    fi
    if [ -n "${{SCRIPT_GUESS_PROJECT_ROOT:-}}" ] && [ -d "$SCRIPT_GUESS_PROJECT_ROOT" ] && [ -f "$SCRIPT_GUESS_PROJECT_ROOT/organs/supervisor/entrypoints/runtime_principals.py" ]; then
        printf '%s\\n' "$SCRIPT_GUESS_PROJECT_ROOT"
        return 0
    fi

    if git rev-parse --show-toplevel >/dev/null 2>&1; then
        local candidate
        candidate="$(git rev-parse --show-toplevel)"
        if [ -f "$candidate/organs/supervisor/entrypoints/runtime_principals.py" ]; then
            printf '%s\\n' "$candidate"
            return 0
        fi
    fi
    echo "ERROR: could not resolve a valid CYNIC project checkout from environment or git context." >&2
    echo "Set CYNIC_PROJECT_ROOT to the correct checkout root and retry." >&2
    exit 64
}}

CANONICAL_PROJECT_ROOT="${{CYNIC_CANONICAL_PROJECT_ROOT:-}}"
SCRIPT_GUESS_PROJECT_ROOT="${{CYNIC_SCRIPT_GUESS_PROJECT_ROOT:-}}"

resolve_entrypoint() {{
    local root_candidates
    local candidate root discovered

    root_candidates=(
        "${{CANONICAL_PROJECT_ROOT}}"
        "${{SCRIPT_GUESS_PROJECT_ROOT}}"
        "${{CYNIC_PROJECT_ROOT:-}}"
    )

    for candidate in "${{root_candidates[@]}}"; do
        if [ -z "$candidate" ]; then
            continue
        fi
        if [ -f "$candidate/organs/supervisor/entrypoints/runtime_principals.py" ]; then
            printf '%s\\n' "$candidate/organs/supervisor/entrypoints/runtime_principals.py"
            return 0
        fi
    done

    if command -v find >/dev/null 2>&1; then
        discovered=$(find "$HOME" -type f -path '*/organs/supervisor/entrypoints/runtime_principals.py' -print -quit 2>/dev/null || true)
        if [ -n "$discovered" ] && [ -f "$discovered" ]; then
            printf '%s\\n' "$discovered"
            return 0
        fi
    fi

    return 0
}}

PROJECT_ROOT="$(resolve_project_root)"
ENTRYPOINT="$(resolve_entrypoint)"
if [ -z "$ENTRYPOINT" ]; then
    echo "ERROR: could not locate CYNIC runtime principals entrypoint." >&2
    echo "Set CYNIC_PROJECT_ROOT to the CYNIC repo checkout path and retry." >&2
    exit 1
fi
if [ -z "$PROJECT_ROOT" ] || [ ! -f "$PROJECT_ROOT/organs/supervisor/entrypoints/runtime_principals.py" ]; then
    PROJECT_ROOT="$(dirname "$(dirname "$(dirname "$ENTRYPOINT")")")"
fi
if [ -z "$PROJECT_ROOT" ]; then
    echo "ERROR: CYNIC_PROJECT_ROOT is not discoverable from project resolution." >&2
    exit 64
fi

CYNIC_PROJECT_ROOT="${{PROJECT_ROOT}}"
CYNIC_HOME="$(getent passwd {user_name} | cut -d: -f6)"
if [ -z "$CYNIC_HOME" ]; then
    echo "ERROR: dedicated user home not found for {user_name}" >&2
    exit 69
fi

runtime_principal_python() {{
    if [ "$(id -un)" = "{user_name}" ]; then
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 python3 "$ENTRYPOINT" "$@"
    fi
    exec sudo -u {user_name} -H env \
        HOME="$CYNIC_HOME" USER="{user_name}" LOGNAME="{user_name}" \
        CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 \
        python3 "$ENTRYPOINT" "$@"
}}

usage() {{
    cat <<'USAGE'
usage: cynic <command> [args...]

commands:
  codex          Launch Codex CLI as the dedicated cynic cortex user.
  agy           Launch Antigravity CLI as the dedicated cynic cortex user.
  hermes        Launch Hermes CLI as the dedicated cynic cortex user.
  update        Update primary cortex CLI dependencies and refresh runtime bridges. Use --dry-run to inspect commands.
             Use --bootstrap to perform projection refresh (cynic update --bootstrap: apply + install-defaults) as root.
  bootstrap     Refresh the full runtime-principal projection (update, apply, install-defaults).
  organism      Print organism signs of life from local contracts and runtime checks. Use --json for machine output.
  actions       Print next organism actions from lifecycle activation pressure. Use --json for machine output.
  converge      Print or dispatch L0-L7 convergence requests. Use --dry-run, --dispatch, or --json.
  activate      Dispatch registry-derived lifecycle activation requests through organ-forge. Use --dry-run to inspect requests or --json for machine output.
  bootstrap-request  Request registry build-runner bootstrap through organ-forge. Use --dry-run to inspect payloads or --json for machine output.
  mcp-reload    Request registry build/reload of the MCP runtime. Use --dry-run to inspect payloads or --json for machine output.
  gate-2        Request registry Level 2 release validation. Use --dry-run to inspect payloads or --json for machine output.
  deploy        Request registry build and deploy to the kernel primary. Use --dry-run to inspect payloads or --json for machine output.
  storage       Inspect/package dedicated storage primary readiness: cynic storage [plan|bundle|status]. Use status --json for machine output.
  bridge        Inspect LXC exec bridge readiness: cynic bridge [plan|status]. Use status --json for machine output.
  fleet         Inspect registry/runtime fleet truth: cynic fleet status. Use --json for machine output.
  mesh          Inspect Synapse mesh policy/DNS status: cynic mesh status. Use --json for machine output.
  credentials   Manage organism credentials: list, add, remove, verify, attest. Source: ~/.cynic-env.
  domain        Public domain ops (Porkbun + Cloudflare): registrar, zone, pages.
  status        Print human-readable runtime-principals status. Use --json for machine output.
  doctor        Print human-readable runtime-principals health.
  plan          Print runtime-principals activation plan JSON.
  apply         Refresh runtime projection artifacts on the host (root required).
  install-defaults  Install interactive and command defaults for dedicated cynic user (root required).
  repos         Manage dedicated cynic repository access metadata. Usage: cynic repos <status|add|remove|sync>.
                Example: cynic repos status, cynic repos add, cynic repos remove, cynic repos sync.
  shell         Open a login shell as cynic in the canonical checkout.
  handicap      Manage physical handicap assistance (voice recording/dispatch, aliases, Telegram daemon).
  hf            Search Hugging Face models, datasets, and spaces, show info, and download files.
USAGE
}}

cmd="${{1:-help}}"
if [ "$#" -gt 0 ]; then
    shift
fi

case "$cmd" in
codex)
        register_repo_for_cynic "$PROJECT_ROOT"
        register_repo_args_for_cynic "$@"
        exec sudo -u {user_name} -H env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/codex-yolo" "$@"
        ;;
    agy)
        register_repo_for_cynic "$PROJECT_ROOT"
        register_repo_args_for_cynic "$@"
        exec sudo -u {user_name} -H env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/antigravity-yolo" "$@"
        ;;
    hermes)
        register_repo_for_cynic "$PROJECT_ROOT"
        register_repo_args_for_cynic "$@"
        exec sudo -u {user_name} -H env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/hermes-yolo" "$@"
        ;;
    antigravity)
        echo "NOTE: 'cynic antigravity' is a compatibility alias; canonical command is 'cynic agy'." >&2
        register_repo_for_cynic "$PROJECT_ROOT"
        exec sudo -u {user_name} -H env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/antigravity-yolo" "$@"
        ;;
    shell)
        exec sudo -iu {user_name} env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" bash -lc 'cd "$CYNIC_PROJECT_ROOT" && exec bash -l'
        ;;
    update)
        if [ "$(id -u)" -eq 0 ]; then
            exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" update "$@"
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" update "$@"
        ;;
    bootstrap)
        if [ "$(id -u)" -eq 0 ]; then
            exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" bootstrap "$@"
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" bootstrap "$@"
        ;;
    apply)
        if [ "$(id -u)" -eq 0 ]; then
            exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" apply "$@"
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" apply "$@"
        ;;
    install-defaults)
        if [ "$(id -u)" -eq 0 ]; then
            exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" install-defaults "$@"
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" install-defaults "$@"
        ;;
    organism)
        runtime_principal_python organism "$@"
        ;;
    actions)
        runtime_principal_python actions "$@"
        ;;
    converge)
        runtime_principal_python converge "$@"
        ;;
    activate)
        runtime_principal_python activate "$@"
        ;;
    bootstrap-request)
        runtime_principal_python bootstrap-request "$@"
        ;;
    mcp-reload)
        runtime_principal_python mcp-reload "$@"
        ;;
    gate-2)
        runtime_principal_python gate-2 "$@"
        ;;
    deploy)
        runtime_principal_python deploy "$@"
        ;;
    storage)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" storage "$@"
        ;;
    bridge)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" bridge "$@"
        ;;
    fleet)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" fleet "$@"
        ;;
    mesh)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" mesh "$@"
        ;;
    status)
        if [ "${{1:-}}" = "--json" ]; then
            if [ "$(id -u)" -eq 0 ]; then
                exec python3 "$ENTRYPOINT" status
            fi
            exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" status
        fi
        if [ "$(id -u)" -eq 0 ]; then
            exec python3 "$ENTRYPOINT" doctor
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" doctor
        ;;
    doctor)
        if [ "$(id -u)" -eq 0 ]; then
            exec python3 "$ENTRYPOINT" doctor
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" doctor
        ;;
    plan)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" plan
        ;;
    repos)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" repos "$@"
        ;;
    credentials)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 \
          "$PROJECT_ROOT/organs/identity/src/entrypoints/cli.py" "$@"
        ;;
    domain)
        exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 \
          "$PROJECT_ROOT/organs/domain/src/entrypoints/cli.py" "$@"
        ;;
    handicap)
        runtime_principal_python handicap "$@"
        ;;
    hf)
        runtime_principal_python hf "$@"
        ;;
    help|-h|--help)
        usage
        ;;
    *)
        echo "ERROR: unknown cynic command: $cmd" >&2
        usage >&2
        if [ "$(id -u)" -eq 0 ]; then
            exec env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" "$cmd" "$@"
        fi
        exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" "$cmd" "$@"
        ;;
esac
"""


def profile_body() -> str:
    return """# CYNIC cortex CLI defaults.
# Interactive shells expose the explicit framework entrypoint.
if [ -n "${PS1:-}" ] && [ "$(id -un 2>/dev/null)" != "cynic" ]; then
    alias codex='cynic codex'
    alias agy='cynic agy'
    alias hermes='cynic hermes'
    alias antigravity='cynic agy'
fi
"""


def plan_payload(
    *,
    user_name: str,
    home_dir: Path,
    canonical_project_root: Path,
    legacy_sudoers: Path,
    operator_cli: Path,
    profile_defaults: Path,
) -> dict[str, Any]:
    return {
        "schema": "cynic.runtime_principals.plan.v1",
        "mutates": False,
        "apply_requires_root": True,
        "actions": [
            f"create group {user_name} if missing",
            f"create user {user_name} with home {home_dir} and shell /bin/bash if missing",
            f"create {home_dir}/bin",
            f"grant {user_name} traverse access to every parent of {canonical_project_root}",
            f"grant {user_name} and the invoking operator read/write/traverse access to canonical checkout {canonical_project_root}",
            "bridge operator-installed codex, agy, and hermes binaries into the dedicated user's bin directory when they are not installed natively for cynic",
            f"install bounded YOLO launchers {home_dir}/bin/codex-yolo, {home_dir}/bin/antigravity-yolo, and {home_dir}/bin/hermes-yolo",
            "launch Codex with --dangerously-bypass-approvals-and-sandbox under the dedicated cynic OS boundary",
            "launch Antigravity with --dangerously-skip-permissions under the dedicated cynic OS boundary",
            "launch Hermes with default launch flags under the dedicated cynic OS boundary",
            "update primary cortex CLI dependencies through cynic update, using native CLI update commands and then refreshing runtime bridges",
            f"install framework CLI {operator_cli} so operators launch with cynic codex, cynic agy, or cynic hermes",
            f"install profile {profile_defaults} so interactive codex/agy/hermes aliases route through the cynic framework",
            "preserve destructive host/provider mutations behind passworded or MCP/helper contracts",
            f"report legacy {legacy_sudoers} pvesh NOPASSWD as a violation if present",
        ],
        "not_done_by_apply": [
            "install Codex or Antigravity binaries for the cynic user",
            "copy secrets from another user's home",
            "create a second cortex checkout",
            "overwrite the canonical checkout .git directory",
            "grant raw pvesh, pct, shell, disk, or network NOPASSWD permissions",
        ],
    }


def doctor_text(report: dict[str, object]) -> str:
    def _ok(value: object) -> str:
        if value is True:
            return "ok"
        if value is None:
            return "unknown"
        return "missing"

    ready = "ready" if report["runtime_ready"] else "not ready"
    posture = report.get("execution_posture", {})
    posture_name = posture.get("name", "unknown") if isinstance(posture, dict) else "unknown"
    posture_default = posture.get("native_default") if isinstance(posture, dict) else None
    posture_default_text = "yes" if posture_default is True else "unknown" if posture_default is None else "no"
    lines = [
        f"CYNIC runtime: {ready}",
        f"User: {report['dedicated_user']}",
        f"Checkout: {report['canonical_project_root']}",
        f"Execution posture: {posture_name}",
        f"Posture native default: {posture_default_text}",
        f"Checkout access for cynic: {_ok(report['canonical_project_root_accessible_by_cynic'])}",
        f"Codex launcher: {_ok(report['codex_launcher_exists'])}",
        f"Codex launcher projection: {_ok(report.get('codex_launcher_projection_ok'))}",
        f"Antigravity launcher: {_ok(report['antigravity_launcher_exists'])}",
        f"Antigravity launcher projection: {_ok(report.get('antigravity_launcher_projection_ok'))}",
        f"Operator CLI: {_ok(report['cynic_cli_exists'])} ({report['cynic_cli']})",
        f"Operator CLI projection: {_ok(report.get('cynic_cli_projection_ok'))}",
        f"Default profile: {_ok(report['default_profile_exists'])} ({report['default_profile']})",
        "Update:",
        "  inspect primary CLIs: cynic update --dry-run",
        "  primary CLIs: cynic update",
        "  primary CLIs + projection refresh: cynic update --bootstrap",
        "Organism:",
        "  signs of life: cynic organism",
        "  next actions: cynic actions",
        "  convergence: cynic converge --dry-run",
        "  dispatch convergence: cynic converge --dispatch",
        "  activation_operator_contract: inspect -> dispatch -> dispatch_json -> observe",
        "  inspect lifecycle activation: cynic activate --dry-run",
        "  lifecycle activation: cynic activate",
        "  inspect build-runner bootstrap: cynic bootstrap-request --dry-run",
        "  request build-runner bootstrap: cynic bootstrap-request",
        "  request build-runner bootstrap json: cynic bootstrap-request --json",
        "  inspect mcp reload: cynic mcp-reload --dry-run",
        "  request mcp reload: cynic mcp-reload",
        "  inspect gate 2: cynic gate-2 --dry-run",
        "  request gate 2: cynic gate-2",
        "  inspect deploy: cynic deploy --dry-run",
        "  request deploy: cynic deploy",
        "Storage:",
        "  plan: cynic storage plan",
        "  bundle: cynic storage bundle --output-dir <dir>",
        "  status: cynic storage status",
        "Bridge:",
        "  plan: cynic bridge plan",
        "  status: cynic bridge status",
        "Fleet:",
        "  status: cynic fleet status",
        "Mesh:",
        "  status: cynic mesh status",
        "  dns: cynic mesh status --resolve-dns --include-records",
        "Credentials:",
        "  list: cynic credentials list",
        "  add: cynic credentials add <KEY>",
        "  verify: cynic credentials verify",
        "  attest (agents): cynic credentials attest",
        "Domain:",
        "  zone backup: cynic domain zone backup",
        "  zone migrate (dry): cynic domain zone migrate --dry-run",
        "  pages deploy (dry): cynic domain pages deploy --app <name> --dry-run",
        "  pages verify: cynic domain pages verify",
        "Launch:",
        f"  codex: {report['launch_commands']['codex']}",
        f"  agy: {report['launch_commands']['agy']}",
        f"  hermes: {report['launch_commands']['hermes']}",
        "  bootstrap: cynic bootstrap",
        f"  shell: {report['runtime_commands']['shell']}",
    ]
    if report.get("requires_root_evidence"):
        lines.append(f"Proof: {report.get('root_evidence_command', 'sudo cynic status --json')}")
    if report.get("cynic_cli_projection_ok") is False:
        lines.append(f"Refresh projection: {report.get('cynic_cli_projection_command', 'cynic bootstrap')}")
    if report["legacy_pvesh_sudoers_violation"]:
        lines.append("Violation: legacy raw pvesh NOPASSWD sudoers is present")
    return "\n".join(lines) + "\n"
