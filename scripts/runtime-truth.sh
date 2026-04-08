#!/usr/bin/env bash
# CYNIC — Unified runtime truth for critical processes.
set -euo pipefail

source ~/.cynic-env 2>/dev/null || true

show_field() {
    local service="$1"
    local field="$2"
    systemctl --user show "$service" --property="$field" --value 2>/dev/null || true
}

parse_execstart_binary() {
    local execstart="$1"
    execstart="${execstart#\{}"
    execstart="${execstart%%;*}"
    execstart="${execstart#-}"
    execstart="${execstart%% *}"
    printf '%s\n' "${execstart:-}"
}

find_pid() {
    local service="$1"
    local pattern="$2"
    local pid

    pid="$(show_field "$service" MainPID)"
    if [[ "$pid" =~ ^[0-9]+$ ]] && [ "$pid" -gt 0 ] && [ -d "/proc/$pid" ]; then
        printf '%s\n' "$pid"
        return
    fi

    pgrep -xo "$pattern" 2>/dev/null || true
}

process_stat() {
    local pid="$1"
    ps -o stat= -p "$pid" 2>/dev/null | tr -d ' ' || true
}

process_ppid() {
    local pid="$1"
    ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' ' || true
}

process_binary() {
    local pid="$1"
    readlink -f "/proc/$pid/exe" 2>/dev/null || true
}

process_ports() {
    local pid="$1"
    ss -H -ltnp 2>/dev/null | awk -v pid="$pid" '$0 ~ ("pid=" pid ",") {print $4}' | paste -sd, - || true
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

    output="$(run_version "$binary" --version)"
    output="${output%%$'\n'*}"
    if [ -n "$output" ]; then
        printf '%s\n' "$output"
        return
    fi

    output="$(run_version "$binary" version)"
    output="${output%%$'\n'*}"
    if [ -n "$output" ]; then
        printf '%s\n' "$output"
        return
    fi

    output="$(run_version "$binary" -V)"
    output="${output%%$'\n'*}"
    if [ -n "$output" ]; then
        printf '%s\n' "$output"
        return
    fi

    output="$(run_version "$binary" -v)"
    output="${output%%$'\n'*}"
    if [ -n "$output" ]; then
        printf '%s\n' "$output"
        return
    fi

    printf '%s\n' "-"
}

print_truth() {
    local label="$1"
    local role="$2"
    local service="$3"
    local pattern="$4"
    local expected_port="$5"
    local active sub state pid ppid binary version ports execstart proc_state

    active="$(systemctl --user is-active "$service" 2>/dev/null || true)"
    sub="$(show_field "$service" SubState)"
    pid="$(find_pid "$service" "$pattern")"

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
    printf '\n'
}

echo "CYNIC Runtime Truth"
echo "═══════════════════"
print_truth "cynic-kernel" "kernel-api" "cynic-kernel" "cynic-kernel" "${CYNIC_REST_ADDR:-127.0.0.1:3030}"
print_truth "surrealdb" "storage" "surrealdb" "surreal" "127.0.0.1:8000"
print_truth "llama-server" "inference-backend" "llama-server" "llama-server" "-"
print_truth "cynic-node" "inference-supervisor (frozen until proven useful)" "cynic-node" "cynic-node" "-"
