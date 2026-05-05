# CYNIC — Work Graph

> Tracks are parallel. Items within a track are ordered by dependency.
> Cortex: read this for scope. Never write here — post discoveries to mempool.
> Human: curates this from mempool + vision. Only the human edits this file.

---

## TRACK: HACKATHON (deadline: May 10 23:59 PT) [tier: opus]

- [ ] **Phase 4: Demo + Submission.** Record video demo, write submission, submit. ← needs: Phase 3 results (done). Evidence: commits 0a491066, 35ca26f0, c0637db7.

---

## TRACK: KERNEL [tier: opus]

- [ ] **MCP poison input hardening.** Small models produce null/invalid tool args. → enables: safe multi-model dispatch. **Falsify:** garbage args to all 22 MCP tools → all return error, none crash.
- [ ] **Two TokenData structs.** `enrichment.rs` vs `stimulus.rs` → merge. **Falsify:** grep returns 1 TokenData definition.

---

## TRACK: HERMES [tier: haiku/gemini]

- [ ] **Hermes crons running.** Start systemd timers. → enables: data-organism deploy. **Falsify:** `systemctl list-timers --user` shows active hermes timers.
- [ ] **Unify dataset paths.** All scripts use MANIFEST.canonical_paths. → enables: clean data flow. **Falsify:** grep returns only canonical path.
- [ ] **Deploy hermes-data-organism.** `systemctl enable --now hermes-data-organism.timer`. → enables: autonomous reflection cycle. **Falsify:** 7 reflection files after 1 week.

---

## TRACK: INFRASTRUCTURE [tier: haiku]

- [ ] **Soma config activation.** Populate `[backend.NAME.remediation]` in backends.toml. Uncomment nightshift with compute budget gate. **Falsify:** kill llama-server → circuit opens <30s → restart <120s → circuit closes.
- [ ] **GPU contention fix.** Hermes vs Dog on same llama-server. **Falsify:** Hermes cron completes with 0 MCP errors alongside nightshift.
- [ ] **`.cynic-env` format.** Remove `export` prefixes for systemd EnvironmentFile compatibility. **Falsify:** same file works for both `source` and systemd.

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
