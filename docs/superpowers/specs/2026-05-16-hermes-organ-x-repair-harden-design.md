# Hermes Organ X — Repair + Harden (Phase B)

> Unblock the K15 feedback loop and harden the executor for autonomous operation.

**Date:** 2026-05-16
**Branch:** `feat/hermes-organ-x-phase-b-<hash>`
**Scope:** Fix executor service, validate E2E loop, add retry/fallback, wire learning.

---

## Context

Hermes Agent (NousResearch, v0.13.0) is installed and configured sovereign:
- LLM: Qwen 3.5 9B via kernel `/v1` proxy → llama-server GPU
- Fallback: `<TAILSCALE_CORE>:8080` (CPU direct)
- MCP: `cynic-kernel-mcp` wired as native MCP server
- Skill: CYNIC v2.3 (17K dataset, PROTOCOL v2, BEHAVIOR.md calibrated)
- System prompt: CYNIC-aware ("You are CYNIC's sense organ on X/Twitter")

The executor (`hermes_agent_task_executor.py` v0.4.0) polls kernel `/agent-tasks`,
claims tasks, spawns `hermes chat -q <prompt>`. But:
- `hermes-agent-executor.service` is enabled but dead (never successfully started)
- K15 loop severed at execution step (tasks created, never executed)
- Soma gate coded but retry = lost task
- SKILL.md learning consumer exists but untested

## What We Fix

### 1. Repair: Executor Service

**Problem:** `hermes-agent-executor.service` is dead.

**Root causes (to verify at implementation):**
- PATH may not include `~/.local/bin` (where `hermes` lives)
- EnvironmentFile injection may be missing or misconfigured
- `hermes chat -q` never tested in subprocess mode

**Fix:**
- Verify/fix `infra/systemd/hermes-agent-executor.service` unit file
- Ensure PATH includes `~/.local/bin/hermes`
- Ensure EnvironmentFile points to `~/.config/cynic/env`
- Manual test: `hermes chat -q "What tools do you have?"` from shell
- Manual test: `python3 hermes_agent_task_executor.py --organ-dir ~/.cynic/organs/hermes/x --interval 999` (single poll)

### 2. Repair: K15 Loop E2E Validation

**The full pipeline:**
```
K15 consumer polls /observations
  -> scores with TwitterDog heuristics
  -> filters high-signal (>=3)
  -> POST /agent-tasks (creates kernel task)
  -> executor polls /agent-tasks
  -> claims task + spawns hermes chat -q <prompt>
  -> Hermes executes via tools + MCP
  -> POST /observe (new observation)
  -> Dogs judge -> verdict
  -> crystal (if HOWL/WAG)
```

**Test protocol:**
1. Inject synthetic high-signal observation via `curl POST /observe`
2. Trigger K15 consumer manually: `python3 k15_observation_consumer.py --once`
3. Verify task appears in kernel: `curl /agent-tasks?status=pending`
4. Trigger executor manually (single poll)
5. Verify Hermes runs, produces output
6. Verify new observation appears in kernel
7. Verify Dogs produce verdict on it

**Success:** One observation traverses the full pipeline. Measured, not assumed.

### 3. Harden: Soma Gate Retry

**Problem:** `consult_soma_gate()` returns `queue` when GPU saturated. Current behavior:
executor fails the task with error message. Task is lost — nobody retries.

**Fix:** On `queue` response, re-POST the task as `pending` to kernel instead of failing.
Bounded: max 3 retries with Soma-provided `wait_secs` backoff. After 3 retries, fail
with explicit "GPU saturated after 3 retries" error.

**Code change:** In `execute_task()` (line ~388), wrap the Soma gate check in a retry loop.

### 4. Harden: Fallback Verification

**Problem:** Hermes config has `fallback_providers` pointing to CPU llama-server.
Unknown if this activates automatically on GPU failure.

**Test protocol:**
1. Stop GPU llama-server (or block port)
2. Run `hermes chat -q "test"` 
3. Verify it falls back to CPU endpoint
4. If yes: zero code needed. If no: add `HERMES_BASE_URL` env override in executor.

**Decision deferred to test result.** No speculative code.

### 5. Harden: SKILL.md Learning Loop

**Problem:** `gemini_learn_from_verdicts.py` exists but untested. Uses Gemini CLI
(non-sovereign) to synthesize verdict patterns into SKILL.md updates.

**Decision:** Keep Gemini for learning synthesis. Rationale:
- Learning consumer is lightweight (runs periodically, not in hot path)
- Gemini quality for text synthesis > Qwen 9B
- Sovereignty applies to the actuator (Hermes+Qwen), not the reflector
- If Gemini unavailable, learning degrades gracefully (SKILL.md stays stale, not wrong)

**Fix:**
- Verify `gemini_learn_from_verdicts.py` runs manually
- Verify it reads verdicts from kernel (not stale local files)
- Verify SKILL.md gets updated with new patterns
- Wire to systemd timer if not already (check hermes-gemini-briefing.service)

**Falsification:** After 5 verdicts produced by the loop, SKILL.md mtime must update.

## What We Don't Touch

- UCB1 navigator (Phase C)
- Hermes-native cron as executor replacement (Phase C evaluation)
- New kernel endpoints (all exist)
- PROTOCOL.md or BEHAVIOR.md
- Hermes config.yaml (already correct)
- K15 consumer logic (already works, just needs executor downstream)

## Success Criteria

| Metric | Baseline | Target | Measurement |
|--------|----------|--------|-------------|
| Executor service status | dead | active (running) | `systemctl --user status` |
| `hermes chat -q` success rate | unknown | >70% on 10 tests | manual test log |
| K15 loop E2E | severed | 1 obs -> verdict | inject + trace |
| Soma retry on queue | task lost | task requeued (max 3) | inject saturated scenario |
| SKILL.md freshness | 2026-05-09 | updated post-verdicts | `stat` mtime |

## Falsification

- If `hermes chat -q` fails >30% of 10 test runs → Approach 1 invalid, pivot to Hermes-native cron (Approach 2)
- If K15 loop takes >10min for single observation → pipeline has unknown bottleneck, investigate before hardening
- If SKILL.md learning produces nonsense after 5 updates → Gemini synthesis inadequate, consider structured JSON instead of prose

## Phase C Gate

Phase C (Autonomy: UCB1 navigator + full organic loop) begins ONLY when:
1. Phase B success criteria met (all 5 metrics green)
2. Executor has run >24h without manual intervention
3. At least 10 observations have completed the full loop
4. SKILL.md shows measurable pattern evolution (diff between pre/post)
