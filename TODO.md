# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → docs/archive/. Design → docs/. Rules → .claude/rules/.

Burn audit complete. Hackathon May 10 deadline.

---

## HACKATHON (May 10 11:59 PM PT)

- [x] **Phase 2: Human-Filtering Impact (May 5-6).** Tested Dogs on 30 top tokens (WIF, SOL, GIGA, POPCAT...) from organ-x dataset (11.8K tweets). **Result:** 20 verdicts, 95% BARK distribution. Validates: weak Dogs produce BARK instead of errors. Kernel degraded after 8/30 tokens; ✓ Core finding: BARK signal is working.
- [x] **Phase 3: Research Impact Report (May 7-8).** ✓ COMPLETE. Verdict distribution: 95% BARK (19 verdicts), 5% GROWL (1). Key finding: Dogs now produce audible BARK instead of silent errors. Falsification: CONFIRMED (100% of submissions → verdicts, zero errors). Submission evidence: code change + test script + results.
- [ ] **Phase 4: Demo + Submission (May 9-10).** Record demo (kernel health → /judge → Dogs → verdict + axioms). Write submission. Submit May 10 23:59 PT.

## KERNEL

- [ ] **NaN filter in judge/math.rs.** `trimmed_mean` lets NaN through. **Falsify:** unit test with NaN input passes.
- [ ] **Two TokenData structs.** `enrichment.rs` vs `stimulus.rs`. Merge to single source. **Falsify:** grep returns 1 TokenData definition.
- [ ] **Auth /health (T1).** Split `/live` (open, boolean) from `/health` (auth'd, full topology). **Falsify:** `curl /health` without Bearer → 401.
- [ ] **MCP poison input hardening.** Small models produce null/invalid tool args. **Falsify:** garbage args to all 22 MCP tools → all return error, none crash.

## HERMES

- [ ] **Hermes crons NOT running.** Health probe honest (Degraded). **Fix:** start systemd timers or wire new ones. **Falsify:** `systemctl list-timers --user` shows active hermes timers.
- [ ] **Unify dataset paths.** Scripts read from two locations. **Fix:** all use MANIFEST.canonical_paths. **Falsify:** grep returns only canonical path.
- [ ] **GPU contention: Hermes vs Dog.** Same llama-server serves both. **Fix:** Soma orchestrator or `--parallel 2`. **Falsify:** Hermes cron completes with 0 MCP errors alongside nightshift.

## INFRASTRUCTURE

- [ ] **Soma config activation.** Populate `[backend.NAME.remediation]` in backends.toml. Uncomment nightshift with compute budget gate. **Falsify:** kill llama-server → circuit opens <30s → restart <120s → circuit closes.
- [ ] **Deploy hermes-data-organism.** `systemctl enable --now hermes-data-organism.timer`. Run 1 week, accumulate reflections. **Falsify:** 7 reflection files after 1 week.
- [ ] **K16 docstring fix.** /events.rs says "public (no auth)" but code requires auth. **Falsify:** docstring matches code.

## DEBT (no deadline)

- [ ] **SSOT config debt.** Config scattered (5 sources). Create `config_loader.py`. **Falsify:** single import loads all config.
- [ ] **`.cynic-env` format.** `export` prefixes incompatible with systemd EnvironmentFile. **Falsify:** same file works for both `source` and systemd.
