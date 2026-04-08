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

### L1 CYNIC kernel — `/judge` E2E ⚠️ degraded

**Kernel version:** `v0.7.7-34-g335b87e-dirty`. Binary is 34 commits past v0.7.7 tag with dirty working tree — acceptable for D+2 dev state.

**`/health` status = "degraded"** (both unauth and auth endpoints). Full auth payload investigated — cause chain below.

**Pipeline responsive**: test governance proposition returned a valid verdict in ~1.4s (qwen-7b-hf latency 1397ms, deterministic 0ms). Judge pipeline is NOT hung.

**MCP vs REST divergence (K13 violation candidate):**
- MCP `cynic_health` reports `dog_count: 3` (deterministic + qwen-7b-hf + qwen35-9b-gpu) — gemma-4b-core **omitted**.
- REST `/health` reports `dog_count: 4`, `/dogs` returns 4 names including gemma-4b-core.
- Both show `circuit: closed, failures: 0` on every Dog.
- The MCP surface should call the same `dog_health()` function as REST. One of them is filtering differently. File candidate: `cynic-kernel/src/api/mcp/mod.rs::cynic_health` vs `cynic-kernel/src/api/rest/health.rs::health_handler`. **Not investigated further during audit — filed as post-hackathon finding.**

**Organ quality — the real story behind `degraded`:**
```json
{
  "qwen-7b-hf":     { "total_calls": 12, "json_valid_rate": 1.00, "capability_limit_rate": 0.00 },
  "qwen35-9b-gpu":  { "total_calls": 11, "json_valid_rate": 0.18, "capability_limit_rate": 0.09 },
  "gemma-4b-core":  { "total_calls":  1, "json_valid_rate": 0.00, "capability_limit_rate": 0.00 }
}
```

**qwen-7b-hf** (HuggingFace remote) is the ONLY LLM Dog producing reliably parseable output.

**qwen35-9b-gpu** (the GPU showpiece) — 82% of its responses are unparseable JSON. Circuit breaker is closed because HTTP 200 returns with garbage body. The scoring pipeline discards them as invalid, so it appears "not voting" without tripping any breaker. **This is a silent failure the circuit breaker cannot catch.** K14 implication.

**gemma-4b-core** — 1 call, 0% valid. Only probed, never used. Same class of silent failure.

**Filter test (falsification):**
- `cynic_judge({dogs: ["qwen35-9b-gpu"]})` on chess prompt → `dogs_used: "deterministic-dog"`, qwen35 contributed nothing.
- `cynic_judge({dogs: ["qwen35-9b-gpu"]})` on governance prompt → same result.
- Historical evidence: `cynic_verdicts` list shows qwen35-9b-gpu voting on chess as recently as some past session (verdicts `92cdd6a4`, `7eacf40d`). **Regression between then and now.**

**Llama-server processes are live**:
```
$ curl http://100.119.192.107:8080/health       → {"status":"ok"}
$ curl http://100.119.192.107:8080/v1/models    → Qwen3.5-9B-Q4_K_M.gguf, 8.95B params, 262K ctx
$ curl http://100.74.31.10:8080/health          → {"status":"ok"}
```
The processes are up, models are loaded. Root cause is in the prompt→response→parse chain, not infrastructure. Deep investigation = post-hackathon.

**Background tasks — 2 stale:**
- `backfill`: last success 4923s (82 min) ago → marked `stale`
- `event_consumer`: last success 3186s (53 min) ago → marked `stale`

Both contribute to the `degraded` overall status. `event_consumer` is particularly relevant: kernel events are being broadcast (`VerdictIssued` etc.) but the consumer that processes them is stale. K15 runtime violation — producer without acting consumer.

**Chain-verified:** true ✓
**Verdict cache size:** 0 ✓ (bypassed via `crystals: false` as planned)

**Audit log probe:** `cynic_audit_query` **fails with parameter type error**:
```
MCP error -32602: failed to deserialize parameters: invalid type: string "5", expected u32
```
The MCP tool schema expects u32 but the JSON layer is passing a string. Bug in `cynic_audit_query` param declaration. Workaround: not tested (query audit via REST /audit instead, deferred to post-L1).

**Actual foundation fact for hackathon:**
> The pitch says "5 independent AI validators reach consensus under mathematical doubt". Runtime truth at D+2 is **2 effective voters on any non-chess content**: `deterministic-dog` (heuristic pattern matching) + `qwen-7b-hf` (single remote HuggingFace LLM, SPOF).

This is not a build problem. It's an epistemic/pitch-integrity problem. See verdict section.

**L1 Gate:** ⚠️ **PARTIAL PASS.** Pipeline responds, verdicts store, chain verifies. But the Dogs fleet is degraded in a way the circuit breaker does not catch. **Not a kill switch for the audit (judge does respond) but a major finding for Phase 6 D4 scope and Phase 7 go/no-go.**

### L2 Dogs fleet
⏳ pending P1.2

### L3 Pinocchio program + devnet state ✅

**Gate PASSED** — on-chain substrate for the "CYNIC × ASDelegate dual-layer" pitch is alive.

Probed via JSON-RPC `getAccountInfo` (not `solana program show` because `solana-cli 3.1.12` refuses read-only ops without a default signer — `/tmp/gasdf-deployer.json` is still the config default and still gone). No keypair needed for RPC read path.

**Program `A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx`:**
```json
{ "executable": true,
  "owner": "BPFLoaderUpgradeab1e11111111111111111111111",
  "lamports": 1141440,
  "data_len": 48,
  "slot": 454018276 }
```
`data_len: 48` = metadata pointing to ProgramData account (normal for upgradeable BPF). Program is still deployed, still executable, still owned by the upgradeable loader.

**Community PDA `8Pyd1hqd6jTX2jR8YvCAjnd3cyP5qB7XaxzwAGtHCSFD`:**
```json
{ "executable": false,
  "owner": "A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx",
  "lamports": 1656480,
  "data_len": 110 }
```

**Critical**: the PDA is owned by the Pinocchio program itself → it was created via `initialize_community` and is still valid. This is the on-chain proof-point the pitch needs.

**Data structure analysis (hex decode, 110 bytes):**
- Bytes 0-1: discriminator `0x00 0x01` (version?)
- Bytes 2-33: mint pubkey (32)
- Bytes 34-65: agent_key (32) — starts with `3b 40 cd 49 55 d5 69 43 28 29 94 c6 ce d1 b4 15 …`
- Bytes 66-97: guardian_key (32) — starts with `c0 ae cd 49 55 d5 69 43 28 29 94 c6 ce d1 b4 15 …`
- Bytes 98-109: trailing 12 bytes (threshold + paused + bump + counter, layout TBD from community.rs)

**Operational finding: agent_key == guardian_key**. Bytes 36-65 and 68-97 are byte-identical for 30 consecutive bytes — not a coincidence at that length. The community was initialized with the same keypair for both the agent (signs verdicts) and the guardian (can pause). When `/tmp/gasdf-deployer.json` was lost, **both** roles were lost — there is no current "asymmetric pause safety" in this community. The pitch's "pause: guardian OR agent; unpause: guardian ONLY (asymmetric)" claim is architecturally supported by the program but vacuous in the current community instance. Post-hackathon: either re-initialize community with distinct agent + guardian keys, or document the single-key test-config as a known limitation in the demo.

**L3 gate summary:**
- ✓ Program still deployed, still executable
- ✓ Community PDA still alive, still owned by program
- ✓ Devnet substrate enables "fork ASDelegate + CYNIC Pinocchio as tally authority" pitch
- ⚠️ agent == guardian in current PDA (single keypair) — non-blocking for demo, fix post-hackathon
- ⚠️ `/tmp/gasdf-deployer.json` still the CLI default — L4 keypair regen still needed before any devnet write operation

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
