# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-26 18:30 | Debt audit: 14 false closures found, Tier-1 fixed (F1 crystal_to_json, F3 wallet-judgment embed, gates added)

---

## HACKATHON (registration May 4, submission May 11)

- [x] **submit_verdict on-chain.** `scripts/submit-verdict.ts` ships. Community PDA `8DVUKmJa…` hardcoded. Devnet tx claimed but no committed proof artifact.
- [x] **Rust 1.95.0 upgrade.** Active (`rustc 1.95.0`). LLVM SROA bug from 1.94.1 resolved.
- [ ] **Deterministic-dog forced consensus fix.** Claimed in d0dd481 (squash-merged, hash gone). No test asserts absence of forced consensus. Untraceable. **Falsify:** regression test + `git log -p -S "forced_consensus"` identifies original code.
- [ ] **Wallet-judgment Dogs (deterministic).** Implement in cynic-kernel/src/domain/dog/ per pseudocode in cynic-kernel/domains/wallet-judgment-dogs.md. Gates: games≥5, replay_risk, suspicious_cluster. Register in Judge.
- [ ] **Colosseum full submission.** Project created on arena.colosseum.org. Need: description longue, video demo (3min), GitHub link, deployed URL. Deadline: May 10 23:59 PDT. Note: 123-line skeleton was never committed — only 60-line draft exists.
- [x] **B&C integration analysis + wallet-judgment domain.** Domain files created. Wallet-judgment now embedded in binary. Enrichment utility exists. Integration spec exists.
- [x] **Personality card stimulus + integration stub.** 7/7 tests passing. No external caller yet — stub only.
- [ ] **Video demo.** **Falsify:** 2-3 min narration + kernel logs visible, q_score + dog_scores visible.
- [ ] **Vercel UI → kernel API path.** Cloudflare tunnel dead. VITE_API_BASE may point to defunct URL. **Falsify:** browser console shows /judge returning 200.

## HERMES X ORGAN

- [x] **Organ X infrastructure.** 3 systemd services active. Dataset: 1772 tweets. Note: kernel reports x-proxy organ silent >24h — heartbeat link gap.
- [x] **Enrichment in x_proxy.py.** Signal score, author tier, coordination detection. No Python test coverage (P2 violation).
- [x] **SOUL.md updated.** Hermes = CYNIC citizen, not X-only.
- [ ] **Hermes cron missions.** **Falsify:** `hermes cron list` shows ≥1 active job.

## ORGANISM (no deadline, compound value)

- [ ] **CCM volume → crystallization.** CCM loop_active=false. **Falsify:** observation count grows → forming crystals appear.
- [ ] **Auth /health (T1/O4).** /metrics + /events require auth in code. **Remaining:** deploy + verify. **Falsify:** `curl funnel/metrics` → 401.
- [ ] **K17 ReconnectableStorage forwards.** lint-drift gate added, caught 4 missing agent_task methods. Fix reconnectable.rs. **Falsify:** `make lint-drift` passes.

## DEBT (fix when touching adjacent code)

- [ ] NaN filter in judge/math.rs (trimmed_mean lets NaN through)
- [ ] Two TokenData structs (enrichment.rs vs stimulus.rs)
- [ ] LUKS full-disk encryption on cynic-core (KC1)
- [ ] `.cynic-env` format — `export` prefixes incompatible with systemd EnvironmentFile
- [x] mitmdump running with `--listen-host 127.0.0.1` (KC4)
