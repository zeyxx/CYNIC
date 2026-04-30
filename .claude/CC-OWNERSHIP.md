# CC-OWNERSHIP.md — What CYNIC Owns in Claude Code

The living ledger of CYNIC's Claude Code integration. Not instructions. A map: what exists, what consumes it, what signals its health, how to verify it works. Updated 2026-04-30.

---

## Hook Inventory

| Event | Script | Purpose | Consumer | Health Signal | Falsification |
|-------|--------|---------|----------|---------------|--------------|
| SessionStart | session-init.sh | Probe kernel, register agent, inject context | LLM session context | `CYNIC SESSION` line in context | Missing → session lacks kernel state |
| UserPromptSubmit | phi-reminder.sh | φ⁻¹ = 0.618 reminder on every turn | LLM epistemic bounds | Reminder visible in every turn | Removed → LLM claims certainty > 0.618 |
| SubagentStart | subagent-context.sh | Inject 6 axioms + TRIAD into subagents | Subagent reasoning | Axioms appear in system context | Missing → subagent lacks axioms |
| PreToolUse | protect-files.sh | Block SSH/env/secret reads/writes | Security gate | Attempt Read ~/.cynic-env → BLOCKED | Allows secret access → breach |
| PreToolUse | coord-claim.sh | Claim file lock before kernel edits | Multi-agent coordination | 409 conflict on concurrent edit | Missing → two agents edit kernel concurrently |
| PostToolUse (async) | observe-tool.sh | Fire-and-forget telemetry → `/observe` | CCM pipeline → crystals → Dog prompts | `/observations` table grows per session | Missing → no telemetry → no CCM |
| PostToolUse | rustfmt-rs.sh | Auto-format Rust files (--edition 2024) | Pre-commit lint gate | No rustfmt diffs in committed code | Missing → formatting drift → CI fails |
| Stop | askesis-claude-end.sh | Seal session cortex history (if binary exists) | cynic-askesis durability | Silent if binary missing (degraded mode) | Missing → session cortex lost on exit |
| Stop | session-stop.sh | Release coord claims, score compliance, post summary | /observations domain=session | session_summary in observations | Missing → no inter-session handover |
| Stop | dream-trigger.sh | Increment dream counter at 5-session threshold | session-init.sh dream check | sessions_since in ~/.claude/.../memory/.dream-state | Never increments → dream never triggered |
| Stop (async) | exercise-scheduler.sh | Calculate daily exercise duration from git | Human (daily lock file) | Lock file in ~/.cynic-exercise/ | Silent if git unavailable |

---

## Commands (/slash-commands)

| Command | When | Purpose | Model-invocation |
|---------|------|---------|-----------------|
| /build | After kernel code change | Full Rust build → test → clippy → lint | enabled (Claude guides flow) |
| /status | Session start or incident | Full CYNIC system diagnostic (10+ checks) | enabled |
| /cc-status | To audit Claude Code health | Hook wiring, hook health, telemetry rates, distill completion | disabled (pure bash) |
| /deploy | After build gates pass | Swap ~/bin/cynic-kernel binary | enabled |
| /run | Dev iteration | Start kernel process locally | enabled |
| /dream | When session-init outputs DREAM_REQUIRED | Dispatch dream-consolidator agent (async) | enabled |
| /loop | Recurring interval checks | Polling/cron dispatch for automated tasks | enabled |
| /e2e | Before PR creation | End-to-end test run (chess, token, wallet domains) | enabled |
| /test-chess | Before CCM changes | Chess domain regression (validates crystal pipeline) | enabled |
| /cynic-kernel | Before kernel changes | Architecture reference (routing, port traits, K-rules) | disabled (pure reference) |
| /cynic-workflow | When workflow unclear | Multi-cortex rules, session protocol, MC1-MC5 | disabled (pure reference) |
| /frontend-dev | UI component changes | Frontend patterns for verdict display, axiom charts | enabled (Claude guides) |

---

## Skills (cynic-skills)

| Skill | Purpose | Trigger | Posts to Kernel | Post-Session Required? |
|-------|---------|---------|-----------------|--------|
| distill | 8-step session compound protocol | After >100 lines changed or significant work | YES — `tool=session_distill domain=session` | **YES — or organism is blind** |
| metathink | Session self-diagnosis (anti-loop pattern check) | Stuck >30min or ratio >3:1 discussion | No | No |
| cynic-judge | Evaluate code/decisions through 6 axioms | Before structural decisions | No | No |
| cynic-burn | Code simplification + hotspot analysis | Before adding abstractions | No | No |
| cynic-empirical | External research reflex | Before building with unknowns | No | No |
| cynic-wisdom | Philosophical grounding (axioms ↔ traditions) | Complex ethical/architectural tension | No | No |
| crystallize-truth | Pattern crystallization methodology | Multi-hypothesis situations | No | No |
| engineering-stack-design | Stack decision framework | Before new tech dependency | No | No |
| ai-infrastructure | LLM pipeline design guidance | Building LLM integrations | No | No |

**Key:** `/distill` is the only skill that **must** POST to kernel at end. The POST is the consumer that makes session reasoning available to the next cortex. If omitted, session work is invisible to the organism.

---

## Rules Files

| File | Scope | Rules Count | Key Enforcements |
|------|-------|-------------|-----------------|
| `universal.md` | `**` (all files) | 23 | Falsifiable claims, zero hardcoded paths, I/O handling, producer/consumer, no dead code |
| `workflow.md` | Session behavior | 8 | Multi-cortex MC1-MC5, pre-commit validation order (check → clippy → build), distill instruction |
| `kernel.md` | `cynic-kernel/**` | 16 | Hexagonal architecture, K1-K17 tier-1 gates (compiler, lint), LLM development principles |
| `python.md` | `cynic-python/**` | 15 | Type annotations, 80% coverage, pinned deps, structured logging, no state |
| `reference.md` | Reference only | — | φ constants, Dogs table, infra endpoints (non-enforced) |
| `cost.md` | Token economy | Model routing | Opus/Sonnet/Haiku by task type (guidance only) |

**Enforcement:** universal + kernel rules are `make lint-rules` gates. workflow rules are behavioral (session-init.sh checks for violations). python rules are `mypy --strict` + pytest coverage gates.

---

## Data Flows

### Observation Pipeline (Session Telemetry → CCM → Dog Prompts)
```
observe-tool.sh
  → fires after every Edit/Write/Bash/Read/Grep/Glob
  → async POST /observe {tool, target, status, context, session_id}
  → SurrealDB observations table
  → CCM cycle (in-kernel)
  → promotes high-confidence observations → crystals
  → crystals injected into session-init.sh context
  → crystals used in /format_crystal_context
  → crystals influence Dog prompts
  → better verdicts → better learning
```

**Current state:** 0 crystallized (as of 2026-04-25). Telemetry is produced, CCM loop is broken. K15 violation: producer (observe-tool.sh) exists, but acting consumer (CCM cycle) is dormant.

### Session Handover Bus (Cortex → Cortex Learning)
```
During session:
  session-stop.sh (Stop event)
    → POST /observe tool=session_summary domain=session {metrics: duration, commits, compliance_score}
  
  /distill skill Step 8 (optional LLM action — SHOULD be mandatory)
    → POST /observe tool=session_distill domain=session {context: "WHAT/WHY/NEXT/BLOCKED"}

Next session:
  session-init.sh (SessionStart event)
    → GET /observations?domain=session&limit=3
    → injects "RECENT SESSIONS (inter-agent bus):" block
    → reads prior cortex reasoning (summary + distill)
    → uses this to inform current session's context

Next-next session:
  session-init.sh reads 5 most recent observations of type session_distill
  → patterns begin to emerge across cortex instances (Claude, Gemini, Hermes)
```

**Current state:** session_summary POSTs reliably (session-stop.sh does it), but session_distill is optional (LLM compliance-based). Gap: 37% of sessions skip distill (as of 2026-04-30).

### Coord Lifecycle (Multi-Agent File Locking)
```
Session N starts:
  session-init.sh
    → POST /coord/register agent_id=claude-{SESSION_ID:0:12}
    → Agent claims "exists and working"

During session (per kernel src file edit):
  coord-claim.sh (PreToolUse, triggered only for cynic-kernel/src/**)
    → POST /coord/claim {agent_id, target: <file>, claim_type: "file"}
    → kernel responds 200 (claimed) or 409 (conflict)
    → 409 → edit BLOCKED, human sees which agent has lock

Every tool use:
  observe-tool.sh (async)
    → POST /coord/heartbeat agent_id=...
    → updates agent's 5-min TTL
    → kernel auto-expires stale agents

Session N ends:
  session-stop.sh
    → POST /coord/release agent_id=... (no target = release all)
    → frees all files claimed by this agent

Session N+1 start:
  session-init.sh
    → GET /coord/who
    → warns if other agents have claimed the same files
    → checks MC4 (auto-partition with escalade)
```

**Current state:** Coord is wired and working. All agents auto-expire if session dies. No orphaned claims observed.

---

## Known Gaps (as of 2026-04-30)

| Gap | Severity | Blocker? | Remediation | Timeline |
|-----|----------|----------|-------------|----------|
| Distill not enforced | HIGH | No | session-stop.sh distill check + warning | This session (D1) |
| Wisdom not injected | MEDIUM | No | session-init.sh read D1-D6 curation files | This session (D2) |
| No observability | MEDIUM | No | /cc-status command | This session (D3) |
| Settings.json is static | LOW | No | CC-OWNERSHIP.md documents it | Done (D4) |
| CCM loop is dormant | MEDIUM | No | Separate session: revive CCM crystallization | Future |
| No `/wisdom` REST endpoint | LOW | No | Curation files are direct, not REST-served | Defer |
| session-proof.json not tracking distill | LOW | No | Add distill_posted field to AT_END | Future |
| askesis-claude-end.sh silent on missing binary | LOW | No | Binary built on demand, acceptable | As-is |

---

## Governance

**Who edits this file:** Any cortex, after verifying state.

**When to update:** After adding/removing hooks, commands, rules, or permissions.

**Staleness detection:**
```bash
diff <(jq -r '.hooks | keys[]' .claude/settings.json | sort) \
     <(grep '^|' .claude/CC-OWNERSHIP.md | awk '{print $3}' | sort)
# If output is non-empty, CC-OWNERSHIP.md is stale relative to settings.json
```

**Falsification:**
- Run `/cc-status` — if any section errors, corresponding entry in this document is inaccurate
- Query `/observations?domain=session` for last 10 — if distill_count / session_count < 0.618 and no distill entries exist, distill enforcement is not working
- Attempt concurrent kernel edits from two sessions — if no 409 conflict, coord-claim.sh is broken

---

## Reference: Critical Paths

**K15 Boundary (Producer/Consumer Audit):**
- observe-tool.sh (producer) → /observe (consumer: CCM)
- session-stop.sh (producer: compliance, cost metrics) → /observe (consumer: session-init)
- /distill skill Step 8 (producer: session_distill) → /observe (consumer: session-init on next cortex)
- coord-claim.sh (producer: claim lock) → /coord/claim (consumer: other agents, MC4 check)

If any producer posts and no consumer reads, it's a K15 violation: storage without acting consumption. Track via `/cc-status` telemetry rates.

**Epistemic Bounds:**
- φ⁻¹ = 0.618 — max confidence on any claim
- Wisdom signal threshold ≥ 0.8 (curated high-confidence only)
- Distill rate target ≥ 0.618 (at least 61.8% of sessions post distill)
- Compliance score target ≥ 0.618 (session end compliance > φ⁻¹)

**Multi-Cortex Rules (MC1-MC5):**
- MC1: One branch per cortex per scope (feat/scope-SESSION_ID-YYYY-MM-DD)
- MC2: PR before new work (don't start while prior branch unmerged)
- MC3: Main always fresh (pull before branch, rebase after merge)
- MC4: Auto-partition with escalade (coord-claim.sh warns on conflict, human re-routes)
- MC5: Atomic scope (one coherent feature per branch, not a catch-all)
