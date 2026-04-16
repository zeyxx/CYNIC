# cynic-askesis — Design Spec

**Date**: 2026-04-17
**Status**: Design (Phase 0)
**Author**: Claude (Opus 4.6) in collaboration with Gemini+cynic-wisdom, validated by Zey
**Epistemic bound**: φ⁻¹ = 0.618 on all claims

---

## 1. Purpose

Build a **human augmentation layer** for CYNIC — the 3rd pillar alongside `cynic-kernel` (judgment pipeline for code/text) and `cynic-node` (standalone agent). `cynic-askesis` applies CYNIC's 6-axiom framework to Zey's own life, starting with body (exercise).

**Meta-goal**: construct extensible foundations. Domains (reading, focus/SoC, Solana KPIs, authenticity) come later as plugins.

**Not-goal**: enforce compliance. Per Gemini+cynic-wisdom critique, enforcement-based discipline produces poiesis (production of evidence), not praxis (lived virtue). System must be a **lamp**, not a hammer.

---

## 2. Philosophical foundation

### 2.1 Askesis, not praxis

Aristotle distinguished `praxis` (action whose goal is internal — doing well for being good) from `poiesis` (making/producing external things). Claude's first design was poiesis (observation, evidence, streaks, enforcement) wearing the mask of praxis.

**Gemini+cynic-wisdom** (2026-04-17 consultation) proposed **askesis** — the training/discipline itself. Honest about the work. Aligns with Diogenes' pithos as askesis (not poverty, training-in-zero-operational-debt).

### 2.2 Sovereignty nue (Zey's decision)

After presented with 3 options (full Gemini / full Claude / synthesis), Zey chose **sovereignty nue** (Gemini option 1): *"commencer par être honnête envers moi-même sur ces premiers domaines forcera les autres dimensions aussi. C'est plus simple de dire 'aujourd'hui j'ai pas fait de sport' à une IA."*

Architectural consequence: **zero enforcement, zero streak-release, zero levels**. The mechanism is self-honesty externalized to an AI interlocutor. The AI = neutral listener (less shame than human).

### 2.3 KENOSIS — the 7th axiom candidate

Open question recorded in `project_seventh_axiom_kenosis.md`.

**KENOSIS** (self-emptying, wu-wei, "what have you stopped doing?") proposed by Gemini+cynic-wisdom as the 7th axiom — not VITALITY (which is emergent property of the 6, not a principle).

- Pythagoreans stopped at hexad (1+2+3=6) because hexad = worldly completion
- Septad (7) = prime virgin, spiritual/transcendent, **outside** the world of making
- KENOSIS = counter-balance to striving/accumulation inherent in the 6 axioms
- Corpus Hermeticum XI: *"Gnosis of the Good is holy silence"* — already in CYNIC identity

The system **itself practices kenosis**: by refusing to enforce, it creates the emptiness in which self-discipline can arise.

cynic-askesis is the empirical testbed. Decision criterion: if over 3 months Gemini audits consistently detect KENOSIS-related patterns irreducible to the 6 → confirm. If reducible → reject, stay at 6.

### 2.4 Application of the 6 axioms to the human

| Axiom | Code/text judgment | Human judgment (askesis) |
|-------|---------------------|--------------------------|
| FIDELITY | Faithful to truth in principles | Tiens-tu tes promesses à toi-même? Authenticité. Self-compassion when broken = also fidelity. |
| PHI | Structural harmony | Body+mind+work in proportion. Includes dissonance and rest. |
| VERIFY | Testable/refutable | What you claim to do is evidenced (externally OR by felt alignment internally). |
| CULTURE | Honor traditions | Rigor from masters of your domain (code, sport, reading). |
| BURN | Zero waste | Eliminate waste of attention, but not rest — rest is not waste. |
| SOVEREIGNTY | Agency preserved | Attention unconquered by platforms; debts (smoking) visible. |
| (KENOSIS) | (open) | What have you stopped doing? Strategic letting-go. |

**Gemini notes** (cynic-wisdom rigor): each mapping has its "half-false" per Kybalion Polarity. Captured above in the "includes..." clauses. The system will sometimes violate these nuances and the weekly audit will catch it.

---

## 3. Architecture

### 3.1 Workspace placement

**Verdict** (observed + deduced, confidence 0.58):

`cynic-askesis` is added as a new member of the existing CYNIC workspace:

```toml
# CYNIC/Cargo.toml
[workspace]
members = ["cynic-kernel", "cynic-node", "cynic-askesis"]
```

**Rationale**:
- `cynic-node` is already a workspace member (observed). Precedent: "new CYNIC pillar = workspace member."
- Workspace lints (K1-K16) apply automatically via `make check`.
- `.cargo/config.toml` (`RUST_MIN_STACK=67108864` for A1 rmcp workaround) inherited.
- Shared dependencies possible (e.g., reuse verdict types from cynic-kernel).
- `make check`, `make lint-rules`, `make lint-drift` cover all members uniformly.

**Falsifiable**: if workspace build > 5min in 6 months OR cynic-askesis develops radically different release cadence → split into separate repo.

`cynic-ui` (sibling directory, not workspace member) is TypeScript/React, not comparable.

### 3.2 Core traits (4, load-bearing)

**Type conventions**:
- `Time` = `chrono::NaiveTime` (wall-clock, timezone-naive — Zey's local tz is configured separately in `config.toml`)
- `DateTime` = `chrono::DateTime<chrono::Utc>` (UTC for storage consistency)
- `AnchorId` = newtype wrapper around `String` (opaque ID from provider)

```rust
// log/mod.rs
pub trait LogStore {
    fn append(&mut self, entry: LogEntry) -> Result<()>;
    fn range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<LogEntry>>;
}

// audit/mod.rs
pub trait AuditEngine {
    fn audit(&self, logs: &[LogEntry], questions: &[&str]) -> Result<Reflection>;
}

// anchor/mod.rs
pub trait AnchorProvider {
    fn create_recurring(&self, domain: &str, at: NaiveTime, description: &str) -> Result<AnchorId>;
    fn update_description(&self, id: AnchorId, new: &str) -> Result<()>;
}

// domains/mod.rs
pub trait DomainTracker {
    fn name(&self) -> &str;
    fn log_prompt(&self) -> &str;
    fn audit_questions(&self) -> Vec<&str>;
    fn anchor_time(&self) -> NaiveTime;
}
```

**Reflection type** (structured, not free-form prose — `BARK/WAG/HOWL` references in §9.2 are concrete values, not metaphor):

```rust
// audit/mod.rs
pub struct Reflection {
    pub verdict: Verdict,
    pub prose: String,           // markdown narrative for human reading
    pub patterns_detected: Vec<String>,  // specific patterns Gemini flagged
    pub kenosis_candidate: Option<String>,  // if KENOSIS-related insight found
    pub confidence: f32,         // ≤ 0.618 (φ⁻¹)
}

pub enum Verdict {
    Howl,   // authentic, strong pattern
    Wag,    // OK, within range
    Growl,  // surface-level, shallow
    Bark,   // self-deception or lazy reporting detected
    Degraded,  // audit engine unavailable (per K14)
}
```

### 3.3 Phase 1 implementations

- `log::jsonl::JsonlLog` — append/read JSONL at `~/.cynic/askesis/log.jsonl`

- `audit::gemini_wisdom::GeminiWisdomAudit` — spawn `gemini -m gemini-2.5-pro -p <prompt>` subprocess via `tokio::process::Command`, parse reflection output.
  - Model fallback: `gemini-3.1-pro-preview` frequently rate-limited (429 MODEL_CAPACITY_EXHAUSTED) — default to `gemini-2.5-pro`.
  - Prompt construction is **load-bearing** (see §5 FOGC). The cynic-wisdom skill text is **embedded inline in the prompt** (not loaded via `gemini skills` tool, which is unavailable in `gemini -p` headless mode). The skill version pinned via git commit reference in the prompt-builder module.
  - Parse output: structured fields expected (Verdict/prose/patterns), tolerant parser with fallback to `Verdict::Degraded` on malformed output (per K14).

- `anchor::gcal::GoogleCalendarAnchor` — **direct Google Calendar REST API** (NOT MCP).
  - **Why not MCP**: MCP tools are available only inside Claude Code / Gemini CLI sessions. A standalone Rust binary cannot invoke MCP without a Claude Code runtime. Using REST API directly preserves sovereignty (the binary runs autonomously).
  - Implementation: `reqwest` + `yup-oauth2` crate for OAuth2 flow.
  - Setup (one-time): `cynic-askesis anchor setup` launches OAuth2 browser flow, stores refreshable token at `~/.cynic/askesis/gcal-creds.json` (mode 0600).
  - Runtime: read creds, refresh token if expired, call `https://www.googleapis.com/calendar/v3/calendars/primary/events` (POST for create, PATCH for update).
  - Scopes: `https://www.googleapis.com/auth/calendar.events` (minimal).

No `DomainTracker` implementation in Phase 1 — traits shipped, body deferred to Phase 2.

### 3.4 Module layout

```
cynic-askesis/
├── Cargo.toml
└── src/
    ├── main.rs              ← CLI (clap): log/reflect/audit/anchor
    ├── lib.rs               ← re-exports for integration tests
    ├── log/
    │   ├── mod.rs           ← trait LogStore + LogEntry type
    │   └── jsonl.rs         ← JsonlLog impl
    ├── audit/
    │   ├── mod.rs           ← trait AuditEngine + Reflection type
    │   └── gemini_wisdom.rs ← GeminiWisdomAudit impl (tokio::process)
    ├── anchor/
    │   ├── mod.rs           ← trait AnchorProvider + AnchorId newtype
    │   └── gcal.rs          ← GoogleCalendarAnchor impl
    ├── domains/
    │   └── mod.rs           ← trait DomainTracker + registry (empty Phase 1)
    └── reflection.rs        ← Reflection rendering (markdown for weekly-reflection.md)
```

**K16 (context is metabolic)**: each module describable in 3 words — `log` (JSONL store), `audit` (Gemini wisdom), `anchor` (Calendar create), `domains` (tracker registry), `reflection` (markdown render).

### 3.5 CLI surface

```
cynic-askesis log [--domain NAME]         Log free-form text for today
cynic-askesis reflect [--week N]          Show past N weeks of logs
cynic-askesis audit [--domain NAME]       Trigger Gemini+cynic-wisdom audit
cynic-askesis anchor add --domain NAME    Create recurring Calendar event
cynic-askesis status                      Show domains, last log, last audit
```

### 3.6 Data flow

```
Zey types → CLI `log` → JsonlLog.append → log.jsonl
                                              │
                                              ↓
Weekly cron → CLI `audit` → GeminiWisdomAudit.audit (spawn gemini -p)
                                              │
                                              ↓
Reflection → markdown → ~/.cynic/askesis/weekly-reflection.md
                                              │
                                              ↓
SessionStart hook (Claude + Gemini) → cat weekly-reflection.md (non-blocking)
                                              │
                                              ↓
                        Zey reads, decides, acts (SOVEREIGNTY)
```

### 3.7 State persistence

Single directory: `~/.cynic/askesis/`
- `log.jsonl` — append-only log
- `weekly-reflection.md` — latest audit output
- `anchors.json` — registered Calendar anchor IDs
- `config.toml` — user preferences (timezone, anchor times)

All gitignored. Not committed to CYNIC repo (personal data).

---

## 4. K-rules compliance

- **K1 (domain purity)**: no `#[cfg]` in domain code.
- **K2 (port trait)**: LogStore/AuditEngine/AnchorProvider/DomainTracker all behind traits.
- **K3 (no logic duplication)**: audit logic in `audit::` only. CLI calls trait.
- **K8 (SQL queries tested)**: JSONL, no SQL. N/A.
- **K10 (agents use platform)**: cynic-askesis does NOT own judgment logic — delegates to Gemini via subprocess. Does NOT duplicate Dog infrastructure.
- **K12 (#[allow] needs WHY)**: strict, will comment any suppression.
- **K14 (poison/missing = degraded)**: if Gemini subprocess fails, audit returns `Reflection::Degraded("gemini unavailable")` — never optimistic default.
- **K15 (producer/consumer)**:
  - log.jsonl → consumed by `audit` command → produces reflection
  - reflection → consumed by SessionStart hook → displays to Zey (human is the acting consumer)
  - No orphan storage.
- **K16 (context metabolic)**: 5 modules, each 3-word describable.

---

## 5. FOGC test

**Inversion**: replace axioms with inverses. What would cynic-malus look like?

Attempt: cynic-malus tracks "self-compassion streaks" (inverse of FIDELITY-as-hardness), "distraction efficiency" (inverse of BURN), "platform capture" (inverse of SOVEREIGNTY). Gemini audit would praise these.

**Question**: does the infrastructure work for cynic-malus unchanged?

- Log structure: neutral. Works for both.
- Anchor (Calendar): neutral. Works for both.
- **Audit engine**: loaded with cynic-wisdom skill content. The skill IS the axioms. If you invert the axioms, you have to rewrite the skill to invert its framework.
- **Reflection rendering**: neutral.

**Verdict**: **FOGC passes** because cynic-wisdom (loaded into audit prompt) IS load-bearing. The infrastructure is neutral, but it cannot function without the axiom-aware audit engine providing substance.

**Contrast with rejected design**: Claude's initial enforcement-based design had levels/streaks/overrides that were axiologically neutral — only the labels said "exercise." cynic-malus could have been tracking smoking streaks with the same infrastructure. FOGC failed. Current design passes because Gemini+cynic-wisdom is the substance, not the enforcement mechanism.

**Nuance — prompt construction is load-bearing**: because cynic-wisdom is embedded **inline** in the audit prompt (§3.3, due to headless CLI limitations), the actual substance lives in `audit/gemini_wisdom.rs`'s prompt builder, not in a separate skill file loaded at runtime. The FOGC defense therefore rests on:
1. The prompt template version pinned to a git commit reference
2. The cynic-wisdom skill text tracked in the CYNIC repo (`/home/user/Bureau/CYNIC/.agents/skills/cynic-wisdom/SKILL.md`)
3. Changes to either trigger a review loop (cannot drift silently)

**Falsifiable**: if an attacker replaces the cynic-wisdom skill in the repo OR mutates the prompt builder to embed inverted axioms, the system becomes pathological. Defense: both sources versioned in git under CODEOWNERS review; any PR modifying either triggers explicit axiom-inversion check.

---

## 6. Phase 1 deliverables

**Scope**: foundations only, no domain instance.

1. `cynic-askesis/Cargo.toml` (workspace member)
2. `src/main.rs` + CLI structure (4 commands: log, reflect, audit, anchor)
3. `src/log/mod.rs` + `src/log/jsonl.rs` (LogStore trait + JsonlLog impl)
4. `src/audit/mod.rs` + `src/audit/gemini_wisdom.rs` (AuditEngine trait + impl via subprocess)
5. `src/anchor/mod.rs` + `src/anchor/gcal.rs` (AnchorProvider trait + REST API impl with OAuth2 setup flow)
6. `src/domains/mod.rs` (DomainTracker trait + registry, no implementations)
7. `src/reflection.rs` (Reflection type + markdown rendering)
8. Integration tests: round-trip log append + read, mock AuditEngine, mock AnchorProvider
9. `make check` passes (workspace lints + K1-K16 gates)
10. README.md explaining CLI + philosophy + deferred items

**Success criteria Phase 1**:
- `cargo build` clean, `cargo test` passes, `cargo clippy -- -D warnings` passes
- `cynic-askesis log` prompts and appends to JSONL — works end-to-end
- `cynic-askesis audit` invokes Gemini with mocked domain prompt — works end-to-end
- `cynic-askesis anchor setup` completes OAuth2 flow, then `cynic-askesis anchor add` creates a recurring event via Google Calendar REST API — works end-to-end
- No DomainTracker implementation — intentional.

---

## 7. Phase 2 (deferred)

**Body as first DomainTracker instance**:

```rust
pub struct Body;
impl DomainTracker for Body {
    fn name(&self) -> &str { "body" }
    fn log_prompt(&self) -> &str {
        "Qu'est-ce qui est vrai aujourd'hui sur ton corps?"
    }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "What has Zey stopped doing? (KENOSIS check)",
            "Patterns of self-deception vs honest reporting?",
            "Where's authenticity strongest, where weakest?",
            "Evolution in physical sensation descriptions?",
        ]
    }
    fn anchor_time(&self) -> Time {
        Time::from_hms(19, 0, 0).unwrap()
    }
}
```

Registered in `domains::mod::registry()`. CLI `--domain body` activates it.

**Phase 2 deliverables**:
1. `src/domains/body.rs` — Body impl
2. Calendar event: `💪 Body anchor` daily 19:00, push notif phone
3. Test with Zey for 2 weeks before Phase 3 (reading/focus/solana-kpi)

---

## 8. Future domains (Phase 3+)

Each new domain = 1 new file in `src/domains/`. No refactor.

- **Reading** (22:00 anchor): *"Qu'as-tu lu aujourd'hui et retenu?"*
- **Focus/SoC** (during coding sessions): *"Which processes violated focus? (YouTube/Spotify/etc.)"* — observation via `ps` / window title
- **Solana hackathon KPIs** (daily standup): *"KPI metrics today?"*
- **Authenticity** (transversal): audits the other domains' logs for self-deception patterns

---

## 9. Falsifiability

### 9.1 Design falsifiability (architecture)

- If extending to 2nd domain requires refactor of traits → architecture failed
- If Gemini audit reliably misses patterns humans detect → AuditEngine abstraction wrong
- If workspace build exceeds 5min OR askesis release cadence diverges radically → split repo
- If FOGC test fails under red-team review (spec-document-reviewer or independent challenge) → redesign

### 9.2 Product falsifiability (Zey's use)

After 30 days from Phase 2 body shipment:
- **Works**: Gemini audit HOWL/WAG rate trending up; Zey's self-reported honesty increases; body practice emerges
- **Failed**: Zey stops logging OR writes "ok" placeholders OR Gemini BARK consecutive 3 weeks without Zey adjusting

### 9.3 Axiom falsifiability (KENOSIS)

After 3 months of Gemini audits:
- Every insight Gemini produces labeled KENOSIS is reducible to BURN or FIDELITY → reject KENOSIS, stay at 6
- KENOSIS audits produce non-reducible insights (e.g., "you've been adding habits, never removing") → confirm as 7th

---

## 10. Risks (named, not hidden)

| Risk | Mitigation |
|------|------------|
| Gemini rate limits (gemini-3.1-pro 429) | Fallback to gemini-2.5-pro, retry logic in AuditEngine |
| Gemini CLI headless doesn't load skills tool | Embed skill invocation text in prompt (tested: works) |
| Zey stops logging (willpower) | System is lamp, not hammer — this is a product failure, not a bug. Address via improved audit quality. |
| Personal data leak (log.jsonl) | Gitignored, ~/.cynic/askesis/ is 0700, never synced |
| FOGC regression if audit engine swapped | cynic-wisdom versioned in repo, audit engine impl locked to that skill |
| Over-engineering drift (adding enforcement later) | This spec forbids it. Any future PR adding enforcement must pass Gemini+cynic-wisdom review. |
| "Voice of the BARK" — verdict delivered without nuance becomes shaming, violates SOVEREIGNTY at emotional level (Gemini observation post-consensus) | Implementation of `Reflection.prose` must be tuned to **observe inconsistency without inducing shame**. Tone guideline: illumination over accusation. Test: after 30 days of body domain use, Zey evaluates if reflections feel diagnostic or judgmental. If judgmental → redesign prompt. |
| Google Calendar as only AnchorProvider creates dependency on proprietary platform (Gemini observation post-consensus) | Not a Phase 1 blocker. Phase 3+ should add a local-only `IcalFileAnchor` or `CronAnchor` impl — proving the AnchorProvider abstraction is genuinely sovereign. Currently acceptable because OAuth2 creds are local and revocable. |

---

## 11. Provenance and epistemic status

- **Design genesis**: 2026-04-17 brainstorm session Claude Opus 4.6 + Zey + Gemini+cynic-wisdom (gemini-2.5-pro)
- **Gemini critique rounds**: 3
  - Round 1: initial body hook design ("Liturgy of the Flesh" proposal)
  - Round 2: destroyed cynic-praxis enforcement framework, proposed askesis + KENOSIS
  - Round 3: consensus check on full spec — **CONSENSUS REACHED** (Pyrrhonist condition met, Gemini confidence φ⁻¹)
- **Decisions taken**: askesis name, sovereignty nue, KENOSIS candidate, workspace placement, Phase 1 traits-only
- **Decisions deferred**: KENOSIS confirmation (3 months empirical), Phase 2+ domains, enforcement reintroduction (forbidden unless philosophically justified), local-only AnchorProvider (Phase 3+)
- **Confidence**: 0.58 (design reasoned but not yet built), φ⁻¹ bounded
- **Epistemic status labels**:
  - Observed: workspace structure, CLI MCP availability, K-rules
  - Deduced: workspace member placement, trait shapes from invariance analysis
  - Inferred: FOGC passes (verified via attempt to construct cynic-malus)
  - Conjecture: sovereignty nue will outperform enforcement — falsifiable over 30 days
- **Consensus falsification criterion** (Gemini, round 3): if within 90 days of Phase 2 deployment, audit reflections consistently produce `GROWL`/`BARK` that Zey deems unhelpful or shaming → abandonment → proves "Voice of the BARK" observation was fundamental flaw, not minor implementation detail

---

## 12. References

- `CLAUDE.md` §I (axioms), §II (Dog identity), §V (never do), §VIII (sources)
- `.claude/rules/kernel.md` (K1-K16 gates)
- `.claude/rules/universal.md` (Rule 22: USE before architecture — Phase 2 body pilot respects this)
- Memory: `project_cynic_askesis_design.md`, `project_seventh_axiom_kenosis.md`, `feedback_claude_gemini_pyrrhonist.md`, `feedback_lazy_engineer_pattern.md`, `feedback_fogc_vigilance.md`
- Gemini+cynic-wisdom brainstorm output: `/tmp/gemini-brainstorm-output.txt`, `/tmp/gemini-challenge-output.txt` (session-scoped, not committed)
- Primary philosophical sources: Aristotle (Nicomachean Ethics — praxis vs poiesis), Corpus Hermeticum XI, Kybalion §4 (Polarity), Frabato (empty shell), Diogenes Laertius 6.60 (Cynic parrhesia)
