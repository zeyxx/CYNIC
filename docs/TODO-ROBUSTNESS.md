# CYNIC Robustness Sprint — 1, 3, 4, 5

> Historical sprint snapshot.
> Not the live protocol source and not the current source of truth for task status.
> Use `TODO.md` for active work. Keep this file only as dated context unless explicitly reactivated.

## 1. GET KERNEL RUNNING — CRITICAL

**Status:** Build passes, tests 469/469 green (need RUSTFLAGS="-C debuginfo=1")

**Action:**
- [ ] Set RUST_MIN_STACK=16777216 + RUSTFLAGS="-C debuginfo=1" in build scripts + docs
- [ ] `cargo build --release` locally
- [ ] Boot cynic-kernel service on <TAILSCALE_CORE>:3030
- [ ] Verify `curl http://localhost:3030/health` returns 200
- [ ] Verify Dogs heartbeat registered (5/5 expected)

**Falsifiable:** Service up + /health responds + Dogs all connected within 5min.

---

## 3. RESOLVE 22 DIRTY FILES — SINGLE SOURCE OF TRUTH

**Current state:**
```
Modified (11):
 M .claude/rules/workflow.md          (docs, safe)
 M TODO.md                            (session log, compress)
 M cynic-kernel.bin                   (binary artifact)
 M Cargo.toml                         (single line change)
 M src/dogs/inference.rs              (code change)
 M src/storage/mod.rs                 (code change)
 M infra/surrealdb/schema.surql       (infra, safe)
 M scripts/git-hooks/pre-commit       (hook, safe)
 M scripts/ouroboros.sh               (script, safe)
 M scripts/ouroboros_persist.py       (script, safe)
 M scripts/ouroboros_scorecard.py     (script, safe)

Untracked (11):
 ?? .claude/session-convention-caplogy.md
 ?? src/domain/stimulus.rs            (DEFER to stimulus session)
 ?? docs/research/AGENT-COMPARISON-PROTOCOL.md
 ?? cynic-kernel.bin                  (duplicate)
 ?? scripts/__pycache__/              (ignore)
 ?? scripts/coord-claim-gemini.sh
 ?? scripts/cynic-api.py
 ?? scripts/experiment_phi.py
 ?? scripts/x-consumer.sh
 ?? scripts/x-interceptor.py
 ?? Dirs: ASDFBurnTracker/, ASDev/, CultScreener/, HolDex/, asdf_grinder/, web/
```

**Action:**
- [ ] Review each M file, commit if intentional
- [ ] Add untracked to .gitignore (scripts/__pycache__/, cynic-kernel.bin, ASDFBurnTracker/, etc.)
- [ ] Defer stimulus.rs to session dédiée
- [ ] `git status --short` returns ONLY stimulus.rs

---

## 4. K15: WIRE REMAINING 6 PRODUCERS — PRODUCERS → ACTING CONSUMERS

**From organism audit, 6 violations:**

| Item | Producer | Current status | Fix |
|------|----------|---|---|
| 16 | dream-trigger.sh counter → nobody reads | Script runs, counter increments | Route to dashboard or Slack log |
| 17 | session-stop.sh compliance → displayed | Compliance printed, no action taken | Gate: fail if compliance < 0.55? |
| 18 | observe-tool.sh dev obs → CCM only chess | Observations captured, CCM ignores non-chess | Extend CCM domain intake |
| 19 | organ_quality metrics → measured, not gated | health_loop publishes, no K14 gate | Wire to /judge or alert consumer |
| 20 | session summaries → stored, never read | Stored in DB, no dashboard | Create summary dashboard or Slack post |
| 21 | event bus SSE → health-watcher not wired | /events endpoint ready, health-watcher ignores | Wire health-watcher to /events |

**Action:**
- [ ] #16: Route dream counter to Slack #cynic channel (post daily scorecard)
- [ ] #17: Add compliance gate in session-stop.sh (warn/block if < 0.55)
- [ ] #18: Extend CCM intake for dev domain (not just chess)
- [ ] #19: Wire organ_quality to /judge circuit breaker or alert
- [ ] #20: Create simple Slack summary post per session
- [ ] #21: Subscribe health-watcher to /events SSE (act on critical alerts)

---

## 5. OPERATIONAL: PEER REVIEW GATE

**Current:** Solo direct-to-main, zéro review

**Protocol:**
1. **Before commit:** all code changes → branch + local validation (cargo test, make check)
2. **Create PR:** GitHub + link in #cynic
3. **Review required:** 
   - If kernel code: Codex reviews (engineering rigor)
   - If research/design: Claude reviews (metathinking)
   - If scripts/ops: T. or maintainer reviews
4. **Approval gate:** 
   - Tests green
   - No new compiler warnings
   - No K1-K5 violations
   - Commit message explains the ONE problem solved
5. **Merge:** Squash or rebase (one logical change = one commit)

**Tools:**
- `gh pr create` (CLI)
- Branch protection on main (2 approvals for kernel/)
- Pre-commit hook to prevent direct push to main

---

## Priority Ordering

1. **Week 1:** #1 (kernel service) + #3 (dirty files) in parallel
2. **Week 2:** #4 (K15 wiring, 6 items)
3. **Week 3+:** #5 (peer review enforcement) + maintain robustness

## Success Criteria

- [ ] Kernel service running 24/7, responding to verdicts
- [ ] `git status --short` = clean (stimulus.rs only)
- [ ] All 6 producers have acting consumers
- [ ] 0 PRs created without peer review
- [ ] Compliance trending upward (track in TODO.md per session)
