#!/usr/bin/env bash
# CYNIC — Unified runtime truth and runtime/ops drift checks for critical processes.
set -euo pipefail

MODE="${1:-truth}"

source ~/.cynic-env 2>/dev/null || true

PROJECT_DIR="${PROJECT_DIR_OVERRIDE:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
UNIT_DIR="${UNIT_DIR_OVERRIDE:-$PROJECT_DIR/infra/systemd}"
OUROBOROS_UNIT="${OUROBOROS_UNIT_OVERRIDE:-$PROJECT_DIR/scripts/cynic-ouroboros.service}"
SYSTEMCTL_BIN="${SYSTEMCTL_BIN_OVERRIDE:-systemctl}"
PGREP_BIN="${PGREP_BIN_OVERRIDE:-pgrep}"
PS_BIN="${PS_BIN_OVERRIDE:-ps}"
SS_BIN="${SS_BIN_OVERRIDE:-ss}"

FAILURES=0
WARNINGS=0

fail() {
    printf 'FAIL Runtime: %s\n' "$*" >&2
    FAILURES=$((FAILURES + 1))
}

warn() {
    printf 'WARN Runtime: %s\n' "$*" >&2
    WARNINGS=$((WARNINGS + 1))
}

show_field() {
    local service="$1"
    local field="$2"
    [ -n "$service" ] || return 0
    "$SYSTEMCTL_BIN" --user show "$service" --property="$field" --value 2>/dev/null || true
}

service_active() {
    local service="$1"
    [ -n "$service" ] || return 0
    "$SYSTEMCTL_BIN" --user is-active "$service" 2>/dev/null || true
}

parse_execstart_binary() {
    local execstart="$1"
    execstart="${execstart#\{}"
    execstart="${execstart%%;*}"
    execstart="${execstart#-}"
    execstart="${execstart%% *}"
    printf '%s\n' "${execstart:-}"
}

process_matches() {
    local pattern="$1"
    "$PGREP_BIN" -xa "$pattern" 2>/dev/null || true
}

is_zombie_pid() {
    local pid="$1"
    local stat
    stat="$(process_stat "$pid")"
    [[ "$stat" == *Z* ]]
}

port_filter() {
    local expected="$1"
    if [ -z "$expected" ] || [ "$expected" = "-" ]; then
        printf '\n'
    else
        printf '%s\n' "${expected##*:}"
    fi
}

find_pid() {
    local service="$1"
    local pattern="$2"
    local expected_port="$3"
    local pid filtered_port matches candidate candidate_pid candidate_ports

    filtered_port="$(port_filter "$expected_port")"

    pid="$(show_field "$service" MainPID)"
    if [[ "$pid" =~ ^[0-9]+$ ]] && [ "$pid" -gt 0 ] && [ -d "/proc/$pid" ]; then
        printf '%s\n' "$pid"
        return
    fi

    matches="$(process_matches "$pattern")"
    [ -n "$matches" ] || return 0

    while IFS= read -r candidate; do
        candidate_pid="${candidate%% *}"
        [ -n "$candidate_pid" ] || continue
        [ -d "/proc/$candidate_pid" ] || continue
        if is_zombie_pid "$candidate_pid"; then
            continue
        fi
        if [ -n "$filtered_port" ]; then
            candidate_ports="$(process_ports "$candidate_pid")"
            if port_contains_expected "$candidate_ports" "$expected_port"; then
                printf '%s\n' "$candidate_pid"
                return
            fi
            continue
        fi
        printf '%s\n' "$candidate_pid"
        return
    done <<< "$matches"
}

process_count() {
    local pattern="$1"
    local expected_port="${2:-}"
    local matches
    local filtered_port count candidate candidate_pid candidate_ports

    matches="$(process_matches "$pattern")"
    [ -n "$matches" ] || {
        printf '0\n'
        return
    }

    filtered_port="$(port_filter "$expected_port")"
    count=0
    while IFS= read -r candidate; do
        candidate_pid="${candidate%% *}"
        [ -n "$candidate_pid" ] || continue
        [ -d "/proc/$candidate_pid" ] || continue
        if is_zombie_pid "$candidate_pid"; then
            continue
        fi
        if [ -n "$filtered_port" ]; then
            candidate_ports="$(process_ports "$candidate_pid")"
            if ! port_contains_expected "$candidate_ports" "$expected_port"; then
                continue
            fi
        fi
        count=$((count + 1))
    done <<< "$matches"
    printf '%s\n' "$count"
}

process_stat() {
    local pid="$1"
    "$PS_BIN" -o stat= -p "$pid" 2>/dev/null | tr -d ' ' || true
}

process_ppid() {
    local pid="$1"
    "$PS_BIN" -o ppid= -p "$pid" 2>/dev/null | tr -d ' ' || true
}

process_binary() {
    local pid="$1"
    readlink -f "/proc/$pid/exe" 2>/dev/null || true
}

process_ports() {
    local pid="$1"
    "$SS_BIN" -H -ltnp 2>/dev/null | awk -v pid="$pid" '$0 ~ ("pid=" pid ",") {print $4}' | paste -sd, - || true
}

run_version() {
    if command -v timeout >/dev/null 2>&1; then
        timeout 2s "$@" 2>/dev/null || true
    else
        "$@" 2>/dev/null || true
    fi
}

detect_version() {
    local binary="$1"
    local output=""

    [ -n "$binary" ] && [ -x "$binary" ] || {
        printf '%s\n' "-"
        return
    }

    for flag in --version version -V -v; do
        output="$(run_version "$binary" "$flag")"
        output="${output%%$'\n'*}"
        if [ -n "$output" ]; then
            printf '%s\n' "$output"
            return
        fi
    done

    printf '%s\n' "-"
}

hardening_value() {
    local unit_file="$1"
    local key="$2"
    awk -F= -v key="$key" '$1 == key {print $2}' "$unit_file" 2>/dev/null | tail -1
}

warn_minimal_profile() {
    local unit_file="$1"
    local label="$2"
    if [ "$(hardening_value "$unit_file" "NoNewPrivileges")" != "true" ]; then
        warn "$label: unit is on a minimal restart-safe profile (NoNewPrivileges missing)"
    fi
}

port_contains_expected() {
    local ports="$1"
    local expected="$2"
    [ -z "$expected" ] && return 0
    [ "$expected" = "-" ] && return 0

    case "$expected" in
        *:*)
            local host="${expected%:*}"
            local port="${expected##*:}"
            case "$ports" in
                *":$port"*|*"$host:$port"*) return 0 ;;
            esac
            ;;
        *)
            case "$ports" in
                *"$expected"*) return 0 ;;
            esac
            ;;
    esac
    return 1
}

print_truth() {
    local label="$1"
    local role="$2"
    local service="$3"
    local pattern="$4"
    local expected_port="$5"
    local active sub state pid ppid binary version ports execstart proc_state duplicates

    active="$(service_active "$service")"
    sub="$(show_field "$service" SubState)"
    pid="$(find_pid "$service" "$pattern" "$expected_port")"
    duplicates="$(process_count "$pattern" "$expected_port")"

    if [ -n "$pid" ]; then
        proc_state="$(process_stat "$pid")"
        if [ -z "$active" ]; then
            state="running/unmanaged"
        elif [ "$active" != "active" ]; then
            state="$active"
            [ -n "$sub" ] && state="$state/$sub"
            state="$state + process-running"
        else
            state="active"
            [ -n "$sub" ] && state="$state/$sub"
        fi
        state="$state ($proc_state)"
        ppid="$(process_ppid "$pid")"
        binary="$(process_binary "$pid")"
        ports="$(process_ports "$pid")"
    else
        state="${active:-inactive}"
        [ -n "$sub" ] && state="$state/$sub"
        ppid="-"
        ports=""
        execstart="$(show_field "$service" ExecStart)"
        binary="$(parse_execstart_binary "$execstart")"
        if [ -z "$binary" ]; then
            binary="$(command -v "$pattern" 2>/dev/null || true)"
        fi
    fi

    [ -n "$binary" ] || binary="-"
    version="$(detect_version "$binary")"
    [ -n "${ports:-}" ] || ports="$expected_port"
    [ -n "${ports:-}" ] || ports="-"

    printf '[%s]\n' "$label"
    printf '  role: %s\n' "$role"
    printf '  state: %s\n' "$state"
    printf '  pid: %s\n' "${pid:--}"
    printf '  parent: %s\n' "${ppid:--}"
    printf '  binary: %s\n' "$binary"
    printf '  version: %s\n' "$version"
    printf '  port: %s\n' "$ports"
    printf '  duplicates: %s\n' "$duplicates"
    printf '\n'
}

check_service_runtime() {
    local label="$1"
    local service="$2"
    local pattern="$3"
    local expected_port="$4"
    local pid active binary ports duplicates

    pid="$(find_pid "$service" "$pattern" "$expected_port")"
    active="$(service_active "$service")"
    binary="-"
    ports=""
    duplicates="$(process_count "$pattern" "$expected_port")"

    if [ "$duplicates" -gt 1 ]; then
        fail "$label: found $duplicates '$pattern' processes"
    fi

    if [ -n "$pid" ]; then
        binary="$(process_binary "$pid")"
        ports="$(process_ports "$pid")"
    fi

    if [ -n "$pid" ] && [ "${active:-}" != "active" ]; then
        fail "$label: service state is '${active:-inactive}' but process pid=$pid is running"
    fi

    if [ "${active:-}" = "active" ] && [ -z "$pid" ]; then
        fail "$label: systemd says active but no process is running"
    fi

    if [ -n "$binary" ] && [[ "$binary" == *" (deleted)"* ]]; then
        fail "$label: running deleted binary '$binary'"
    fi

    if [ -n "$pid" ] && [ -n "$expected_port" ] && [ "$expected_port" != "-" ] && ! port_contains_expected "$ports" "$expected_port"; then
        fail "$label: expected port '$expected_port', got '${ports:-<none>}'"
    fi
}

check_unit_present() {
    local unit_file="$1"
    [ -f "$unit_file" ] || fail "missing repo unit: $unit_file"
}

check_oneshot_service_not_failed() {
    local label="$1"
    local service="$2"
    local active

    active="$(service_active "$service")"
    if [ "$active" = "failed" ]; then
        fail "$label: systemd unit is failed"
    fi
}

run_check_mode() {
    local kernel_unit="$UNIT_DIR/cynic-kernel.service"
    local surreal_unit="$UNIT_DIR/surrealdb.service"
    local health_unit="$UNIT_DIR/cynic-healthcheck.service"

    check_unit_present "$kernel_unit"
    check_unit_present "$surreal_unit"
    check_unit_present "$health_unit"

    if [ ! -f "$UNIT_DIR/llama-server.service" ]; then
        fail "missing repo unit: $UNIT_DIR/llama-server.service"
    fi

    # Hardening must be restart-safe to count. Keep it as diagnosis, not as a
    # hard failure, until the profile is proven compatible under user systemd.
    warn_minimal_profile "$kernel_unit" "cynic-kernel.service"
    warn_minimal_profile "$surreal_unit" "surrealdb.service"

    check_service_runtime "cynic-kernel" "cynic-kernel" "cynic-kernel" "${CYNIC_REST_ADDR:-127.0.0.1:3030}"
    check_service_runtime "surrealdb" "surrealdb" "surreal" "127.0.0.1:8000"
    check_service_runtime "llama-server" "llama-server" "llama-server" "127.0.0.1:8080"
    check_oneshot_service_not_failed "cynic-healthcheck" "cynic-healthcheck"

    # cynic-node stays informational until there is proof of real usage.
    if [ -f "$OUROBOROS_UNIT" ]; then
        if [ "$(hardening_value "$OUROBOROS_UNIT" "NoNewPrivileges")" != "true" ]; then
            warn "$(basename "$OUROBOROS_UNIT"): NoNewPrivileges not set"
        fi
    fi

    if [ "$FAILURES" -gt 0 ]; then
        printf 'Runtime check: %s failure(s), %s warning(s)\n' "$FAILURES" "$WARNINGS" >&2
        exit 1
    fi

    printf '✓ Runtime check passed (%s warning(s))\n' "$WARNINGS"
}

case "$MODE" in
    truth)
        echo "CYNIC Runtime Truth"
        echo "═══════════════════"
        print_truth "cynic-kernel" "kernel-api" "cynic-kernel" "cynic-kernel" "${CYNIC_REST_ADDR:-127.0.0.1:3030}"
        print_truth "surrealdb" "storage" "surrealdb" "surreal" "127.0.0.1:8000"
        print_truth "llama-server" "inference-backend" "llama-server" "llama-server" "127.0.0.1:8080"
        print_truth "embedding-backend" "embedding" "" "llama-server" "127.0.0.1:8081"
        print_truth "cynic-node" "inference-supervisor (frozen until proven useful)" "cynic-node" "cynic-node" "-"
        ;;
    check)
        run_check_mode
        ;;
    *)
        echo "usage: $0 [truth|check]" >&2
        exit 2
        ;;
esac
