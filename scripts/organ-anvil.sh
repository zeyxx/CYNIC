#!/bin/bash
# Organ Anvil - Repo Perception & State Manager
# Usage: organ-anvil.sh [perceive|state|audit|signal|triage|repo-health]

set -euo pipefail

# Force C locale to avoid comma decimals in awk/jq
export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="${CYNIC_PROJECT_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"
INFRA_DIR="$PROJECT_DIR/infra/organ-anvil"
STATE_FILE="$INFRA_DIR/state.json"
AUDIT_FILE="$INFRA_DIR/audit.jsonl"
SCHEMA_FILE="$INFRA_DIR/schema.json"
POH_FILE="$INFRA_DIR/poh.json"
DASHBOARD_FILE="$INFRA_DIR/dashboard.html"

# Ensure infra directory exists
mkdir -p "$INFRA_DIR"

# Default schema if missing
if [ ! -f "$SCHEMA_FILE" ]; then
    cat > "$SCHEMA_FILE" << 'SCHEMAEOF'
{
  "version": "1.0.0",
  "updated": "2026-06-06T00:00:00Z",
  "state_fields": {
    "repo_health_score": "float 0-1",
    "dirty_tree": "boolean",
    "branches_local": "array of objects",
    "branches_remote": "array of objects",
    "prs_open": "number",
    "worktrees": "array of objects",
    "stashes": "number",
    "gate_markers": "object: .gate-0/.gate-1/.gate-2/.gate-passed presence and mtime",
    "push_force_supported": "boolean",
    "last_perception": "ISO timestamp",
    "last_run": "ISO timestamp",
    "run_count": "number",
    "alerts": "array of objects"
  },
  "audit_fields": {
    "timestamp": "ISO timestamp",
    "action": "string: perception|decision|action|alert",
    "details": "object",
    "outcome": "string: success|failed|partial|pending"
  },
  "poh_fields": {
    "run": "number - incremental run id",
    "timestamp": "ISO timestamp",
    "repo_health_score": "float 0-1",
    "dirty_tree": "boolean",
    "branches_local": "array of objects",
    "branches_remote": "array of objects",
    "prs_open": "number",
    "worktrees": "array of objects",
    "stashes": "number",
    "gate_markers": "object",
    "push_force_supported": "boolean",
    "alerts": "array of objects"
  }
}
SCHEMAEOF
fi

TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

atomic_write() {
    local target="$1"
    local tmp
    tmp=$(mktemp "${target}.tmp.XXXXXX")
    cat > "$tmp"
    mv "$tmp" "$target"
}

json_uint_or_zero() {
    local file="$1"
    local filter="$2"
    local value
    value=$(jq -r "$filter" "$file" 2>/dev/null || echo 0)
    case "$value" in
        ''|*[!0-9]*) echo 0 ;;
        *) echo "$value" ;;
    esac
}

marker_json() {
    local file="$1"
    if [ -f "$file" ]; then
        local mtime
        mtime=$(date -u -d "@$(stat -c '%Y' "$file")" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%SZ)
        jq -n --arg mtime "$mtime" '{present:true,mtime:$mtime}'
    else
        echo '{"present":false,"mtime":null}'
    fi
}

push_force_supported() {
    if grep -q 'PUSH_FORCE' "$PROJECT_DIR/.git/hooks/pre-push" 2>/dev/null; then
        echo 'true'
    else
        echo 'false'
    fi
}

gate_markers_json() {
    local g0 g1 g2 gp
    g0=$(marker_json "$PROJECT_DIR/.gate-0")
    g1=$(marker_json "$PROJECT_DIR/.gate-1")
    g2=$(marker_json "$PROJECT_DIR/.gate-2")
    gp=$(marker_json "$PROJECT_DIR/.gate-passed")
    jq -n \
        --argjson gate0 "$g0" \
        --argjson gate1 "$g1" \
        --argjson gate2 "$g2" \
        --argjson gate_passed "$gp" \
        '{gate_0:$gate0,gate_1:$gate1,gate_2:$gate2,gate_passed:$gate_passed}'
}

build_alerts() {
    local dirty="$1"
    local stashes="$2"
    local push_force="$3"
    local gate_markers="$4"

    jq -n \
        --argjson dirty "$dirty" \
        --argjson stashes "$stashes" \
        --argjson push_force "$push_force" \
        --argjson gates "$gate_markers" \
        '[
          (if $dirty > 0 then {severity:"warning",message:"worktree dirty",action_needed:"commit, stash, or intentionally leave scoped dirty state"} else empty end),
          (if $stashes > 0 then {severity:"warning",message:"stashes pending",action_needed:"restore, branch, or drop stashes after backup"} else empty end),
          (if ($push_force | not) then {severity:"critical",message:"documented PUSH_FORCE fallback missing from pre-push hook",action_needed:"teach hook the fallback or update AGENTS.md"} else empty end),
          (if (($gates.gate_0.present == true) and ($gates.gate_1.present == false)) then {severity:"warning",message:"gate-0 passed but gate-1 marker missing",action_needed:"inspect clippy/gate-1 failure before merge"} else empty end)
        ]'
}

# ============================================================
# PERCEPTION - Capteurs du repo
# ============================================================
perceive() {
    echo "Perception: $TIMESTAMP"

    cd "$PROJECT_DIR"

    # 1. Dirty tree
    DIRTY_TREE=$(git status --porcelain | wc -l | tr -d ' ')

    # 2. Branches locales (exclut main)
    BRANCHES_LOCAL=$(git branch | grep -v "^*" | grep -v "^  main$" | sed 's/^[* ]*//' | jq -R '.' | jq -s '.')

    # 3. Branches remote
    BRANCHES_REMOTE=$(git branch -r | grep origin/ | grep -v HEAD | sed 's|origin/||' | jq -R '.' | jq -s '.')

    # 4. PRs ouverts (via gh CLI si dispo)
    PRS_OPEN=$(gh pr list --state open --json number 2>/dev/null | jq '. | length' 2>/dev/null || echo "0")

    # 5. Worktrees
    WORKTREES=$(git worktree list | awk '{print $1, $2, $3}' | jq -R '.' | jq -s '.')

    # 6. Stashes
    STASHES=$(git stash list | wc -l | tr -d ' ')

    # 7. Gate/fallback perception
    GATE_MARKERS=$(gate_markers_json)
    PUSH_FORCE_SUPPORTED=$(push_force_supported)

    # Calcul du health score (0-1)
    HEALTH_SCORE=$(calculate_health_score "$DIRTY_TREE" "$BRANCHES_LOCAL" "$BRANCHES_REMOTE" "$PRS_OPEN" "$WORKTREES" "$STASHES")

    echo "Health: $HEALTH_SCORE"
    echo "Dirty: $DIRTY_TREE"

    local local_count=$(echo "$BRANCHES_LOCAL" | jq 'length')
    local remote_count=$(echo "$BRANCHES_REMOTE" | jq 'length')
    echo "Branches: $((local_count + remote_count)) (local: $local_count, remote: $remote_count)"
    echo "Stashes: $STASHES"
    echo "PUSH_FORCE fallback: $PUSH_FORCE_SUPPORTED"
    echo "Gate markers: $(echo "$GATE_MARKERS" | jq -c '.')"
}

# ============================================================
# STATE - Écrit l'état dans state.json
# ============================================================
write_state() {
    DIRTY_TREE=$(git status --porcelain | wc -l | tr -d ' ')
    BRANCHES_LOCAL=$(git branch | grep -v "^*" | grep -v "^  main$" | sed 's/^[* ]*//' | jq -R '.' | jq -s '.')
    BRANCHES_REMOTE=$(git branch -r | grep origin/ | grep -v HEAD | sed 's|origin/||' | jq -R '.' | jq -s '.')
    PRS_OPEN=$(gh pr list --state open --json number 2>/dev/null | jq '. | length' 2>/dev/null || echo "0")
    WORKTREES=$(git worktree list | awk '{print $1, $2, $3}' | jq -R '.' | jq -s '.')
    STASHES=$(git stash list | wc -l | tr -d ' ')
    GATE_MARKERS=$(gate_markers_json)
    PUSH_FORCE_SUPPORTED=$(push_force_supported)

    # Use the highest observed run across state and PoH so the sequence stays
    # monotonic even if state.json is regenerated or temporarily corrupted.
    if [ -f "$STATE_FILE" ]; then
        PREV_ALERTS=$(jq '.alerts // []' "$STATE_FILE" 2>/dev/null || echo '[]')
    else
        PREV_ALERTS='[]'
    fi
    PREV_STATE_COUNT=$(json_uint_or_zero "$STATE_FILE" '.run_count // 0')
    PREV_POH_COUNT=$(json_uint_or_zero "$POH_FILE" 'map(.run // 0) | max // 0')
    if [ "$PREV_POH_COUNT" -gt "$PREV_STATE_COUNT" ]; then
        PREV_COUNT="$PREV_POH_COUNT"
    else
        PREV_COUNT="$PREV_STATE_COUNT"
    fi
    NEXT_COUNT=$((PREV_COUNT + 1))

    # Calcul health score and live alerts
    HEALTH_SCORE=$(calculate_health_score "$DIRTY_TREE" "$BRANCHES_LOCAL" "$BRANCHES_REMOTE" "$PRS_OPEN" "$WORKTREES" "$STASHES")
    ALERTS=$(build_alerts "$DIRTY_TREE" "$STASHES" "$PUSH_FORCE_SUPPORTED" "$GATE_MARKERS")

    # Écriture atomique du state
    jq -n \
        --arg timestamp "$TIMESTAMP" \
        --argjson health "$HEALTH_SCORE" \
        --argjson dirty "$( [ "$DIRTY_TREE" -gt 0 ] && echo true || echo false )" \
        --argjson branches_local "$BRANCHES_LOCAL" \
        --argjson branches_remote "$BRANCHES_REMOTE" \
        --argjson prs_open "$PRS_OPEN" \
        --argjson worktrees "$WORKTREES" \
        --argjson stashes "$STASHES" \
        --argjson gate_markers "$GATE_MARKERS" \
        --argjson push_force_supported "$PUSH_FORCE_SUPPORTED" \
        --argjson run_count "$NEXT_COUNT" \
        --argjson alerts "$ALERTS" \
        '{version:"1.1.0",updated:$timestamp,repo_health_score:$health,dirty_tree:$dirty,branches_local:$branches_local,branches_remote:$branches_remote,prs_open:$prs_open,worktrees:$worktrees,stashes:$stashes,gate_markers:$gate_markers,push_force_supported:$push_force_supported,last_perception:$timestamp,last_run:$timestamp,run_count:$run_count,alerts:$alerts}' \
        | atomic_write "$STATE_FILE"

    echo "State written: $STATE_FILE (run #$NEXT_COUNT)"

    # Append snapshot to proof-of-history (poh.json)
    append_poh

    # Send observations to kernel
    observe_kernel "$HEALTH_SCORE" "$ALERTS"
}

# ============================================================
# POH - Append snapshot to proof-of-history (poh.json)
# ============================================================
append_poh() {
    # Read current state.json and append to poh.json array
    if [ ! -f "$POH_FILE" ]; then
        echo "[]" > "$POH_FILE"
    fi

    local current_state=$(cat "$STATE_FILE")
    local run=$(echo "$current_state" | jq '.run_count')
    local timestamp=$(echo "$current_state" | jq -r '.updated')
    local health=$(echo "$current_state" | jq '.repo_health_score')
    local dirty=$(echo "$current_state" | jq '.dirty_tree')
    local branches_local=$(echo "$current_state" | jq '.branches_local')
    local branches_remote=$(echo "$current_state" | jq '.branches_remote')
    local prs=$(echo "$current_state" | jq '.prs_open')
    local worktrees=$(echo "$current_state" | jq '.worktrees')
    local stashes=$(echo "$current_state" | jq '.stashes')
    local alerts=$(echo "$current_state" | jq '.alerts')
    local gate_markers=$(echo "$current_state" | jq '.gate_markers')
    local push_force_supported=$(echo "$current_state" | jq '.push_force_supported')

    # Build snapshot object
    local snapshot=$(jq -n \
        --argjson run "$run" \
        --arg timestamp "$timestamp" \
        --argjson health "$health" \
        --argjson dirty "$dirty" \
        --argjson branches_local "$branches_local" \
        --argjson branches_remote "$branches_remote" \
        --argjson prs "$prs" \
        --argjson worktrees "$worktrees" \
        --argjson stashes "$stashes" \
        --argjson alerts "$alerts" \
        --argjson gate_markers "$gate_markers" \
        --argjson push_force_supported "$push_force_supported" \
        '{run:$run,timestamp:$timestamp,repo_health_score:$health,dirty_tree:$dirty,branches_local:$branches_local,branches_remote:$branches_remote,prs_open:$prs,worktrees:$worktrees,stashes:$stashes,gate_markers:$gate_markers,push_force_supported:$push_force_supported,alerts:$alerts}'
    )

    # Append to array
    local poh=$(cat "$POH_FILE")
    echo "$poh" | jq --argjson snap "$snapshot" '. += [$snap]' | atomic_write "$POH_FILE"

    echo "POH updated: $POH_FILE (snapshot #$(jq 'length' "$POH_FILE"))"
}

# ============================================================
# AUDIT - Append dans audit.jsonl
# ============================================================
audit() {
    local action="$1"
    local details="$2"
    local outcome="$3"

    jq -c -n --arg timestamp "$TIMESTAMP" --arg action "$action" --argjson details "$details" --arg outcome "$outcome" \
        '{timestamp:$timestamp,action:$action,details:$details,outcome:$outcome}' >> "$AUDIT_FILE"
}

# ============================================================
# HEALTH SCORE - Calcul adaptatif
# ============================================================
calculate_health_score() {
    local dirty=$1
    local branches_local=$2
    local branches_remote=$3
    local prs=$4
    local worktrees=$5
    local stashes=$6

    # Score de base = 100 (scale 0-100)
    local score=100

    # Penalty dirty tree (-30)
    if [ "$dirty" -gt 0 ]; then
        score=$((score - 30))
    fi

    # Penalty branches (>3 = -20, >5 = -40)
    local local_count=$(echo "$branches_local" | jq 'length')
    if [ "$local_count" -gt 5 ]; then
        score=$((score - 40))
    elif [ "$local_count" -gt 3 ]; then
        score=$((score - 20))
    fi

    # Penalty stashes (>2 = -20)
    if [ "$stashes" -gt 2 ]; then
        score=$((score - 20))
    fi

    # Clamp entre 0 et 100, puis convertir en float
    if [ $score -lt 0 ]; then score=0; fi
    if [ $score -gt 100 ]; then score=100; fi

    # Convert to strict JSON number (0.00-1.00). Force C locale to avoid comma decimals.
    LC_ALL=C awk -v score="$score" 'BEGIN { printf "%.2f", score / 100 }'
}

# ============================================================
# OBSERVE - Envoyer alertes au kernel via cynic_observe
# ============================================================
observe_kernel() {
    local health_score="$1"
    local alerts="$2"

    # Source env for kernel address
    source ~/.cynic-env 2>/dev/null || return 0

    local kernel_addr="${CYNIC_REST_ADDR:-}"
    local api_key="${CYNIC_API_KEY:-}"
    [[ -n "$kernel_addr" ]] || return 0

    local auth_header=""
    [[ -n "$api_key" ]] && auth_header="Authorization: Bearer $api_key"

    # Check if kernel is reachable
    local health_code
    health_code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 \
        -H "$auth_header" \
        "http://${kernel_addr}/health" 2>/dev/null || echo "000")

    [[ "$health_code" == "200" ]] || [[ "$health_code" == "503" ]] || return 0

    # Send each alert as a separate observation
    local count
    count=$(echo "$alerts" | jq 'length')
    for ((i=0; i<count; i++)); do
        local severity message action_needed
        severity=$(echo "$alerts" | jq -r ".[$i].severity")
        message=$(echo "$alerts" | jq -r ".[$i].message")
        action_needed=$(echo "$alerts" | jq -r ".[$i].action_needed")

        # POST to /observe
        curl -s --max-time 3 -X POST "http://${kernel_addr}/observe" \
            -H "Content-Type: application/json" \
            -H "$auth_header" \
            -d "{
                \"agent_id\": \"organ-anvil\",
                \"tool\": \"repo_perception\",
                \"target\": \"$message\",
                \"domain\": \"repo-health\",
                \"context\": \"severity=$severity health_score=$health_score action=$action_needed\",
                \"tags\": [\"organ-anvil\", \"repo-health\", \"$severity\"]
            }" > /dev/null 2>&1 || true
    done

    # Also send health score as a heartbeat observation
    curl -s --max-time 3 -X POST "http://${kernel_addr}/observe" \
        -H "Content-Type: application/json" \
        -H "$auth_header" \
        -d "{
            \"agent_id\": \"organ-anvil\",
            \"tool\": \"repo_health_pulse\",
            \"target\": \"health_pulse\",
            \"domain\": \"repo-health\",
            \"context\": \"health_score=$health_score run_count=$(json_uint_or_zero \"$STATE_FILE\" '.run_count' 2>/dev/null || echo 0)\",
            \"tags\": [\"organ-anvil\", \"health-pulse\"]
        }" > /dev/null 2>&1 || true
}

# ============================================================
# SIGNAL - Consumer JSON for cortices/Hermes
# ============================================================
signal() {
    if [ ! -f "$STATE_FILE" ]; then
        write_state >/dev/null
    fi

    jq -c \
        --arg timestamp "$TIMESTAMP" \
        '{
          version:"1.0.0",
          source:"organ-anvil",
          timestamp:$timestamp,
          observed_run:(.run_count // 0),
          repo_health_score:(.repo_health_score // 0),
          dirty_tree:(.dirty_tree // false),
          stashes:(.stashes // 0),
          prs_open:(.prs_open // 0),
          push_force_supported:(.push_force_supported // false),
          gate_readiness:{
            gate_0:(.gate_markers.gate_0.present // false),
            gate_1:(.gate_markers.gate_1.present // false),
            gate_2:(.gate_markers.gate_2.present // false),
            gate_passed:(.gate_markers.gate_passed.present // false)
          },
          max_severity:(if any((.alerts // [])[]; .severity == "critical") then "critical" elif any((.alerts // [])[]; .severity == "warning") then "warning" else "ok" end),
          actions:((.alerts // []) | map(.action_needed) | unique)
        }' "$STATE_FILE"
}

# ============================================================
# TRIAGE - Non-mutating scope diagnosis for dirty worktrees
# ============================================================
triage() {
    cd "$PROJECT_DIR"

    local status_lines conflict_lines
    status_lines=$(git status --porcelain=v1 -uall)
    conflict_lines=$(grep -RInE '^(<<<<<<<|=======$|>>>>>>>)' .claude AGENTS.md CODEX.md HERMES_AGENT.md infra scripts cynic-kernel 2>/dev/null || true)

    jq -n \
        --arg timestamp "$TIMESTAMP" \
        --arg status_lines "$status_lines" \
        --arg conflict_lines "$conflict_lines" \
        '
        def scope_for($p):
          if ($p | startswith("infra/organ-anvil/")) or $p == "scripts/organ-anvil.sh" then "organ-anvil"
          elif ($p | startswith("cynic-python/curation/")) then "hermes-curation"
          elif ($p | startswith(".claude/")) or $p == "CLAUDE.md" then "claude-adapter"
          elif ($p | startswith(".gemini/")) or $p == "GEMINI.md" then "gemini-adapter"
          elif ($p | startswith(".codex/")) or $p == "CODEX.md" then "codex-adapter"
          elif $p == "AGENTS.md" or $p == "HERMES_AGENT.md" then "coord-protocol"
          elif ($p | startswith("cynic-kernel/src/llm/")) then "llm-roadmap"
          elif ($p | startswith("cynic-kernel/src/domains/poh/")) or $p == "cynic-kernel/src/api/rest/poh.rs" then "poh-stub"
          elif $p == "cynic-kernel/src/lib.rs" then "kernel-module-wiring"
          elif ($p | startswith("infra/systemd/")) or ($p | endswith(".service")) then "systemd-runtime"
          elif $p == "TOPOLOGY.md" then "generated-topology"
          else "other"
          end;
        def action_for($scope):
          if $scope == "organ-anvil" then "review/commit with Anvil PR or park as organ-anvil stash"
          elif $scope == "hermes-curation" then "park as live-data stash unless this PR is about curation"
          elif $scope == "generated-topology" then "park or regenerate in its owning workflow"
          elif $scope == "poh-stub" or $scope == "llm-roadmap" or $scope == "kernel-module-wiring" then "move to dedicated branch before merge"
          elif $scope == "systemd-runtime" then "template or park; do not mix with audit PR"
          elif ($scope | endswith("adapter")) or $scope == "coord-protocol" then "commit only in coordination-scope PR"
          else "inspect manually"
          end;
        ($status_lines | split("\n") | map(select(length > 0)) | map({status: .[0:2], path: .[3:]})) as $entries |
        ($conflict_lines | split("\n") | map(select(length > 0))) as $conflicts |
        ($entries | group_by(.path | scope_for(.)) | map({
          scope: (.[0].path | scope_for(.)),
          count: length,
          paths: map(.path),
          statuses: map(.status) | unique,
          recommended_action: action_for(.[0].path | scope_for(.))
        })) as $scopes |
        {
          version:"1.0.0",
          source:"organ-anvil",
          mode:"triage",
          timestamp:$timestamp,
          dirty_count:($entries | length),
          scope_count:($scopes | length),
          conflict_marker_count:($conflicts | length),
          conflict_markers:$conflicts,
          scopes:$scopes,
          recommendation:(if ($conflicts | length) > 0 then "fix conflict markers before stashing or committing" elif ($scopes | length) > 1 then "split dirty worktree by scope before merge" elif ($scopes | length) == 1 then "single scope dirty; commit, stash, or continue intentionally" else "clean" end)
        }'
}


# ============================================================
# REPO HEALTH - Non-mutating repository coordination radar
# ============================================================
repo_health() {
    cd "$PROJECT_DIR"

    local current_branch upstream head_sha status_lines dirty_count
    current_branch=$(git branch --show-current 2>/dev/null || echo "")
    upstream=$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || echo "")
    head_sha=$(git rev-parse HEAD 2>/dev/null || echo "")
    status_lines=$(git status --porcelain=v1 -uall 2>/dev/null || true)
    dirty_count=$(printf '%s\n' "$status_lines" | sed '/^$/d' | wc -l | tr -d ' ')

    local local_branches remote_branches open_prs stashes coord gates
    local_branches=$(
        git for-each-ref \
            --format='%(refname:short)%09%(objectname:short)%09%(upstream:short)' \
            refs/heads 2>/dev/null |
        jq -R 'split("\t") | {name:.[0], sha:.[1], upstream:(.[2] // "")}' |
        jq -s '.'
    )
    remote_branches=$(
        git for-each-ref \
            --format='%(refname:short)%09%(objectname:short)' \
            refs/remotes/origin 2>/dev/null |
        awk -F '\t' '$1 ~ /^origin\// && $1 != "origin/HEAD" {print}' |
        jq -R 'split("\t") | {name:.[0], sha:.[1]}' |
        jq -s '.'
    )
    open_prs=$(gh pr list --state open --json number,title,headRefName,baseRefName,isDraft,url,mergeStateStatus 2>/dev/null || echo '[]')
    stashes=$(
        git stash list --format='%gd%x09%gs' 2>/dev/null |
        jq -R 'split("\t") | {name:.[0], message:(.[1] // "")} | . + ((.message | capture("^On (?<branch>[^:]+): (?<scope>.+)$"))? // {branch:null,scope:null})' |
        jq -s '.'
    )
    gates=$(gate_markers_json)
    coord=$(coord_snapshot_json)

    jq -n \
        --arg timestamp "$TIMESTAMP" \
        --arg current_branch "$current_branch" \
        --arg upstream "$upstream" \
        --arg head_sha "$head_sha" \
        --argjson dirty_count "$dirty_count" \
        --arg status_lines "$status_lines" \
        --argjson local_branches "$local_branches" \
        --argjson remote_branches "$remote_branches" \
        --argjson open_prs "$open_prs" \
        --argjson stashes "$stashes" \
        --argjson gates "$gates" \
        --argjson coord "$coord" \
        '
        def strip_origin: sub("^origin/"; "");
        def rec($severity; $kind; $target; $action):
          {severity:$severity, kind:$kind, target:$target, action:$action};

        ($open_prs | map(.headRefName)) as $pr_heads |
        ($remote_branches | map(.name)) as $remote_names |
        ($remote_branches | map(select(.name | startswith("origin/feat/")) | .name | strip_origin)) as $remote_feat |
        ($remote_feat - $pr_heads) as $remote_feat_without_pr |
        ($local_branches | map(select((.upstream // "") != "" and ((.upstream as $u | $remote_names | index($u)) | not)))) as $stale_upstreams |
        ($open_prs | group_by(.title) | map(select(length > 1) | {title:.[0].title, prs:map({number, headRefName, url})})) as $duplicate_titles |
        ($stashes | group_by(.branch // "unknown") | map({branch:(.[0].branch // "unknown"), count:length, scopes:map(.scope // .message), entries:map({name, scope:(.scope // .message)})})) as $stashes_by_branch |
        ($stashes | group_by(.scope // .message) | map({scope:(.[0].scope // .[0].message), count:length, branches:(map(.branch // "unknown") | unique), entries:map(.name)})) as $stashes_by_scope |
        ($status_lines | split("\n") | map(select(length > 0)) | map({status:.[0:2], path:.[3:]})) as $dirty_entries |
        [
          (if $dirty_count > 0 then rec("warning"; "dirty_worktree"; ($dirty_count | tostring); "run organ-anvil triage, then commit or stash by scope") else empty end),
          (if ($stashes | length) > 0 then rec("warning"; "pending_stashes"; (($stashes | length) | tostring); "review stash scopes before merge") else empty end),
          ($remote_feat_without_pr[]? | rec("warning"; "remote_feat_without_open_pr"; .; "inspect branch; create PR or delete remote after owner confirms")),
          ($stale_upstreams[]? | rec("warning"; "local_branch_stale_upstream"; (.name + " -> " + .upstream); "unset upstream, retarget to an active remote, or delete local branch after stashes are safe")),
          ($duplicate_titles[]? | rec("warning"; "duplicate_open_pr_title"; .title; "pick canonical PR and mark older PR superseded")),
          (if (($gates.gate_0.present // false) and (($gates.gate_1.present // false) | not)) then rec("warning"; "gate_1_missing"; ".gate-1"; "run or debug gate-1 before merge") else empty end),
          (if (($coord.available // false) | not) then rec("warning"; "coord_unavailable"; "kernel"; "do not trust coordination state alone") else empty end),
          (if (($coord.available // false) and (($coord.agents | length) == 0) and (($coord.claims | length) == 0)) then rec("info"; "coord_empty"; "kernel"; "cross-check with Git/PR/stashes/processes before assuming no parallel work") else empty end)
        ] as $recommendations |
        {
          version:"1.0.0",
          source:"organ-anvil",
          mode:"repo-health",
          mutating:false,
          timestamp:$timestamp,
          current:{branch:$current_branch, upstream:$upstream, head_sha:$head_sha},
          worktree:{dirty_count:$dirty_count, entries:$dirty_entries},
          branches:{
            local:$local_branches,
            remote:$remote_branches,
            remote_feat_without_open_pr:$remote_feat_without_pr,
            stale_upstreams:$stale_upstreams
          },
          prs:{
            open_count:($open_prs | length),
            open:$open_prs,
            duplicate_titles:$duplicate_titles
          },
          stashes:{count:($stashes | length), entries:$stashes, by_branch:$stashes_by_branch, by_scope:$stashes_by_scope},
          gates:$gates,
          coord:$coord,
          recommendations:$recommendations
        }'
}

coord_snapshot_json() {
    source ~/.cynic-env 2>/dev/null || {
        echo '{"available":false,"error":"missing_env","agents":[],"claims":[]}'
        return 0
    }

    local kernel_addr="${CYNIC_REST_ADDR:-}"
    local api_key="${CYNIC_API_KEY:-}"
    if [ -z "$kernel_addr" ]; then
        echo '{"available":false,"error":"missing_CYNIC_REST_ADDR","agents":[],"claims":[]}'
        return 0
    fi

    case "$kernel_addr" in
        http://*|https://*) ;;
        *) kernel_addr="http://$kernel_addr" ;;
    esac

    local auth_args=()
    if [ -n "$api_key" ]; then
        auth_args=(-H "Authorization: Bearer $api_key")
    fi

    local payload
    payload=$(curl -sS --max-time 3 "${auth_args[@]}" "$kernel_addr/coord/who" 2>/dev/null || true)
    if [ -z "$payload" ] || ! printf '%s' "$payload" | jq empty >/dev/null 2>&1; then
        jq -n --arg error "unreachable_or_invalid_json" '{available:false,error:$error,agents:[],claims:[]}'
        return 0
    fi

    printf '%s' "$payload" | jq '{available:true,agents:(.agents // []),claims:(.claims // [])}'
}

# ============================================================
# MAIN - Execute selon le mode
# ============================================================
mode="${1:-state}"

case "$mode" in
    perceive)
        cd "$PROJECT_DIR"
        perceive
        ;;
    state)
        cd "$PROJECT_DIR"
        write_state
        ;;
    audit)
        audit "$2" "$3" "$4"
        ;;
    signal)
        cd "$PROJECT_DIR"
        signal
        ;;
    triage)
        triage
        ;;
    repo-health)
        repo_health
        ;;
    *)
        echo "Usage: organ-anvil.sh [perceive|state|audit|signal|triage|repo-health]"
        exit 1
        ;;
esac
