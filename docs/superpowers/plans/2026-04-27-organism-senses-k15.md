# Organism Senses K15 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire K15-compliant consumers for RTK and add TailscaleReader OrganPort with preemptive Dog circuit breaking.

**Architecture:** Three additions: (1) `senses/tailscale.rs` — new OrganPort reading `tailscale status --json` via async subprocess, (2) introspection consumes RTK snapshots for metabolism alerts, (3) health_loop consumes Tailscale snapshots for preemptive Dog marking with 30s hysteresis.

**Tech Stack:** Rust, tokio::process::Command, serde_json (existing), `which` crate (new)

**Spec:** `docs/superpowers/specs/2026-04-27-organism-senses-k15-design.md`

---

### Task 1: TailscaleReader — OrganPort impl

**Files:**
- Create: `cynic-kernel/src/senses/tailscale.rs`
- Modify: `cynic-kernel/src/senses/mod.rs`
- Modify: `cynic-kernel/Cargo.toml` (add `which`)

- [ ] **Step 1: Add `which` dependency**

In `cynic-kernel/Cargo.toml` under `[dependencies]`:
```toml
which = "7"
```

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 2: Write TailscaleReader struct + unit test for health_dead**

Create `cynic-kernel/src/senses/tailscale.rs`:

```rust
//! TailscaleReader — fleet nervous system via `tailscale status --json`.
//! Async subprocess call (not spawn_blocking — tokio::process is already async).
//! Filters funnel-ingress-node peers (Tailscale infra, not our fleet).

use crate::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

#[derive(Debug)]
pub struct TailscaleReader {
    /// Peer hostnames to track (our fleet, not all Tailscale peers)
    fleet_nodes: Vec<String>,
}

impl TailscaleReader {
    pub fn new(fleet_nodes: Vec<String>) -> Self {
        Self { fleet_nodes }
    }

    /// Parse `tailscale status --json` output into metrics.
    /// Extracted for testability — unit tests pass mock JSON.
    fn parse_status(json: &str, fleet_nodes: &[String]) -> Result<Vec<Metric>, OrganError> {
        let v: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| OrganError::ReadFailed(format!("JSON parse: {e}")))?;

        let peers = v.get("Peer")
            .and_then(|p| p.as_object())
            .ok_or_else(|| OrganError::ReadFailed("missing Peer object".into()))?;

        let mut online_count = 0i64;
        let mut ssh_ready_count = 0i64;
        let mut fleet_total = 0i64;
        let mut metrics = Vec::new();

        for peer in peers.values() {
            let hostname = peer.get("HostName")
                .and_then(|h| h.as_str())
                .unwrap_or("");

            // Skip funnel-ingress-node (Tailscale infra, 22 of 29 peers)
            if hostname == "funnel-ingress-node" {
                continue;
            }

            // Only track fleet nodes if specified
            if !fleet_nodes.is_empty() && !fleet_nodes.iter().any(|n| n == hostname) {
                continue;
            }

            fleet_total += 1;
            let is_online = peer.get("Online")
                .and_then(|o| o.as_bool())
                .unwrap_or(false);
            if is_online {
                online_count += 1;
            }

            // SSH ready = online + has SSH capability
            let has_ssh = peer.get("SSH").and_then(|s| s.as_bool()).unwrap_or(false)
                || peer.get("Capabilities").and_then(|c| c.as_object())
                    .map(|c| c.contains_key("ssh"))
                    .unwrap_or(false);
            if is_online && has_ssh {
                ssh_ready_count += 1;
            }

            // Per-node online status for fleet nodes
            metrics.push(Metric {
                key: format!("node_{}_online", hostname),
                value: MetricValue::Bool(is_online),
                kind: MetricKind::Gauge,
                unit: None,
            });
        }

        metrics.insert(0, Metric {
            key: "nodes_online".into(),
            value: MetricValue::I64(online_count),
            kind: MetricKind::Gauge,
            unit: Some("count".into()),
        });
        metrics.insert(1, Metric {
            key: "nodes_total".into(),
            value: MetricValue::I64(fleet_total),
            kind: MetricKind::Gauge,
            unit: Some("count".into()),
        });
        metrics.insert(2, Metric {
            key: "nodes_ssh_ready".into(),
            value: MetricValue::I64(ssh_ready_count),
            kind: MetricKind::Gauge,
            unit: Some("count".into()),
        });

        Ok(metrics)
    }

    /// Run `tailscale status --json` with a 5s timeout.
    async fn run_tailscale_cmd() -> Result<String, OrganError> {
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("tailscale")
                .args(["status", "--json"])
                .output(),
        )
        .await
        .map_err(|_| OrganError::ReadFailed("tailscale status timed out (5s)".into()))?
        .map_err(|e| OrganError::ReadFailed(format!("tailscale exec: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OrganError::ReadFailed(format!(
                "tailscale status exit {}: {stderr}",
                output.status
            )));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| OrganError::ReadFailed(format!("UTF-8: {e}")))
    }
}

#[async_trait]
impl OrganPort for TailscaleReader {
    fn name(&self) -> &str {
        "tailscale"
    }

    async fn health(&self) -> OrganHealth {
        match Self::run_tailscale_cmd().await {
            Ok(json) => {
                // Verify we can parse it
                match Self::parse_status(&json, &self.fleet_nodes) {
                    Ok(_) => OrganHealth::Alive,
                    Err(OrganError::ReadFailed(reason)) => OrganHealth::Degraded { reason },
                    Err(OrganError::Unavailable(reason)) => OrganHealth::Dead { reason },
                }
            }
            Err(OrganError::ReadFailed(reason)) => OrganHealth::Degraded { reason },
            Err(OrganError::Unavailable(reason)) => OrganHealth::Dead { reason },
        }
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        // Tailscale status is always live — freshness = 0.
        // Unlike RTK (reads historical DB) or Hermes (reads cached files),
        // tailscale status probes the daemon's current state.
        Ok(Duration::ZERO)
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        let json = Self::run_tailscale_cmd().await?;
        let metrics = Self::parse_status(&json, &self.fleet_nodes)?;
        Ok(OrganSnapshot {
            taken_at: Utc::now(),
            metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_STATUS: &str = r#"{
        "Self": {"HostName": "cynic-core"},
        "Peer": {
            "abc": {"HostName": "cynic-gpu", "Online": true, "SSH": true, "OS": "windows"},
            "def": {"HostName": "kairos", "Online": false, "SSH": true, "OS": "linux"},
            "ghi": {"HostName": "funnel-ingress-node", "Online": true}
        }
    }"#;

    #[test]
    fn parse_filters_funnel_ingress() {
        let metrics = TailscaleReader::parse_status(
            MOCK_STATUS,
            &["cynic-gpu".into(), "kairos".into()],
        ).unwrap();
        // Should NOT include funnel-ingress-node
        assert!(metrics.iter().all(|m| !m.key.contains("funnel")));
    }

    #[test]
    fn parse_counts_online_nodes() {
        let metrics = TailscaleReader::parse_status(
            MOCK_STATUS,
            &["cynic-gpu".into(), "kairos".into()],
        ).unwrap();
        let online = metrics.iter().find(|m| m.key == "nodes_online").unwrap();
        match &online.value {
            MetricValue::I64(v) => assert_eq!(*v, 1), // only cynic-gpu is online
            other => panic!("expected I64, got {other:?}"),
        }
    }

    #[test]
    fn parse_per_node_status() {
        let metrics = TailscaleReader::parse_status(
            MOCK_STATUS,
            &["cynic-gpu".into(), "kairos".into()],
        ).unwrap();
        let gpu = metrics.iter().find(|m| m.key == "node_cynic-gpu_online").unwrap();
        match &gpu.value {
            MetricValue::Bool(v) => assert!(*v),
            other => panic!("expected Bool, got {other:?}"),
        }
        let kairos = metrics.iter().find(|m| m.key == "node_kairos_online").unwrap();
        match &kairos.value {
            MetricValue::Bool(v) => assert!(!*v),
            other => panic!("expected Bool, got {other:?}"),
        }
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = TailscaleReader::parse_status("not json", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_fleet_tracks_all_non_funnel() {
        let metrics = TailscaleReader::parse_status(MOCK_STATUS, &[]).unwrap();
        let total = metrics.iter().find(|m| m.key == "nodes_total").unwrap();
        match &total.value {
            MetricValue::I64(v) => assert_eq!(*v, 2), // cynic-gpu + kairos (no funnel)
            other => panic!("expected I64, got {other:?}"),
        }
    }
}
```

Run: `cargo test -p cynic-kernel senses::tailscale --lib`
Expected: 5 tests pass

- [ ] **Step 3: Register TailscaleReader in senses/mod.rs**

Add to `cynic-kernel/src/senses/mod.rs`:
```rust
pub mod tailscale;
```

And in `build_sense_registry()`:
```rust
// Tailscale — fleet nervous system (local CLI)
if which::which("tailscale").is_ok() {
    senses.push(Arc::new(tailscale::TailscaleReader::new(
        vec![
            "cynic-core".into(),
            "cynic-gpu".into(),
            "kairos".into(),
        ],
    )));
}
```

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/senses/tailscale.rs cynic-kernel/src/senses/mod.rs cynic-kernel/Cargo.toml
git commit -m "feat(senses): add TailscaleReader OrganPort — fleet nervous system"
```

---

### Task 2: RTK metabolism alerts in introspection

**Files:**
- Modify: `cynic-kernel/src/introspection.rs`

- [ ] **Step 1: Write failing test for metabolism alert**

Add to `introspection.rs` `mod tests`:
```rust
#[test]
fn metabolism_low_savings_triggers_alert() {
    let alerts = check_sense_health(&make_rtk_snapshot(25.0, 10));
    assert!(alerts.iter().any(|a| a.kind == "metabolism_anomaly"));
}

#[test]
fn metabolism_healthy_savings_no_alert() {
    let alerts = check_sense_health(&make_rtk_snapshot(85.0, 0));
    assert!(alerts.iter().all(|a| a.kind != "metabolism_anomaly"));
}
```

With test helpers:
```rust
fn make_rtk_snapshot(savings_pct: f64, parse_failures: i64) -> Vec<(String, crate::domain::organ::OrganSnapshot)> {
    use crate::domain::organ::{Metric, MetricKind, MetricValue, OrganSnapshot as OSnap};
    vec![("rtk".into(), OSnap {
        taken_at: chrono::Utc::now(),
        metrics: vec![
            Metric { key: "savings_pct".into(), value: MetricValue::F64(savings_pct), kind: MetricKind::Gauge, unit: None },
            Metric { key: "parse_failures".into(), value: MetricValue::I64(parse_failures), kind: MetricKind::Counter, unit: None },
        ],
    })]
}
```

Run: `cargo test -p cynic-kernel introspection::tests::metabolism --lib`
Expected: FAIL — `check_sense_health` not found

- [ ] **Step 2: Implement `check_sense_health`**

Add to `introspection.rs`:
```rust
use crate::domain::organ::{MetricValue, OrganSnapshot as SenseSnapshot};

/// φ⁻² = 0.382 — minimum acceptable savings rate.
const SAVINGS_PCT_THRESHOLD: f64 = 0.382 * 100.0; // gauge is 0-100

/// Check sense snapshots for anomalies. Pure function — no I/O.
pub fn check_sense_health(
    snapshots: &[(String, SenseSnapshot)],
) -> Vec<Alert> {
    let mut alerts = Vec::new();

    for (name, snap) in snapshots {
        for metric in &snap.metrics {
            match (name.as_str(), metric.key.as_str()) {
                ("rtk", "savings_pct") => {
                    if let MetricValue::F64(pct) = &metric.value {
                        if *pct < SAVINGS_PCT_THRESHOLD {
                            alerts.push(Alert {
                                kind: "metabolism_anomaly",
                                message: format!(
                                    "RTK savings {:.1}% below threshold {:.1}% — token filtering degraded",
                                    pct, SAVINGS_PCT_THRESHOLD
                                ),
                                severity: if *pct < 20.0 { "critical" } else { "warning" },
                            });
                        }
                    }
                }
                _ => {} // Other senses: extend here as consumers are added
            }
        }
    }

    alerts
}
```

Run: `cargo test -p cynic-kernel introspection::tests::metabolism --lib`
Expected: PASS

- [ ] **Step 3: Wire `check_sense_health` into `analyze()`**

Modify `analyze()` signature to accept sense snapshots:
```rust
pub async fn analyze(
    storage: &dyn StoragePort,
    metrics: &Metrics,
    environment: &Option<EnvironmentSnapshot>,
    sense_snapshots: &[(String, SenseSnapshot)],
) -> Vec<Alert> {
```

Add at end of `analyze()`, before the log block:
```rust
    // ── Sense metabolism (K15 consumer for RTK + future senses) ──
    alerts.extend(check_sense_health(sense_snapshots));
```

Update all callers (in `tasks/runtime_loops.rs` — `spawn_introspection`) to pass `&[]` or read from senses.

Run: `cargo check -p cynic-kernel`
Expected: compiles (callers pass empty slice initially)

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/introspection.rs
git commit -m "feat(introspection): RTK metabolism alerts — K15 consumer for savings_pct"
```

---

### Task 3: Introspection reads live sense snapshots

**Files:**
- Modify: `cynic-kernel/src/infra/tasks.rs` (where `spawn_introspection` lives, line 376)
- Modify: `cynic-kernel/src/main.rs` (where `spawn_introspection` is called, line 629)

- [ ] **Step 1: Add `senses` parameter to `spawn_introspection`**

In `cynic-kernel/src/infra/tasks.rs`, modify `spawn_introspection` signature (line 376):
```rust
pub fn spawn_introspection(
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    introspection_alerts: Arc<std::sync::RwLock<Vec<crate::introspection::Alert>>>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    senses: Vec<Arc<dyn crate::domain::organ::OrganPort>>,  // NEW
    shutdown: CancellationToken,
) -> JoinHandle<()> {
```

Inside the task, before calling `analyze()` (line 409), add:
```rust
// Read all sense snapshots — best-effort, skip failures
let mut sense_snapshots = Vec::new();
for sense in &senses {
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sense.snapshot(),
    ).await {
        Ok(Ok(snap)) => sense_snapshots.push((sense.name().to_string(), snap)),
        Ok(Err(e)) => tracing::debug!(organ = sense.name(), error = %e, "sense snapshot failed"),
        Err(_) => tracing::debug!(organ = sense.name(), "sense snapshot timed out"),
    }
}
```

Pass `&sense_snapshots` to `analyze()`.

- [ ] **Step 2: Update caller in main.rs**

In `main.rs` (line 629), add the new parameter:
```rust
infra::tasks::spawn_introspection(
    Arc::clone(&storage_port),
    Arc::clone(&metrics),
    Arc::clone(&environment),
    Arc::clone(&rest_state.introspection_alerts),
    event_tx.clone(),
    Arc::clone(&task_health),
    rest_state.senses.clone(),  // NEW — pass senses from AppState
    shutdown.clone(),
);
```

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/infra/tasks.rs cynic-kernel/src/main.rs
git commit -m "feat(introspection): wire live sense snapshots into analyze()"
```

---

### Task 4: Fleet node mapping in BackendConfig

**Files:**
- Modify: `cynic-kernel/src/infra/config.rs`
- Modify: `cynic-kernel/src/infra/boot.rs`

- [ ] **Step 1: Add `fleet_node` to BackendEntry**

In `config.rs`, add to `BackendEntry`:
```rust
/// Tailscale hostname for fleet awareness — maps this Dog to a fleet node.
/// Used by health_loop for preemptive circuit breaking when node goes offline.
fleet_node: Option<String>,
```

Add to `BackendConfig`:
```rust
/// Tailscale hostname — if set, health_loop can preemptively open circuit
/// when this node goes offline (before Dog times out).
pub fleet_node: Option<String>,
```

In `load_backends()`, add to the config construction:
```rust
fleet_node: entry.fleet_node,
```

- [ ] **Step 2: Expose dog-to-node map from boot.rs**

Add to `DogsAndOrgan`:
```rust
/// Dog ID → Tailscale hostname. Used by health_loop for preemptive marking.
pub dog_to_fleet_node: HashMap<String, String>,
```

In `build_dogs_and_organ()`, build the map:
```rust
let mut dog_to_fleet_node: HashMap<String, String> = HashMap::new();
```

And for each backend, after constructing the Dog:
```rust
if let Some(node) = &cfg.fleet_node {
    dog_to_fleet_node.insert(name.clone(), node.clone());
}
```

- [ ] **Step 3: Update backends.toml**

Add `fleet_node` to relevant Dogs:
```toml
[backend.qwen35-9b-gpu]
fleet_node = "cynic-gpu"

[backend.gemma-4-e4b-core]
fleet_node = "cynic-core"
```

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/infra/config.rs cynic-kernel/src/infra/boot.rs backends.toml
git commit -m "feat(config): add fleet_node mapping for preemptive Dog marking"
```

---

### Task 5: Preemptive circuit breaking with hysteresis

**Files:**
- Modify: `cynic-kernel/src/infra/health_loop.rs`

- [ ] **Step 1: Write failing test for hysteresis logic**

```rust
#[cfg(test)]
mod hysteresis_tests {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use super::FleetAwareness;

    #[test]
    fn first_offline_does_not_trigger() {
        let mut fa = FleetAwareness::new(HashMap::from([
            ("gpu-dog".into(), "cynic-gpu".into()),
        ]));
        let offline_nodes: Vec<String> = vec!["cynic-gpu".into()];
        let to_mark = fa.tick(&offline_nodes);
        assert!(to_mark.is_empty(), "first tick should not mark anything");
    }

    #[test]
    fn sustained_offline_triggers_after_threshold() {
        let mut fa = FleetAwareness::new(HashMap::from([
            ("gpu-dog".into(), "cynic-gpu".into()),
        ]));
        // First tick: record offline
        fa.tick(&["cynic-gpu".into()]);
        // Simulate time passing (>30s) by backdating the Instant
        fa.node_offline_since.insert(
            "cynic-gpu".into(),
            Instant::now() - Duration::from_secs(31),
        );
        // Second tick: should trigger
        let to_mark = fa.tick(&["cynic-gpu".into()]);
        assert_eq!(to_mark, vec!["gpu-dog"]);
    }

    #[test]
    fn node_comes_back_clears_state() {
        let mut fa = FleetAwareness::new(HashMap::from([
            ("gpu-dog".into(), "cynic-gpu".into()),
        ]));
        fa.tick(&["cynic-gpu".into()]);
        // Node comes back
        fa.tick(&[]);
        assert!(!fa.node_offline_since.contains_key("cynic-gpu"));
    }
}
```

Run: `cargo test -p cynic-kernel health_loop::hysteresis_tests --lib`
Expected: FAIL — `FleetAwareness` not found

- [ ] **Step 2: Implement FleetAwareness**

Add to `health_loop.rs`:

```rust
use std::collections::HashMap;
use std::time::Instant;

/// Fleet-aware hysteresis — tracks node offline duration.
/// Only marks Dogs after a node has been offline for HYSTERESIS_THRESHOLD
/// across at least 2 consecutive ticks.
pub(crate) struct FleetAwareness {
    /// Dog ID → Tailscale hostname
    dog_to_node: HashMap<String, String>,
    /// Node hostname → first time seen offline
    pub(crate) node_offline_since: HashMap<String, Instant>,
}

const HYSTERESIS_THRESHOLD: Duration = Duration::from_secs(30);

impl FleetAwareness {
    pub fn new(dog_to_node: HashMap<String, String>) -> Self {
        Self {
            dog_to_node,
            node_offline_since: HashMap::new(),
        }
    }

    /// Process one tick. Returns Dog IDs that should be preemptively marked dead.
    pub fn tick(&mut self, offline_nodes: &[String]) -> Vec<String> {
        let mut to_mark = Vec::new();

        // Clear nodes that came back online
        self.node_offline_since.retain(|node, _| offline_nodes.contains(node));

        // Process offline nodes
        for node in offline_nodes {
            let entry = self.node_offline_since.entry(node.clone()).or_insert_with(Instant::now);
            if entry.elapsed() >= HYSTERESIS_THRESHOLD {
                // Find all Dogs mapped to this node
                for (dog_id, mapped_node) in &self.dog_to_node {
                    if mapped_node == node {
                        to_mark.push(dog_id.clone());
                    }
                }
            }
        }

        to_mark
    }
}
```

Run: `cargo test -p cynic-kernel health_loop::hysteresis_tests --lib`
Expected: PASS

- [ ] **Step 3: Wire FleetAwareness into spawn_health_loop**

Modify `spawn_health_loop` signature to accept:
```rust
senses: Vec<Arc<dyn OrganPort>>,
dog_to_fleet_node: HashMap<String, String>,
```

Inside the loop, after Dog probes, add:
```rust
// Fleet awareness: preemptive circuit breaking via Tailscale sense
if let Some(ts_sense) = senses.iter().find(|s| s.name() == "tailscale") {
    match tokio::time::timeout(Duration::from_secs(5), ts_sense.snapshot()).await {
        Ok(Ok(snap)) => {
            let offline_nodes: Vec<String> = snap.metrics.iter()
                .filter(|m| m.key.starts_with("node_") && m.key.ends_with("_online"))
                .filter(|m| matches!(&m.value, crate::domain::organ::MetricValue::Bool(false)))
                .filter_map(|m| {
                    m.key.strip_prefix("node_")
                        .and_then(|s| s.strip_suffix("_online"))
                        .map(|s| s.to_string())
                })
                .collect();

            let dogs_to_mark = fleet_awareness.tick(&offline_nodes);
            for dog_id in &dogs_to_mark {
                if let Some(idx) = configs.iter().position(|c| c.dog_id == *dog_id) {
                    breakers[idx].record_failure(
                        crate::domain::health_gate::FailureReason::FleetOffline
                    );
                    klog!("[health_loop] Dog '{}' preemptively marked — fleet node offline >30s", dog_id);
                }
            }
        }
        Ok(Err(e)) => tracing::debug!(error = %e, "tailscale snapshot failed — skipping fleet check"),
        Err(_) => tracing::debug!("tailscale snapshot timed out — skipping fleet check"),
    }
}
```

Note: `FailureReason::FleetOffline` must be added to `domain/health_gate.rs`.

- [ ] **Step 4: Add `FleetOffline` to FailureReason enum**

In `cynic-kernel/src/domain/health_gate.rs`, add variant to `FailureReason` enum (line 14):
```rust
/// Fleet node offline — preemptive marking via Tailscale sense.
FleetOffline,
```

And add the `as_str()` match arm (line 33, inside `impl FailureReason`):
```rust
Self::FleetOffline => "fleet_offline",
```

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/infra/health_loop.rs cynic-kernel/src/domain/health_gate.rs
git commit -m "feat(health_loop): preemptive Dog marking via Tailscale fleet awareness (30s hysteresis)"
```

---

### Task 6: Fleet drift + organ death alerts

**Files:**
- Modify: `cynic-kernel/src/introspection.rs`

- [ ] **Step 1: Write failing test for fleet drift alert**

```rust
#[test]
fn fleet_drift_gpu_offline_triggers_alert() {
    let snaps = vec![("tailscale".into(), make_tailscale_snapshot(false))];
    let alerts = check_sense_health(&snaps);
    assert!(alerts.iter().any(|a| a.kind == "fleet_drift"));
}

fn make_tailscale_snapshot(gpu_online: bool) -> SenseSnapshot {
    use crate::domain::organ::{Metric, MetricKind, MetricValue, OrganSnapshot as OSnap};
    OSnap {
        taken_at: chrono::Utc::now(),
        metrics: vec![
            Metric { key: "node_cynic-gpu_online".into(), value: MetricValue::Bool(gpu_online), kind: MetricKind::Gauge, unit: None },
            Metric { key: "nodes_online".into(), value: MetricValue::I64(if gpu_online { 2 } else { 1 }), kind: MetricKind::Gauge, unit: None },
        ],
    }
}
```

Run: `cargo test -p cynic-kernel introspection::tests::fleet_drift --lib`
Expected: FAIL

- [ ] **Step 2: Add fleet_drift detection to `check_sense_health`**

In `check_sense_health`, add:
```rust
("tailscale", key) if key.starts_with("node_") && key.ends_with("_online") => {
    if let MetricValue::Bool(false) = &metric.value {
        let node = key.strip_prefix("node_").unwrap()
            .strip_suffix("_online").unwrap();
        alerts.push(Alert {
            kind: "fleet_drift",
            message: format!("Fleet node '{node}' offline"),
            severity: "warning",
        });
    }
}
```

Run: `cargo test -p cynic-kernel introspection::tests::fleet_drift --lib`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/introspection.rs
git commit -m "feat(introspection): fleet drift alerts for offline nodes — K15 consumer for Tailscale"
```

---

### Task 7: Wire everything in main.rs + integration test

**Files:**
- Modify: `cynic-kernel/src/main.rs`
- Create: `cynic-kernel/tests/integration_tailscale_reader.rs`

- [ ] **Step 1: Pass senses + dog_to_fleet_node to health_loop in main.rs**

In `main.rs` where `spawn_health_loop` is called, add:
```rust
let senses_for_health = state.senses.clone();
let dog_to_fleet_node = dogs_and_organ.dog_to_fleet_node.clone();
```

Pass these to `spawn_health_loop(configs, breakers, task_health, shutdown, senses_for_health, dog_to_fleet_node)`.

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 2: Write integration test**

Create `cynic-kernel/tests/integration_tailscale_reader.rs`:
```rust
//! Integration test — requires `tailscale` binary on PATH.
//! Run with: cargo test -p cynic-kernel --test integration_tailscale_reader -- --ignored

use cynic_kernel::domain::organ::{MetricValue, OrganHealth, OrganPort};
use cynic_kernel::senses::tailscale::TailscaleReader;

#[tokio::test]
#[ignore] // Requires real tailscale daemon
async fn health_alive_on_real_tailscale() {
    let reader = TailscaleReader::new(vec![]);
    let health = reader.health().await;
    assert!(
        matches!(health, OrganHealth::Alive),
        "expected Alive, got {health:?}"
    );
}

#[tokio::test]
#[ignore]
async fn snapshot_has_nodes_online() {
    let reader = TailscaleReader::new(vec![]);
    let snap = reader.snapshot().await.expect("snapshot should succeed");
    let online = snap.metrics.iter().find(|m| m.key == "nodes_online");
    assert!(online.is_some(), "missing nodes_online metric");
    match &online.unwrap().value {
        MetricValue::I64(v) => assert!(*v >= 0, "nodes_online should be >= 0"),
        other => panic!("expected I64, got {other:?}"),
    }
}

#[tokio::test]
#[ignore]
async fn freshness_is_zero() {
    let reader = TailscaleReader::new(vec![]);
    let freshness = reader.freshness().await.expect("freshness should succeed");
    assert_eq!(freshness.as_secs(), 0);
}
```

Run: `cargo test -p cynic-kernel --test integration_tailscale_reader -- --ignored`
Expected: 3 tests pass (on cynic-core where tailscale is running)

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/main.rs cynic-kernel/tests/integration_tailscale_reader.rs
git commit -m "feat: wire senses into health_loop + tailscale integration tests"
```

---

### Task 8: Full gate check

- [ ] **Step 1: Run full gate**

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --tests
```

Fix any issues.

- [ ] **Step 3: Run make check**

```bash
make check
```

Expected: all gates pass

- [ ] **Step 4: Final commit with any fixes**

```bash
git add -A
git commit -m "chore: gate fixes + backends.toml fleet_node mapping"
```

- [ ] **Step 5: Create branch + PR**

```bash
git checkout -b feat/organism-senses-k15-2026-04-27
git push -u origin feat/organism-senses-k15-2026-04-27
gh pr create --base main --title "feat(senses): K15 consumers for RTK + TailscaleReader OrganPort" --body "..."
```
