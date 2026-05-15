# Crystal Vivant — Mycelium Universal Transport

> Crystals evolve from static statistical accumulators into living knowledge units that mutate, shatter, and recycle — becoming the universal transport layer of the CYNIC mycelium.

**Date:** 2026-05-15
**Epistemic status:** DESIGN (grounded in 8-domain crystallization study + observed codebase state)
**Falsification:** If after 30 days no non-verdict crystal reaches Crystallized state, the model is too strict for mycelium transport and needs revision.

---

## Problem

CYNIC crystals today are **verdict accumulators**. The only path to crystallization is through the Dog judgment pipeline: verdict → `observe_crystal` → Welford accumulation → state transition. This serves one hypha (Dogs) but blocks the mycelium.

The organism has multiple hyphae (Hermes organ, Claude Code sessions, Gemini CLI, NousResearch Hermes Agent, nightshift, systemd services) that each learn things. None of them can feed their learnings into the crystal substrate. Knowledge stays trapped in the hypha that discovered it.

**K15 violation:** The crystal system has one producer (verdicts) and one consumer (Dog prompts). A universal transport needs N producers and N consumers.

### Current state (observed)

- `observe_crystal()` requires `verdict_id`, `verdict_kind`, `voter_count` — coupled to judgment pipeline
- `POST /crystal` creates orphan Forming crystals that never grow (no observation path)
- `is_crystallizable_domain()` is a hardcoded whitelist — new hyphae can't crystallize without code changes
- `DECAY_DAYS = 90` is global — same rate for chess positions and crypto tokens
- No mechanism for instant invalidation (shatter)
- No mechanism for content evolution (mutation/compression)
- No tracking of source diversity (which hyphae contributed)
- Crystal `content` is fixed at creation — never abstracts or compresses

### Study foundation

Crystallization patterns studied across 8 domains (physical crystals, mycelium, human brain, scientific method, cultural transmission, software patterns, memes, trading) reveal 9 mechanisms. CYNIC implements 1 (concordance). The 3 highest-impact gaps:

1. **Mutation/compression** — content should abstract as the crystal matures
2. **Shatter** — catastrophic events should instantly dissolve a crystal
3. **Source diversity** — independent hyphae confirming > repeated observations from same source

---

## Design

### Principle: The substrate doesn't dictate what grows on it

Any process that speaks HTTP and has a Bearer token can be a hypha. The crystal API must be hypha-agnostic. A NousResearch Hermes Agent skill, a Claude Code session insight, a Gemini dialectic conclusion, and a Dog verdict all flow through the same crystal observation path.

### 1. Source diversity tracking

**New field on Crystal:**

```rust
/// Sources that have contributed observations to this crystal.
/// Key = source identifier (e.g. "deterministic-dog", "hermes-agent", "claude-session-abc")
/// Value = observation count from that source.
/// BTreeMap for deterministic serialization (K19).
#[serde(default)]
pub contributing_sources: BTreeMap<String, u32>,
```

**Source diversity score:**

```rust
/// Number of distinct sources that have contributed.
/// Used in crystallization gate alongside volume.
pub fn source_diversity(&self) -> u32 {
    self.contributing_sources.len() as u32
}
```

**Impact on crystallization:** Source diversity is tracked and exposed but does NOT change the crystallization gate in Phase 1. The existing Welford certainty + 21-observation threshold remains. Phase 2 can weight diversity into the gate after measuring whether diversity correlates with crystal quality.

**Why not gate on diversity now:** We don't have data. Gating on something unmeasured is the anti-pattern CYNIC already learned with domain whitelist hardcoding. Track first, gate second.

### 2. Hypha observation endpoint

**New REST endpoint: `POST /crystal/{id}/observe`**

```json
{
  "score": 0.55,
  "source": "hermes-agent",
  "domain": "hermes-skill",
  "content": "Token with holder concentration > 40% signals poor distribution"
}
```

**Behavior:**
- If crystal exists: Welford update (same math as verdict path). Source recorded in `contributing_sources`.
- If crystal doesn't exist: create new crystal in Forming state, then apply first observation.
- `content` is optional: if provided AND score > running mean, content is updated (same logic as verdict path line 165 of `crystals.rs`). This is the **mutation** seed — higher-scoring observations can refine the content.
- No `verdict_id` or `verdict_kind` required. These fields are verdict-specific provenance.
- No quorum requirement. Quorum is a multi-Dog concept. A single hypha's observation is valid on its own (the diversity gate handles multi-source validation).
- Domain whitelist (`is_crystallizable_domain`) does NOT apply. That gate is for the verdict pipeline (prevents internal noise from crystallizing). Hypha observations are explicit and intentional — the hypha chose to observe.

**Polarity tracking for non-verdict sources:**
- Non-verdict observations don't have HOWL/WAG/GROWL/BARK polarity.
- Add a `sentiment` optional field: `"positive"`, `"negative"`, `"neutral"`. Maps to polarity counters:
  - `positive` → `wag_count += 1`
  - `negative` → `growl_count += 1`
  - `neutral` or omitted → no polarity update
- This preserves the polarity dimension without forcing the verdict vocabulary onto non-verdict hyphae.

**Auth:** Same Bearer token as all other endpoints. RBAC: Organ role can observe (like `/observe`). Cortex/Internal can observe.

**Rate limiting:** Same global limiter. No special judge limiter (observation is cheap, no inference cost).

### 3. MCP tool: `cynic_crystal_observe`

Mirror of the REST endpoint for MCP-connected hyphae (NousResearch Hermes Agent).

```
Tool: cynic_crystal_observe
Params:
  - crystal_id: string (optional — auto-generated if absent, using content hash)
  - content: string (required on first observation, optional on updates)
  - domain: string (required)
  - score: float (0.0 to 1.0)
  - source: string (identifies the calling hypha)
  - sentiment: string (optional: "positive", "negative", "neutral")
```

### 4. Domain-dependent decay

**Replace global `DECAY_DAYS = 90` with per-domain configuration:**

```rust
fn decay_days(domain: &str) -> f64 {
    match domain {
        "chess" => 180.0,        // positions are timeless
        "token" | "token-analysis" => 14.0,  // crypto moves fast
        "twitter" | "hermes" => 30.0,        // social signal decays
        "hermes-skill" => 90.0,  // learned skills are durable
        _ => 90.0,               // default unchanged
    }
}
```

**Why these values (conjecture, φ⁻²):** Based on domain characteristics, not measurement. Phase 2 should measure actual crystal usage patterns and derive decay rates from data (CHAOS→MATRIX).

**Impact:** `decay_relevance()` in `engine.rs` calls `decay_days(domain)` instead of the `DECAY_DAYS` constant. Pure function change, no storage impact.

### 5. Shatter

**New REST endpoint: `POST /crystal/{id}/shatter`**

```json
{
  "reason": "Smart contract exploited — token is worthless",
  "source": "hermes-agent"
}
```

**Behavior:**
- Sets crystal state to `Dissolved` immediately, regardless of current state.
- Records shatter event: `shattered_at`, `shatter_reason`, `shatter_source` fields on the crystal.
- Emits `KernelEvent::CrystalShattered { crystal_id, domain, reason }`.
- Idempotent: shattering a Dissolved crystal is a no-op.
- Auth: Cortex/Internal only (Organ role cannot shatter — prevents a compromised organ from nuking the crystal store).

**New fields on Crystal:**

```rust
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

**MCP tool: `cynic_crystal_shatter`** — same interface, Cortex/Internal role only.

### 6. Content evolution (mutation/compression) — Phase 2

**Not implemented in Phase 1.** Tracked here for completeness.

Content evolution requires an LLM to compress/abstract crystal content as it matures. This is expensive and introduces a dependency on inference availability.

**Phase 2 design sketch:**
- When a crystal transitions Forming → Crystallized (21 observations from diverse sources), trigger a compression job.
- The compression LLM (sovereign, via Soma slot) reads all contributing observations and produces an abstracted template.
- Example: `"BONK holder concentration 47% → BARK"` becomes `"Tokens with holder concentration > 40% historically receive negative judgment"`.
- The abstracted content replaces the verbatim content. `content_version` counter increments.
- A second compression at Crystallized → Canonical (233 observations) further abstracts.

**Why defer:** Content evolution needs inference (LLM call). Phase 1 focuses on the substrate (API, source tracking, decay, shatter) — pure infrastructure with no inference dependency. Phase 2 adds the intelligence layer.

### 7. Recycling (autophagy) — Phase 2

When a crystal dissolves (decay or shatter), its insights should feed back into the system rather than being silently deleted.

**Phase 2 design sketch:**
- On dissolution: emit `POST /observe` with `domain=crystal-autophagy`, `context=<what the crystal was and why it died>`.
- Other hyphae can consume these death observations to learn what DOESN'T work.
- This is the negative-knowledge path (anti-patterns, counter-memes, falsified hypotheses).

---

## Implementation scope — Phase 1

### New Rust code

| File | Change |
|------|--------|
| `domain/ccm/crystal.rs` | Add `contributing_sources: BTreeMap<String, u32>`, `shattered_at`, `shatter_reason`, `shatter_source` fields |
| `domain/ccm/engine.rs` | `decay_days(domain)` function, update `decay_relevance` to use it (replaces `DECAY_DAYS` constant) |
| `domain/events.rs` | Add `CrystalShattered { crystal_id, domain, reason }` variant to `KernelEvent` enum |
| `api/rest/data.rs` | `POST /crystal/{id}/observe` handler, `POST /crystal/{id}/shatter` handler. Update `crystal_to_json` field count test (16 → 20) |
| `api/rest/mod.rs` | Route registration for new endpoints |
| `api/rest/middleware.rs` | Add `/crystal/` prefix to Organ allowlist (observe only, not shatter — shatter gated in handler) |
| `api/mcp/judge_tools.rs` | `cynic_crystal_observe` and `cynic_crystal_shatter` MCP tools |
| `storage/surreal/crystals.rs` | `observe_crystal_hypha()` — Welford UPSERT without verdict coupling + Dissolved guard, `shatter_crystal()` |
| `storage/memory.rs` | In-memory implementations of new storage methods |
| `storage/reconnectable.rs` | Forward new methods (K17) |
| `domain/storage/mod.rs` | New `StoragePort` methods |
| `domain/storage/null.rs` | Null implementations |

### No changes to

- `pipeline/crystal_observer.rs` — verdict observation path unchanged
- `is_crystallizable_domain()` — verdict whitelist unchanged (hypha path bypasses it)
- `MIN_CRYSTALLIZATION_CYCLES` / `CANONICAL_CYCLES` — thresholds unchanged
- Welford math — reused as-is
- `format_crystal_context()` — consumers see all mature crystals regardless of source

### Edge case: observing a Dissolved crystal

The hypha observe SQL MUST guard against Dissolved state. Observing a shattered/dissolved crystal would silently resurrect it. The `observe_crystal_hypha` UPSERT includes:
```sql
IF state = 'dissolved' THEN THROW 'crystal is dissolved — cannot observe' END
```
Callers receive a 409 Conflict. To re-nucleate after shatter, create a new crystal explicitly.

### SurrealDB schema

```sql
-- New fields on crystal table (backward compatible — all have defaults)
-- contributing_sources: object (BTreeMap serialized as JSON object)
-- shattered_at: option<datetime>
-- shatter_reason: option<string>
-- shatter_source: option<string>
-- No migration needed — UPSERT with ?? defaults handles missing fields
```

### Tests

| Test | Validates |
|------|-----------|
| Hypha observe creates Forming crystal | New endpoint creates crystals |
| Hypha observe feeds existing crystal | Welford accumulation works without verdict |
| Source diversity tracked correctly | `contributing_sources` populated and incremented |
| Multiple sources recorded | BTreeMap tracks distinct hyphae |
| Shatter transitions to Dissolved | Any state → Dissolved on shatter |
| Shatter records reason and source | Provenance fields set |
| Shatter is idempotent | Shattering Dissolved crystal succeeds silently |
| Domain-specific decay | Token decays faster than chess |
| Organ role cannot shatter | RBAC enforcement |
| Hypha observation bypasses domain whitelist | `is_crystallizable_domain` not checked |
| Content updates on higher score | Mutation seed works |
| Observe Dissolved crystal returns 409 | Dissolved guard prevents resurrection |
| Organ role can observe but not shatter | RBAC allowlist correct for both endpoints |
| Domain-specific decay returns correct values | `decay_days()` unit tests per domain |
| `crystal_to_json` field count updated | Serialization includes all 20 fields |

---

## What this enables (mycelium vision)

```
BEFORE (1 producer, 1 consumer)
════════════════════════════════
/judge → observe_crystal → Dog prompts

AFTER (N producers, N consumers)
════════════════════════════════
/judge verdict      ──→ observe_crystal      ──→ Dog prompts
Hermes Agent skill  ──→ crystal_observe_hypha ──→ Hermes next cycle
Claude insight      ──→ crystal_observe_hypha ──→ session-init context
Gemini thesis       ──→ crystal_observe_hypha ──→ Gemini briefing
ops learning        ──→ crystal_observe_hypha ──→ sovereign-ops
trading signal      ──→ crystal_observe_hypha ──→ token screener
                            │
shatter event ──────────────┘ (instant death)
                            │
            domain-specific decay (14d token, 180d chess)
```

Every hypha reads `GET /crystals?domain=X`. Every hypha writes `POST /crystal/{id}/observe`. The crystal substrate doesn't know or care what kind of hypha produced the observation. The Welford math treats all observations equally. Source diversity is tracked for Phase 2 gating.

---

## Phases

| Phase | Scope | Depends on |
|-------|-------|------------|
| **1 (this spec)** | Source tracking, hypha observe, domain decay, shatter | Nothing — pure infra |
| **2** | Content evolution (LLM compression at state transitions) | Phase 1 + Soma inference slots |
| **3** | Diversity-weighted crystallization gate | Phase 1 + 30 days of diversity data |
| **4** | Recycling/autophagy (dissolution → observation feedback) | Phase 1 |
| **5** | Topology-as-memory (crystal graph, not just crystal store) | Phase 2 + Phase 3 |

---

## Falsification criteria

| Claim | Test | Threshold |
|-------|------|-----------|
| Non-verdict crystals can crystallize | Run NousResearch Hermes for 30 days, count Crystallized crystals | ≥ 1 crystal reaches Crystallized |
| Source diversity correlates with crystal quality | Compare confidence of multi-source vs single-source crystals | Multi-source mean confidence > single-source (p < 0.05) |
| Domain-specific decay improves relevance | Compare token-domain crystal freshness before/after | Stale token crystals reduced by > 50% |
| Shatter prevents stale crystals from poisoning | Inject known-bad crystal, shatter it, verify Dogs don't see it | Shattered crystal absent from `format_crystal_context` output |
| Hypha observe doesn't break verdict path | Run `make check` + existing crystal tests | All tests pass |

---

## Open questions (for Phase 2+)

1. **Compression prompt:** What does the LLM prompt for crystal abstraction look like? How do we measure whether compression improved or degraded the crystal's utility?
2. **Diversity threshold:** How many independent sources should be required before diversity-weighted gating kicks in? 3 (software Rule of Three)? φ⁻¹ × 10 ≈ 6?
3. **Cross-domain transport:** Should a crystal observed in domain "token" be discoverable by a hypha querying domain "hermes-skill"? Currently no (domain-exact filter). Embedding similarity handles cross-domain discovery but only for Dog prompts.
4. **Reflexivity guard:** If CYNIC acts on its own crystals (auto-trading), how do we detect circular confirmation? The crystal confirms itself because the action it prompted created the observation that fed it.
5. **Edge decay:** Should crystal confidence decay FASTER when more hyphae read it? (Trading insight: shared knowledge self-destructs.) This would be a radical departure from the scientific model (where replication strengthens).
