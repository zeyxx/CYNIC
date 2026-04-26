# Chronic Failure Gate — Organism Self-Healing for Slow Dogs

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the organism automatically degrade Dogs with chronically low success rates (any failure type), stopping the Gemma E4B pattern where 84% failure rate never triggers action because timeouts are excluded from gates.

**Architecture:** One-line fix in `promote_if_gate_clear` — add json_valid_rate check before promoting.

**Root cause (confirmed via logs — 954 Gemma promotions observed):**
1. `sync_parse_gate_health` degrades Gemma via `json_valid_rate < 0.5` (K14 gate 2)
2. Every 30s, FleetProbe ticks → calls `promote_if_gate_clear` for all non-mismatched Dogs
3. `promote_if_gate_clear` only checks `!guard.gate.is_tripped()` (ParseFailureGate)
4. ParseFailureGate is clean because timeouts/api_errors are excluded from it
5. Gemma promoted back to Healthy — overriding the json_valid_rate degradation
6. Next judgment: Gemma participates, times out, sync_parse_gate_health degrades again → infinite oscillation

**Fix:** `promote_if_gate_clear` must also check `json_valid_rate >= 0.5` after baseline before promoting.

**Tech Stack:** Rust, existing organ module, no new dependencies.

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `cynic-kernel/src/organ/mod.rs` | Modify | Fix `promote_if_gate_clear` to check json_valid_rate; add introspection alert for chronic failure |
| `cynic-kernel/src/organ/mod.rs` (tests) | Modify | Add test: promote blocked when json_valid_rate < 0.5 |
| `cynic-kernel/src/introspection.rs` | Modify | Add per-Dog chronic failure alert (optional — existing `dog_failure_rate` is aggregate) |

---

### Task 1: Write failing test — promote_if_gate_clear must respect json_valid_rate

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs` (test section, after line ~755)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn promote_if_gate_clear_blocked_by_json_valid_rate() {
    // A Dog degraded by FleetProbe with chronic low json_valid_rate
    // must NOT be promoted when parse gate is clear.
    // This is the Gemma E4B bug: FleetProbe promotes every 4s because
    // parse gate doesn't track timeouts, but json_valid_rate is 16%.
    let mut organ = InferenceOrgan::boot_empty();
    let _handle = organ.register_backend(make_backend("slow-dog"));

    // Accumulate 25 calls: 5 successes + 20 timeouts → 20% json_valid_rate
    // Timeouts don't feed ParseFailureGate, so gate stays clean.
    let handle = organ.entries.get(&BackendId("slow-dog".to_string())).unwrap();
    {
        let mut guard = handle.0.lock().unwrap();
        guard.stats = DogStats {
            total_calls: 25,
            success_count: 5,
            zero_flood_count: 0,
            collapse_count: 0,
            parse_error_count: 0,
            timeout_count: 20,
            api_error_count: 0,
            last_success: Some("2026-04-26T14:00:00Z".to_string()),
            total_latency_ms: 300000,
            total_completion_tokens: 0,
            max_completion_tokens: 0,
            max_content_tokens: 0,
            max_thinking_tokens: 0,
        };
        // Simulate FleetProbe degradation (different reason string)
        guard.backend.health = BackendHealth::Degraded {
            reason: "fleet probe signal".into(),
            since: Instant::now() - Duration::from_secs(31),
        };
    }

    // Parse gate is NOT tripped (no parse failures fed to it)
    assert!(!handle.0.lock().unwrap().gate.is_tripped());

    // promote_if_gate_clear should NOT promote because json_valid_rate is 20% < 50%
    organ.promote_if_gate_clear("slow-dog");

    let guard = handle.0.lock().unwrap();
    assert!(
        matches!(guard.backend.health, BackendHealth::Degraded { .. }),
        "promote must be blocked when json_valid_rate < 0.5 — was: {:?}",
        guard.backend.health
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cynic-kernel promote_if_gate_clear_blocked -- --nocapture`
Expected: FAIL — promote_if_gate_clear currently promotes because it only checks `!guard.gate.is_tripped()`.

---

### Task 2: Fix promote_if_gate_clear to check json_valid_rate

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs:296-308` — `promote_if_gate_clear` method

- [ ] **Step 3: Write minimal fix**

Replace the `promote_if_gate_clear` method body:

```rust
pub fn promote_if_gate_clear(&self, dog_id: &str) {
    let key = BackendId(dog_id.to_string());
    if let Some(handle) = self.entries.get(&key)
        && let Ok(mut guard) = handle.0.lock()
        && matches!(guard.backend.health, BackendHealth::Degraded { .. })
        && !guard.gate.is_tripped()
    {
        // K14 gate 2: also verify json_valid_rate is acceptable.
        // Without this, FleetProbe promotes Dogs with chronic timeout/api_error
        // patterns because timeouts don't feed the ParseFailureGate.
        // Gemma E4B: 16% success rate, parse gate clean, promoted every 4s.
        if guard.stats.is_baseline_established() && guard.stats.json_valid_rate() < 0.5 {
            tracing::debug!(
                backend = %dog_id,
                json_valid_rate = format!("{:.1}%", guard.stats.json_valid_rate() * 100.0),
                "promote blocked — json_valid_rate below 50% threshold"
            );
            return;
        }
        tracing::info!(
            backend = %dog_id,
            "organ: fleet signal clear + gate clear — promoting to Healthy"
        );
        guard.backend.health = BackendHealth::Healthy;
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cynic-kernel promote_if_gate_clear_blocked -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p cynic-kernel`
Expected: all existing tests pass (no regressions — promote_if_gate_clear is only called by FleetProbe, and existing tests don't set up json_valid_rate < 0.5 before calling it).

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/organ/mod.rs
git commit -m "fix(organ): block promote when json_valid_rate < 50%

FleetProbe promoted Dogs every 4s even with 16% success rate because
ParseFailureGate doesn't track timeouts. promote_if_gate_clear now
also checks json_valid_rate after baseline (K14 gate 2 alignment).

Root cause: Gemma E4B CPU Dog — 84% timeout rate, parse gate clean,
promoted to Healthy by FleetProbe, re-degraded by sync_parse_gate_health,
promoted again 4s later — infinite oscillation invisible in /health."
```

---

### Task 3: Write test — Dog with chronic timeouts eventually stays degraded

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs` (test section)

- [ ] **Step 7: Write integration-style test for the full cycle**

```rust
#[test]
fn chronic_timeout_dog_stays_degraded() {
    // Full cycle: Dog starts Healthy, accumulates timeouts, gets degraded
    // by json_valid_rate gate, FleetProbe tries to promote, promotion blocked.
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("chronic-timeout-dog"));

    // 5 successes + 20 timeouts (timeouts are ScoreFailureKind::Timeout)
    for _ in 0..5 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 50000,
                completion_tokens: 200,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..20 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::Timeout),
        );
    }

    // json_valid_rate = 5/25 = 20% → K14 gate 2 fires → Degraded
    assert!(
        handle.is_quality_degraded(),
        "Dog must be degraded after 20% success rate with baseline"
    );

    // FleetProbe would call promote_if_gate_clear — must NOT promote
    organ.promote_if_gate_clear("chronic-timeout-dog");
    assert!(
        handle.is_quality_degraded(),
        "FleetProbe must not undo json_valid_rate degradation"
    );
}
```

- [ ] **Step 8: Run test**

Run: `cargo test -p cynic-kernel chronic_timeout_dog_stays -- --nocapture`
Expected: PASS (this should pass with the fix from Task 2).

- [ ] **Step 9: Commit**

```bash
git add cynic-kernel/src/organ/mod.rs
git commit -m "test(organ): chronic timeout Dog stays degraded after promote attempt"
```

---

### Task 4: Verify with clippy and full build

- [ ] **Step 10: Pre-commit validation**

```bash
export RUST_MIN_STACK=67108864
export RUSTFLAGS="-C debuginfo=1"
cargo build --tests
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: 0 errors, 0 warnings.

---

### Task 5: Verify fix against live Gemma behavior (manual, post-deploy)

This task is for after the next deploy. Not automated.

- [ ] **Step 11: After deploy, check /health after 5 minutes**

```bash
curl -s ${CYNIC_REST_ADDR}/health -H "Authorization: Bearer ${CYNIC_API_KEY}" | \
  python3 -c "import sys,json; d=json.load(sys.stdin); [print(f'{dog[\"dog\"]}: circuit={dog.get(\"circuit\",\"?\")}') for dog in d.get('dogs',[])]"
```

Expected: gemma-4-e4b-core shows quality degraded (not promoted by FleetProbe).

**Falsification:** If Gemma keeps oscillating between Healthy and Degraded in the state_log after the fix, the promote_if_gate_clear path is not the only promotion source. Grep for "promoting to Healthy" in kernel logs to find the other caller.
