#!/bin/bash
# Organism Health Probe — Gather complete state snapshot
# Output: JSON to /tmp/organism_state.json
# Cost: ~4 credits (kernel calls) + gratuit (local probes)

set -e

CYNIC_ROOT="$(git rev-parse --show-toplevel)"
KERNEL_ADDR="${CYNIC_REST_ADDR:-http://<TAILSCALE_CORE>:3030}"
STATE_FILE="/tmp/organism_state.json"
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Helper: safe curl with timeout
probe_kernel() {
  local endpoint="$1"
  curl -s -m 3 ${CYNIC_API_KEY:+-H "Authorization: Bearer ${CYNIC_API_KEY}"} \
    "${KERNEL_ADDR}${endpoint}" 2>/dev/null || echo "{\"error\":\"unreachable\"}"
}

# Helper: JSON encode string
json_string() {
  printf '%s' "$1" | jq -Rs .
}

# === TIER 1: FREE PROBES ===

# Git state
git_status=$(cd "$CYNIC_ROOT" && git status --short 2>/dev/null | wc -l)
git_branch=$(cd "$CYNIC_ROOT" && git rev-parse --abbrev-ref HEAD 2>/dev/null)
git_last_commit=$(cd "$CYNIC_ROOT" && git log -1 --format=%h 2>/dev/null)

# Kernel process
kernel_running=$(pgrep -f "cynic-kernel" > /dev/null 2>&1 && echo "true" || echo "false")
kernel_pid=$(pgrep -f "cynic-kernel" 2>/dev/null || echo "")

# Hermes-X crons
hermes_crons=$(systemctl list-timers --no-pager 2>/dev/null | grep -i "hermes" | wc -l)
hermes_services=$(systemctl list-units --all --no-pager 2>/dev/null | grep -i "hermes" | wc -l)

# Llama-server
llama_running=$(pgrep -i "llama-server" > /dev/null 2>&1 && echo "true" || echo "false")

# Memory usage (top 3 processes)
mem_top3=$(ps aux --sort=-%mem | head -4 | tail -3 | awk '{printf "{\"cmd\":\"%s\",\"mem_mb\":%.1f},", $11, $6/1024}' | sed 's/,$//')

# === TIER 2: KERNEL CALLS (~4 credits) ===

# /health (kernel diagnosis)
kernel_health=$(probe_kernel "/health")

# /state-history (recent state changes)
kernel_history=$(probe_kernel "/state-history?limit=10")

# === BUILD JSON OUTPUT ===

cat > "$STATE_FILE" << EOF
{
  "timestamp": "$TIMESTAMP",
  "git": {
    "branch": "$git_branch",
    "last_commit": "$git_last_commit",
    "dirty_files": $git_status
  },
  "kernel": {
    "running": $kernel_running,
    "pid": "$kernel_pid",
    "address": "$KERNEL_ADDR",
    "health": $kernel_health,
    "history": $kernel_history
  },
  "hermes": {
    "cron_timers": $hermes_crons,
    "systemd_services": $hermes_services
  },
  "llama_server": {
    "running": $llama_running
  },
  "system": {
    "memory_top3": [$mem_top3]
  }
}
EOF

cat "$STATE_FILE"
echo "[$(date '+%H:%M:%S')] Organism state probed → $STATE_FILE"
