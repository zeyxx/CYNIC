# CYNIC — TODO

> *Single source of truth for what needs doing. Updated every session.*
> *Strong foundation > no foundation > weak foundation.*

Last updated: 2026-04-12 | Session: body work

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
- [ ] **Crystal index contention** — 176 compaction conflicts on crystal IX:2 over 24h. Concurrent crystal writes (backfill, crystal_observer, verdict storage) create hot-index pressure. Serialize or batch to prevent SurrealKV recurrence.
- [ ] **Kernel running outside systemd** — PID present but `cynic-kernel.service` is `inactive (dead)`. Dependency chain (`Wants=surrealdb.service`) is bypassed. Run via systemd for proper lifecycle management.
- [ ] **Kairos hl_collector.py missing pyarrow import** — `import pyarrow as pa` needed. Fix before restarting kairos-snapshot and kairos-stream.

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

- Body work session (2026-04-12): 5/5 P1s fixed, 3 structural issues discovered+fixed
- SurrealDB crash audit: SurrealKV 3.0.3 key ordering bug on crystal table. Data recovered via WAL. `Restart=always` now set.
- Subagent DNA: SubagentStart hook injects axioms + φ⁻¹ + triad. health-watcher + kernel-auditor have proper frontmatter.
- Embedding pipeline: sovereign. qwen35 json_mode fix needs manual apply to backends.toml.
- Kairos: CEX ingesting, snapshot+stream stopped (pyarrow import fix needed in KAIROS repo).
- Kernel runs outside systemd — consider migrating to `systemctl --user start cynic-kernel`.
- All P1 + P2 complete. Remaining: P3 (T3a verification, coord-claim if field test) + structural debt (crystal contention, kernel outside systemd, Kairos pyarrow).
- json_mode=true applied to backends.toml for qwen35 — kernel restart needed to pick up.
- Settings.json hook additions don't live-reload — verify coord-claim if field next session.
- observe-tool.sh → type:http deferred (needs kernel /observe endpoint change).
