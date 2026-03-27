# CYNIC Roadmap v0.8 → v1.0 — Design Spec

*2026-03-27. Crystallized from metathinking + SRE review + AI infra review.*

## North Star

CYNIC is an organism, not an application. Every version is a **complete organism at its scale** — fractal self-similarity from single node to federation. Organs are interconnected: the crystal loop feeds Dog prompts, events connect organs, observations become knowledge, knowledge sharpens judgment. No organ is an island. Every gate must strengthen connections between organs, not just build isolated features.

## Principles

1. **Organic** — Gates emerge from work. This document is living. New gates appear as we build.
2. **3 gates max per version** — More = split. Each gate owned by one person.
3. **Machine-verifiable** — Every gate has a bash command. No subjective gates.
4. **Compound value** — Prioritize what feeds the most downstream systems.
5. **30% unplanned** — Emergence is the source of CYNIC's best features.
6. **No fix-to-escape** — Every gate builds foundation, never closes tickets for closure's sake.
7. **Fractal** — Each organ must work at its current scale AND be ready for the next scale. Design for the single node, but never in a way that prevents federation.

## Current Reality (2026-03-27)

| Metric | Value |
|--------|-------|
| Rust LOC | 18,982 (54 files) |
| Tests | 299 (242 unit + 57 integration/contract) |
| StoragePort methods | 30 |
| StoragePort adapters | 1 (SurrealDB). NullStorage exists but is a stub, not a real adapter |
| Security findings | 43 fixed / 9 partial / 38 open (of 90) |
| Dogs | 5 configured (1 deterministic + 2 API + 2 sovereign) |
| Event bus | 6 types, 0 internal consumers |
| Crystal domains validated | 1 (chess, Δ=+0.02-0.04, n=20-30, not statistically rigorous) |
| CI/CD | Local only (make check + pre-commit + pre-push). No remote CI. |
| Cargo.toml version | 0.7.3 |
| Git tags | v0.5.0 → v0.7.3 |
| Documentation | 69 files, 24K lines (more than code). 5 stale state dumps at root. |

## v0.8 — "Fondation Prouvée"

Theme: The foundation is mechanically verified. Security gaps closed. StoragePort proven agnostic. Workflow aligned.

| Gate | Verification | Status |
|------|-------------|--------|
| G1: Security closure | 0 CRIT open. All HIGH either FIXED or accepted-by-design (with rationale in tracker). `awk '/CRIT.*OPEN\|HIGH.*OPEN/' docs/audit/CYNIC-FINDINGS-TRACKER.md \| wc -l` = 0. "Accepted" means: attack surface documented, rationale written, finding row updated. | In progress. RC1-1 (MCP zero auth) needs decision: accept-by-design (stdio = process trust) or implement auth. |
| G2: StoragePort agnostic | InMemory adapter passes same contract tests as SurrealDB. ≥12 `fn contract_` tests exist. `cargo test --test integration_storage -- contract_ 2>&1 \| grep -c "test .* ok"` ≥ 12. Both adapters green. | Not started (currently 8 contract tests, InMemory doesn't exist) |
| G3: Workflow alignment | Cargo.toml version = git tag = VERSION.md. State dumps deleted. Doc lifecycle tags present. Requires creating `make lint-version` and `make lint-docs` targets as part of this gate. | Partial |

### Design constraints (from reviews)

- **RC1-1 (MCP zero auth)**: Decide accept-by-design (stdio = process trust) or implement auth. Do not leave PARTIAL. If accepted: document attack surface in tracker row, explain why process co-location = trust, close as "Accepted by design."
- **RC1 family**: RC1 includes sub-findings (RC1-1 through RC1-6). "RC1 full closure" in v0.9 G3 means ALL sub-findings resolved (FIXED or accepted). Currently: RC1-1 PARTIAL, RC1-2 through RC1-6 FIXED.
- **InMemory adapter**: Required before v0.9 — without it, contract tests skip silently on machines without SurrealDB. NullStorage is a stub (rejects all writes), not a real adapter.
- **Contract tests ≥ 12**: Currently 8. Fold into G2 — the gate enforces both the count and that both adapters pass. Need coverage on: quorum, set-once, voter_count, state transitions (21/233 thresholds), decay, read-side gate, sanitization, Canonical threshold, embedding round-trip.
- **Doc SoC**: 4 categories (LIVING/ARCHITECTURE/HISTORICAL/DRAFT). Rules 37-41 will be created in `.claude/rules/docs.md` as part of G3.
- **make lint-docs**: ~10 lines of shell checking lifecycle tags on all docs/*.md. Created as part of G3 implementation, not a precondition.
- **State dumps**: Delete immediately (quick-state.md, state-check.md, state-v3.md, state-v3b.md, v4-state.md). Git history preserves them.
- **Stale root files**: Evaluate HACKATHON-RULES.md, replit.md, algomancer-learning.md for deletion or move to docs/sessions/ (historical).

## v0.9 — "L'Organisme Apprend"

Theme: The crystal loop becomes multi-domain. Knowledge enters the system. The moat compounds.

| Gate | Verification | Sequence |
|------|-------------|----------|
| G1: φ-convergence | Score ≠ Confidence as separate fields in Verdict + Crystal structs. HOWL threshold = 0.528. `grep -c 'confidence:' cynic-kernel/src/domain/verdict.rs` ≥ 1 (distinct from score). `/test-chess` before/after with frozen crystal state — Δ direction preserved. | **First** — changes confidence semantics, must precede new domain measurements |
| G2: KAIROS domain | Crystal Δ > +0.02 on trading domain. `cargo test -- kairos` green. n≥100 stimuli with paired A/B design. Oracle = realized P&L where available. | **Second** — measured on new φ-convergence semantics |
| G3: cynic_learn | `grep 'cynic_learn' cynic-kernel/src/api/mcp/mod.rs` exists. Quarantine state: `grep 'source.*agent' cynic-kernel/src/domain/ccm.rs`. Dog evaluation gate before crystallization. Rate limit per agent. Precondition: all RC1 sub-findings (RC1-1 through RC1-6) resolved. | **Last** — highest risk feature, all security gates must be closed first |

### Design constraints (from reviews)

- **φ-convergence is a full migration**: Touches QScore, CrystalState SQL, StoragePort, pipeline, REST schema (breaking change), frontend. Do NOT bundle with other features.
- **Crystal Δ measurement rigor**: Need n≥100, paired design, preregistered hypothesis. Ceiling effect at 95% baseline means Δ=0.02 may be noise. External oracle required (P&L for trading, Stockfish for chess).
- **cynic_learn poisoning defense**: Quarantine state (source=agent, confidence=0.01). Must pass Dog evaluation before entering prompts. Max N crystals/hour/agent.
- **Crystal decay mechanism needed**: Without it, crystals accumulate forever. Trading regime change makes old crystals harmful. TTL-based + contradiction-driven decay.
- **Event bus consumers**: Pick 2 specific consumers — (1) CCM aggregator subscribes to VerdictIssued, (2) health monitor subscribes to DogFailed. Event bus is currently dead infrastructure.
- **Observability**: /metrics endpoint for crystal state transitions, Dog latency percentiles, verdict cache hit rates.

## v1.0 — "Souverain et Ouvert"

Theme: Someone else can deploy CYNIC and get value from it.

| Gate | Verification |
|------|-------------|
| G1: Dogs as Data | `ls cynic-kernel/dogs.toml` exists. `cargo test -- dog_runtime` green (add Dog via config, verify it participates in judgment). Dynamic quorum: `grep 'fn quorum' cynic-kernel/src/judge.rs` computes from healthy Dog count, not constant. |
| G2: API stable + observable | `curl -s $CYNIC_REST_ADDR/metrics` returns Prometheus-format data. Crystal state transitions, Dog latency percentiles, cache hit rates visible. Git log shows 0 breaking API changes in last 30 days. |
| G3: Contributor-ready | QUICKSTART.md exists. `make quickstart-test` runs a scripted deploy from clean state (Docker or fresh machine). UI connected to live kernel (S.). API.md covers all endpoints. |

### Design constraints (from reviews)

- **Dogs as Data hot-reload**: New config takes effect on NEXT judgment cycle only. Pipeline sees consistent Dog snapshot start-to-finish. DogConfigChanged event on bus. Signed/hash-pinned manifests to prevent config injection.
- **API stability requires observability first**: Can't claim stability if you can't measure it. /metrics must exist before the 30-day clock starts.
- **Contributor-ready is partially subjective**: `make quickstart-test` automates what we can. For the human element, ask ≥1 outsider to follow QUICKSTART and report blockers. Not a bash gate — an acceptance test.
- **Architecture federation-ready**: Check with existing make lint-rules (Rule 17, 32). May already be green — verify before treating as future work. If green, this is a confirmed property, not a gate.

## Post-v1 (Horizon — not planned, will emerge)

- Boot integrity + crash recovery (verify_integrity, dirty flag, read-only mode)
- Federation foundation (CRDT crystal design NOW if serious, GossipProtocol port)
- Embed agent (overnight code quality scanning)
- Inference metabolism native (replaces rtk)
- Memory bootstrap (80+ memory files → Forming crystals)
- Backup/restore verified round-trip

## Documentation SoC

### Categories

| Category | Rule | Location |
|----------|------|----------|
| LIVING | Must match code. Updated in same commit. | Root + docs/audit/TRACKER |
| ARCHITECTURE | One doc per concern. Evolves, never duplicated. git = version history. | docs/architecture/ |
| HISTORICAL | Frozen, dated, never modified. | docs/sessions/, docs/research/, docs/audit/ (snapshots) |
| DRAFT | Proposals under consideration. May be abandoned. | docs/design/, docs/superpowers/ |

### Rules (proposed for .claude/rules/)

37. Living docs match code — update in same commit as the code change.
38. One truth per concern — never create a second file on same topic. git is the version history.
39. Historical docs are frozen — never edit a dated file.
40. No state dumps — intermediate state belongs in memory files or git commits, not root .md files.
41. Lifecycle tag — every doc in docs/ starts with `<!-- lifecycle: living|architecture|historical|draft -->`.

### Immediate cleanup

- DELETE: quick-state.md, state-check.md, state-v3.md, state-v3b.md, v4-state.md
- DELETE or ARCHIVE: HACKATHON-RULES.md, replit.md, algomancer-learning.md
- MERGE: CYNIC-ARCHITECTURE-TRUTHS.md + CYNIC-ARCHITECTURAL-TRUTHS-V08.md → TRUTHS.md
- MOVE: CYNIC-CRYSTALLIZED-TRUTH.md → docs/sessions/ (historical)
- MOVE: SKILL-ROUTER-DESIGN.md, CCM-PRODUCT-CRYSTALLIZATION.md → docs/design/ (draft)
- ADD: `make lint-docs` gate (check lifecycle tags, 10 lines of shell)

## Review Sources

- SRE review: gate measurability, ordering, missing gates (observability, backup, degraded Dog health)
- AI Infra review: crystal Δ rigor, poisoning risks, moat evaluation, CRDT for federation, decay mechanism
- Metathinking: "fix-to-escape" risk, version misalignment, docs larger than code

## Open Questions (will resolve through work)

- Crystal decay: TTL-based vs contradiction-driven vs both?
- Federation: CRDT design now or defer? Cost of retrofit vs cost of premature design.
- Domain feedback signal: What does a non-chess, non-trading user use as oracle?
- DeterministicDog: structural ceiling on crystal Δ (~0.055 max). Research or accept?
- StoragePort trait split: when? Contract tests first (Rule 36), then split. v0.9 or v1.0?
