# CYNIC — Work Graph (Updated 2026-05-29)

> Tracks are sequential (dependencies enforced).
> Cortex: read for scope. Never write — post discoveries to mempool.
> Human: curates from mempool + vision. Only human edits this file.
> Dates are absolute (YYYY-MM-DD). Falsification tests are pre-written.

---

## NOW (2026-05-29)

**Phase 2.0: K15 Closure — Outcome Measurement (T+7)**
- **Due: 2026-06-02** (4 days from now)
- **What**: Compare baseline verdicts (2026-05-25) vs enriched verdicts (today) on 45 tokens
- **Why**: Proves token enrichment + token_postprocessor wiring are effective (not dead code)
- **Owner**: Auto-dispatch cron (outcome_measurement_t7.sh), human validates
- **Blocker**: None (baseline + enriched verdicts exist as of today)
- **Falsify**: If measurement runs and divergence ≤ 0.05 on Q-score, enrichment is noise
- **Success**: Divergence report generated, feed to calibration phase
- **Action items**:
  - [ ] Write `scripts/outcome_measurement_t7.sh` — compares baseline vs enriched on 45 tokens, outputs divergence.csv
  - [ ] Register cron: systemd timer fires 2026-06-02 06:00 UTC
  - [ ] Pre-test: script runs on historical data, outputs sample report

**Governance: Session Continuity (Parallel)**
- **Due: 2026-05-31** (2 days from now)
- **What**: Make TODO.md reliable + auto-trigger next phase when current completes
- **Why**: Token/wallet pipeline spinning because next work not queued
- **Owner**: Human (design) + systemd (execution)
- **Blocker**: None
- **Falsify**: If session ends and next phase not auto-posted, governance still broken
- **Success**: Post-session hook auto-posts outcome measurement to `/observe domain=mempool`
- **Action items**:
  - [ ] Add to `session-stop.sh`: POST outcome measurement task to mempool if phase complete
  - [ ] Test: end session, verify mempool POST fires
  - [ ] Verify: next session reads mempool, sees measurement task queued

---

## IMMEDIATE (2026-06-02 to 2026-06-09)

**Phase 2.1: Calibration Loop (Depends on Outcome Measurement)**
- **Due: 2026-06-09**
- **What**: Retrain Dogs on outcome feedback if enrichment divergence > threshold (0.10)
- **Why**: Closes the K15 loop — verdicts feed back as training signal
- **Owner**: Human (curation) + Dogs (training)
- **Blocker**: Phase 2.0 measurement complete
- **Falsify**: If Dogs still disagree equally (max_disagreement unchanged) after calibration
- **Success**: BURN and VERIFY scores improve on enriched domains (calibration signal visible)
- **Action items**: TBD (depends on Phase 2.0 report)

---

## DEFERRED (Next Phase, Depends on Calibration)

**Phase 3: Convergence Loop (K15 + Verdict Feedback)**
- **Due: 2026-06-16**
- **What**: Wire convergence_trigger → observations → dog retrain cycle
- **Why**: Closes CCM loop — verdicts inform future judgments
- **Owner**: TBD (depends on Phase 2 results)
- **Blocker**: Phase 2.1 calibration complete
- **Falsify**: If convergence observations produced but no consumer acts on them
- **Success**: Deterministic + LLM Dogs show ρ > 0.70 agreement on repeat stimuli
- **Action items**: TBD (defer until Phase 2.0 report)

---

## PARALLEL: Governance & Infrastructure

**Governance: TODO.md Enforcement**
- **Due: 2026-05-31**
- **What**: Add pre-push hook that validates TODO dates and falsification tests
- **Why**: Prevent stale items, catch spinning patterns early
- **Owner**: systemd hook
- **Blocker**: None
- **Falsify**: If TODO item is 10+ days old without status update, hook warns
- **Success**: Hook runs on every `git push`, validates structure
- **Action items**:
  - [ ] Add to `.claude/hooks/pre-push`: TODO date validation
  - [ ] Format: `- [ ] **Item.** Due: YYYY-MM-DD. Falsify: [test]`

**Infrastructure: Debt Reduction**
- **Stale** (moved to archive, see below)

---

## Completed (Archive Reference)

### TRACK: HACKATHON (CLOSED 2026-05-10)
- [x] Phase 4: Demo + Submission (Evidence: commits 0a491066, 35ca26f0, c0637db7)

### TRACK: KERNEL
- [x] **Nightshift K15 violation** (2026-05-29). Disabled loop due to orphan dev domain crystals. Memory: 166M→19.9M. PR#270.
- [x] **Token enrichment pipeline** (2026-05-29). token_postprocessor wired into judge. Class-aware caps live. PR#271.
- [ ] **MCP poison input hardening** (stale since 2026-05-09). 21 tests pass. Low priority.
- [ ] **Supply chain config drift** (ongoing). config-sync.sh deployed. fleet.toml convention mismatch remains.
- [ ] **qwen35 axiom scoring** (blocker). qwen35: 0.05 on all axioms despite enriched data. det-dog scores 0.62 on same. Prompt engineering needed.

### TRACK: HERMES
- [x] **Hermes crons** (2026-05-17). All 5 timers: curation, search-generator, feedback-loop, k15-consumer, gemini-briefing.
- [x] **Hermes data organism** (2026-05-17). Timer active.
- [ ] **Dataset path unification** (partial). MANIFEST.canonical_paths in use. Remaining: all scripts canonical.

### TRACK: INFRASTRUCTURE
- [x] **Soma L1 validated** (2026-05-09). Circuit control: critical 50s → restart 110s → closed 130s. PR#121.
- [x] **Soma L2 slot awareness** (2026-05-14). Priority routing live. PR#153.
- [x] **GPU contention fix** (2026-05-14). Hermes priority routing. PR#160.
- [x] **`.cynic-env` format** (2026-05-20). config-sync.sh derives systemd env. PR#120.

### TRACK: TOKEN INTELLIGENCE
- [x] **Sovereign conviction pipeline** (2026-05-16). ρ=0.776, 81.8% accuracy. PRs #204-207.
- [x] **Trajectory cron** (2026-05-17). daily_snapshot.py classifies DYING/DECLINING/STABLE. Checkpoint T+7: 2026-05-24.
- [x] **Holder context enrichment** (2026-05-17). HolderContext wired. DexScreener fixed. PRs #210-211.
- [x] **Heuristics restructure** (2026-05-17). SoC: collection/data/experiments/ with MANIFEST. PR#209.
- [ ] **Trajectory validation** (checkpoint 2026-05-24). 7-day snapshots → compare DYING decay. **Falsify**: MOJO doesn't stay DYING (decay ≤ 0.30).

### TRACK: DEBT
- [x] **SSOT config debt** (2026-05-14). 3-layer SSOT wired. PRs #172-175.
- [ ] **Python tier governance** (ongoing). 54 archived, 57 salvageable. Wire `make lint-python-tiers` to CI.

---

## Session Notes

**2026-05-29 (This Session)**
- Disabled nightshift (K15 violation): memory 166M→19.9M, Dogs responsive
- Wired token_postprocessor into judge pipeline: class-aware caps live for token-analysis domain
- Deployed kernel with enrichment wiring
- **Identified**: K15 measurement due 2026-06-02, governance loop broken
- **Action**: TODO rewrite + outcome_measurement_t7.sh + session continuity hook

**Next Session (2026-05-30 or later)**
- Implement outcome_measurement_t7.sh (if 2026-05-30)
- OR wait for 2026-06-02, run measurement, post results to mempool
- Governance hook implementation (session-stop.sh → mempool POST)
