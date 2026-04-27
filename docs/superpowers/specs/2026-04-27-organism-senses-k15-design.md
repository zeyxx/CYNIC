# Organism Senses — K15 Consumers + TailscaleReader

**Date:** 2026-04-27
**Status:** Design
**Scope:** RTK K15 compliance, TailscaleReader OrganPort, health_loop integration

---

## Problem

Three organism senses exist or are available, none with K15-compliant consumers:

1. **RTK** — OrganPort shipped (`senses/rtk.rs`), 7 metrics, displayed in `/health` under `senses[]`. No consumer changes system behavior. K15 deadline: 2026-05-26.
2. **Tailscale** — 13 MCP tools available, `tailscale status --json` works locally. Kernel has zero visibility into fleet state. Dogs timeout after 60s before kernel knows the node is down.
3. **Hermes-X** — OrganPort shipped (`senses/hermes_x.rs`), displayed in `/health`. Same K15 gap.

Display is not consumption. `/health` showing `"senses": [{"name": "rtk", "health": "alive"}]` does not change system behavior.

## Design

### Part 1: RTK K15 Consumers

#### Consumer A — Metabolism Alerts

RTK snapshot metrics feed the existing alert system in `health_loop`.

**Delta tracking:** Health_loop maintains `SenseState` — a `HashMap<String, OrganSnapshot>` storing the previous snapshot per organ. On each tick: `delta = current_counter - previous_counter`. This is required for Counter metrics (cumulative values need diffing to produce rates).

```rust
struct SenseState {
    previous: HashMap<String, OrganSnapshot>,
}
```

**Trigger conditions:**
- `savings_pct` gauge < 38.2% (phi^-2) — token filtering is degraded
- `parse_failures` counter delta > 50 per tick interval (20s) — RTK is broken/misconfigured

**Output:** New alert kind `metabolism_anomaly` in the `alerts[]` array of `/health`.

```rust
AlertKind::MetabolismAnomaly {
    metric: String,    // "savings_pct" | "parse_failures" | "burn_rate"
    value: f64,
    threshold: f64,
}
```

**Consumer contract:** The alert array is consumed by session-init hook which reads `/health` and **prints alerts prominently in the terminal at every session start**. This is not passive display — the human sees the alert before any work begins, creating a forcing function to investigate. The `introspection` background task also clusters alerts for trend detection.

K15 satisfied: alert routed to human → human investigates → behavior change.

#### Consumer B — Session Cost Budget Alert

RTK tracks session-level token consumption (Claude Code tokens). This is a different cost pool from `estimated_cost_usd` (which tracks Dog LLM tokens). Add a new field and alert.

**Mechanism:**
- Health_loop stores previous RTK `tokens_input` snapshot in `SenseState` (see delta tracking below)
- On each tick, compute `session_tokens_delta = current.tokens_input - previous.tokens_input`
- Compute `session_burn_rate = delta / elapsed_seconds * 3600` (tokens/hour)
- If burn rate > 2M tokens/hour → emit `MetabolismAnomaly` alert (budget warning)

**Output:** New field `session_cost` in `/health` (separate from `estimated_cost_usd` which tracks Dog costs):

```rust
"session_cost": {
    "tokens_input": 1_200_000,
    "tokens_saved": 950_000,
    "savings_pct": 79.2,
    "burn_rate_per_hour": 480_000
}
```

**Consumer:** The burn rate alert triggers the same alert pipeline as Consumer A. The budget warning is routed to the human via session-init and introspection. K15-compliant: system behavior changes (human sees warning, adjusts session behavior).

### Part 2: TailscaleReader OrganPort

New `senses/tailscale.rs` implementing `OrganPort`.

**Data source:** `tailscale status --json` executed via `tokio::process::Command::output().await` wrapped in `tokio::time::timeout(Duration::from_secs(5), ...)`. No SSH, no MCP — local async CLI call on cynic-core.

**NOT `spawn_blocking`** — `tokio::process::Command` is already async (unlike rusqlite in RtkReader which is synchronous and requires `spawn_blocking`). Pattern matches `infra/remediation.rs` subprocess calls.

**Metrics:**
| Key | Kind | Unit | Source |
|-----|------|------|--------|
| `nodes_online` | Gauge | count | `Peer[*].Online == true` |
| `nodes_total` | Gauge | count | `len(Peer)` excluding funnel-ingress |
| `nodes_ssh_ready` | Gauge | count | `Peer[*].SSH == true && Online == true` |
| `gpu_online` | Gauge | bool | `Peer["cynic-gpu"].Online` |
| `gpu_last_seen` | Gauge | seconds_ago | `now - Peer["cynic-gpu"].LastSeen` |

**Health logic:**
- `Alive` — `tailscale status --json` succeeds, self node present
- `Degraded` — command succeeds but no peers found, or parse error
- `Dead` — command fails (tailscale not running)

**Freshness:** Age of newest `LastSeen` across all SSH-capable peers.

**Filtering:** Skip `funnel-ingress-node` peers (observed: 22 of 29 peers are Tailscale infrastructure, not our fleet).

```rust
pub struct TailscaleReader {
    /// Peer hostnames to track (from fleet.toml or hardcoded initial set)
    fleet_nodes: Vec<String>,
}
```

**Registration in `senses/mod.rs`:**
```rust
// Tailscale — fleet nervous system (local CLI)
if which::which("tailscale").is_ok() {
    senses.push(Arc::new(tailscale::TailscaleReader::new(
        vec!["cynic-core", "cynic-gpu", "kairos"],
    )));
}
```

### Part 3: Health Loop Consumes TailscaleReader

The `health_loop` already ticks every 20s and updates Dog circuit state. Add fleet awareness:

**Mechanism:**
1. On each tick, read TailscaleReader snapshot
2. For each Dog with a known fleet node mapping (e.g., `qwen35-9b-gpu` → `cynic-gpu`):
   - If node is offline → preemptively open Dog's circuit breaker
   - If node comes back online → allow normal health probe to close circuit

**Dog-to-node mapping:** Extend `backends.toml` with optional `node` field:

```toml
[qwen35-9b-gpu]
url = "http://<TAILSCALE_GPU>:8080/v1/chat/completions"
fleet_node = "cynic-gpu"  # Tailscale hostname for fleet awareness (distinct from remediation.node which is SSH target)
```

**Parse path:** `infra/boot.rs::build_fleet_targets` currently parses Dog config as TOML tables. The `node` field is added as `Option<String>` to the existing `DogConfig` struct (or equivalent). At health_loop startup, build a `HashMap<String, String>` mapping `dog_id → node_hostname` from all backends with a `node` field. This map is stored in `SenseState` and consulted on each tick.

```rust
// Built once at startup from backends.toml
dog_to_node: HashMap<String, String>,
// e.g. {"qwen35-9b-gpu" => "cynic-gpu", "gemma-4-e4b-core" => "cynic-core"}
```

**Consumer contract:** Circuit breaker state change = system behavior change. A Dog marked Dead stops receiving `/judge` traffic. This is K15-compliant.

**Hysteresis (required from day 1):** `cynic-gpu` is a Windows host that sleeps regularly. Tailscale reports `Online=false` during sleep, but the node recovers within seconds of wake. Without hysteresis, the circuit breaker oscillates on every sleep/wake cycle.

**Rule:** Node must be offline for **>30 seconds across 2 consecutive ticks** before marking Dogs preemptively Dead. Health_loop tracks `first_offline_at: Option<Instant>` per fleet node in `SenseState`.

```rust
// In SenseState
node_offline_since: HashMap<String, Instant>,
```

- Tick N: node offline → record `first_offline_at = now`
- Tick N+1 (20s later): node still offline, elapsed > 30s → open circuit
- If node comes back before 30s → clear `first_offline_at`, no action

**Preemptive vs reactive (with hysteresis):**
- Current: Dog fails → 3 retries → circuit opens → 60s recovery window
- With fleet sense: Node offline >30s → circuit opens immediately → zero wasted retries
- Saves: 3 timeout waits (12s each = 36s) per Dog per sustained outage
- Sleep/wake flaps: no false positives (30s buffer absorbs typical <10s reconnect)

### Part 4: Alert on Fleet Drift

New alert kind for fleet state:

```rust
AlertKind::FleetDrift {
    node: String,
    expected: String,  // "online"
    actual: String,    // "offline since 2h"
}
```

Trigger: Any fleet node in `TailscaleReader.fleet_nodes` offline for > 5 minutes.

**Also triggers on organ death:** If any registered OrganPort returns `OrganHealth::Dead` for > 2 consecutive ticks → emit `FleetDrift` alert with `node: "organ:<name>"`. This catches the "registered but dead" state (e.g., tailscale binary removed from PATH after kernel startup — W3).

Consumer: Same alert pipeline as Consumer A.

### Part 5: What This Design Does NOT Include

- **Soma module** — premature. Let patterns emerge from 3+ senses before naming.
- **Auto-heal via ts_manage** — requires kernel → MCP bridge (not designed). Future work.
- **RTK on remote nodes** — maintenance cost too high for 2-node fleet. Revisit at N>4.
- **MCP Tailscale OS-normalization** — separate concern, lives in MCP server code.
- **Dashboard/cynic-ui** — no consumer (K15).
- **Hermes-X K15 consumer** — same pattern, separate spec.

## File Changes

| File | Change |
|------|--------|
| `cynic-kernel/src/senses/tailscale.rs` | NEW — TailscaleReader OrganPort impl |
| `cynic-kernel/src/senses/mod.rs` | Add tailscale module + registration |
| `cynic-kernel/src/introspection.rs` | Add `metabolism_anomaly` + `fleet_drift` alert kinds (alerts use `kind: &'static str`, not enum) |
| `cynic-kernel/src/background/health_loop.rs` | Consume RTK + Tailscale snapshots |
| `cynic-kernel/src/background/introspection.rs` | Cluster new alert kinds |
| `cynic-kernel/Cargo.toml` | Add `which` crate (check tailscale binary exists) |
| `backends.toml` | Add optional `node` field to Dog entries |
| `cynic-kernel/tests/integration_tailscale_reader.rs` | NEW — #[ignore] tests against real tailscale |
| `cynic-kernel/tests/integration_rtk_consumer.rs` | NEW — alert trigger tests |

## Dependency Budget

| Crate | Why | Size |
|-------|-----|------|
| `which` | Check if `tailscale` binary exists at startup | ~15KB, 0 transitive deps |

`serde_json` already in workspace (for parsing `tailscale status --json` output).

## K15 Compliance Audit

| Producer | Consumer | Action | K15 |
|----------|----------|--------|-----|
| RTK snapshot (savings_pct) | health_loop → MetabolismAnomaly alert | Human alerted at session start, investigates | PASS |
| RTK snapshot (parse_failures delta) | health_loop → MetabolismAnomaly alert | Human alerted, RTK is broken | PASS |
| RTK snapshot (tokens_input delta) | health_loop → burn_rate alert | Human warned on excessive spend | PASS |
| Tailscale snapshot (gpu_online) | health_loop → circuit breaker (30s hysteresis) | Dog traffic rerouted preemptively | PASS |
| Tailscale snapshot (node offline >5m) | alert pipeline → FleetDrift | Human alerted on fleet outage | PASS |
| Any OrganPort health → Dead (>2 ticks) | alert pipeline → FleetDrift | Human alerted on organ death | PASS |

## Falsification

- If RTK savings_pct never drops below 38.2% in 30 days → threshold too low, recalibrate
- If TailscaleReader adds >5ms to health_loop tick → `tailscale status` too slow, cache with TTL
- If 30s hysteresis still causes false positives on cynic-gpu sleep/wake → increase to 60s
- If 30s hysteresis misses real outages (GPU crashes but flap buffer delays detection) → decrease to 15s
- If `which::which("tailscale")` fails on production cynic-core → binary not in PATH, fix systemd env
- If parse_failures delta > 50/tick never triggers → threshold too high, check actual parse_failures growth rate

## Testing Strategy

1. **Unit tests** (in-module): TailscaleReader with mock JSON output, alert threshold triggers
2. **Delta tracking tests**: Inject two consecutive snapshots with parse_failures delta=51 → verify alert emitted; delta=49 → no alert. Same for burn_rate threshold.
3. **Hysteresis tests**: Simulate node offline for 1 tick (20s) → no circuit break. Node offline for 2 ticks (40s) → circuit opens. Node back online mid-sequence → clear state.
4. **Integration tests** (#[ignore]): Real `tailscale status --json` on cynic-core, real RTK history.db
5. **Gate test** (make test-gates): Inject fake RTK snapshot with savings_pct=0.10 → verify MetabolismAnomaly alert emitted
