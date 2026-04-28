# Tailscale Fleet Introspection Probe

**Status**: Phase 1 (sense organ) — on-demand observation. Action (soma) deferred to Phase 2.

## Architecture

```
hermes_task_runner (Phase 1: live)
    └─ Before task execution
       ├─ probe_service(node, service) — fire-and-forget probe
       │  ├─ Phase 1: stubbed, returns {running:true, failure_reason:"none"}
       │  └─ Phase 2: calls ts_introspect via MCP for real diagnostics
       ├─ Wrap result as Event { tool=ts_introspect, node, success, metadata }
       └─ POST /event (kernel) — stores to activity_log (fire-and-forget, 5s timeout)
           ↓
/fleet-stats (kernel — Phase 2: ready, Phase 1: no-op)
    └─ aggregates Events over time window (success_rate, avg_latency, last_seen)
        ↓
/inference/router (kernel — ACTING CONSUMER)
    └─ queries /fleet-stats for node quality
    └─ degrades/skips nodes marked "dead" or "degraded" (Phase 2 enhancement)
```

**K15 Compliance (Phase 1 → Phase 2)**:
- **Phase 1 (live)**: Producer (hermes_task_runner) → Event storage (fire-and-forget)
- **Phase 2 (ready)**: Consumer (/inference/router) reads fleet_stats, degrades/skips nodes based on failure_reason

**Why on-demand instead of cron?**
- No new daemon (ts_introspect stays stateless MCP tool)
- Probe only when needed (before committing work)
- Fresh data at decision point (Hermes self-preservation)
- Event still flows to fleet_stats for long-term trends

## Current Status

**Phase 1 (live 2026-04-27):**
- ✓ `ts_introspect.go` implemented with FailureReason enum and ProbeData
- ✓ `hermes_task_runner.py` calls `probe_service()` before task execution (stubbed)
- ✓ Events POSTed to `/event` endpoint for K15 storage
- ✗ MCP integration: probe_service() does not yet call ts_introspect via MCP (deferred to Phase 2)
- ✗ Consumer: /inference/router does not yet use failure_reason to degrade nodes (Phase 2 enhancement)

## Setup

### 1. Deploy Tailscale MCP with ts_introspect

```bash
cd /home/user/Bureau/tailscale-mcp
git pull origin main
go build -o tailscale-mcp
./tailscale-mcp  # runs on stdio (MCP protocol)
```

### 2. Install systemd timer

```bash
sudo cp infra/systemd/tailscale-fleet-probe.service /etc/systemd/system/
sudo cp infra/systemd/tailscale-fleet-probe.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now tailscale-fleet-probe.timer
```

### 3. Verify

```bash
# Check probe runs
systemctl status tailscale-fleet-probe.timer
journalctl -u tailscale-fleet-probe.service -f

# Check Events in kernel
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  "http://localhost:3030/fleet-stats?window_secs=120"
```

Expected output:
```json
{
  "nodes": [
    {
      "node": "cynic-gpu",
      "avg_latency_ms": 150,
      "success_rate": 1.0,
      "last_seen_secs": 5,
      "quality": "excellent"
    },
    {
      "node": "cynic-gpu",
      "avg_latency_ms": 0,
      "success_rate": 0.0,
      "last_seen_secs": 35,
      "quality": "dead"  // if llama-server down
    }
  ]
}
```

## Configuration

Edit `scripts/probe-fleet-introspect.sh` to change:

```bash
# Critical services to probe (add more as needed)
declare -a SERVICES=(
    "cynic-gpu:llama-server:8080"   # node:service:port
    "cynic-gpu:hermes:3000"
    "cynic-core:cynic-kernel:3030"
)
```

Add port to `ServiceRegistry` in `tailscale-mcp/mcp/introspect.go` if service is unknown:

```go
var ServiceRegistry = map[string]int{
    "llama-server": 8080,
    "hermes":       3000,
    "nginx":        80,
    "cynic-kernel": 3030,
    "surrealdb":    8000,
    "my-service":   9999,  // add here
}
```

## Observability

### Logs

```bash
# Real-time probe activity
journalctl -u tailscale-fleet-probe.service -f

# Recent degradations
journalctl -u tailscale-fleet-probe.service | grep DEGRADED
```

### Metrics (via /health)

Kernel `/health` includes event statistics:

```json
{
  "activity_log_count": 1247,
  "events_last_1h": 120,
  "nodes_degraded": 1,
  "nodes_dead": 0
}
```

## Troubleshooting

### Probes not running

```bash
systemctl list-timers --all | grep fleet-probe
systemctl start tailscale-fleet-probe.service  # manual trigger
```

### No Events appearing in /fleet-stats

1. Check probe script is executable:
   ```bash
   ./scripts/probe-fleet-introspect.sh  # should print nothing on success
   ```

2. Check Tailscale MCP is running:
   ```bash
   curl http://localhost:8765  # MCP server should respond
   ```

3. Check kernel /event handler:
   ```bash
   curl -X POST http://localhost:3030/event \
     -H "Content-Type: application/json" \
     -d '{"tool":"test","node":"cynic-gpu","elapsed_ms":1,"output_bytes":0,"success":true}'
   ```

4. Check DB connectivity:
   ```bash
   # Kernel logs for store_event errors
   journalctl -u cynic-kernel.service | grep store_event
   ```

## Phase 2: Action (Next Steps)

Once Phase 1 data flow is validated (events flowing → fleet_stats aggregating → /inference/router consuming), move to Phase 2:

### Phase 2a: MCP Integration (hermes_task_runner)
- Replace `probe_service()` stub with real MCP calls to ts_introspect
- Call: `ts_introspect(node, service)` for domain-specific probes
- Update event metadata with actual {running, failure_reason, port_bound, process_id, ...}

### Phase 2b: Consumer Integration (/inference/router)
- Read `metadata` field from fleet_stats aggregation
- Decode failure_reason enum and quality tier (excellent/good/degraded/dead)
- Enhance routing logic: degrade/skip nodes based on failure_reason pattern (not just success_rate)

### Phase 2c: Auto-Recovery (soma organ)
- Watch /fleet-stats for persistent degradation patterns (same node failing >3 consecutive probes)
- Auto-restart failed services (for known service types)
- Rate-limit probes on dead nodes (exponential backoff)
- Route critical degradations to human (T. via Slack #cynic)

## Design Decisions

### Why 30s interval?

- Fast enough to detect outages (~30s latency)
- Slow enough to avoid probe overhead (180 Events/hour)
- Adjustable: edit `tailscale-fleet-probe.timer` `OnUnitActiveSec=`

### Why fire-and-forget?

- Don't block probe on kernel response (probe is lightweight)
- Kernel /event handler bounds task pool (bounded + timeout)
- Failed Events still logged to stderr for debugging

### Why metadata is JSON?

- Structured data (failure_reason enum, port, process_id)
- Extensible (add more fields without schema change)
- Queryable (future: filter /fleet-stats by failure_reason)

## References

- **ts_introspect design**: `tailscale-mcp/mcp/introspect.go`
- **Event struct**: `cynic-kernel/src/domain/storage/types.rs`
- **Fleet stats query**: `cynic-kernel/src/api/rest/event.rs`
- **Consumer (inference router)**: `cynic-kernel/src/api/rest/inference_router.rs`
- **K15 rule**: CLAUDE.md (producer-consumer law)
