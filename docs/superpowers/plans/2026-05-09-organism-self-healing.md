# Organism Self-Healing — Correctifs & Évolutions

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make CYNIC self-healing at the infrastructure layer — the organism manages its own backends, ports, and services instead of requiring human SSH intervention. Inspired by the shared responsibility model (IaaS→PaaS): the organism owns infra ops, the human owns intent.

**Architecture:** Five correctifs (C1-C5) fix the three active pathologies (embedding port mismatch, x-proxy port conflict, slot saturation without remediation). Three évolutions (E1-E3) elevate the organism from manual IaaS to self-managing PaaS: organ probing with remediation, slot saturation auto-restart, and embedding auto-discovery with hot-swap.

**Tech Stack:** Rust (cynic-kernel), systemd unit files, llama-server `/slots` + `/v1/models` APIs

**Shared Responsibility Matrix (target state):**

| Layer | Organism manages | Human manages |
|-------|-----------------|---------------|
| Inference backends | Health probe, circuit break, slot saturation restart, discovery | Model selection, hardware placement |
| Embedding | Auto-discovery across fleet ports, hot-swap at runtime | Model download, initial deployment |
| Organs (x-proxy, hermes) | Silence detection → restart, port conflict prevention | Intent (what to capture), curation |
| Config | Boot validation, drift detection, readiness gates | backends.toml, fleet.toml authoring |

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `infra/systemd/hermes-proxy.service` | Modify | C1: Add `Conflicts=mitmproxy.service` |
| `cynic-kernel/src/infra/health_loop.rs` | Modify | C2/E2: Add slot saturation tracking + remediation trigger |
| `cynic-kernel/src/backends/embedding.rs` | Modify | C3/E3: Fallback port discovery, probe `/v1/models` |
| `cynic-kernel/src/main.rs` | Modify:289-315 | C5/E3: Boot validation + embedding hot-swap |
| `cynic-kernel/src/backends/auto_embed.rs` | Create | E3: `AutoRecoveryEmbedding` wrapper (infra layer, not domain — K5) |
| `cynic-kernel/src/introspection.rs` | Modify:361-381 | E1: Organ silence → actionable remediation |
| `cynic-kernel/src/infra/tasks/mod.rs` | Modify:491-557 | E1/E2: Organ remediation + slot saturation extension |
| `cynic-kernel/src/domain/health_gate.rs` | Modify:14-49 | C2: Add `SlotSaturation` variant to `FailureReason` |

---

## Task 1: C1 — Port conflict prevention (systemd)

**Files:**
- Modify: `infra/systemd/hermes-proxy.service`

- [ ] **Step 1: Add Conflicts directive**

In `infra/systemd/hermes-proxy.service`, add `Conflicts=mitmproxy.service` to `[Unit]` section.
This tells systemd: these two services are mutually exclusive. Starting one stops the other.

```ini
[Unit]
Description=CYNIC Hermes proxy — passive X/Twitter capture (mitmdump)
After=cynic-kernel.service
Conflicts=mitmproxy.service
StartLimitIntervalSec=300
StartLimitBurst=5
```

- [ ] **Step 2: Verify the change is valid**

```bash
grep -c 'Conflicts=' infra/systemd/hermes-proxy.service
# Expected: 1
```

- [ ] **Step 3: Commit**

```bash
git add infra/systemd/hermes-proxy.service
git commit -m "fix(infra): add Conflicts=mitmproxy.service to hermes-proxy

Prevents port 8888 collision that caused 31h x-proxy silence.
Two mitmdump instances cannot coexist — systemd now enforces this."
```

---

## Task 2: C2 — Slot saturation remediation trigger

**Files:**
- Modify: `cynic-kernel/src/infra/health_loop.rs` (add saturation counter + remediation signal)
- Modify: `cynic-kernel/src/domain/slot_tracker.rs` (add consecutive saturation tracking)
- Test: existing tests in both files + new tests

The current remediation fires only on circuit-open (10 consecutive probe failures → open for 90s). A stuck slot keeps the Dog healthy (probes pass) but permanently saturated. The Dog is silently excluded from /judge forever — no circuit break, no remediation, no alert.

**Design:** Track consecutive ticks where `utilization == 1.0` per Dog in `SlotTracker`. After N ticks (default: 3 = 90s at 30s interval), expose this via a new method `saturated_dogs()`. The health loop calls this and emits remediation requests into the existing `spawn_remediation_watcher` path.

- [ ] **Step 1: Write failing test for saturation tracking**

In `cynic-kernel/src/domain/slot_tracker.rs`, add test:

```rust
#[test]
fn consecutive_saturation_tracked() {
    let tracker = SlotTracker::new();
    let dog = "stuck-dog";
    // 3 consecutive all-busy updates
    for _ in 0..3 {
        tracker.update(dog, BackendSlots {
            total: 1, busy: 1, per_slot_ctx: 32768,
            updated_at: Instant::now(), slots: vec![],
        });
        tracker.tick_saturation(dog);
    }
    let saturated = tracker.saturated_dogs(3);
    assert_eq!(saturated, vec!["stuck-dog"]);
}

#[test]
fn saturation_resets_on_free_slot() {
    let tracker = SlotTracker::new();
    let dog = "recover-dog";
    // 2 busy ticks
    for _ in 0..2 {
        tracker.update(dog, BackendSlots {
            total: 1, busy: 1, per_slot_ctx: 32768,
            updated_at: Instant::now(), slots: vec![],
        });
        tracker.tick_saturation(dog);
    }
    // 1 free tick
    tracker.update(dog, BackendSlots {
        total: 1, busy: 0, per_slot_ctx: 32768,
        updated_at: Instant::now(), slots: vec![],
    });
    tracker.tick_saturation(dog);
    let saturated = tracker.saturated_dogs(3);
    assert!(saturated.is_empty());
}
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
cd cynic-kernel && cargo test slot_tracker -- --nocapture 2>&1 | tail -5
# Expected: FAIL — tick_saturation and saturated_dogs don't exist
```

- [ ] **Step 3: Implement saturation tracking in SlotTracker**

Add to `SlotTracker`:

```rust
// In SlotTracker struct, add field:
saturation_ticks: RwLock<BTreeMap<String, u32>>,

// In SlotTracker::new(), initialize:
saturation_ticks: RwLock::new(BTreeMap::new()),

/// Called by health loop after each slot probe tick.
/// Increments if all_busy (fresh data), resets to 0 otherwise.
pub fn tick_saturation(&self, dog_id: &str) {
    let is_saturated = self.all_slots_busy(dog_id);
    if let Ok(mut guard) = self.saturation_ticks.write() {
        let counter = guard.entry(dog_id.to_string()).or_insert(0);
        if is_saturated {
            *counter += 1;
        } else {
            *counter = 0;
        }
    }
}

/// Returns Dog IDs that have been continuously saturated for >= threshold ticks.
pub fn saturated_dogs(&self, threshold: u32) -> Vec<String> {
    let Ok(guard) = self.saturation_ticks.read() else {
        return vec![];
    };
    guard.iter()
        .filter(|(_, count)| **count >= threshold)
        .map(|(dog_id, _)| dog_id.clone())
        .collect()
}
```

- [ ] **Step 4: Run tests, verify they pass**

```bash
cd cynic-kernel && cargo test slot_tracker -- --nocapture 2>&1 | tail -10
# Expected: all tests PASS
```

- [ ] **Step 5: Add `SlotSaturation` variant to FailureReason (BEFORE wiring into health_loop)**

In `cynic-kernel/src/domain/health_gate.rs`, add the variant and its `as_str` arm:

```rust
// In enum FailureReason (after FleetOffline):
/// Inference slot stuck at 100% for multiple consecutive probe ticks.
SlotSaturation,

// In impl FailureReason::as_str (after FleetOffline arm):
Self::SlotSaturation => "slot_saturation",
```

- [ ] **Step 6: Wire saturation check into health_loop**

In `cynic-kernel/src/infra/health_loop.rs`, INSIDE the existing slot results loop (lines 286-296), add the `tick_saturation` call per-Dog. Then AFTER the loop, check for saturated dogs.

Modify the existing loop at line 286 to also tick saturation:

```rust
// Replace the existing `for (dog_id, maybe_slots) in slot_results {` block:
for (dog_id, maybe_slots) in slot_results {
    if let Some(slots) = maybe_slots {
        tracing::debug!(
            dog_id = %dog_id,
            total = slots.total,
            busy = slots.busy,
            "slot probe update"
        );
        slot_tracker.update(&dog_id, slots);
    }
    // Soma L2+: tick saturation counter for this Dog
    slot_tracker.tick_saturation(&dog_id);
}
// After the loop: check for stuck slots
for dog_id in slot_tracker.saturated_dogs(3) {
    klog!(
        "[health_loop] Dog '{}' slots saturated for 3+ ticks — signaling remediation",
        dog_id
    );
    // Record a synthetic failure so the circuit breaker opens
    // and the existing remediation watcher can restart the backend
    if let Some(idx) = configs.iter().position(|c| c.dog_id == dog_id) {
        breakers[idx].record_failure(
            crate::domain::health_gate::FailureReason::SlotSaturation
        );
    }
}
```

- [ ] **Step 7: Run full check**

```bash
cd cynic-kernel && cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/src/domain/slot_tracker.rs cynic-kernel/src/domain/health_gate.rs cynic-kernel/src/infra/health_loop.rs
git commit -m "feat(soma): slot saturation → remediation trigger (C2/E2)

Track consecutive ticks at 100% utilization per Dog. After 3 ticks
(90s), record SlotSaturation failure → circuit opens → existing
remediation watcher restarts the backend via SSH.

Closes the gap where a stuck llama-server slot permanently excludes
a Dog from /judge without triggering any recovery."
```

---

## Task 3: C3 — Embedding fallback port discovery

**Files:**
- Modify: `cynic-kernel/src/backends/embedding.rs`
- Test: existing test + new test

The current code hardcodes port 8081. No llama-server runs there. Port 8080 has the actual inference server which supports `/v1/embeddings` if started with `--embedding`. Instead of hardcoding either port, probe known ports.

- [ ] **Step 1: Write failing test for fallback discovery**

```rust
#[test]
fn from_env_tries_fallback_ports() {
    // When CYNIC_EMBED_URL is not set and 8081 is unreachable,
    // the backend should be constructable (URL is just built, not probed at construction)
    std::env::remove_var("CYNIC_EMBED_URL");
    std::env::remove_var("CYNIC_EMBED_PORT");
    let backend = EmbeddingBackend::from_env().unwrap();
    // Should contain the primary port in base_url
    assert!(backend.base_url.contains("8081") || backend.base_url.contains("8080"));
}
```

- [ ] **Step 2: Add `base_url()` accessor + `discover` method**

In `embedding.rs`, add an accessor for `base_url` (field is private) and the discovery method:

```rust
impl EmbeddingBackend {
    /// Accessor for logging — base_url is private (encapsulation).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Probe known embedding ports and return the first reachable one.
/// Order: CYNIC_EMBED_URL (explicit) → :8081 (dedicated) → :8080 (shared with inference).
/// Called at boot to find a working embedding server.
pub async fn discover(host: &str, api_key: Option<String>, model: &str) -> Option<Self> {
    let ports = [
        std::env::var("CYNIC_EMBED_PORT").unwrap_or_else(|_| "8081".into()),
        "8080".into(),
    ];
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    for port in &ports {
        let base_url = format!("http://{host}:{port}/v1");
        let health_url = format!("http://{host}:{port}/health");
        let mut req = client.get(&health_url);
        if let Some(ref key) = api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            req.send()
        ).await {
            Ok(Ok(resp)) if resp.status().is_success() => {
                klog!("[Embedding] discovered server at {}:{}", host, port);
                return Self::new(&base_url, api_key, model).ok();
            }
            _ => {
                klog!("[Embedding] port {} unreachable on {}, trying next", port, host);
                continue;
            }
        }
    }
    None
}
```

- [ ] **Step 3: Run check**

```bash
cd cynic-kernel && cargo check --workspace --all-targets
```

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/backends/embedding.rs
git commit -m "feat(embedding): multi-port discovery (C3)

EmbeddingBackend::discover() probes :8081 then :8080 at boot.
No more hardcoded port assumption — finds the first reachable
server. Enables shared inference+embedding on same llama-server."
```

---

## Task 4: C5/E3 — Boot validation + embedding hot-swap

**Depends on:** Task 3 (uses `EmbeddingBackend::discover()` and `base_url()` accessor)

**Files:**
- Create: `cynic-kernel/src/backends/auto_embed.rs` (infra layer — K5 compliant, tokio dependency OK here)
- Modify: `cynic-kernel/src/backends/mod.rs` (add `pub mod auto_embed;`)
- Modify: `cynic-kernel/src/main.rs:289-315`

The current boot logic commits to NullEmbedding permanently if embedding is down at startup. The organism should retry periodically and hot-swap when the backend becomes available.

**K5 compliance:** `AutoRecoveryEmbedding` uses `tokio::sync::RwLock` (infra dependency). It lives in `backends/` (infra layer), not `domain/`. It implements the domain `EmbeddingPort` trait — clean hexagonal boundary.

- [ ] **Step 1: Create `AutoRecoveryEmbedding` in backends layer**

Create `cynic-kernel/src/backends/auto_embed.rs`:

```rust
//! AutoRecoveryEmbedding — infra wrapper that hot-swaps embedding backends at runtime.
//! Lives in backends/ (infra layer) because it depends on tokio::sync::RwLock (K5).

use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::domain::embedding::{Embedding, EmbeddingError, EmbeddingPort};

/// Wraps an EmbeddingPort that can be hot-swapped at runtime.
/// Starts with NullEmbedding, upgrades to real backend when discovered.
#[derive(Debug)]
pub struct AutoRecoveryEmbedding {
    inner: RwLock<Arc<dyn EmbeddingPort>>,
}

impl AutoRecoveryEmbedding {
    pub fn new(initial: Arc<dyn EmbeddingPort>) -> Self {
        Self {
            inner: RwLock::new(initial),
        }
    }

    /// Hot-swap the inner backend. Called by discovery loop.
    pub async fn upgrade(&self, backend: Arc<dyn EmbeddingPort>) {
        let mut guard = self.inner.write().await;
        *guard = backend;
    }
}

#[async_trait]
impl EmbeddingPort for AutoRecoveryEmbedding {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let guard = self.inner.read().await;
        guard.embed(text).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        let guard = self.inner.read().await;
        guard.embed_batch(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::embedding::{NullEmbedding, FixedEmbedding};

    #[tokio::test]
    async fn starts_null_then_upgrades() {
        let auto = AutoRecoveryEmbedding::new(Arc::new(NullEmbedding));
        assert!(auto.embed("test").await.is_err());

        let fixed = Arc::new(FixedEmbedding::new(vec![1.0, 0.0, 0.0]));
        auto.upgrade(fixed).await;

        let result = auto.embed("test").await.unwrap();
        assert_eq!(result.dimensions, 3);
    }

    #[tokio::test]
    async fn double_upgrade_uses_latest() {
        let auto = AutoRecoveryEmbedding::new(Arc::new(NullEmbedding));
        auto.upgrade(Arc::new(FixedEmbedding::new(vec![1.0, 0.0]))).await;
        auto.upgrade(Arc::new(FixedEmbedding::new(vec![1.0, 0.0, 0.0, 0.0]))).await;

        let result = auto.embed("test").await.unwrap();
        assert_eq!(result.dimensions, 4); // latest wins
    }
}
```

- [ ] **Step 2: Register module**

In `cynic-kernel/src/backends/mod.rs`, add:

```rust
pub mod auto_embed;
```

- [ ] **Step 3: Run tests, verify they pass**

```bash
cd cynic-kernel && cargo test auto_embed -- --nocapture
```

- [ ] **Step 4: Update main.rs boot sequence**

Replace `main.rs:289-315` with:

```rust
// ─── RING 2: Embedding backend (sovereign, auto-recovery) ────
// AutoRecoveryEmbedding: starts with whatever is available (or Null),
// background task retries discovery every 60s until a backend is found.
let initial_embed: Arc<dyn domain::embedding::EmbeddingPort> = {
    if let Ok(url) = std::env::var("CYNIC_EMBED_URL") {
        let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
        let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
        match backends::embedding::EmbeddingBackend::new(&url, api_key, &model) {
            Ok(b) if b.health().await.is_available() => {
                klog!("[Ring 2] Embedding: explicit URL {} (sovereign)", url);
                Arc::new(b)
            }
            _ => {
                klog!("[Ring 2] Embedding: explicit URL {} unavailable", url);
                Arc::new(domain::embedding::NullEmbedding)
            }
        }
    } else {
        let host = std::env::var("CYNIC_REST_ADDR")
            .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.into())
            .split(':').next().unwrap_or("127.0.0.1").to_string();
        let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
        let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
        match backends::embedding::EmbeddingBackend::discover(&host, api_key, &model).await {
            Some(b) => {
                klog!("[Ring 2] Embedding: discovered at {}", b.base_url());
                Arc::new(b)
            }
            None => {
                klog!("[Ring 2] Embedding: no server found — NullEmbedding, will auto-discover");
                Arc::new(domain::embedding::NullEmbedding)
            }
        }
    }
};
let embedding: Arc<backends::auto_embed::AutoRecoveryEmbedding> =
    Arc::new(backends::auto_embed::AutoRecoveryEmbedding::new(initial_embed));
```

Note: `embedding` type changes from `Arc<dyn EmbeddingPort>` to `Arc<AutoRecoveryEmbedding>`. Downstream consumers in `AppState` (`pub embedding: Arc<dyn EmbeddingPort>`) accept it via cast: `Arc::clone(&embedding) as Arc<dyn EmbeddingPort>`. Update the `AppState` construction at `main.rs:705`:

```rust
embedding: Arc::clone(&embedding) as Arc<dyn domain::embedding::EmbeddingPort>,
```

Same for MCP state and `spawn_background_tasks` call.

- [ ] **Step 5: Add background discovery task**

Add a new spawn in main.rs after the embedding init, before the background task block:

```rust
// Background embedding discovery — retries every 60s if currently NullEmbedding
{
    let embed_ref = Arc::clone(&embedding);
    let host = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.into())
        .split(':').next().unwrap_or("127.0.0.1").to_string();
    let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
    let shutdown = shutdown.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await; // skip first
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = interval.tick() => {
                    // Only retry if current embed is failing
                    if embed_ref.embed("health-probe").await.is_err() {
                        if let Some(backend) = backends::embedding::EmbeddingBackend::discover(
                            &host, api_key.clone(), &model
                        ).await {
                            klog!("[Ring 2] Embedding: auto-discovered backend, hot-swapping");
                            embed_ref.upgrade(Arc::new(backend)).await;
                        }
                    }
                }
            }
        }
    });
}
```

- [ ] **Step 6: Run full check + fix compiler errors**

```bash
cd cynic-kernel && cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings
```

Follow compiler errors for any remaining `Arc<dyn EmbeddingPort>` ↔ `Arc<AutoRecoveryEmbedding>` mismatches. All cast sites: `main.rs:705` (AppState), `main.rs` (MCP state init), `spawn_background_tasks` call.

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/backends/auto_embed.rs cynic-kernel/src/backends/mod.rs cynic-kernel/src/backends/embedding.rs cynic-kernel/src/main.rs
git commit -m "feat(embedding): auto-recovery + hot-swap (C5/E3)

AutoRecoveryEmbedding (backends/ layer, K5 compliant) wraps the port
with runtime hot-swap via tokio::sync::RwLock.
Boot sequence uses discover() across ports 8081→8080.
Background task retries every 60s if embedding is down.

The organism no longer permanently degrades on boot-time
embedding failure — it self-heals when the server appears."
```

---

## Task 5: C4/E1 — Organ silence → actionable remediation

**Files:**
- Modify: `cynic-kernel/src/infra/config.rs` (add `OrganRemediation` struct + TOML parsing)
- Modify: `cynic-kernel/src/infra/tasks/mod.rs` (new `spawn_organ_remediation` function)
- Modify: `cynic-kernel/src/main.rs` (wire `spawn_organ_remediation` at boot)
- Test: new unit tests in `infra/tasks/mod.rs`

Currently organ silence produces an alert in `/health` but triggers no action. The organism detects the problem and does nothing — pure observation, K15 violation.

**Design:** A separate `spawn_organ_remediation` loop (not merged into Dog remediation). It reads organ silence from `last_observation_per_source()` (already exists in `StoragePort`), compares against threshold, and calls `ssh_restart()`. Own `RecoveryTracker` instance for organ cooldown/retries.

- [ ] **Step 1: Define organ remediation config**

In `cynic-kernel/src/infra/config.rs`, add:

```rust
/// Remediation config for organs (services outside the Dog pipeline).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OrganRemediation {
    pub source: String,           // organ name (e.g. "x-proxy")
    pub restart_command: String,  // e.g. "systemctl --user restart hermes-proxy"
    pub node: String,             // "localhost" or Tailscale hostname
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold_secs: u64,
    #[serde(default = "default_organ_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_organ_cooldown")]
    pub cooldown_secs: u64,
}

fn default_silence_threshold() -> u64 { 3600 }
fn default_organ_max_retries() -> u32 { 3 }
fn default_organ_cooldown() -> u64 { 300 }
```

Also add parsing in the TOML loader — look for `[[organs]]` sections:

```rust
// In the BackendsConfig (or equivalent top-level TOML struct):
#[serde(default)]
pub organs: Vec<OrganRemediation>,
```

- [ ] **Step 2: Add organ remediation config to backends.toml**

```toml
[[organs]]
source = "x-proxy"
node = "localhost"
restart_command = "systemctl --user restart hermes-proxy && systemctl --user restart hermes-x-ingest"
silence_threshold_secs = 3600
max_retries = 3
cooldown_secs = 300
```

- [ ] **Step 3: Write failing test for organ remediation logic**

In `infra/tasks/mod.rs`, add test:

```rust
#[cfg(test)]
mod organ_remediation_tests {
    use super::*;

    #[test]
    fn organ_silence_above_threshold_is_remediable() {
        // Organ silent for 4000s, threshold 3600 → should remediate
        let silence_secs = 4000u64;
        let threshold = 3600u64;
        assert!(silence_secs > threshold);
    }

    #[test]
    fn organ_silence_below_threshold_is_not_remediable() {
        let silence_secs = 1800u64;
        let threshold = 3600u64;
        assert!(silence_secs <= threshold);
    }
}
```

- [ ] **Step 4: Implement `spawn_organ_remediation`**

New function in `infra/tasks/mod.rs`:

```rust
/// Organ remediation loop — restarts silent organs via ssh_restart().
/// Independent from Dog remediation (different trigger: silence, not circuit open).
pub fn spawn_organ_remediation(
    organ_configs: Vec<crate::infra::config::OrganRemediation>,
    storage: Arc<dyn crate::domain::storage::StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let organ_count = organ_configs.len();
    let handle = tokio::spawn(async move {
        let tracker = crate::infra::remediation::RecoveryTracker::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Organ remediation stopped");
                    break;
                }
                _ = interval.tick() => {
                    // Read organ silence from storage via last_observation_per_source()
                    // Returns Vec<(source, last_at, total)>
                    let sources = match storage.last_observation_per_source().await {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::debug!(error = %e, "organ remediation: storage query failed");
                            continue;
                        }
                    };
                    let now = chrono::Utc::now();

                    for organ_config in &organ_configs {
                        // Find this organ's last observation
                        let silence_secs = sources.iter()
                            .find(|(source, _, _)| source == &organ_config.source)
                            .and_then(|(_, last_at, _)| {
                                chrono::DateTime::parse_from_rfc3339(last_at).ok()
                            })
                            .map(|t| (now - t.with_timezone(&chrono::Utc)).num_seconds().max(0) as u64)
                            .unwrap_or(u64::MAX); // K14: unknown = assume degraded

                        if silence_secs > organ_config.silence_threshold_secs {
                            // Build a temporary BackendRemediation for the shared tracker
                            let compat = crate::infra::config::BackendRemediation {
                                node: organ_config.node.clone(),
                                restart_command: organ_config.restart_command.clone(),
                                max_retries: organ_config.max_retries,
                                cooldown_secs: organ_config.cooldown_secs,
                            };
                            if tracker.should_restart(&organ_config.source, &compat) {
                                klog!(
                                    "[Remediation] Organ '{}' silent for {}s (threshold {}s), attempting restart",
                                    organ_config.source, silence_secs, organ_config.silence_threshold_secs
                                );
                                let node = organ_config.node.clone();
                                let cmd = organ_config.restart_command.clone();
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(15),
                                    tokio::task::spawn_blocking(move || {
                                        crate::infra::remediation::ssh_restart(&node, &cmd)
                                    }),
                                ).await {
                                    Ok(Ok(Ok(output))) => {
                                        klog!("[Remediation] Organ '{}' restart OK: {}", organ_config.source, output.trim());
                                    }
                                    Ok(Ok(Err(e))) => {
                                        klog!("[Remediation] Organ '{}' restart failed: {}", organ_config.source, e);
                                    }
                                    _ => {
                                        klog!("[Remediation] Organ '{}' restart timed out", organ_config.source);
                                    }
                                }
                                tracker.record_attempt(&organ_config.source, organ_config.max_retries);
                            }
                        }
                    }
                }
            }
        }
    });
    klog!(
        "[Ring 2] Organ remediation started ({} organs configured)",
        organ_count
    );
    handle
}
```

**Key design decisions:**
- Uses `last_observation_per_source()` (already in `StoragePort` — no new method needed)
- Builds a temporary `BackendRemediation` from `OrganRemediation` fields to reuse `RecoveryTracker::should_restart()` (avoids type mismatch)
- Own `RecoveryTracker` instance (not shared with Dog remediation)
- `u64::MAX` for unknown organs (K14: assume degraded)

- [ ] **Step 5: Wire into main.rs**

In `main.rs`, after `spawn_remediation_watcher`, add:

```rust
// Organ remediation (separate from Dog remediation)
if !organ_configs.is_empty() {
    let _organ_remediation = spawn_organ_remediation(
        organ_configs,
        Arc::clone(&storage),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
}
```

Where `organ_configs` is parsed from the same `backends.toml` file as Dog configs.

- [ ] **Step 6: Run check**

```bash
cd cynic-kernel && cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 7: Run tests**

```bash
cd cynic-kernel && cargo test organ_remediation -- --nocapture
cd cynic-kernel && cargo test -- --nocapture 2>&1 | tail -20
```

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/src/infra/config.rs cynic-kernel/src/infra/tasks/mod.rs cynic-kernel/src/main.rs
git commit -m "feat(soma): organ silence → auto-remediation (C4/E1)

New spawn_organ_remediation loop: checks organ silence via
last_observation_per_source(), restarts via ssh_restart() when
silent beyond threshold. Reuses RecoveryTracker for cooldown/retries.

backends.toml [[organs]] section configures which organs get
auto-restarted. Closes K15 violation: organ_silence was detected
but never acted upon."
```

---

## Task 6: Final validation + deploy

- [ ] **Step 1: Run full gate**

```bash
cd cynic-kernel && make check
```

- [ ] **Step 2: Manual ops — clear active pathologies**

These are one-time ops the human runs to unblock before the new self-healing code is deployed:

```bash
# 1. Kill the phantom mitmproxy holding port 8888
systemctl --user stop mitmproxy.service
systemctl --user disable mitmproxy.service

# 2. Restart hermes-proxy (now with Conflicts= guard)
systemctl --user reset-failed hermes-proxy.service
systemctl --user start hermes-proxy.service

# 3. Restart llama-server on core to free the stuck slot
# (SSH to cynic-core or local if on core)
systemctl --user restart llama-server

# 4. Verify embedding flag
# Check if llama-server was started with --embedding
ps aux | grep llama-server | grep -o '\-\-embedding'
# If not present, add --embedding to llama-server.service ExecStart
```

- [ ] **Step 3: Deploy kernel (mv+cp pattern — ETXTBSY safe)**

```bash
cd cynic-kernel && cargo build --release
mv ~/bin/cynic-kernel ~/bin/cynic-kernel.old 2>/dev/null || true
cp target/release/cynic-kernel ~/bin/cynic-kernel
systemctl --user restart cynic-kernel
```

Note: `mv` then `cp` — never `cp` over a running binary. MCP holds the inode open; `cp` causes ETXTBSY.

- [ ] **Step 4: Verify organism health**

```bash
# Wait 60s for health loop + discovery to run
sleep 60
source ~/.cynic-env
curl -s "${CYNIC_REST_ADDR}/health" -H "Authorization: Bearer ${CYNIC_API_KEY}" | python3 -c "
import sys, json
h = json.load(sys.stdin)
print('Status:', h['status'])
print('Embedding:', h['embedding'])
print('Alerts:', len(h.get('alerts', [])))
for a in h.get('alerts', []):
    print(f'  [{a[\"severity\"]}] {a[\"message\"][:80]}')
print('Slots:')
for s in h.get('slot_utilization', []):
    print(f'  {s[\"dog_id\"]}: {s[\"busy\"]}/{s[\"total\"]} ({s[\"utilization\"]:.0%})')
"
```

Expected:
- `Status: ok` (or `degraded` only for non-critical reasons)
- `Embedding: available` (not `unavailable`)
- No `organ_silence` alert for `x-proxy`
- `qwen25-7b-core: 0/1 (0%)`

- [ ] **Step 5: Commit any remaining changes + create PR**

```bash
git add -A
git commit -m "chore: deploy organism self-healing v1"
gh pr create --base main --title "feat: organism self-healing — C1-C5 + E1-E3" --body "..."
```

---

## Évolution Summary (what this achieves)

| ID | Before | After |
|----|--------|-------|
| **E1** | Organ silence = alert only | Organ silence = auto-restart via ssh_restart() |
| **E2** | Stuck slot = invisible exclusion | Stuck slot → circuit open → backend restart |
| **E3** | Embedding = hardcoded port, permanent NullEmbedding | Auto-discovery + hot-swap at runtime |

**Shared responsibility shift:** The human no longer needs to SSH to restart services, fix port conflicts, or notice stuck slots. The organism handles IaaS-layer remediation autonomously. The human focuses on intent: which models to run, what to capture, which domains to judge.

**Falsification:**
- E1: Stop hermes-proxy. Within `silence_threshold_secs`, the organism should restart it. If it doesn't → E1 is broken.
- E2: Kill a llama-server slot artificially (send a request and drop TCP). Within 3 ticks (90s), the Dog should get a SlotSaturation failure, circuit opens, remediation restarts the server. If the Dog stays excluded > 5 min → E2 is broken.
- E3: Start kernel with no embedding server. Start llama-server with `--embedding` 2 minutes later. Within 60s of server startup, embedding should hot-swap from Null to real. If `/health` still shows `unavailable` after 3 min → E3 is broken.
