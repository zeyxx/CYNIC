# Universal Workflow Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the three-layer universal workflow (L0 + L1 + L2) so any agent (Claude/Gemini/human) follows the same coordination protocol via ergonomics alone.

**Architecture:** L0 (git hooks) is already stable. L1 ergonomics needs deploy + GEMINI.md + Makefile scope targets + session hook. L2 adoption needs cynic_coord_* tools deployed + all agent configs updated to reference the lifecycle protocol.

**Tech Stack:** Bash (Makefile/hooks), Markdown (CLAUDE.md/GEMINI.md), Rust (kernel already done — coord tools at commit 250c396), SurrealDB (coord state store).

**Spec:** `docs/superpowers/specs/2026-03-16-workflow-universal-design.md`

---

## File Map

| File | Change | Phase |
|------|--------|-------|
| `Makefile` | Add `scope`, `done`, `agents` targets | A |
| `GEMINI.md` | Replace raw cargo commands with `make check` + Session Protocol section | A |
| `.claude/hooks/session-init.sh` | Add agent_id generation + coord reminder in session context | A |
| `CLAUDE.md` | Phase 0: reduce trigger table to 12 items. Phase B: add Session Lifecycle section | 0 + B |
| `.claude/commands/cynic-worktrees.md` | New CYNIC-specific worktree skill with coord integration | B |

**Not modified:** `cynic-kernel/` (all coord tools already committed at 250c396), `.gitignore`, test files.

---

## Chunk 1: IMMEDIATE + PHASE 0

### Task 1: Deploy coord tools

**Files:** No code changes — runtime action only.

The 5 security commits (52622cb–250c396) including `cynic_coord_*` tools and `/agents` endpoint are pushed but not running.

- [ ] **Step 1: Verify current deploy state**

```bash
make status
```
Expected: kernel shows commit older than 250c396 OR `cynic_coord_who()` returns "not found".

- [ ] **Step 2: Deploy**

```bash
make deploy
```
Expected output ends with: `Kernel: sovereign | Storage: connected | Dogs: N`

- [ ] **Step 3: Verify coord tools are live**

```bash
source ~/.cynic-env
curl -s "http://${CYNIC_REST_ADDR}/health" | python3 -m json.tool
```
Expected: `"status": "sovereign"`. If deploy failed mid-way, restore from backup:
`systemctl --user stop cynic-kernel && cp ~/bin/cynic-kernel.backup ~/bin/cynic-kernel && systemctl --user start cynic-kernel`
(backup created automatically by `make deploy` before replacing binary).

- [ ] **Step 4: Verify /agents endpoint**

```bash
source ~/.cynic-env
curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" "http://${CYNIC_REST_ADDR}/agents" | python3 -m json.tool
```
Expected: valid JSON (empty object `{}` or list — not a 404 or 401).

If `/agents` returns 404: the endpoint exists in code but the kernel binary may not match. Re-run `make deploy` ensuring the release binary is rebuilt first (`make check` then `make deploy`).

---

### Task 2: Phase 0 — Vocabulary burn (CLAUDE.md trigger table)

**Files:**
- Modify: `CLAUDE.md` (lines 112–141 — the "When to invoke what" table)

The current table has 27 rows covering HF plugins, redundant MCP tools, and orphaned skills. Target: exactly 12 rows matching the vocabulary registry in the spec.

**Note on cynic-learn:** The spec's Phase 0 description lists `cynic-learn` as a skill to promote, but the spec's own 12-item Vocabulary Registry (the acceptance criterion) does not include it. The registry is authoritative — `cynic-learn` is NOT in the 12-item table. The Phase 0 text is superseded by the registry.

- [ ] **Step 1: Read current CLAUDE.md trigger table**

```bash
sed -n '110,145p' CLAUDE.md
```
Confirm the current row count. Expected: ~27 rows.

- [ ] **Step 2: Replace trigger table**

Edit `CLAUDE.md`. Replace the entire "When to invoke what" table (from the `| Trigger | Tool/Skill | Type |` header through the last table row) with the 12-item canonical table:

```markdown
### When to invoke what

| Trigger | Tool/Skill | Type |
|---------|-----------|------|
| **Any code change** | `make check` / `/build` | Makefile / Slash command |
| **Session start** | `cynic_coord_register` | CYNIC MCP |
| **Before file edit** | `cynic_coord_who` + `cynic_coord_claim` | CYNIC MCP |
| **After ILC done** | `cynic_coord_release` | CYNIC MCP |
| **See active agents** | `make agents` / `GET /agents` | Makefile / REST |
| **Deploy to production** | `make deploy` / `/deploy` | Makefile / Slash command |
| **Evaluate quality** | `cynic-judge` | Skill |
| **Simplify/reduce code** | `cynic-burn` | Skill |
| **Complex decisions** | `crystallize-truth` | Skill |
| **Building/modifying CYNIC** | `cynic-kernel` | Skill |
| **Research / investigate** | `cynic-empirical` | Skill |
| **Workflow reference** | `cynic-workflow` | Skill |
```

- [ ] **Step 3: Verify row count**

Count only within the trigger table (scoped between its heading and the next heading):
```bash
awk '/### When to invoke what/{found=1; next} found && /^(##|###)/{exit} found && /^\| \*\*/{count++} END{print count}' CLAUDE.md
```
Expected: `12` (counts only non-separator data rows within the trigger table section).

- [ ] **Step 4: Verify no HF skill triggers remain**

```bash
grep -i "huggingface\|gradio\|trackio\|hf-cli" CLAUDE.md
```
Expected: no output (or only in the Dogs table, which lists `huggingface` as a validator — that's correct to keep).

- [ ] **Step 5: Commit**

```bash
make check 2>/dev/null || true  # CLAUDE.md is not Rust — skip build gate
git add CLAUDE.md
git commit -m "docs(workflow): Phase 0 — burn trigger table to 12 canonical items

Remove HF plugin suite, redundant MCP entries, and low-invocation-rate
skills from active vocabulary. Promote coord tools + cynic-empirical +
cynic-workflow to trigger table. Acceptance: exactly 12 rows."
```

---

## Chunk 2: PHASE A — L1 Ergonomics

### Task 3: Fix GEMINI.md build commands

**Files:**
- Modify: `GEMINI.md` (lines 58–64 — Build Commands section)

GEMINI.md currently has raw `cargo` commands instead of `make check`. This is T3 (61% confidence): most concrete L1 gap.

- [ ] **Step 1: Read current Build Commands section**

```bash
sed -n '56,66p' GEMINI.md
```
Confirm presence of `cargo build`, `cargo test`, `cargo clippy` lines.

- [ ] **Step 2: Replace Build Commands section**

Edit `GEMINI.md`. Replace the entire Build Commands section with the following
(shown here with `~~~` outer fence to avoid collision with inner backtick fences):

~~~markdown
## Build Commands

```bash
make check   # build + test + clippy (--release) — use this, not raw cargo
```

For individual stages when debugging:
```bash
cargo build -p cynic-kernel --release
cargo test -p cynic-kernel --release
cargo clippy -p cynic-kernel --release -- -D warnings
```
~~~

- [ ] **Step 3: Add Session Protocol section**

Append after the Build Commands section in `GEMINI.md`:

```markdown
## Session Protocol

Every session follows this lifecycle:

| Stage | Action | Tool |
|-------|--------|------|
| Start | Register intent | `cynic_coord_register(agent_id="gemini-<timestamp>", intent="<what>")` |
| Before edit | Check + claim | `cynic_coord_who()` → `cynic_coord_claim(agent_id, target-file)` |
| After ILC | Validate + release | `make check` → `cynic_coord_release(agent_id, target-file)` |
| End | Release session | `cynic_coord_release(agent_id)` |

**ILC (Independent Logical Component):** Unit of work. One branch per ILC:
`session/gemini/<slug>` (e.g., `session/gemini/ccm-decay-threshold`)

Git rejects duplicate branch names — hard enforcement against parallel work collision.
`cynic_coord_claim` adds soft visibility before work starts.

**Gemini MCP config:** cynic_coord_* tools must be in your MCP configuration.
Verify: `cynic_coord_who()` returns valid JSON (not "tool not found").
If missing: add the CYNIC MCP server to your ~/.gemini/ config.
```

- [ ] **Step 4: Verify no raw cargo commands remain as primary build**

```bash
grep "cargo build\|cargo test\|cargo clippy" GEMINI.md
```
Expected: matches only show indented fallback lines (inside a fenced block, not as the primary instruction).
The `## Build Commands` section's primary instruction must be `make check`.

- [ ] **Step 5: Commit**

```bash
git add GEMINI.md
git commit -m "docs(workflow): Phase A — GEMINI.md make check + Session Protocol

Replace raw cargo commands with make check (L1 ergonomics for Gemini).
Add Session Protocol with ILC lifecycle, coord tool invocations, and
Gemini MCP config verification note. Addresses T3 (most concrete gap)."
```

---

### Task 4: Add scope/done/agents targets to Makefile

**Files:**
- Modify: `Makefile` (append after `backup` target)

Three new targets enabling the ILC workflow from the command line for any agent or human.

- [ ] **Step 1: Read end of Makefile**

```bash
tail -10 Makefile
```
Confirm ends after `backup` target.

- [ ] **Step 2: Append scope/done/agents targets**

Edit `Makefile`. Append after the `backup` target:

```makefile
# ── Scope Management ─────────────────────────────────────────
.PHONY: scope
scope:  ## make scope SLUG=rest-audit — create isolated worktree for an ILC
	@if [ -z "$(SLUG)" ]; then echo "ERROR: provide SLUG=<name> (e.g. make scope SLUG=rest-audit)" >&2; exit 1; fi
	@BRANCH="session/$$(id -un)/$(SLUG)"; \
	WORKTREE="$(PROJECT_DIR)/../cynic-$(SLUG)"; \
	git worktree add -b "$$BRANCH" "$$WORKTREE"; \
	echo ""; \
	echo "✓ Worktree: $$WORKTREE"; \
	echo "✓ Branch:   $$BRANCH"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  1. cynic_coord_register(agent_id, intent)"; \
	echo "  2. cynic_coord_claim(agent_id, <target-file>)"; \
	echo "  3. cd $$WORKTREE && work"; \
	echo "  4. make check && cynic_coord_release(agent_id, <target-file>)"; \
	echo "  5. make done SLUG=$(SLUG) when finished"

.PHONY: done
done:  ## make done SLUG=rest-audit — remove worktree and branch
	@if [ -z "$(SLUG)" ]; then echo "ERROR: provide SLUG=<name>" >&2; exit 1; fi
	@BRANCH="session/$$(id -un)/$(SLUG)"; \
	WORKTREE="$(PROJECT_DIR)/../cynic-$(SLUG)"; \
	git worktree remove "$$WORKTREE" 2>/dev/null && echo "✓ Worktree removed" || echo "Note: worktree not found (may already be removed)"; \
	git branch -d "$$BRANCH" 2>/dev/null && echo "✓ Branch $$BRANCH deleted" || echo "Note: delete branch manually if merge is pending"; \
	echo "✓ Scope $(SLUG) released"

.PHONY: agents
agents:  ## Show active agents and work claims (requires kernel running)
	@$(source_env)
	@curl -sf -H "Authorization: Bearer $${CYNIC_API_KEY}" "http://$${CYNIC_REST_ADDR}/agents" | python3 -m json.tool 2>/dev/null || echo "ERROR: kernel not responding at $${CYNIC_REST_ADDR}"
```

- [ ] **Step 3: Verify acceptance criteria — scope + done roundtrip**

Ensure Tasks 3–5 are committed before running this (git must be clean for worktree creation):
```bash
git status  # should be clean
make scope SLUG=test-workflow-plan
```
Expected: creates worktree at `../cynic-test-workflow-plan`, prints branch `session/<user>/test-workflow-plan`.

```bash
make done SLUG=test-workflow-plan
```
Expected: "Scope test-workflow-plan released" with no errors.

- [ ] **Step 4: Verify agents target**

```bash
make agents
```
Expected: valid JSON (empty `{}` or list of active agents). Not `ERROR:`.

- [ ] **Step 5: Commit**

```bash
git add Makefile
git commit -m "feat(workflow): Phase A — add scope/done/agents Makefile targets

scope: creates git worktree + prints ILC protocol steps
done: removes worktree + branch after ILC merge
agents: GET /agents via curl (human + agent visibility)
Acceptance: make scope SLUG=test && make done completes clean."
```

---

### Task 5: Update session-init.sh with coord context

**Files:**
- Modify: `.claude/hooks/session-init.sh`

Hook must output agent_id and coord protocol instructions into session context. Must not fail if kernel is down (registration is best-effort — bash hooks can't call MCP tools directly; the output becomes a session instruction).

- [ ] **Step 1: Read current hook**

```bash
cat .claude/hooks/session-init.sh
```
Confirm current output ends at line `RULES: Public repo — no secrets...`

- [ ] **Step 2: Add agent_id generation and coord reminder**

Edit `.claude/hooks/session-init.sh`. Replace the `cat <<EOF` output block with:

```bash
# ── Generate stable agent ID for this session ──
AGENT_ID="claude-$(date +%s)"

# ── Output context (injected into conversation) ──
cat <<EOF
CYNIC SESSION — Pipeline initialized.
Kernel: ${KERNEL_STATUS} | DB: ${SURREAL_STATUS} | Git: ${GIT_BRANCH} (${GIT_DIRTY} dirty files)
Env: CYNIC_REST_ADDR=${CYNIC_REST_ADDR:-NOT SET}
Agent: ${AGENT_ID}

WORKFLOW: Use /build after edits, /deploy for production, /status for full dashboard.
COORD: Register → cynic_coord_register("${AGENT_ID}", intent) | Claim → cynic_coord_who + cynic_coord_claim | Release → cynic_coord_release
RULES: Public repo — no secrets, no real IPs, no names. Use skills before acting.
EOF
```

The `${AGENT_ID}` appears in the session context, making it available for the AI to use in coord tool calls without generating a new timestamp each time.

- [ ] **Step 3: Verify hook still exits 0 when kernel is down**

```bash
systemctl --user stop cynic-kernel
bash .claude/hooks/session-init.sh <<< '{"cwd":"/tmp"}'
echo "Exit: $?"
systemctl --user start cynic-kernel
```
Expected: outputs session context with `Kernel: down`, exits 0.

- [ ] **Step 4: Verify hook outputs agent_id when kernel is up**

```bash
bash .claude/hooks/session-init.sh <<< '{"cwd":"'"$(pwd)"'"}'
```
Expected: output includes `Agent: claude-<timestamp>` and `COORD:` line.

- [ ] **Step 5: Commit**

```bash
git add .claude/hooks/session-init.sh
git commit -m "feat(workflow): Phase A — session-init outputs agent_id + coord protocol

Generate deterministic agent_id (claude-<epoch>) at session start.
Output coord protocol reminder into session context (best-effort —
bash hooks cannot call MCP tools directly; output becomes instruction).
Hook continues to exit 0 when kernel is down (non-blocking)."
```

---

## Chunk 3: PHASE B — L2 Adoption

**Prerequisites before starting Chunk 3:**
- Chunk 1 + Chunk 2 complete (deploy, vocabulary, ergonomics)
- OQ3: Verify Gemini MCP config includes cynic_coord_* (**blocks Phase B**)

  Check Gemini config location (path varies by version):
  ```bash
  ls ~/.gemini/ 2>/dev/null || ls ~/.config/gemini/ 2>/dev/null
  cat ~/.gemini/settings.json 2>/dev/null | python3 -m json.tool | grep -i "cynic\|mcp"
  ```
  Expected: CYNIC MCP server entry visible in config.
  If absent: add the CYNIC MCP server (`cynic-kernel --mcp`) to Gemini's MCP config before proceeding.
  Then ask Gemini to call `cynic_coord_who()` and confirm it returns valid JSON, not "tool not found".

---

### Task 6: Add Session Lifecycle to CLAUDE.md

**Files:**
- Modify: `CLAUDE.md` (add new section after Tool Ecosystem)

- [ ] **Step 1: Read end of Tool Ecosystem section**

```bash
grep -n "Canonical References\|Environment\|All skills" CLAUDE.md | head -5
```
Identify the line number of `### Environment` or `## Canonical References`.

- [ ] **Step 2: Add Session Lifecycle section**

Edit `CLAUDE.md`. Insert before `## Canonical References` the following section.
The numbered list is plain text (no inner fences needed — avoid nested fence rendering issues):

    ## Session Lifecycle (REQUIRED)

    Every working session follows this protocol:

    1. START:        cynic_coord_register(agent_id="claude-<epoch>", intent="<what you're doing>")
    2. BEFORE EDIT:  cynic_coord_who() → cynic_coord_claim(agent_id, "<target-file>")
    3. WORK:         git worktree add (use make scope SLUG=<name> for ILC isolation)
    4. VALIDATE:     make check
    5. SHIP:         make commit m="..." + make ship (L0 gates: gitleaks + build + test + clippy)
    6. RELEASE:      cynic_coord_release(agent_id, "<target-file>")
    7. END:          cynic_coord_release(agent_id)

    **agent_id** is in your session context (injected by session-init.sh as `claude-<epoch>`).
    **ILC branch naming:** `session/claude/<slug>` — git enforces uniqueness (hard gate).
    **Conflict response:** If `cynic_coord_claim` returns CONFLICT, call `cynic_coord_who()` first and coordinate.

- [ ] **Step 3: Verify lifecycle section appears correctly**

```bash
grep -A 20 "Session Lifecycle" CLAUDE.md
```
Expected: 7 numbered steps visible, `cynic_coord_*` calls present.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(workflow): Phase B — add Session Lifecycle to CLAUDE.md

7-step protocol: register → claim → worktree → check → ship → release → end.
References agent_id from session-init.sh context. Soft enforcement via
system prompt injection. Coord tools are live (deployed in Task 1)."
```

---

### Task 7: Create cynic-worktrees command skill

**Files:**
- Create: `.claude/commands/cynic-worktrees.md`

CYNIC-specific worktree skill that wraps `superpowers:using-git-worktrees` with coord integration. Uses `make scope` / `make done` for ILC management.

- [ ] **Step 1: Create the skill file**

Create `.claude/commands/cynic-worktrees.md`:

```markdown
# CYNIC Worktrees — Isolated ILC Workspace

CYNIC extension of `superpowers:using-git-worktrees` with coordination protocol.

**Announce at start:** "I'm using cynic-worktrees to set up an isolated ILC workspace."

## When to Use

Use when starting any independent logical component (ILC) — a unit of work that:
- Touches specific files (claims prevent parallel edits)
- Needs to be validated independently (make check)
- Integrates without conflict (branch-per-ILC is hard enforcement)

## Setup Protocol

### 1. Register intent
```
cynic_coord_register(agent_id="<your-session-agent-id>", intent="<ILC description>")
```

### 2. Check for conflicts
```
cynic_coord_who()
```
If another agent holds the target file → coordinate before claiming.

### 3. Claim target files
```
cynic_coord_claim(agent_id, "<target-file>")
```
Repeat for each file in the ILC scope.

### 4. Create worktree
```bash
make scope SLUG=<ilc-name>
# e.g., make scope SLUG=rest-audit-middleware
```
This creates `../cynic-<slug>` worktree on branch `session/<user>/<slug>`.
Git rejects duplicate branch names — hard enforcement against collision.

### 5. Work in the isolated worktree
```bash
cd ../cynic-<slug>
# ... make changes ...
make check   # validate before releasing
```

### 6. Validate and release
```bash
make check
cynic_coord_release(agent_id, "<target-file>")
```
Release each claimed file after validation passes.

### 7. Ship and clean up
```bash
# Merge/PR the branch, then:
make done SLUG=<ilc-name>
cynic_coord_release(agent_id)  # release full session if done
```

## Conflict Resolution

If `cynic_coord_claim` returns `CONFLICT: held by <other-agent>`:

1. Call `cynic_coord_who()` to see full claim state
2. If the other agent is inactive (session expired), claim proceeds automatically after 5 min TTL
3. If active: coordinate directly (different ILC, or wait)
4. Never overwrite another agent's claimed scope without coordination

## Quick Reference

| Stage | Command |
|-------|---------|
| Start ILC | `make scope SLUG=<name>` |
| Check claims | `cynic_coord_who()` |
| Claim file | `cynic_coord_claim(agent_id, file)` |
| Release file | `cynic_coord_release(agent_id, file)` |
| End ILC | `make done SLUG=<name>` |
| See all agents | `make agents` |
```

- [ ] **Step 2: Verify skill file syntax**

```bash
head -5 .claude/commands/cynic-worktrees.md
```
Expected: starts with `# CYNIC Worktrees`.

- [ ] **Step 3: Commit**

```bash
git add .claude/commands/cynic-worktrees.md
git commit -m "feat(workflow): Phase B — cynic-worktrees skill with coord integration

New project skill extending using-git-worktrees with CYNIC coord protocol.
7-step ILC lifecycle: register → who → claim → scope → work → release → done.
Conflict resolution: 5-min TTL auto-expiry + explicit coordination path."
```

---

### Task 8: L2 Adoption verification (6-step lifecycle test)

Full acceptance test per spec. Requires running kernel with coord tools deployed.

**Important:** `cynic_coord_*` tools are MCP stdio tools — they are called directly by the AI agent as MCP tool invocations, not via HTTP/curl. The MCP server has no HTTP bridge. This test is performed by the implementing agent (Claude Code) calling the tools using the MCP tool mechanism.

In a Claude Code session with the CYNIC MCP server connected, execute each step as a tool call:

- [ ] **Step 1: Register test-agent-A**

Call MCP tool: `cynic_coord_register`
Arguments: `{"agent_id": "test-agent-A", "intent": "testing lifecycle"}`
Expected response: success/registered (not an error).

- [ ] **Step 2: test-agent-A claims rest.rs**

Call MCP tool: `cynic_coord_claim`
Arguments: `{"agent_id": "test-agent-A", "target": "rest.rs"}`
Expected response: contains `CLAIMED`.

- [ ] **Step 3: test-agent-B claim triggers CONFLICT**

Call MCP tool: `cynic_coord_claim`
Arguments: `{"agent_id": "test-agent-B", "target": "rest.rs"}`
Expected response: contains `CONFLICT` and `test-agent-A`.

- [ ] **Step 4: who() shows state**

Call MCP tool: `cynic_coord_who`
Arguments: `{}`
Expected response: shows `test-agent-A` holding `rest.rs` with timestamp.

- [ ] **Step 5: test-agent-A releases**

Call MCP tool: `cynic_coord_release`
Arguments: `{"agent_id": "test-agent-A", "target": "rest.rs"}`
Expected response: contains `RELEASED`.

- [ ] **Step 6: test-agent-B now claims successfully**

Call MCP tool: `cynic_coord_claim`
Arguments: `{"agent_id": "test-agent-B", "target": "rest.rs"}`
Expected response: contains `CLAIMED`.

All 6 steps passing = Phase B acceptance criterion met.

- [ ] **Step 7: Cleanup**

Call MCP tool: `cynic_coord_release`
Arguments: `{"agent_id": "test-agent-B"}`
(Releases all claims for test-agent-B.)

- [ ] **Step 8: Verify clean state via REST**

```bash
make agents
```
Expected: valid JSON with no `test-agent-*` entries remaining.

---

## Open Questions (from spec)

| OQ | Status | Action |
|----|--------|--------|
| OQ1: coord latency <500ms | Verify after Task 1 | Check response time of cynic_coord_register |
| OQ2: session TTL 5min | Non-blocking | Monitor in Phase B; may need configurable TTL |
| OQ3: Gemini MCP config | **Blocks Task 6+** | Verify before Chunk 3 start |
| OQ4: agent_id stability on resume | Non-blocking for now | session-init.sh generates new id per session; H7 blocked until resolved |
| OQ5: S. machine API contract changes | Non-blocking | Post Phase B |

---

## Acceptance Summary

| Phase | Acceptance Criterion | Test |
|-------|---------------------|------|
| IMMEDIATE | Kernel running coord tools | `make agents` returns valid JSON |
| Phase 0 | Trigger table = exactly 12 items | `awk` count within trigger table section → `12` (see Task 2 Step 3) |
| Phase A | Scope roundtrip works | `make scope SLUG=test && make done SLUG=test` → no errors |
| Phase B | Full 6-step lifecycle | All steps in Task 8 return expected status |
