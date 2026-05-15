# Crystal Vivant Phase 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open the crystal substrate to all mycelium hyphae — source diversity tracking, hypha observation endpoint, domain-dependent decay, and shatter mechanism.

**Architecture:** Four independent changes to the Crystal domain: (1) new fields on the Crystal struct for source tracking and shatter provenance, (2) a new `observe_crystal_hypha` storage method decoupled from verdicts, (3) domain-specific decay replacing the global constant, (4) a shatter mechanism for instant dissolution. All changes flow bottom-up: domain types → storage port → storage adapters → REST handlers → routes.

**Tech Stack:** Rust (Axum, SurrealDB, serde), existing CYNIC kernel patterns

**Spec:** `docs/superpowers/specs/2026-05-15-crystal-vivant-mycelium-transport-design.md`

---

### Task 1: Extend Crystal struct with new fields

**Files:**
- Modify: `cynic-kernel/src/domain/ccm/crystal.rs:18-65` (Crystal struct)
- Modify: `cynic-kernel/src/domain/ccm/crystal.rs:225-244` (test helper `make_crystal`)

- [ ] **Step 1: Add fields to Crystal struct**

In `cynic-kernel/src/domain/ccm/crystal.rs`, add after `bark_count` (line 64):

```rust
    // ── Mycelium transport ───────────────────────────────────
    /// Sources that have contributed observations to this crystal.
    /// Key = source identifier (e.g. "deterministic-dog", "hermes-agent")
    /// Value = observation count from that source.
    /// BTreeMap for deterministic serialization (K19).
    #[serde(default)]
    pub contributing_sources: std::collections::BTreeMap<String, u32>,
    /// Timestamp when this crystal was shattered (if applicable).
    #[serde(default)]
    pub shattered_at: Option<String>,
    /// Reason for shattering — the catastrophic event.
    #[serde(default)]
    pub shatter_reason: Option<String>,
    /// Source that triggered the shatter.
    #[serde(default)]
    pub shatter_source: Option<String>,
```

- [ ] **Step 2: Add `source_diversity()` method**

In the `impl Crystal` block (after `dominant_polarity`, line 84):

```rust
    /// Number of distinct sources that have contributed observations.
    pub fn source_diversity(&self) -> u32 {
        self.contributing_sources.len() as u32
    }
```

- [ ] **Step 3: Update `make_crystal` test helper**

In `crystal.rs` test module, update the `make_crystal` helper to include the new fields:

```rust
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
```

- [ ] **Step 4: Update `make_crystal` helper in `engine.rs` tests**

Same four fields in `cynic-kernel/src/domain/ccm/engine.rs` test helper `make_crystal` (line ~311).

- [ ] **Step 5: Write source_diversity test**

```rust
    #[test]
    fn source_diversity_counts_distinct_sources() {
        let mut c = make_crystal(0.5, 10, CrystalState::Forming);
        c.contributing_sources.insert("dog-a".into(), 5);
        c.contributing_sources.insert("dog-b".into(), 3);
        c.contributing_sources.insert("hermes".into(), 2);
        assert_eq!(c.source_diversity(), 3);
    }

    #[test]
    fn source_diversity_zero_when_empty() {
        let c = make_crystal(0.5, 10, CrystalState::Forming);
        assert_eq!(c.source_diversity(), 0);
    }
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check --workspace --all-targets`
Expected: PASS (new fields have `#[serde(default)]`, backward compatible)

- [ ] **Step 7: Run existing tests**

Run: `cargo test -p cynic-kernel -- ccm`
Expected: All existing crystal/engine tests pass (defaults fill new fields)

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/src/domain/ccm/crystal.rs cynic-kernel/src/domain/ccm/engine.rs
git commit -m "feat(ccm): add mycelium fields to Crystal — source tracking + shatter provenance"
```

---

### Task 2: Add CrystalShattered event variant

**Files:**
- Modify: `cynic-kernel/src/domain/events.rs:10-50` (KernelEvent enum)

- [ ] **Step 1: Add variant**

In `cynic-kernel/src/domain/events.rs`, add before `StorageReconnected` (line 49):

```rust
    CrystalShattered {
        crystal_id: String,
        domain: String,
        reason: String,
    },
```

- [ ] **Step 2: Update exhaustive matches on KernelEvent**

In `cynic-kernel/src/api/rest/events.rs`, update `event_type_name` (line ~73):

```rust
        KernelEvent::CrystalShattered { .. } => "crystal_shattered",
```

In `cynic-kernel/src/infra/tasks/runtime_loops.rs` (line ~67), the match uses `_ => None` as fallback — no change needed. The new variant falls through to `None` (no self-observation for shatter — the shatter handler already emits the event).

- [ ] **Step 3: Verify compilation**

Run: `cargo check --workspace --all-targets`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/domain/events.rs
git commit -m "feat(events): add CrystalShattered kernel event variant"
```

---

### Task 3: Domain-dependent decay

**Files:**
- Modify: `cynic-kernel/src/domain/ccm/engine.rs:139-154` (decay_relevance function)

- [ ] **Step 1: Write failing tests for domain-specific decay**

In `engine.rs` test module, add:

```rust
    #[test]
    fn decay_days_returns_domain_specific_values() {
        assert!((decay_days("chess") - 180.0).abs() < 0.1);
        assert!((decay_days("token") - 14.0).abs() < 0.1);
        assert!((decay_days("token-analysis") - 14.0).abs() < 0.1);
        assert!((decay_days("twitter") - 30.0).abs() < 0.1);
        assert!((decay_days("hermes") - 30.0).abs() < 0.1);
        assert!((decay_days("hermes-skill") - 90.0).abs() < 0.1);
        assert!((decay_days("unknown-domain") - 90.0).abs() < 0.1);
    }

    #[test]
    fn token_crystal_decays_faster_than_chess() {
        let now = "2026-04-15T12:00:00Z";
        let updated_30d = "2026-03-16T12:00:00Z";
        let token_rel = decay_relevance(0.6, updated_30d, now, "token");
        let chess_rel = decay_relevance(0.6, updated_30d, now, "chess");
        assert!(
            token_rel < chess_rel,
            "token ({token_rel}) should decay faster than chess ({chess_rel})"
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p cynic-kernel -- decay_days_returns`
Expected: FAIL — `decay_days` function doesn't exist yet

- [ ] **Step 3: Implement domain-specific decay**

Replace the `DECAY_DAYS` constant and update `decay_relevance` in `engine.rs`:

```rust
/// Domain-specific decay constant (days). Crypto moves faster than chess.
/// Values are conjecture (φ⁻² confidence) — derive from data in Phase 2.
pub fn decay_days(domain: &str) -> f64 {
    match domain {
        "chess" => 180.0,
        "token" | "token-analysis" => 14.0,
        "twitter" | "hermes" => 30.0,
        "hermes-skill" => 90.0,
        _ => 90.0,
    }
}

/// Compute decay relevance: `confidence * e^(-age_days / decay_days(domain))`.
/// Pure function — caller provides `now` for testability.
/// Returns 0.0 on unparseable timestamps (defensive, not silent).
pub fn decay_relevance(confidence: f64, updated_at: &str, now: &str, domain: &str) -> f64 {
    let Ok(updated) = chrono::DateTime::parse_from_rfc3339(updated_at) else {
        return 0.0;
    };
    let Ok(now_dt) = chrono::DateTime::parse_from_rfc3339(now) else {
        return 0.0;
    };
    let age_days = (now_dt - updated).num_seconds().max(0) as f64 / 86400.0;
    confidence * (-age_days / decay_days(domain)).exp()
}
```

- [ ] **Step 4: Update all callers of `decay_relevance`**

`decay_relevance` is called in `format_crystal_context` (engine.rs line ~180). The `domain` parameter is already in scope as the function argument. Update the calls:

```rust
    let ra = decay_relevance(a.confidence(), a.updated_at(), &now, domain);
    let rb = decay_relevance(b.confidence(), b.updated_at(), &now, domain);
```

Also update `create_crystal_handler` in `data.rs` (line ~159-176) — it constructs a `Crystal` struct literal that needs the 4 new fields:

```rust
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
```

- [ ] **Step 5: Update existing `decay_relevance` tests**

All existing tests in `engine.rs` that call `decay_relevance` need the new `domain` parameter. Add `"test"` as the domain (falls through to 90-day default, preserving existing behavior):

```rust
    // Example: update each existing test call
    let rel = decay_relevance(0.7, updated, now, "test");
```

- [ ] **Step 6: Run all tests**

Run: `cargo test -p cynic-kernel -- decay`
Expected: All PASS (old tests use "test" domain → 90d default, new tests validate per-domain)

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/domain/ccm/engine.rs
git commit -m "feat(ccm): domain-specific decay — token 14d, chess 180d, default 90d"
```

---

### Task 4: Storage port + adapters for hypha observe and shatter

**Files:**
- Modify: `cynic-kernel/src/domain/storage/mod.rs:86` (after observe_crystal)
- Modify: `cynic-kernel/src/domain/storage/null.rs` (add null impls)
- Modify: `cynic-kernel/src/storage/memory.rs` (add in-memory impls)
- Modify: `cynic-kernel/src/storage/reconnectable.rs` (forward methods, K17)
- Modify: `cynic-kernel/src/storage/surreal/crystals.rs` (SurrealDB impls)
- Modify: `cynic-kernel/src/storage/surreal.rs` (delegate to crystals module)
- Modify: `cynic-kernel/src/api/mcp/mod.rs` (MCP NullStorage stub)

- [ ] **Step 1: Add port trait methods**

In `cynic-kernel/src/domain/storage/mod.rs`, after the `observe_crystal` method (line ~86):

```rust
    /// Observe a crystal from a mycelium hypha (non-verdict source).
    /// Same Welford math as observe_crystal, but without verdict coupling.
    /// Rejects observation on Dissolved crystals (409 Conflict).
    async fn observe_crystal_hypha(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
        source: &str,
        sentiment: Option<&str>,
    ) -> Result<(), StorageError>;

    /// Shatter a crystal — instant transition to Dissolved state.
    /// Records reason and source for provenance. Idempotent on Dissolved.
    async fn shatter_crystal(
        &self,
        id: &str,
        reason: &str,
        source: &str,
        timestamp: &str,
    ) -> Result<(), StorageError>;
```

- [ ] **Step 2: Add NullStorage implementations**

In `cynic-kernel/src/domain/storage/null.rs`, after `observe_crystal`:

```rust
    async fn observe_crystal_hypha(
        &self, _id: &str, _content: &str, _domain: &str, _score: f64,
        _timestamp: &str, _source: &str, _sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: hypha observation not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn shatter_crystal(
        &self, _id: &str, _reason: &str, _source: &str, _timestamp: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: shatter not persisted (DEGRADED mode)".into(),
        ))
    }
```

- [ ] **Step 3: Add InMemoryStorage implementations**

In `cynic-kernel/src/storage/memory.rs`, after the `observe_crystal` method:

```rust
    async fn observe_crystal_hypha(
        &self, id: &str, content: &str, domain: &str, score: f64,
        timestamp: &str, source: &str, sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        use crate::domain::ccm::{compute_certainty, classify};

        let sanitized = sanitize_crystal_content(content);
        let mut s = self.state.lock().await;

        // Dissolved guard — cannot observe a shattered crystal
        if let Some(existing) = s.crystals.get(id) {
            if existing.state == CrystalState::Dissolved {
                return Err(StorageError::QueryFailed(
                    "crystal is dissolved — cannot observe".into(),
                ));
            }
        }

        let crystal = s.crystals.entry(id.to_string()).or_insert_with(|| Crystal {
            id: id.to_string(),
            content: sanitized.clone(),
            domain: domain.to_string(),
            confidence: 0.0,
            observations: 0,
            state: CrystalState::Forming,
            created_at: timestamp.to_string(),
            updated_at: timestamp.to_string(),
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
        });

        // Welford update
        let old_mean = crystal.confidence;
        crystal.observations += 1;
        let n = crystal.observations as f64;
        crystal.confidence = if n == 1.0 { score } else { (old_mean * (n - 1.0) + score) / n };
        let delta = score - old_mean;
        let delta2 = score - crystal.confidence;
        crystal.variance_m2 += delta * delta2;
        crystal.certainty = compute_certainty(crystal.variance_m2, crystal.observations);
        crystal.state = classify(crystal.certainty, crystal.observations);
        crystal.updated_at = timestamp.to_string();

        // Content mutation: higher score can refine content
        if score > old_mean && !content.is_empty() {
            crystal.content = sanitized;
        }

        // Source tracking
        *crystal.contributing_sources.entry(source.to_string()).or_insert(0) += 1;

        // Sentiment → polarity
        match sentiment {
            Some("positive") => crystal.wag_count += 1,
            Some("negative") => crystal.growl_count += 1,
            _ => {}
        }

        Ok(())
    }

    async fn shatter_crystal(
        &self, id: &str, reason: &str, source: &str, timestamp: &str,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        if let Some(crystal) = s.crystals.get_mut(id) {
            if crystal.state == CrystalState::Dissolved {
                return Ok(()); // idempotent
            }
            crystal.state = CrystalState::Dissolved;
            crystal.shattered_at = Some(timestamp.to_string());
            crystal.shatter_reason = Some(reason.to_string());
            crystal.shatter_source = Some(source.to_string());
            crystal.updated_at = timestamp.to_string();
            Ok(())
        } else {
            Err(StorageError::QueryFailed(format!("crystal {id} not found")))
        }
    }
```

- [ ] **Step 4: Add ReconnectableStorage forwarding (K17)**

In `cynic-kernel/src/storage/reconnectable.rs`, after the `observe_crystal` forward:

```rust
    async fn observe_crystal_hypha(
        &self, id: &str, content: &str, domain: &str, score: f64,
        timestamp: &str, source: &str, sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        self.current()
            .observe_crystal_hypha(id, content, domain, score, timestamp, source, sentiment)
            .await
    }
    async fn shatter_crystal(
        &self, id: &str, reason: &str, source: &str, timestamp: &str,
    ) -> Result<(), StorageError> {
        self.current()
            .shatter_crystal(id, reason, source, timestamp)
            .await
    }
```

- [ ] **Step 5: Add MCP DownStorage stubs (test-only)**

In `cynic-kernel/src/api/mcp/mod.rs`, in the `DownStorage` impl block (test-only struct near line ~441), add:

```rust
        async fn observe_crystal_hypha(
            &self, _: &str, _: &str, _: &str, _: f64,
            _: &str, _: &str, _: Option<&str>,
        ) -> Result<(), StorageError> { Ok(()) }
        async fn shatter_crystal(
            &self, _: &str, _: &str, _: &str, _: &str,
        ) -> Result<(), StorageError> { Ok(()) }
```

- [ ] **Step 6: Add SurrealDB `observe_crystal_hypha`**

In `cynic-kernel/src/storage/surreal/crystals.rs`, add:

```rust
pub(super) async fn observe_crystal_hypha(
    storage: &SurrealHttpStorage,
    id: &str,
    content: &str,
    domain: &str,
    score: f64,
    timestamp: &str,
    source: &str,
    sentiment: Option<&str>,
) -> Result<(), StorageError> {
    let safe_id = sanitize_id(id)?;
    let sanitized_content = crate::domain::sanitize::sanitize_crystal_content(content);
    let score = if score.is_finite() { score } else { 0.0 };

    // Sentiment → polarity field
    let polarity_increment = match sentiment {
        Some("positive") => "wag_count = (wag_count ?? 0) + 1,",
        Some("negative") => "growl_count = (growl_count ?? 0) + 1,",
        _ => "",
    };

    let sql = format!(
        "BEGIN TRANSACTION; \
         LET $cur_state = (SELECT VALUE state FROM crystal:`{id}`)[0] ?? 'none'; \
         IF $cur_state = 'dissolved' THEN \
             THROW 'crystal is dissolved — cannot observe'; \
         END; \
         LET $prev_obs = (SELECT VALUE observations FROM crystal:`{id}`)[0] ?? 0; \
         LET $prev_conf = (SELECT VALUE confidence FROM crystal:`{id}`)[0] ?? 0.0; \
         LET $prev_m2 = (SELECT VALUE variance_m2 FROM crystal:`{id}`)[0] ?? 0.0; \
         LET $new_obs = $prev_obs + 1; \
         LET $new_conf = IF $prev_obs > 0 THEN ($prev_conf * $prev_obs + {score}) / $new_obs ELSE {score} END; \
         LET $delta = {score} - $prev_conf; \
         LET $delta2 = {score} - $new_conf; \
         LET $new_m2 = IF $prev_obs > 0 THEN $prev_m2 + $delta * $delta2 ELSE 0.0 END; \
         LET $stddev = IF $new_obs > 1 THEN math::sqrt($new_m2 / ($new_obs - 1)) ELSE 0.0 END; \
         LET $ratio = $stddev / {phi_inv3}; \
         LET $concordance = 1.0 / (1.0 + $ratio * $ratio); \
         LET $volume = IF $new_obs >= {t_cryst} THEN 1.0 ELSE <float> $new_obs / {t_cryst}.0 END; \
         LET $certainty = $concordance * $volume; \
         LET $new_state = IF $new_obs >= {t_canon} AND $certainty >= {c_high} THEN 'canonical' \
             ELSE IF $new_obs >= {t_cryst} AND $certainty >= {c_high} THEN 'crystallized' \
             ELSE IF $new_obs >= {t_cryst} AND $certainty < {c_low} THEN 'decaying' \
             ELSE 'forming' END; \
         LET $prev_sources = (SELECT VALUE contributing_sources FROM crystal:`{id}`)[0] ?? {{}}; \
         LET $source_count = $prev_sources.`{source}` ?? 0; \
         UPSERT crystal:`{id}` SET \
             content = IF {score} > $prev_conf OR content IS NONE THEN '{content}' ELSE content END, \
             domain = domain ?? '{domain}', \
             observations = $new_obs, \
             confidence = $new_conf, \
             certainty = $certainty, \
             variance_m2 = $new_m2, \
             {polarity_increment} \
             state = $new_state, \
             contributing_sources = object::set($prev_sources, '{source}', $source_count + 1), \
             created_at = created_at ?? '{ts}', \
             updated_at = '{ts}'; \
         COMMIT TRANSACTION;",
        id = safe_id,
        content = escape_surreal(&sanitized_content),
        domain = escape_surreal(domain),
        source = escape_surreal(source),
        score = score,
        polarity_increment = polarity_increment,
        phi_inv3 = crate::domain::dog::PHI_INV3,
        t_canon = CANONICAL_CYCLES,
        t_cryst = MIN_CRYSTALLIZATION_CYCLES,
        c_high = PHI_INV,
        c_low = PHI_INV2,
        ts = escape_surreal(timestamp),
    );
    storage.query(&sql).await?;
    Ok(())
}
```

- [ ] **Step 7: Add SurrealDB `shatter_crystal`**

In same file:

```rust
pub(super) async fn shatter_crystal(
    storage: &SurrealHttpStorage,
    id: &str,
    reason: &str,
    source: &str,
    timestamp: &str,
) -> Result<(), StorageError> {
    let safe_id = sanitize_id(id)?;
    // Unconditional UPDATE — setting dissolved on already-dissolved is idempotent.
    // Uses query() not query_one() because UPDATE on a non-existent record returns
    // empty array which query_one interprets as an error.
    let sql = format!(
        "UPDATE crystal:`{id}` SET \
         state = 'dissolved', \
         shattered_at = shattered_at ?? '{ts}', \
         shatter_reason = shatter_reason ?? '{reason}', \
         shatter_source = shatter_source ?? '{source}', \
         updated_at = '{ts}'",
        id = safe_id,
        ts = escape_surreal(timestamp),
        reason = escape_surreal(reason),
        source = escape_surreal(source),
    );
    storage.query(&sql).await?;
    Ok(())
}
```

- [ ] **Step 8: Delegate in SurrealHttpStorage**

In `cynic-kernel/src/storage/surreal.rs`, add delegation methods in the `StoragePort` impl:

```rust
    async fn observe_crystal_hypha(
        &self, id: &str, content: &str, domain: &str, score: f64,
        timestamp: &str, source: &str, sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        crystals::observe_crystal_hypha(self, id, content, domain, score, timestamp, source, sentiment).await
    }
    async fn shatter_crystal(
        &self, id: &str, reason: &str, source: &str, timestamp: &str,
    ) -> Result<(), StorageError> {
        crystals::shatter_crystal(self, id, reason, source, timestamp).await
    }
```

- [ ] **Step 9: Update `row_to_crystal` to read new fields**

In `cynic-kernel/src/storage/surreal/crystals.rs`, update `row_to_crystal` (line ~286):

```rust
        contributing_sources: row["contributing_sources"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as u32)))
                    .collect()
            })
            .unwrap_or_default(),
        shattered_at: row["shattered_at"].as_str().map(String::from),
        shatter_reason: row["shatter_reason"].as_str().map(String::from),
        shatter_source: row["shatter_source"].as_str().map(String::from),
```

- [ ] **Step 10: Verify compilation**

Run: `cargo check --workspace --all-targets`
Expected: PASS

- [ ] **Step 11: Commit**

```bash
git add cynic-kernel/src/domain/storage/ cynic-kernel/src/storage/ cynic-kernel/src/api/mcp/mod.rs
git commit -m "feat(storage): add observe_crystal_hypha + shatter_crystal to all adapters"
```

---

### Task 5: REST handlers and routes

**Files:**
- Modify: `cynic-kernel/src/api/rest/data.rs` (add handlers + request types)
- Modify: `cynic-kernel/src/api/rest/mod.rs:118-122` (add routes)
- Modify: `cynic-kernel/src/api/rest/middleware.rs:91` (Organ allowlist)

- [ ] **Step 1: Add request types**

In `cynic-kernel/src/api/rest/data.rs`, after `CreateCrystalRequest`:

```rust
#[derive(Debug, Deserialize)]
pub struct ObserveCrystalHyphaRequest {
    pub score: f64,
    pub source: String,
    pub domain: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub sentiment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShatterCrystalRequest {
    pub reason: String,
    pub source: String,
}
```

- [ ] **Step 2: Add observe handler**

```rust
/// POST /crystal/{id}/observe — hypha observation (non-verdict source).
pub async fn observe_crystal_hypha_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ObserveCrystalHyphaRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    if req.source.trim().is_empty() || req.source.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "source must be 1-100 characters".into() }),
        ));
    }
    if req.domain.trim().is_empty() || req.domain.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "domain must be 1-100 characters".into() }),
        ));
    }
    if !req.score.is_finite() || req.score < 0.0 || req.score > 1.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "score must be between 0.0 and 1.0".into() }),
        ));
    }
    let content = req.content.as_deref().unwrap_or("");
    if content.chars().count() > 2000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "content must be <= 2000 characters".into() }),
        ));
    }
    let sentiment = req.sentiment.as_deref();
    if let Some(s) = sentiment {
        if !matches!(s, "positive" | "negative" | "neutral") {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: "sentiment must be positive, negative, or neutral".into() }),
            ));
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    match state.storage.observe_crystal_hypha(&id, content, &req.domain, req.score, &now, &req.source, sentiment).await {
        Ok(()) => Ok((StatusCode::OK, Json(serde_json::json!({
            "crystal_id": id,
            "observed": true,
            "source": req.source,
        })))),
        Err(ref e) if e.to_string().contains("dissolved") => Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse { error: "crystal is dissolved — cannot observe".into() }),
        )),
        Err(e) => {
            tracing::warn!(error = %e, "observe_crystal_hypha failed");
            Err(storage_error())
        }
    }
}
```

- [ ] **Step 3: Add shatter handler**

```rust
/// POST /crystal/{id}/shatter — instant dissolution.
/// Cortex/Internal only — Organ role checked via extensions.
pub async fn shatter_crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    role: Option<axum::Extension<super::types::Role>>,
    Json(body): Json<ShatterCrystalRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // RBAC: Cortex/Internal only
    if role.map(|r| r.0) == Some(super::types::Role::Organ) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "ORGAN role cannot shatter crystals".into() }),
        ));
    }
    if body.reason.trim().is_empty() || body.reason.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "reason must be 1-500 characters".into() }),
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();
    match state.storage.shatter_crystal(&id, &body.reason, &body.source, &now).await {
        Ok(()) => {
            if let Some(tx) = &state.event_tx {
                let _ = tx.send(crate::domain::events::KernelEvent::CrystalShattered {
                    crystal_id: id.clone(),
                    domain: String::new(), // domain not available without a read
                    reason: body.reason.clone(),
                });
            }
            Ok((StatusCode::OK, Json(serde_json::json!({
                "crystal_id": id,
                "shattered": true,
            }))))
        }
        Err(e) => {
            tracing::warn!(error = %e, "shatter_crystal failed");
            Err(storage_error(e))
        }
    }
}
```

- [ ] **Step 4: Register routes**

In `cynic-kernel/src/api/rest/mod.rs`, after the `/crystal/{id}` route (line ~122):

```rust
        .route("/crystal/{id}/observe", post(observe_crystal_hypha_handler))
        .route("/crystal/{id}/shatter", post(shatter_crystal_handler))
```

- [ ] **Step 5: Update Organ allowlist in middleware**

In `cynic-kernel/src/api/rest/middleware.rs`, update the Organ allowlist (line ~91):

```rust
            let allowed = path == "/observe"
                || path.starts_with("/coord/")
                || path == "/health"
                || path == "/events"
                || path.starts_with("/v1/")
                || (path.starts_with("/crystal/") && path.ends_with("/observe"));
```

- [ ] **Step 6: Update `crystal_to_json` to include new fields**

In `cynic-kernel/src/api/rest/data.rs`, update `crystal_to_json` function (line ~17):

```rust
fn crystal_to_json(c: &Crystal) -> serde_json::Value {
    serde_json::json!({
        "id": c.id,
        "content": c.content,
        "domain": c.domain,
        "confidence": c.confidence,
        "observations": c.observations,
        "contributing_verdicts": c.contributing_verdicts,
        "state": c.state.to_string(),
        "created_at": c.created_at,
        "updated_at": c.updated_at,
        "certainty": c.certainty,
        "variance_m2": c.variance_m2,
        "mean_quorum": c.mean_quorum,
        "howl_count": c.howl_count,
        "wag_count": c.wag_count,
        "growl_count": c.growl_count,
        "bark_count": c.bark_count,
        "contributing_sources": c.contributing_sources,
        "shattered_at": c.shattered_at,
        "shatter_reason": c.shatter_reason,
        "shatter_source": c.shatter_source,
    })
}
```

- [ ] **Step 7: Update field count test**

Update the `crystal_to_json` test assertion from 16 to 20:

```rust
            "crystal_to_json must serialize all 20 Crystal fields"
```

- [ ] **Step 8: Verify compilation**

Run: `cargo check --workspace --all-targets`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add cynic-kernel/src/api/rest/
git commit -m "feat(rest): add /crystal/{id}/observe and /crystal/{id}/shatter endpoints"
```

---

### Task 6: MCP tools for crystal observe and shatter

**Files:**
- Modify: `cynic-kernel/src/api/mcp/judge_tools.rs` (add 2 MCP tools)
- Modify: `cynic-kernel/src/api/mcp/proxy.rs` (add 2 proxy tools)

- [ ] **Step 1: Add `cynic_crystal_observe` MCP tool**

In `cynic-kernel/src/api/mcp/judge_tools.rs`, after the `cynic_crystals` tool, add:

```rust
    #[tool(
        name = "cynic_crystal_observe",
        description = "Observe a crystal from a mycelium hypha. Creates if not exists, feeds Welford accumulation. No verdict coupling."
    )]
    pub(crate) async fn cynic_crystal_observe(
        &self,
        #[tool(param)]
        #[serde(default)]
        crystal_id: Option<String>,
        #[tool(param)]
        content: String,
        #[tool(param)]
        domain: String,
        #[tool(param)]
        score: f64,
        #[tool(param)]
        source: String,
        #[tool(param)]
        #[serde(default)]
        sentiment: Option<String>,
        #[tool(param)]
        agent_id: String,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        validate_agent_id(&agent_id)?;
        let id = crystal_id.unwrap_or_else(|| {
            format!("{:x}", crate::domain::ccm::content_hash(&format!("{domain}:{content}")))
        });
        let now = chrono::Utc::now().to_rfc3339();
        let sentiment_ref = sentiment.as_deref();
        match self.storage.observe_crystal_hypha(&id, &content, &domain, score, &now, &source, sentiment_ref).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({"crystal_id": id, "observed": true}).to_string()
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
```

- [ ] **Step 2: Add `cynic_crystal_shatter` MCP tool**

```rust
    #[tool(
        name = "cynic_crystal_shatter",
        description = "Shatter a crystal — instant dissolution. Requires Cortex/Internal role."
    )]
    pub(crate) async fn cynic_crystal_shatter(
        &self,
        #[tool(param)]
        crystal_id: String,
        #[tool(param)]
        reason: String,
        #[tool(param)]
        source: String,
        #[tool(param)]
        agent_id: String,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        validate_agent_id(&agent_id)?;
        let now = chrono::Utc::now().to_rfc3339();
        match self.storage.shatter_crystal(&crystal_id, &reason, &source, &now).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({"crystal_id": crystal_id, "shattered": true}).to_string()
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
```

- [ ] **Step 3: Add corresponding proxy tools in `proxy.rs`**

Follow the same pattern as existing proxy tools (delegate to REST via HTTP call). Mirror the tool signatures from judge_tools.rs.

- [ ] **Step 4: Verify compilation**

Run: `cargo check --workspace --all-targets`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/api/mcp/
git commit -m "feat(mcp): add cynic_crystal_observe and cynic_crystal_shatter MCP tools"
```

---

### Task 7: Unit tests for new functionality

**Files:**
- Modify: `cynic-kernel/src/api/rest/data.rs` (handler tests)
- Modify: `cynic-kernel/src/storage/memory.rs` (storage tests)

- [ ] **Step 1: Write in-memory storage tests**

Add at the end of `storage/memory.rs` test module (or create one if absent):

```rust
#[cfg(test)]
mod crystal_hypha_tests {
    use super::*;

    async fn make_storage() -> InMemoryStorage {
        InMemoryStorage::new()
    }

    #[tokio::test]
    async fn hypha_observe_creates_forming_crystal() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "insight", "hermes-skill", 0.5, "2026-05-15T00:00:00Z", "hermes-agent", None).await.unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.state, CrystalState::Forming);
        assert_eq!(c.observations, 1);
        assert_eq!(c.source_diversity(), 1);
    }

    #[tokio::test]
    async fn hypha_observe_feeds_existing_crystal() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "insight", "hermes-skill", 0.5, "2026-05-15T00:00:00Z", "hermes-agent", None).await.unwrap();
        s.observe_crystal_hypha("test-1", "", "hermes-skill", 0.6, "2026-05-15T01:00:00Z", "hermes-agent", None).await.unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.observations, 2);
        assert!((c.confidence - 0.55).abs() < 0.01);
    }

    #[tokio::test]
    async fn hypha_observe_tracks_multiple_sources() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "insight", "skill", 0.5, "2026-05-15T00:00:00Z", "hermes", None).await.unwrap();
        s.observe_crystal_hypha("test-1", "", "skill", 0.6, "2026-05-15T01:00:00Z", "claude", None).await.unwrap();
        s.observe_crystal_hypha("test-1", "", "skill", 0.55, "2026-05-15T02:00:00Z", "gemini", None).await.unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.source_diversity(), 3);
        assert_eq!(c.contributing_sources["hermes"], 1);
        assert_eq!(c.contributing_sources["claude"], 1);
    }

    #[tokio::test]
    async fn hypha_observe_rejected_on_dissolved() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "insight", "skill", 0.5, "2026-05-15T00:00:00Z", "hermes", None).await.unwrap();
        s.shatter_crystal("test-1", "contract exploited", "admin", "2026-05-15T01:00:00Z").await.unwrap();
        let result = s.observe_crystal_hypha("test-1", "", "skill", 0.6, "2026-05-15T02:00:00Z", "hermes", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shatter_transitions_to_dissolved() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "insight", "skill", 0.5, "2026-05-15T00:00:00Z", "hermes", None).await.unwrap();
        s.shatter_crystal("test-1", "rugpull", "watchdog", "2026-05-15T01:00:00Z").await.unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.state, CrystalState::Dissolved);
        assert_eq!(c.shatter_reason.as_deref(), Some("rugpull"));
        assert_eq!(c.shatter_source.as_deref(), Some("watchdog"));
    }

    #[tokio::test]
    async fn shatter_is_idempotent() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "insight", "skill", 0.5, "2026-05-15T00:00:00Z", "hermes", None).await.unwrap();
        s.shatter_crystal("test-1", "reason1", "admin", "2026-05-15T01:00:00Z").await.unwrap();
        s.shatter_crystal("test-1", "reason2", "admin", "2026-05-15T02:00:00Z").await.unwrap(); // should not error
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.shatter_reason.as_deref(), Some("reason1")); // first shatter preserved
    }

    #[tokio::test]
    async fn sentiment_maps_to_polarity() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "good pattern", "skill", 0.7, "2026-05-15T00:00:00Z", "hermes", Some("positive")).await.unwrap();
        s.observe_crystal_hypha("test-1", "", "skill", 0.3, "2026-05-15T01:00:00Z", "hermes", Some("negative")).await.unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.wag_count, 1);
        assert_eq!(c.growl_count, 1);
    }

    #[tokio::test]
    async fn content_updates_on_higher_score() {
        let s = make_storage().await;
        s.observe_crystal_hypha("test-1", "first version", "skill", 0.5, "2026-05-15T00:00:00Z", "hermes", None).await.unwrap();
        s.observe_crystal_hypha("test-1", "better version", "skill", 0.8, "2026-05-15T01:00:00Z", "hermes", None).await.unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.content, "better version");
    }
}
```

- [ ] **Step 2: Run new tests**

Run: `cargo test -p cynic-kernel -- crystal_hypha`
Expected: All PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p cynic-kernel`
Expected: All existing tests still PASS

- [ ] **Step 4: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/storage/memory.rs
git commit -m "test(ccm): add crystal hypha observation and shatter unit tests"
```

---

### Task 8: Full validation gate

**Files:** None (validation only)

- [ ] **Step 1: Run `make check`**

Run: `make check`
Expected: All gates pass (build + test + clippy + lint-rules + lint-drift + audit)

- [ ] **Step 2: Verify lint-drift method count**

`make lint-drift` checks that `ReconnectableStorage` forwards all `StoragePort` methods (K17). With 2 new methods added to both, the count should match.

Expected: PASS

- [ ] **Step 3: Manual smoke test (if kernel running)**

```bash
# Create a crystal via hypha observe
curl -s -X POST "${CYNIC_REST_ADDR}/crystal/test-mycelium/observe" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"score":0.55,"source":"test-session","domain":"test","content":"mycelium test crystal"}'

# Read it back
curl -s "${CYNIC_REST_ADDR}/crystal/test-mycelium" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq .

# Shatter it
curl -s -X POST "${CYNIC_REST_ADDR}/crystal/test-mycelium/shatter" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"reason":"smoke test cleanup","source":"test-session"}'

# Verify dissolved
curl -s "${CYNIC_REST_ADDR}/crystal/test-mycelium" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq .state
# Expected: "dissolved"
```

- [ ] **Step 4: Final commit (if any fixes)**

```bash
git add -A && git commit -m "fix: address make check findings"
```

---

### Summary

| Task | What | Estimated steps |
|------|------|-----------------|
| 1 | Crystal struct fields + source_diversity | 8 |
| 2 | KernelEvent variant + exhaustive matches | 4 |
| 3 | Domain-dependent decay | 7 |
| 4 | Storage port + all adapters | 11 |
| 5 | REST handlers + routes + middleware | 9 |
| 6 | MCP tools (observe + shatter) | 5 |
| 7 | Unit tests | 5 |
| 8 | Full validation gate | 4 |
| **Total** | | **53 steps** |

**Dependencies:** Task 1 first (struct changes propagate everywhere). Tasks 2, 3 after Task 1 (both touch engine.rs test helpers). Task 4 depends on Task 1. Task 5 depends on Tasks 1, 2, 4. Task 6 depends on Task 4. Task 7 depends on Tasks 4, 5. Task 8 depends on all.
