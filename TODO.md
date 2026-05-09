# CYNIC — Work Graph

> Tracks are parallel. Items within a track are ordered by dependency.
> Cortex: read this for scope. Never write here — post discoveries to mempool.
> Human: curates this from mempool + vision. Only the human edits this file.

---

## TRACK: HACKATHON (deadline: May 10 23:59 PT) [tier: opus]

- [ ] **Phase 4: Demo + Submission.** Record video demo, write submission, submit. ← needs: Phase 3 results (done). Evidence: commits 0a491066, 35ca26f0, c0637db7.

---

## TRACK: KERNEL [tier: opus]

- [ ] **MCP poison input hardening.** ~~Small models produce null/invalid tool args.~~ **STALE (2026-05-09):** 21 poison tests + null handler tests already pass. Grep shows 1 TokenData. Residual: end-to-end handler fuzzing (low priority).
- [ ] **Supply chain / config drift detection.** 21 config sources, 3 active duplications. `config-sync.sh` deployed (PR#120). Remaining: fleet.toml placeholder convention mismatch, Hermes config not linked. **Falsify:** `config-sync.sh check` in pre-push hook catches drift.

---

## TRACK: HERMES [tier: haiku/gemini]

- [ ] **Hermes crons running.** Start systemd timers. → enables: data-organism deploy. **Falsify:** `systemctl list-timers --user` shows active hermes timers.
- [ ] **Unify dataset paths.** All scripts use MANIFEST.canonical_paths. → enables: clean data flow. **Falsify:** grep returns only canonical path.
- [ ] **Deploy hermes-data-organism.** `systemctl enable --now hermes-data-organism.timer`. → enables: autonomous reflection cycle. **Falsify:** 7 reflection files after 1 week.

---

## TRACK: INFRASTRUCTURE [tier: haiku]

- [x] **Soma L1 validated (2026-05-09).** Remediation config + live test: kill→circuit critical (50s)→restart (110s)→closed (130s). PR#121. Next: L2 slot awareness.
- [ ] **Soma L2: slot awareness.** Consumers (Hermes, Dogs, Colosseum) need priority/queuing on sovereign inference slots. **Falsify:** 2 concurrent consumers complete without timeout starvation.
- [ ] **GPU contention fix.** Hermes vs Dog on same llama-server. No slot awareness in any consumer. **Falsify:** Hermes cron completes with 0 MCP errors alongside nightshift.
- [x] **`.cynic-env` format.** `config-sync.sh sync-env` derives systemd env from cynic-env (PR#120).

---

## TRACK: DEBT [tier: haiku, no deadline]

- [ ] **SSOT config debt.** Config scattered (5 sources). Create `config_loader.py`. **Falsify:** single import loads all config.
- [ ] **Python tier governance enforcement.** 54 modules archived, 57 salvageable. Wire `make lint-python-tiers` to CI. **Falsify:** CI rejects new untiered .py file.

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
