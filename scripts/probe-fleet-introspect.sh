#!/bin/bash
# Tailscale MCP fleet introspection probe — fire-and-forget to kernel /event
# Runs every 30s: probes critical services, logs results as Events
# K15 consumer: inference_router uses fleet_stats for node routing

set -euo pipefail

KERNEL_ADDR="${CYNIC_REST_ADDR:-http://localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
MCP_SERVER="${TAILSCALE_MCP_ADDR:-http://localhost:8765}"  # Tailscale MCP JSON-RPC endpoint

# Critical services to probe (add more as needed)
declare -a SERVICES=(
    "cynic-gpu:llama-server:8080"
    "cynic-gpu:hermes:3000"
    "cynic-core:cynic-kernel:3030"
)

log_event() {
    local node=$1
    local service=$2
    local success=$3
    local failure_reason=$4
    local elapsed_ms=$5

    # Construct Event JSON (fire-and-forget to kernel)
    local metadata="{\"service\":\"$service\",\"failure_reason\":\"$failure_reason\"}"

    local event=$(jq -c -n \
        --arg tool "ts_introspect" \
        --arg node "$node" \
        --arg metadata "$metadata" \
        --argjson elapsed_ms "$elapsed_ms" \
        --argjson success "$success" \
        '{tool: $tool, node: $node, elapsed_ms: $elapsed_ms, output_bytes: 256, success: $success, metadata: $metadata, agent_id: "probe-fleet"}')

    # POST to kernel /event (async, fire-and-forget)
    curl -s -X POST "$KERNEL_ADDR/event" \
        -H "Content-Type: application/json" \
        ${API_KEY:+-H "Authorization: Bearer $API_KEY"} \
        -d "$event" > /dev/null 2>&1 || true
}

probe_service() {
    local probe_id=$1
    local node=$2
    local service=$3
    local port=$4

    local start_ms=$(date +%s%N | cut -b1-13)

    # Call ts_introspect via JSON-RPC (MCP)
    local result=$(curl -s -X POST "$MCP_SERVER" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":$probe_id,\"method\":\"tools/call\",\"params\":{\"name\":\"ts_introspect\",\"arguments\":{\"node\":\"$node\",\"service\":\"$service\",\"port\":$port}}}" 2>/dev/null || echo '{}')

    local elapsed_ms=$(($(date +%s%N | cut -b1-13) - start_ms))

    # Extract running + failure_reason from JSON-RPC result
    local running=$(echo "$result" | jq -r '.result.running // false' 2>/dev/null || echo "false")
    local failure_reason=$(echo "$result" | jq -r '.result.failure_reason // "unknown"' 2>/dev/null || echo "unknown")

    # Log Event to kernel
    log_event "$node" "$service" "$running" "$failure_reason" "$elapsed_ms"

    # stderr for debugging (optional: remove for production)
    if [ "$running" != "true" ]; then
        echo "[DEGRADED] $node:$service — $failure_reason (${elapsed_ms}ms)" >&2
    fi
}

main() {
    local probe_id=1

    for entry in "${SERVICES[@]}"; do
        IFS=':' read -r node service port <<< "$entry"
        probe_service "$probe_id" "$node" "$service" "$port"
        probe_id=$((probe_id + 1))
    done
}

main
