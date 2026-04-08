# Colosseum Frontier Foundations Audit — 2026-04-08

> **Status:** in progress. This is a living document updated phase-by-phase during the audit.
> **Plan:** `~/.claude/plans/foamy-spinning-sprout.md` (approved)
> **Budget:** 5h wall-clock hard cap.
> **Started:** 2026-04-08 — D+2 of Colosseum Frontier (Apr 6 → May 11)

---

## TL;DR (top of document, updated last)

_Pending Phase 7._

**Foundation verdict:** ⏳
**Q-score:** ⏳
**Go for D4:** ⏳
**Next session objective:** ⏳

---

## 1. Context

The user ("on doit s'assurer que les fondations sont strong déjà non ?") triggered a foundations audit before committing any code to the hackathon critical path (D4 → D3 → D1). Universal rule 14: *Strong > no > weak foundation. Prove E2E with real data before building on a subsystem.*

Initial memory-based plan was to start coding D4 on day 2. Phase 1 exploration (2 Explore agents, ~5 min) surfaced critical issues that rewrote the plan entirely.

**Dogs' pre-approval judgment on the audit plan itself** (via `mcp__cynic__cynic_judge`, verdict_id `68484451-9b37-4bf8-b70f-f13ac9623e77`): **WAG** (Q = 0.48 / 0.618 phi-max). Weakest axioms: BURN (0.35) and VERIFY (0.40). The plan was amended in response before execution.

Notable: only 2 of 3 Dogs voted (`deterministic-dog + qwen-7b-hf`). `qwen35-9b-gpu` was absent — free L2 finding before Phase 1 even started.

---

## 2. Critical pre-findings (surfaced by exploration, not by probes)

| # | Severity | Finding | Source |
|---|---|---|---|
| PF-1 | CATASTROPHIC | `bridge/`, `programs/`, `HACKATHON-SPEC.md` ALL untracked in GASdf — zero commits since 2026-03-30 spec freeze | `cd GASdf && git log --since=2026-03-30 --oneline` → empty |
| PF-2 | HIGH | D4 `/judge/status/{id}` is architectural change (15-25h), not 4h endpoint. `pipeline.rs::run` is fully synchronous; no job store, no streaming, no partial write | Explore agent source read of `cynic-kernel/src/pipeline.rs` + `api/rest/judge.rs:76` |
| PF-3 | MEDIUM | Pre-hack milestone missed: spec says "April 5 (pre-hack): D4 done". D4 does not exist in any form at D+2 | `HACKATHON-SPEC.md` line 185 (timeline) vs `grep -r "judge/status" cynic-kernel/src/` → empty |
| PF-4 | LOW | Memory claimed `@solana/kit v5+`; reality is `@solana/web3.js ^1.95.0`. Hallucinated framework version | `GASdf/package.json` + spec line "Zero deps beyond @solana/web3.js" |
| PF-5 | MEDIUM | gemma-4b-core Dog down; session-start counter "4/5 Dogs" inconsistent with live `cynic_health` (3 active); pre-approval judge returned only 2 Dogs (qwen35-9b-gpu absent) | `mcp__cynic__cynic_health` + `mcp__cynic__cynic_judge` response `dogs_used` field |

**PF-1 resolved in Phase 0** (commit `43d0521` on GASdf main, 17 files, 5400 insertions, no keypair/binary/secret leaked).

---

## 3. Audit matrix — 5 layers × 4 prisms

| | Mechanical | Adversarial | Temporal | Epistemic |
|---|---|---|---|---|
| **L1 CYNIC kernel** | ⏳ P1.1 | ⏳ P3 | ⏳ P4 | ⏳ P5 |
| **L2 Dogs fleet** | ⏳ P1.2 | ⏳ P3 | ⏳ P4 | ⏳ P5 |
| **L3 Pinocchio program** | ⏳ P1.3 | ⏳ P3 | ⏳ P4 | ⏳ P5 |
| **L4 Bridge + devnet** | ⏳ P1.4 | ⏳ P3 | ⏳ P4 | ⏳ P5 |
| **L5 GASdf repo** | ⏳ P1.5 | ⏳ P3 | ⏳ P4 | ⏳ P5 |

Legend: ⏳ pending · 🔄 in progress · ✅ passed · ⚠️ degraded (documented) · ❌ kill switch

---

## 4. Phase 0 — Disaster prevention ✅

**Gate passed.** Commit `43d0521` on GASdf `main`, authored locally (no push).

```
$ cd /home/user/Bureau/GASdf && git log --oneline -3
43d0521 feat(hackathon): version-control Colosseum Frontier governance work
82a0b2e fix(health): don't fail healthcheck on low fee payer balance
aca2b3d fix(docker): add python3/make/g++ for bigint-buffer native build

$ git status --short
(empty — working tree clean)
```

**Files committed (17):** `.gitignore` (+4 lines excluding `target/` and `test-ledger/`), `bridge/agent-bridge.js`, `bridge/governance.js`, `docs/HACKATHON-SPEC.md`, `package-lock.json`, `programs/cynic-governance/{Cargo.toml, Cargo.lock, src/**}`.

**Files intentionally excluded** (via the new `.gitignore` entries):
- `programs/cynic-governance/target/deploy/cynic_governance-keypair.json` — program authority keypair
- `programs/cynic-governance/target/deploy/cynic_governance.so` — build artifact (can rebuild from src)
- `programs/cynic-governance/test-ledger/` — solana-test-validator state

**Secret scan result:** clean. Only match on line 76 of `bridge/agent-bridge.js` was `Bearer ${CYNIC_API_KEY}` — env var reference, not a hardcoded secret. `HACKATHON-SPEC.md` uses `T.` placeholder convention (line 11) per CLAUDE.md rules.

**Not done:** `git push origin main`. Remote is `git@github.com:zeyxx/GASdf.git`. Requires explicit user approval before push.

---

## 5. Phase 1 — Mechanical layer probes

### L1 CYNIC kernel — `/judge` E2E
⏳ pending P1.1

### L2 Dogs fleet
⏳ pending P1.2

### L3 Pinocchio program + devnet state
⏳ pending P1.3

### L4 Bridge + devnet keys
⏳ pending P1.4

### L5 GASdf repo health
⏳ pending P1.5

---

## 6. Phase 2 — V2 E2E loop proof

⏳ pending. Gated on L1-L5 all green.

---

## 7. Phase 3 — Adversarial failure modes

⏳ pending. Capped at ~12 entries total across 5 layers (top 3-5 per layer, P×I ≥ 12).

---

## 8. Phase 4 — Temporal drift log

⏳ pending.

**Already known (from exploration):**
- GASdf `git log --since=2026-03-30`: **0 commits until Phase 0 fixed this** (`43d0521`).
- CYNIC `git log --since=2026-03-30`: 30 commits, all consistent with memory (dynamic roster shipped, judge hang fixed, cynic-node Phase B merged).
- `.so` size drift: spec says 24.4KB, actual 25.2KB (+0.8KB).
- Memory claim "@solana/kit v5+" → hallucination, falsified by `package.json` reading `@solana/web3.js ^1.95.0`.
- Spec timeline "April 5 pre-hack: D2 + D4 done" → D4 missed by 3 days.

---

## 9. Phase 5 — Epistemic verdict (live Dogs + self-applied framework)

⏳ pending P5.

**Pre-approval Dogs' verdict on the audit plan itself** (captured above, for reference in convergence check):
- verdict_id: `68484451-9b37-4bf8-b70f-f13ac9623e77`
- verdict: **Wag** (0.48 / 0.618 phi-max)
- Per-axiom: FIDELITY 0.60 · PHI 0.47 · VERIFY 0.40 · CULTURE 0.618 · BURN 0.35 · SOVEREIGNTY 0.50
- Dogs used: `deterministic-dog + qwen-7b-hf` (2 of 3 expected; qwen35-9b-gpu absent)

---

## 10. Phase 6 — D4 re-scope decision

⏳ pending P6. Options:
- **A (polling-on-store, ~7h):** /judge returns id, background pipeline, /judge/status reads storage when done. Honest but dull.
- **B (progressive writes, ~15-20h):** refactor `judge.evaluate` to stream per-Dog, new `verdict_jobs` table. Matches spec intent ("drip effect wow moment").
- **C (animation cheat, ~2h):** UI fakes arrival with timings. Ruled out (R20 violation).

Decision tied to Phase 5 verdict (HOWL/WAG → Option B; GROWL → Option A; BARK → stop).

---

## 11. Phase 7 — Go/No-Go verdict

⏳ pending P7. See TL;DR at top of document.

---

## 12. Phase 8 — Rule candidate K16

⏳ pending P8. Draft to live in `docs/audit/rule-candidates-2026-04-08.md`. Will NOT modify `kernel.md` or `universal.md` mid-sprint.

---

## 13. Known weaknesses accepted (populated in Phase 7)

⏳ pending.

---

## Changelog of this audit doc

- 2026-04-08 — skeleton created, Phase 0 results embedded
