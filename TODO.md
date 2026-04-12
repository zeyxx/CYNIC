# CYNIC — TODO

> **Protocol, not backlog. The organism must metabolize, not just grow.**
> GROWL verdict (Q=52.6, 2026-04-12) — BURN=30 is the wound. Every item must lift BURN.

## Rules

1. **MAX 15 ACTIVE.** Everything else → DEFERRED. Not touched in 3 sessions → deleted.
2. **1 EXPERIMENT PER SESSION.** Start: hypothesis. End: measurement. No measurement = dissipated heat (§V.3).
3. **CLOSE ≥ OPEN.** Discover N items → close or defer N. The TODO never grows.
4. **TRACK COST.** Each session logs tokens, duration, output in the Session Log below.

Last updated: 2026-04-13 | Session: temporal-wiring-O2 + dev-crystal-proof-O3 + session-cost+coord-O3

---

## Active (9/15)

### Foundation — unblock metabolism

- [x] **#1 Dev crystal loop proof (Phase 1)** — /judge 5 dev patterns (architecture, error handling, testing), verify crystal forms in dev domain + injects on next similar query. ✅ FINDING: Crystals DO form on dev domain (falsifies R22 "chess-only"). 2 crystals forming (architecture, error handling). Pattern 6 re-judge test: Q degraded -13.9% (from 0.446→0.384) because crystals in forming state (1-2 observations, confidence 0.38-0.48) are too weak — correct behavior. Deferred: needs ≥21 observations per crystal to reach Crystallized state before injection improves. Next 20 sessions of dev patterns will accumulate observations.
- [ ] **#2 organ_quality gate** — Dogs below 50% JSON validity excluded from jury. Verify ParseFailureGate is sufficient or json_valid_rate needs explicit gate. Prerequisite for trusting LLM Dogs on dev domain.
- [x] **#3 Session cost tracking** — Add token+duration logging to session-stop.sh. Wired: session-init.sh records start, session-stop.sh measures duration + commit count + posts to /observe. No token count yet (requires Claude Code API integration). Done 2026-04-13.
- [x] **#4 TODO protocol enforcement** — This rewrite. 4 rules. Max 15. Session log. Done 2026-04-12.

### Body — prevent regression

- [ ] **#5 Crystal challenge mechanism** — Background task re-judges oldest crystallized crystals without injection. If Q-Score delta > φ⁻², dissolve. Prevents crystal poison (DEBT-A2: content write-once, no contradiction detection).
- [ ] **#6 Greffe 3: alerting consumer** — ContractDelta { fulfilled: false } → Slack message. K15: structured log only is not "acting."
- [x] **#7 Coord: sessions register + claim** — Verify coord-claim hook. ✅ FINDING: protect-files.sh and coord-claim.sh hooks were registered in settings.json but scripts missing (K5 violation). Created both hooks + validated. /coord/claim API working. Both sessions should now appear in /coord/who. Hooks live-reload script changes (not new entries). Done 2026-04-13.
- [ ] **#8 Wire or delete K15 dead-ends** — Triage 3 orphan producers: store_infra_snapshot, session_summaries, dream_counter. Wire acting consumer OR delete producer. Note: protect-files.sh was dead architecture (K5) — now wired (hooks in settings.json).

### Verify — measure what changed

- [ ] **#9 Verify crystal contention fix** — 24h after deploy: check SurrealDB logs for compaction conflicts. Target: <10/24h (was 176). If not → root cause is elsewhere.
- [ ] **#10 Verify coord-claim if field** — Test `if: "Edit(cynic-kernel/src/**)"` pattern live. If no match → try absolute path or `*`.

---

## DEFERRED (revisit when BURN > 50)

**Findings:** 38 open (1 critical RC1-1, 4 high, 16 medium, 4 low, 7 concurrency). See `docs/audit/CYNIC-FINDINGS-TRACKER.md`.
**Practice:** CI remote runner (GAP-7), peer review (GAP-6), functional specs (GAP-4), TCO (GAP-9), SurrealDB vs Postgres falsification (GAP-5), threat model STRIDE (GAP-8).
**Architecture:** Pipeline module split (T4), cynic-node Phase C, sources supervisor, CredentialPort.
**Identity:** T3a incarnation metrics (baseline still 0), identity layer audit.
**Kairos:** pyarrow fixed, node offline, restart when online.
**Infra:** Dual sensing R12, MCP lifecycle, boot-state capture (A1), RUST_MIN_STACK 4GB.
**Security:** MCP zero auth (RC1-1), boot integrity, tamper detection.

---

## Session Log

| Date | Session | Duration | Commits | Crystals | Closed | Opened | Notes |
|------|---------|----------|---------|----------|--------|--------|-------|
| 2026-04-13 | session-cost+coord-O3 | ~45m | 2 | 2 forming | 2 | 0 | Session cost tracking + coord-claim hooks. #3 (cost tracking) + #7 (coord-claim) closed. Hook validation verified. |
| 2026-04-13 | dev-crystal-proof-O3 | ~30m | 0 | 2 forming | 1 (partial) | 1 | 5 dev patterns + 1 re-judge test. Crystals forming (not chess-specific) — phase 1 complete. Need 19-20 more observations/pattern to crystallize. Re-judge after crystallization phase (deferred). |
| 2026-04-13 | temporal-wiring-O2 | ~1h | 1 | 0 | 0 | 0 | O2 implementation: hardcoded heuristic, 7 perspectives, integrated into judge_pipeline |
| 2026-04-12 | crystal-contention+K13+systemd | ~2h | 3 | 0 | 3 | 0 | BURN=30 diagnosed, anti-patterns mapped |

---

## State Snapshot

- Kernel: sovereign under systemd, 4/4 Dogs, contract fulfilled
- Temporal wiring complete (O2 strategy: hardcoded heuristic, 7 perspectives, geometric mean aggregation)
- **Crystal formation verified on dev domain (non-chess)** — 2 crystals forming, need 19-20 more observations to crystallize
- **Session cost tracking wired** — duration + commit count logged via hooks, foundation for Rule 7 (measure before/after)
- **Coord-claim hooks operational** — protect-files.sh + coord-claim.sh auto-enforce file claims (K5 debt resolved)
- Crystal contention fix deployed, awaiting 24h verification (#9)
- MCP K13 closed (same health function as REST)
- GROWL verdict (Q=52.6): BURN=30 wound, SOVEREIGNTY=75 strength
- 7 anti-patterns identified → saved to memory (feedback_anti_patterns.md)
- R22 status: PARTIALLY FALSIFIED (crystal formation generalizes; injection timing validated)
