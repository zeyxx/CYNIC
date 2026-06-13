# Hermes Organ X Phase B — Repair + Harden

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unblock the K15 feedback loop so Hermes Agent executes tasks autonomously and learns from verdicts.

**Architecture:** Python executor (`hermes_agent_task_executor.py`) polls kernel `/agent-tasks`, spawns `hermes chat -q` (NousResearch Hermes v0.13.0, Qwen 3.5 9B via sovereign llama-server). Soma gate prevents GPU starvation. `gemini_learn_from_verdicts.py` (pure Python pattern extractor — no LLM call despite the name) closes the compound learning loop.

**Tech Stack:** Python 3.11, systemd user services, Hermes Agent CLI, CYNIC kernel REST API

**Spec:** `docs/superpowers/specs/2026-05-16-hermes-organ-x-repair-harden-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `infra/systemd/hermes-agent-executor.service` | Modify | Fix PATH, WantedBy |
| `infra/systemd/hermes-feedback-loop.service` | Modify | Fix ordering cycle |
| `scripts/hermes-x/core/hermes_agent_task_executor.py` | Modify | Soma retry loop, PATH hardening |
| `scripts/hermes-x/tests/test_executor_smoke.sh` | Create | E2E smoke test script |

---

## Task 1: Fix hermes-agent-executor.service

Three bugs identified (observed):
1. No PATH override — `hermes` lives at `~/.local/bin/hermes`, not in systemd default PATH
2. `WantedBy=multi-user.target` — wrong for user-level service (SYS1 rule), should be `default.target`
3. Never started — no journal entries ever

**Files:**
- Modify: `infra/systemd/hermes-agent-executor.service`

- [ ] **Step 1: Read current service file and confirm bugs**

```bash
cat infra/systemd/hermes-agent-executor.service
# Verify: no PATH= or Environment=PATH in [Service]
# Verify: WantedBy=multi-user.target (wrong for user service)
```

- [ ] **Step 2: Fix the service file**

Add `Environment=PATH=...` with `~/.local/bin` prepended, fix WantedBy:

```ini
[Unit]
Description=CYNIC Hermes Agent Executor — autonomous meta-agent task runner
After=cynic-kernel.service hermes-x-ingest.service
Wants=hermes-x-ingest.service

[Service]
Type=simple
EnvironmentFile=%h/.config/cynic/env
Environment=PATH=%h/.local/bin:/usr/local/bin:/usr/bin:/bin
ExecStart=/usr/bin/python3 %h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/hermes-x/core/hermes_agent_task_executor.py --organ-dir %h/.cynic/organs/hermes/x
Restart=on-failure
RestartSec=30s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

- [ ] **Step 3: Deploy and validate (SYS3)**

```bash
cp infra/systemd/hermes-agent-executor.service ~/.config/systemd/user/hermes-agent-executor.service
systemctl --user daemon-reload
systemctl --user restart hermes-agent-executor
systemctl --user status hermes-agent-executor
journalctl --user -eu hermes-agent-executor --no-pager | tail -20
```

Expected: service Active (running), journal shows "hermes_agent_task_executor v0.4.0 starting".

- [ ] **Step 4: Commit**

```bash
git add infra/systemd/hermes-agent-executor.service
git commit -m "fix(hermes): add PATH to executor service, fix WantedBy for user-level"
```

---

## Task 2: Test hermes chat -q manually

Before trusting the executor subprocess, verify `hermes chat -q` works standalone.

**Files:** None (manual test)

- [ ] **Step 1: Simple tool-less test**

```bash
hermes chat -q "What is 2+2? Answer in one word." --quiet 2>&1 | head -20
```

Expected: Hermes responds via Qwen 9B. Note response time.

- [ ] **Step 2: MCP tool test**

```bash
hermes chat -q "Use cynic_health to check the kernel status. Report the result." --quiet 2>&1 | head -40
```

Expected: Hermes calls `cynic_health` MCP tool, returns kernel status.

- [ ] **Step 3: Browser tool test (if available)**

```bash
hermes chat -q "List your available tools. Do you have browser_navigate?" --quiet 2>&1 | head -30
```

Expected: Lists tools including browser and MCP tools. This determines what the agent can do.

- [ ] **Step 4: Record results**

Note in commit message: success rate, response time, tools available. This is the baseline for the >70% success criterion.

---

## Task 3: Soma gate retry loop

Current behavior: on GPU saturation (`queue` response), task fails and is lost.
Fix: retry with backoff, max 3 attempts. On exhaustion, re-POST task as pending.

**Files:**
- Modify: `scripts/hermes-x/core/hermes_agent_task_executor.py:331-428`

- [ ] **Step 1: Write the retry logic**

Replace the single Soma gate check in `execute_task()` with a bounded retry:

```python
def execute_task(task: dict, organ_dir: str) -> tuple[str | None, str | None]:
    """Execute task via `hermes chat` subprocess."""
    task_id = task.get("id", "?")
    objective = task.get("objective", "")
    actions = task.get("actions", [])
    domain = task.get("domain", "unknown")

    logger.info("executing task %s: domain=%s objective=%s", task_id, domain, objective[:60])

    # Soma gate with bounded retry
    max_soma_retries = 3
    for attempt in range(max_soma_retries):
        gate_decision = consult_soma_gate(task_id)
        if gate_decision.get("decision") != "queue":
            break
        wait_secs = gate_decision.get("data", {}).get("wait_secs", 5)
        if attempt < max_soma_retries - 1:
            logger.warning(
                "GPU saturated for task %s (attempt %d/%d), waiting %ds",
                task_id, attempt + 1, max_soma_retries, wait_secs,
            )
            time.sleep(wait_secs)
        else:
            logger.error(
                "GPU saturated after %d retries for task %s — requeuing",
                max_soma_retries, task_id,
            )
            _requeue_task(task)
            return None, f"GPU saturated after {max_soma_retries} retries — task requeued"

    # ... rest of execute_task unchanged (prompt building + subprocess) ...
    # NOTE: Fix pre-existing bug while editing this function:
    # Line ~422 says "timed out (5 min)" but timeout is 600s (10 min). Fix to "10 min".
```

- [ ] **Step 2: Add requeue helper**

**IMPORTANT:** The kernel's `POST /agent-tasks/{id}/result` only accepts `result` and `error` fields.
Sending `error` marks the task `"failed"` (hardcoded in kernel). There is no way to set status back
to `"pending"` via this endpoint. Instead, create a NEW task with the same content via `POST /agent-tasks`.

Add after `consult_soma_gate()`:

```python
def _requeue_task(task: dict):
    """Clone and re-POST task to kernel as a new pending task.

    The kernel's /agent-tasks/{id}/result endpoint hardcodes status='failed' when error is set.
    There is no way to reset status to 'pending'. Instead, we create a fresh task with the
    same content. The original task is left in 'processing' state (will be cleaned up by TTL).
    """
    if not KERNEL_ADDR or not KERNEL_API_KEY:
        logger.warning("cannot requeue task: kernel not configured")
        return
    try:
        headers = {
            "Authorization": f"Bearer {KERNEL_API_KEY}",
            "Content-Type": "application/json",
        }
        # Clone essential fields for a new task
        payload = {
            "kind": task.get("kind", "hermes"),
            "domain": task.get("domain", "unknown"),
            "content": task.get("content", task.get("objective", "")),
        }
        url = f"{KERNEL_ADDR}/agent-tasks"
        response = requests.post(url, json=payload, headers=headers, timeout=5)
        if response.status_code in (200, 201):
            logger.info("requeued task as new pending (original: %s)", task.get("id", "?"))
        else:
            logger.warning("requeue failed: HTTP %d", response.status_code)
    except requests.RequestException as e:
        logger.warning("requeue failed: %s", e)
```

- [ ] **Step 3: Verify existing tests still pass**

```bash
cd scripts/hermes-x && python3 -m pytest tests/ -v 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add scripts/hermes-x/core/hermes_agent_task_executor.py
git commit -m "fix(hermes): bounded Soma gate retry + requeue on GPU saturation"
```

---

## Task 4: Fix hermes-feedback-loop.timer ordering cycle

The timer is dead due to systemd ordering cycle (observed in journal: "Found ordering cycle on hermes-feedback-loop.timer/start").

**Files:**
- Modify: `infra/systemd/hermes-feedback-loop.service`

- [ ] **Step 1: Diagnose the cycle**

```bash
journalctl --user -eu hermes-feedback-loop.timer --no-pager | grep -i cycle
systemctl --user cat hermes-feedback-loop.timer
```

The cycle is likely: timer → service → After=cynic-kernel → wants timers.target → timer.

- [ ] **Step 2: Fix the service — remove unnecessary After dependency**

The feedback loop is a oneshot that reads local files + calls Gemini. It does NOT need
`After=cynic-kernel.service` (it doesn't call the kernel). Remove:

```ini
[Unit]
Description=CYNIC Hermes Feedback Loop — ingests verdicts and updates domain wisdom (1h cron)
After=hermes-x-ingest.service
PartOf=hermes-feedback-loop.timer

[Service]
Type=oneshot
EnvironmentFile=%h/.config/cynic/env
Environment=X_ORGAN_DIR=%h/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 %h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/hermes-x/analysis/gemini_learn_from_verdicts.py --organ-dir %h/.cynic/organs/hermes/x
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=hermes-feedback-loop.timer
```

- [ ] **Step 3: Deploy and validate (SYS3)**

```bash
cp infra/systemd/hermes-feedback-loop.service ~/.config/systemd/user/hermes-feedback-loop.service
systemctl --user daemon-reload
systemctl --user start hermes-feedback-loop.timer
systemctl --user status hermes-feedback-loop.timer
```

Expected: timer Active (waiting), no ordering cycle in journal.

- [ ] **Step 4: Test the learning consumer manually (falsifiable)**

Note: `gemini_learn_from_verdicts.py` is a pure Python pattern extractor (no LLM call
despite the name). It reads `observation-verdicts/*.json` and writes Markdown tables.

```bash
# Record mtime BEFORE
BEFORE=$(stat -c %Y ~/.cynic/organs/hermes/x/SKILL.md)

# Run the consumer
python3 scripts/hermes-x/analysis/gemini_learn_from_verdicts.py \
  --organ-dir ~/.cynic/organs/hermes/x 2>&1

# Verify mtime changed (falsifiable)
AFTER=$(stat -c %Y ~/.cynic/organs/hermes/x/SKILL.md)
[ "$AFTER" -gt "$BEFORE" ] && echo "LEARNING WIRED" || echo "FAIL: SKILL.md not updated"
```

Expected: "analyzing N observation verdict(s)" (N = number of files in observation-verdicts/)
followed by "SKILL.md updated" and "LEARNING WIRED".

- [ ] **Step 6: Commit**

```bash
git add infra/systemd/hermes-feedback-loop.service
git commit -m "fix(hermes): break ordering cycle in feedback-loop timer"
```

---

## Task 5: E2E Smoke Test

Inject a synthetic observation and trace it through the full K15 pipeline.

**Files:**
- Create: `scripts/hermes-x/tests/test_executor_smoke.sh`

- [ ] **Step 1: Write the smoke test script**

```bash
#!/usr/bin/env bash
# Hermes Organ X — K15 E2E Smoke Test
# Injects a synthetic observation, verifies it traverses the full pipeline.
#
# Prerequisites:
#   - cynic-kernel running (curl /health returns 200)
#   - hermes-agent-executor running or invoked manually
#   - CYNIC_REST_ADDR and CYNIC_API_KEY set
#
# Usage: bash test_executor_smoke.sh

set -euo pipefail

KERNEL="${CYNIC_REST_ADDR:?CYNIC_REST_ADDR not set}"
API_KEY="${CYNIC_API_KEY:?CYNIC_API_KEY not set}"
AUTH="Authorization: Bearer ${API_KEY}"

# Ensure http:// prefix
[[ "$KERNEL" == http* ]] || KERNEL="http://${KERNEL}"

echo "=== Step 1: Inject synthetic high-signal observation ==="
OBS_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${KERNEL}/observe" \
  -H "${AUTH}" -H "Content-Type: application/json" \
  -d '{
    "tool": "smoke_test",
    "target": "SMOKE_TEST_TOKEN_'$(date +%s)'",
    "domain": "twitter",
    "context": "E2E smoke test: synthetic high-signal observation for K15 pipeline validation",
    "agent_id": "smoke-test",
    "tags": ["smoke-test", "high-signal"]
  }')
HTTP_CODE=$(echo "$OBS_RESPONSE" | tail -1)
echo "POST /observe: HTTP ${HTTP_CODE}"
if [ "$HTTP_CODE" != "200" ] && [ "$HTTP_CODE" != "201" ]; then
  echo "FAIL: Could not inject observation"
  exit 1
fi
echo "OK: Observation injected"

echo ""
echo "=== Step 2: Check pending agent tasks ==="
TASKS=$(curl -s -H "${AUTH}" "${KERNEL}/agent-tasks?status=pending")
echo "Pending tasks: ${TASKS}"

echo ""
echo "=== Step 3: Verify executor service is running ==="
systemctl --user is-active hermes-agent-executor.service && echo "OK: Executor running" || echo "WARN: Executor not running — run manually for test"

echo ""
echo "=== Step 4: Check recent verdicts ==="
# Give pipeline time if needed
echo "Waiting 10s for pipeline processing..."
sleep 10
RECENT_OBS=$(curl -s -H "${AUTH}" "${KERNEL}/observations?domain=twitter&limit=3")
echo "Recent observations: ${RECENT_OBS}" | head -5

echo ""
echo "=== Smoke test complete ==="
echo "Manual verification needed:"
echo "  1. Check executor logs: journalctl --user -eu hermes-agent-executor | tail -20"
echo "  2. Check K15 consumer: journalctl --user -eu hermes-k15-consumer | tail -20"
echo "  3. Check verdicts: curl -s -H '${AUTH}' '${KERNEL}/verdicts?limit=3'"
```

- [ ] **Step 2: Make executable and run**

```bash
chmod +x scripts/hermes-x/tests/test_executor_smoke.sh
source ~/.cynic-env && bash scripts/hermes-x/tests/test_executor_smoke.sh
```

- [ ] **Step 3: Trigger executor manually (deterministic single poll)**

The executor has no `--once` flag. Use `--interval 999` with `timeout` to get one poll cycle:

```bash
source ~/.cynic-env && timeout 120 python3 \
  scripts/hermes-x/core/hermes_agent_task_executor.py \
  --organ-dir ~/.cynic/organs/hermes/x --interval 999 2>&1 | tail -30
```

The executor polls once, then sleeps 999s. `timeout 120` kills it after the first poll
completes (allowing up to 2min for `hermes chat` execution). Check output for
"executing task" or "found 0 pending task(s)".

- [ ] **Step 4: Verify full loop (manual)**

After smoke test, check each pipeline stage:
```bash
# Executor picked up task?
journalctl --user -eu hermes-agent-executor | tail -20

# K15 consumer scored it?
journalctl --user -eu hermes-k15-consumer | tail -20

# Hermes produced output?
journalctl --user -eu hermes-agent-executor | grep "completed\|failed"
```

- [ ] **Step 5: Commit**

```bash
git add scripts/hermes-x/tests/test_executor_smoke.sh
git commit -m "test(hermes): K15 E2E smoke test script"
```

---

## Task 6: Fallback verification (test-driven, no speculative code)

**Files:** None unless test fails

- [ ] **Step 1: Simulate GPU failure**

```bash
# Check current model endpoint
hermes config show 2>&1 | grep -A3 "Model"

# Test with GPU endpoint (should work)
hermes chat -q "Say OK" --quiet 2>&1 | head -5

# Test fallback: temporarily block GPU port (if safe to do)
# OR: check if hermes handles connection timeout gracefully
timeout 5 hermes chat -q "Say OK" --quiet 2>&1 | head -10
```

- [ ] **Step 2: Decision point**

If Hermes handles fallback natively → no code change needed, document in commit.
If not → add `Environment=HERMES_BASE_URL=...` override logic in executor. Write that code then.

- [ ] **Step 3: Commit decision**

```bash
git commit --allow-empty -m "docs(hermes): fallback verification — [RESULT]"
```

---

## Task 7: Final validation — all success criteria

- [ ] **Step 1: Check all 5 metrics**

```bash
echo "=== 1. Executor service ==="
systemctl --user is-active hermes-agent-executor.service

echo "=== 2. hermes chat success rate ==="
echo "(recorded in Task 2 — check commit message)"

echo "=== 3. K15 E2E ==="
echo "(recorded in Task 5 — check smoke test output)"

echo "=== 4. Soma retry ==="
echo "(code change in Task 3 — review diff)"

echo "=== 5. SKILL.md freshness ==="
stat ~/.cynic/organs/hermes/x/SKILL.md | grep Modify
```

- [ ] **Step 2: Commit all remaining changes and open PR**

```bash
git add infra/systemd/hermes-agent-executor.service \
       infra/systemd/hermes-feedback-loop.service \
       scripts/hermes-x/core/hermes_agent_task_executor.py \
       scripts/hermes-x/tests/test_executor_smoke.sh
git commit -m "feat(hermes): Phase B complete — K15 loop unblocked, executor hardened"
gh pr create --base main --title "feat(hermes): Organ X Phase B — repair + harden" \
  --body "$(cat <<'EOF'
## Summary
- Fix executor service (PATH, WantedBy) — now starts and runs
- Bounded Soma gate retry (max 3) with task requeue on GPU saturation
- Fix feedback-loop timer ordering cycle — learning consumer unblocked
- E2E smoke test for K15 pipeline validation
- Fallback verification documented

## Spec
docs/superpowers/specs/2026-05-16-hermes-organ-x-repair-harden-design.md

## Test plan
- [ ] Executor service stays active >1h
- [ ] hermes chat -q success rate >70% on 10 runs
- [ ] Smoke test passes: observation → task → execution → verdict
- [ ] SKILL.md updated after feedback-loop timer fires
- [ ] Soma retry requeues task on simulated GPU saturation

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
