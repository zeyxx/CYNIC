# CYNIC — TODO

> *Single source of truth for what needs doing. Updated every session.*
> *Strong foundation > no foundation > weak foundation.*

Last updated: 2026-04-12 | Session: crystal-contention + K13 + systemd

---

## P1 — Body Sick (organs need healing)

- [x] **Embedding failures 130/130** — fixed 2026-04-12. Was stale metric from prior kernel instance. Embedding pipeline sovereign: SOVEREIGN_API_KEY in systemd env, endpoint responds, successes=1 failures=0 on live test.
- [x] **qwen35-9b-gpu JSON validity 64.7%** — diagnosed 2026-04-12. Root cause: `json_mode` not set in backends.toml. Fix: add `json_mode = true` under `[backend.qwen35-9b-gpu]`. **Manual apply needed** — verify with `/test-chess` when GPU active.

## P1 — Body Nervous System (Claude Code tooling)

- [x] **Fix subagent frontmatter** — fixed 2026-04-12. health-watcher: +mcpServers: [cynic], +permissionMode: plan. kernel-auditor: +permissionMode: plan, +isolation: worktree, +skills: [cynic-judge-framework].
- [x] **Wire SubagentStart hook** — fixed 2026-04-12. `.claude/hooks/subagent-context.sh` outputs axioms + φ⁻¹ + triad via `additionalContext`. Wired in settings.json for all subagents.
- [x] **Wire CronCreate for health-watcher** — fixed 2026-04-12. CronCreate hourly at :17. Session-scoped (7-day auto-expire). K15 closed.

## P2 — Body Upgrade (improve what works)

- [x] **UserPromptSubmit hook** — fixed 2026-04-12. `phi-reminder.sh` injects φ⁻¹ + epistemic labels via `additionalContext`. Zero model calls, ~1ms.
- [x] **Conditional hooks (if field)** — fixed 2026-04-12. Split protect-files.sh: security stays on all tools, coord claims moved to `coord-claim.sh` with `if: Edit/Write(cynic-kernel/src/**)`.
- [x] **Migrate skills to SKILL.md frontmatter** — fixed 2026-04-12. 7 commands: deploy/build/run/e2e/test-chess/status → disable-model-invocation + allowed-tools. cynic-kernel → paths. distill/empirical skipped (plugin).
- [x] **Create .claude/loop.md** — fixed 2026-04-12. Maintenance loop: health → dream debt → dog quality → delta report. context:fork + model:haiku.
- [x] **Memory pressure 92.9%** — diagnosed 2026-04-12. Was transient (44% current). Restarted embedding server (913MB swap freed). Gemma 7.7GB is expected (mmap+KV). No MemoryMax needed.

## P1 — Structural (discovered during body work session)

- [x] **SurrealDB crashed — SurrealKV key ordering bug** — fixed 2026-04-12. Restarted, data backed up. Root cause: crystal IX:2 compaction failures (176 over 24h) → 88MB memtable → key ordering violation on flush. WAL recovery succeeded.
- [x] **SurrealDB Restart=on-failure insufficient** — fixed 2026-04-12. Changed to `Restart=always`. SurrealDB exits 0 on data loss, `on-failure` doesn't catch it.
- [x] **Kairos snapshot+stream death loop** — stopped 2026-04-12. 27K+ restarts on `NameError: name 'pa' is not defined` in hl_collector.py. kairos-cex still running.
- [x] **Crystal index contention** — fixed 2026-04-12. Skip redundant embedding writes for existing crystals (already have HNSW embedding). Rate-limit backfill (50ms between writes). Expected ~70-80% reduction in HNSW mutations.
- [x] **Kernel running outside systemd** — fixed 2026-04-12. Killed bare process, deployed new binary (mv+cp), started via `systemctl --user start cynic-kernel.service`. Status: sovereign, 4/4 Dogs, contract fulfilled.
- [ ] **Kairos hl_collector.py missing pyarrow import** — `import pyarrow as pa` needed. Fix before restarting kairos-snapshot and kairos-stream.

## P1 — Proprioception (Greffe 3 — next)

- [x] **Discovery loop reads SystemContract** — done 2026-04-12 (2096e05). Compares contract vs roster each 60s cycle, emits ContractDelta event.
- [x] **Event bus emits ContractDelta + DogDiscovered** — done 2026-04-12 (2096e05). First acting consumer: PROPRIOCEPTION logs.
- [x] **Deploy + live verify** — done 2026-04-12. Log: "contract fulfilled — all 4 Dogs present". SSE: contract_delta visible.
- [ ] **organ_quality gated** — Dog below JSON validity threshold excluded from jury? Verify ParseFailureGate is sufficient or json_valid_rate needs explicit gate.
- [x] **MCP health uses contract** — fixed 2026-04-12. MCP cynic_health now uses same `system_health_assessment_with_contract` as REST /health. K13 closed.
- [ ] **Greffe 3: alerting consumer** — ContractDelta { fulfilled: false } → Slack or crystal alert. Current consumer is structured log only.

## P2 — Architecture Clarity (cosmetic, P3 priority)

- [ ] **T4: Pipeline module split** — Extract pipeline/consensus.rs + pipeline/observer.rs from pipeline/mod.rs (5 functions). Improves onboarding, zero reliability impact. Defer 1-2 sessions.

## P3 — Polish + Verify

- [x] **Replace observe-tool.sh with type: "http" hook** — DEFERRED 2026-04-12. Requires kernel-side /observe to accept raw PostToolUse hook JSON. Current async:true already non-blocking (~50ms spawn overhead).
- [ ] **T3a verification** — track incarnation metrics over 5 real decisions: organism analogies (baseline: 0), human challenges (baseline: ~0), probes before assertions. Threshold: ≥2 analogies + ≥1 challenge/session. If zero → incarnation is cosmetic.
- [ ] **Verify coord-claim if field** — settings.json hook additions don't live-reload mid-session. Test `if: "Edit(cynic-kernel/src/**)"` pattern in next session. If it doesn't match, try absolute path or `*` instead of `**`.

---

## Continuity Protocol

Each session:
1. Read this file first
2. Pick ONE P1 item (or P2 if all P1 done)
3. Scientific Protocol: diagnose → hypothesis → experiment → measure
4. Mark done with date: `- [x] **Item** — fixed 2026-04-XX (commit abc1234)`
5. Update "Last updated" header
6. If new items emerge → add with priority, don't hoard

## Context for Next Session

**SoC Audit Crystallized (2026-04-12):**
- **T1: Cancellation tokens** ✓ Implemented (637b43b+). All spawns listen to shutdown token. Measurement test needed: verify kernel exits <5s.
- **T2: Config async leak** ✓ Fixed (validate_config moved to background). Boot <100ms measurable benefit with unreachable backends.
- **T3a: Storage access** ✓ Measured. 20 direct calls not K3 violation (infrastructure CRUD ≠ pipeline logic). Acceptable pattern.
- **T3b: Unwrap/expect** ✓ Measured. 78 total; all test/build (no request-path panic). Acceptable pattern.
- **T4: Pipeline clarity** Deferred (cosmetic, low priority).

**Organic priorities (next session):**
1. Crystal contention fix: skip redundant HNSW writes + backfill rate-limited (788c6d6, 2026-04-12)
2. Kernel systemd migration (already done, 2026-04-12)
3. Measurements: shutdown latency (<5s), boot latency with unreachable backends (<100ms)
4. Kairos pyarrow fix (15min, unblock snapshot service)

- **K13 closed**: MCP + REST use same `system_health_assessment_with_contract`. Both report contract delta (f93dd9c, 2026-04-12).
- **Greffe 2 complete** (2096e05). Greffe 3 (alerting consumer for ContractDelta) → Slack alert on contract gap.
- **69 open items mapped** across 6 categories (TODO, Findings Tracker, code TODOs, arch debt, practice gaps, K15 violations). Deduplicated to ~26 distinct items across 6 tiers.
- **R22 elephant**: 1261 dev sessions → 0 dev wisdom crystals. Crystal pipeline only compounds chess. This is the compound loop's falsification test.
- json_mode=true in backends.toml for qwen35 — now active (kernel restarted).
- Kairos pyarrow still open (deprioritized, offline).
