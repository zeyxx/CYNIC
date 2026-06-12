#!/usr/bin/env bash
# organ-anvil.sh — Sensor/tool layer for the Anvil organ.
# Writes infra/organ-anvil/state.json and supports perception commands.
# The Hermes agent is the organ; this script is only the sensor.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo '$HOME/Bureau/CYNIC')"
STATE_DIR="${REPO_ROOT}/infra/organ-anvil"
STATE_FILE="${STATE_DIR}/state.json"
AUDIT_FILE="${STATE_DIR}/audit.jsonl"
mkdir -p "${STATE_DIR}"

cmd="${1:-help}"
shift 2>/dev/null || true

now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

get_repo_state() {
    local branch current_branch dirty_count stashes worktrees
    local local_branches remote_branches gate_0 gate_1 gate_2 gate_passed
    local health_score dirty_tree="clean"
    
    current_branch="$(git symbolic-ref --short HEAD 2>/dev/null || git describe --tags --exact-match 2>/dev/null || echo 'unknown')"
    
    # Dirty status
    dirty_count="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
    if [ "$dirty_count" -gt 0 ]; then
        dirty_tree="dirty"
    fi
    
    # Stashes
    stashes="$(git stash list 2>/dev/null | wc -l | tr -d ' ')"
    
    # Worktrees (excluding main)
    worktrees="$(git worktree list 2>/dev/null | grep -v "${REPO_ROOT}" | wc -l | tr -d ' ')"
    
    # Branch counts
    local_branches="$(git branch 2>/dev/null | wc -l | tr -d ' ')"
    remote_branches="$(git branch -r 2>/dev/null | wc -l | tr -d ' ')"
    
    # Gate markers
    gate_0=""; [ -f "${REPO_ROOT}/.gate-0" ] && gate_0="$(stat -c %Y "${REPO_ROOT}/.gate-0" 2>/dev/null || echo '')"
    gate_1=""; [ -f "${REPO_ROOT}/.gate-1" ] && gate_1="$(stat -c %Y "${REPO_ROOT}/.gate-1" 2>/dev/null || echo '')"
    gate_2=""; [ -f "${REPO_ROOT}/.gate-2" ] && gate_2="$(stat -c %Y "${REPO_ROOT}/.gate-2" 2>/dev/null || echo '')"
    gate_passed=""; [ -f "${REPO_ROOT}/.gate-passed" ] && gate_passed="$(stat -c %Y "${REPO_ROOT}/.gate-passed" 2>/dev/null || echo '')"
    
    # Health score computation (0.0-1.0)
    # Factors: dirty tree (-0.3 per dirty state), too many branches, stale gates
    health_score="1.0"
    
    # Dirty penalty
    if [ "$dirty_count" -gt 0 ]; then
        health_score="0.7"
    fi
    
    # Stale gate penalty (older than 1 day)
    local now_epoch
    now_epoch="$(date +%s)"
    for g in "$gate_0" "$gate_1" "$gate_2"; do
        if [ -n "$g" ]; then
            local diff=$(( (now_epoch - g) / 86400 ))
            if [ "$diff" -gt 2 ]; then
                health_score="$(echo "$health_score - 0.1" | bc 2>/dev/null || echo "$health_score")"
            fi
        fi
    done
    
    # Branch penalty (>5 local branches)
    if [ "$local_branches" -gt 5 ]; then
        health_score="$(echo "$health_score - 0.15" | bc 2>/dev/null || echo "$health_score")"
    fi
    
    # Ensure minimum 0.0
    health_score="$(echo "$health_score" | awk '{if ($1 < 0) print 0; else print $1}')"
    
    cat <<EOF
{
  "timestamp": "$(now)",
  "repo": "${REPO_ROOT}",
  "branch": "${current_branch}",
  "dirty": {
    "status": "${dirty_tree}",
    "changed_files": ${dirty_count}
  },
  "stashes": ${stashes},
  "worktrees": ${worktrees},
  "branches": {
    "local": ${local_branches},
    "remote": ${remote_branches}
  },
  "gates": {
    "gate-0": ${gate_0:-null},
    "gate-1": ${gate_1:-null},
    "gate-2": ${gate_2:-null},
    "gate-passed": ${gate_passed:-null}
  },
  "health_score": ${health_score}
}
EOF
}

cmd_perceive() {
    get_repo_state
}

cmd_state() {
    local state
    state="$(get_repo_state)"
    echo "$state" > "${STATE_FILE}"
    echo "State written to ${STATE_FILE}" >&2
    echo "$state"
}

cmd_signal() {
    get_repo_state | jq -c '{
      branch: .branch,
      dirty: .dirty.status,
      files: .dirty.changed_files,
      health: .health_score,
      branches: .branches.local
    }' 2>/dev/null || get_repo_state
}

cmd_triage() {
    local dirty_files
    dirty_files="$(git status --porcelain 2>/dev/null)"
    if [ -z "$dirty_files" ]; then
        echo '{"triage": "clean", "scope": []}'
        return
    fi
    
    local scopes=""
    local rust_count=0 md_count=0 config_count=0 other_count=0
    
    while IFS= read -r line; do
        local file_path="${line#??}"
        case "$file_path" in
            *.rs|*Cargo.*) ((rust_count++)) ;;
            *.md) ((md_count++)) ;;
            *.toml|*.yaml|*.json|*.env|*Makefile) ((config_count++)) ;;
            *) ((other_count++)) ;;
        esac
    done <<< "$dirty_files"
    
    echo "{\"triage\": \"dirty\", \"scope\": {\"rust\": ${rust_count}, \"docs\": ${md_count}, \"config\": ${config_count}, \"other\": ${other_count}}}"
}

cmd_repo_health() {
    get_repo_state | jq -c . 2>/dev/null || get_repo_state
}

cmd_branch_report() {
    local target_branch="${1:-$(git symbolic-ref --short HEAD 2>/dev/null)}"
    local branch_info
    branch_info="$(git log --oneline -5 "$target_branch" 2>/dev/null || echo "branch not found")"
    echo "{\"branch\": \"${target_branch}\", \"recent_commits\": \"${branch_info}\"}"
}

cmd_audit() {
    local source="${1:-unknown}"
    local payload="${2:-{}}"
    local result="${3:-unknown}"
    echo "{\"timestamp\": \"$(now)\", \"source\": \"${source}\", \"payload\": ${payload}, \"result\": \"${result}\"}" >> "${AUDIT_FILE}"
    echo "Audit recorded." >&2
}

cmd_help() {
    cat <<EOF
Usage: organ-anvil.sh <command> [args]

Commands:
  perceive       — Read repo state (no write)
  state          — Refresh and write repo state to state.json
  signal         — Compact one-line repo signal
  triage         — Classify dirty worktree by scope
  repo-health    — Full health radar (JSON)
  branch-report  — Branch commit history summary
  audit          — Append to audit log (internal)
EOF
}

case "$cmd" in
    perceive)    cmd_perceive ;;
    state)       cmd_state ;;
    signal)      cmd_signal ;;
    triage)      cmd_triage ;;
    repo-health) cmd_repo_health ;;
    branch-report) cmd_branch_report ;;
    audit)         cmd_audit "$@" ;;
    help|-h|--help) cmd_help ;;
    *) echo "Unknown command: $cmd" >&2; cmd_help; exit 1 ;;
esac
