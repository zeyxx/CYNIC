# CYNIC — Work Graph

> Tracks are parallel. Items within a track are ordered by dependency.
> Cortex: read this for scope. Never write here — post discoveries to mempool.
> Human: curates this from mempool + vision. Only the human edits this file.

---

## TRACK: HACKATHON (deadline: May 10 23:59 PT) [tier: opus] — CLOSED

- [x] **Phase 4: Demo + Submission.** Submitted 2026-05-10. Evidence: commits 0a491066, 35ca26f0, c0637db7.

---

## TRACK: KERNEL [tier: opus]

- [ ] **MCP poison input hardening.** ~~Small models produce null/invalid tool args.~~ **STALE (2026-05-09):** 21 poison tests + null handler tests already pass. Grep shows 1 TokenData. Residual: end-to-end handler fuzzing (low priority).
- [ ] **Supply chain / config drift detection.** 21 config sources, 3 active duplications. `config-sync.sh` deployed (PR#120). Remaining: fleet.toml placeholder convention mismatch, Hermes config not linked. **Falsify:** `config-sync.sh check` in pre-push hook catches drift.
- [ ] **qwen35-9b-gpu prompt comprehension.** Scores 0.05 on all axioms for enriched token-analysis stimuli despite correct data (det-dog scores phi=0.62 on same stimulus). Not cache (PR#211 fixed). Prompt engineering or model change needed. **Falsify:** qwen35 produces varied scores (>0.15 spread) on enriched JUP stimulus.

---

## TRACK: HERMES [tier: haiku/gemini]

- [x] **Hermes crons running (2026-05-17).** All 5 timers active + success: curation, search-generator, feedback-loop, k15-consumer, gemini-briefing.
- [ ] **Unify dataset paths.** All scripts use MANIFEST.canonical_paths. → enables: clean data flow. **Falsify:** grep returns only canonical path. *Partially addressed by heuristics restructure (PR#209).*
- [x] **Deploy hermes-data-organism (2026-05-17).** Timer active, last run success.

---

## TRACK: INFRASTRUCTURE [tier: haiku]

- [x] **Soma L1 validated (2026-05-09).** Remediation config + live test: kill→circuit critical (50s)→restart (110s)→closed (130s). PR#121. Next: L2 slot awareness.
- [x] **Soma L2: slot awareness (2026-05-14).** Priority routing live. PR#153.
- [x] **GPU contention fix (2026-05-14).** Hermes priority routing. PR#160. Validated: concurrent crons + nightshift.
- [x] **`.cynic-env` format.** `config-sync.sh sync-env` derives systemd env from cynic-env (PR#120).

---

## TRACK: DEBT [tier: haiku, no deadline]

- [x] **SSOT config debt (2026-05-14).** 3-layer SSOT wired. PRs #172-175. K15 validation active.
- [ ] **Python tier governance enforcement.** 54 modules archived, 57 salvageable. Wire `make lint-python-tiers` to CI. **Falsify:** CI rejects new untiered .py file.

---

## TRACK: TOKEN INTELLIGENCE [tier: opus]

- [x] **Sovereign conviction pipeline (2026-05-16).** ρ=0.776 with CultScreener, 81.8% accuracy. PRs #204-207.
- [x] **Trajectory cron (2026-05-17).** daily_snapshot.py --trajectory classifies DYING/DECLINING/STABLE. Systemd timer 06:00 UTC. PR#209. Checkpoint: 2026-05-24 (T+7).
- [x] **Holder context enrichment (2026-05-17).** HolderContext distinguishes vesting/LP from whale. DexScreener endpoint fixed ($2K→$1.26M). Verdict cache skip for enriched domains. PRs #210-211.
- [x] **Heuristics restructure (2026-05-17).** SoC: collection/data/experiments/ with MANIFEST.yaml per experiment. PR#209.
- [ ] **Trajectory validation (checkpoint 2026-05-24).** 7 days of daily snapshots → compare day-over-day conviction. **Falsify:** MOJO doesn't stay DYING (decay > 0.30) across 7 independent snapshots.

---

## Completed (archive reference)

- [x] T1 Auth /health — PR#93: /health requires Bearer, topology exposure closed.
- [x] K18 zombie fix — PR#94: kill_on_drop(true) on all subprocess spawns.
- [x] K19 BTreeMap — PR#94: deterministic serialization for DB + hash chain.
- [x] Kernel lifecycle audit — 81K logs analyzed, 20 bg tasks catalogued.
- [x] Python lifecycle governance — 132 modules audited, tier 1/2/3 system enacted.
- [x] Phase 2: Human-Filtering Impact — 20 verdicts, 95% BARK signal.
- [x] Phase 3: Research Impact Report — 100% submissions → verdicts.
- [x] NaN filter — already guarded.
- [x] K16 docstring — /events.rs corrected.
- [x] Burn audit — -3599 lines, 27 deleted, 34 relocated (PR#89).
- [x] Agent taxonomy + DAG format (PR#92).
- [x] Multi-cortex isolation rules (PR#91).
