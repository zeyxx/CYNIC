# CYNIC — Universal Development Workflow Design

**Date:** 2026-03-16
**Method:** Brainstorming → Crystallize-Truth (3 layers) → Cynic-Burn → Live probe (Tailscale MCP + ts_exec)
**Status:** Approved — ready for implementation planning
**Confidence:** φ⁻¹ = 61.8% max — all claims falsifiable
**Q-Score:** WAG 71.2/100 (design complete, not yet implemented)

---

## Why This Document Exists

Session 2026-03-16: two agents worked in parallel without coordination while we designed coordination. 5 commits landed during the brainstorm by another session. The problem being designed was happening live. This document crystallizes what we found and what to build.

---

## Crystallized Truths (confidence ≥ 50%)

Items below 50% are hypotheses — see section below.

| T# | Truth | Confidence | Design Impact |
|----|-------|------------|---------------|
| T1 | L2 infrastructure exists (cynic_coord_* — register/claim/release/who). L2 adoption does not. No agent calls these tools. | 58% | Work is integration, not construction |
| T1a | Coord tools are in code (HEAD 250c396) but not in running kernel (deployed at fe4c840). make deploy is immediate blocker. | 61% | Deploy before any adoption work |
| T2 | Soft enforcement is the only mechanism above L0 for all tools, all agents. Architecturally unchangeable. | 61% | Solution = ergonomics (cheapest path = right path), not enforcement |
| T3 | L1 is absent for Gemini. GEMINI.md lists raw cargo commands instead of make check. Most concrete, most fixable gap. | 61% | Replace raw commands with make targets in GEMINI.md |
| T4 | Adoption problem is symmetric: tools built but not adopted, workflow steps defined but not followed. More tools without adoption makes it worse. | 55% | Adopt before building. Burn vocabulary before adding. |
| T5 | CYNIC is the right coordination backbone: SurrealDB is shared state, MCP is the protocol, all agents already trust it. | 52% | No new infrastructure. Adopt what exists. |
| T6 | 30+ items in active vocabulary → ~20% invocation rate. 8-12 canonical items → target 80%+. | 56% | Burn vocabulary before Phase A |
| T8 | 5 commits landed by another session during this brainstorming session without coordination. T10 confirmed live. | 58% | L2 adoption is urgent, not future work |

## Hypotheses (confidence < 50% — not yet crystallized)

| H# | Hypothesis | Confidence | Note |
|----|-----------|------------|------|
| H7 | cynic_coord_register at SessionStart could query last session scores → closes open feedback loop | 48% | Depends on OQ4 (agent_id stability). Cannot crystallize until OQ4 resolved. |
| H9 | 2 machines online (ubuntu + S.). Real problem = intra-T. coordination. | 45% | Infrastructure changes frequently. Verify at implementation time. |

---

## Architecture — Three Independent Layers

```
L2  SCOPE COORDINATION          exists in code — needs deploy + adoption
    SessionStart  → cynic_coord_register(agent_id, intent)   [best-effort]
    Before work   → cynic_coord_who() → cynic_coord_claim(agent_id, target)
                  → git worktree add  (physical isolation)
    After work    → make check → cynic_coord_release(agent_id, target)
    SessionEnd    → cynic_coord_release(agent_id)
    Dashboard     → GET /agents  (REST — human visibility)

L1  ERGONOMICS                  absent for Gemini — partial for Claude
    Claude Code: skills + hooks → make targets (coord in session hook)
    Gemini CLI:  GEMINI.md: raw commands → make targets + coord protocol
    Human:       make scope SLUG=<name> / make done / make agents

L0  UNIVERSAL GATES             stable — non-negotiable for all tools
    gitleaks pre-commit (160+ patterns + custom CYNIC rules)
    build + test + clippy pre-push
    GitHub push protection
    Ownership zones: cynic-kernel/ T. only, cynic-ui/ S. only
```

### Hard vs Soft Enforcement

```
Hard (physically blocks operation)     Soft (guidance — can be rationalized away)
────────────────────────────────       ──────────────────────────────────────────
pre-commit gitleaks                    CLAUDE.md skill triggers
pre-push build+test+clippy             system-reminder instructions
git rejects duplicate branch names     using-superpowers "not negotiable"
                                       cynic_coord_claim conflict response
                                       (agent can read CONFLICT and ignore it)
```

Note: cynic_coord_claim returns a conflict message but does not block execution.
It is soft enforcement at L2, not hard. The hard enforcement is git (duplicate branch names).

Design principle: maximize hard at L0. At L1/L2, make right action structurally cheaper than wrong action.

---

## The ILC (Independent Logical Component)

Unit of work. Declared before starting, validated independently, integrated without conflict.

Branch naming is the scope registry:
```
session/claude/rest-audit-middleware     ← rest.rs claimed
session/gemini/ccm-decay-threshold      ← ccm.rs claimed
session/s/ui-verdict-panel              ← cynic-ui/ claimed
```
Git rejects duplicate branch names = hard enforcement against parallel work on identical scope.
cynic_coord_claim = visibility layer (soft) — shows conflicts before work starts.

---

## What Already Exists (do not rebuild)

| Component | Location | Status |
|-----------|----------|--------|
| cynic_coord_register/claim/release/who | mcp.rs (HEAD 250c396) | Code only — not deployed |
| GET /agents (human dashboard) | rest.rs | Code only — not deployed |
| make deploy | Makefile | Works — not run after recent commits |
| using-git-worktrees skill | superpowers plugin | Exists — never used in practice |
| cynic_judge MCP | running kernel | Verified live (WAG Q=0.411, ~15s latency) |
| L0 git hooks | .git/hooks/ | Working — gitleaks 106ms, pre-push ~50s |
| Makefile pipeline (check/commit/ship/deploy/e2e/status/backup) | Makefile | Working |
| Makefile targets: scope/done/agents | Makefile | Not yet added — Phase A deliverables |
| SessionStart hook | .claude/hooks/session-init.sh | Working — probes kernel+DB+git at boot |

---

## Implementation Phases

### IMMEDIATE (one command, unblocks everything)
```bash
make deploy
# Deploys 5 pending commits including cynic_coord_* tools to running kernel
# If deploy fails mid-way: check make status + restore ~/bin/cynic-kernel from backup
```
Acceptance: `curl $CYNIC_REST_ADDR/health` returns status=sovereign AND `make agents` returns valid JSON.

### PHASE 0 — Vocabulary (1 session)

**Burn from CLAUDE.md trigger table:**
- Remove HuggingFace plugin suite (7 skills) from "when to invoke what" table
  (keep installed — remove only from the trigger table)
- Reason: 0 documented usage in CYNIC development workflow

**Promote to active vocabulary:**
- cynic-workflow — add trigger: "when in doubt about git/deploy/workflow steps"
- cynic-empirical — add trigger: "when researching practices or investigating problems"
- cynic-learn — add trigger: "when capturing session learnings or updating skills"

Acceptance: CLAUDE.md trigger table has exactly 12 items, each with unambiguous trigger.

### PHASE A — L1 Ergonomics (1-2 sessions)

**GEMINI.md (highest priority — most concrete gap):**
```diff
- cargo build -p cynic-kernel --release
- cargo test -p cynic-kernel
- cargo clippy --workspace -- -D warnings
+ make check
```
Add session protocol to GEMINI.md:
```
## Session Protocol
Start:      cynic_coord_register(agent_id="gemini-<session>", intent="<what>")
Before edit: cynic_coord_who() + cynic_coord_claim(agent_id, target)
After ILC:  make check + cynic_coord_release(agent_id, target)
End:        cynic_coord_release(agent_id)
```

**Verify Gemini MCP config (blocks Phase B if missing):**
- Check ~/.gemini/ or equivalent for MCP server configuration
- If cynic_coord_* tools absent from Gemini's MCP config → add before Phase B
- This is a Phase A deliverable that gates Phase B

**session-init.sh addition:**
- Add cynic_coord_register call at session start
- Registration MUST be best-effort: hook must not exit non-zero if kernel is down
  (kernel could be in maintenance; blocking every Claude session is not acceptable)

**Makefile additions:**
```makefile
scope:   # make scope SLUG=rest-audit
         # Creates git worktree -b session/<user>/<slug>
         # Prints: worktree path, branch name, next steps (register + claim)

done:    # Removes worktree, deletes branch
         # Prints: confirm scope released

agents:  # Calls GET /agents via curl
         # Shows active agents + claims as JSON
```

Acceptance: `make scope SLUG=test && make done` completes without error. `make agents` returns valid JSON.

### PHASE B — L2 Adoption (1 session)

Prerequisites:
- Phase A complete
- Gemini MCP config verified (OQ3 resolved — see Open Questions)
- agent_id stability strategy decided (OQ4 resolved)

**using-git-worktrees skill:** integrate cynic_coord_claim before worktree work, cynic_coord_release after merge.

**CLAUDE.md session lifecycle section:**
```
## Session Lifecycle
1. SessionStart: cynic_coord_register (auto via hook)
2. Before edit:  cynic_coord_who + cynic_coord_claim(target)
3. Work:         git worktree (physical isolation)
4. Validate:     make check
5. Ship:         commit + push (L0 gates)
6. Release:      cynic_coord_release(target)
7. SessionEnd:   cynic_coord_release(agent_id)
```

Acceptance (complete lifecycle, not just conflict detection):
```bash
# Step 1: register
cynic_coord_register("agent-A", "testing lifecycle")   # → OK

# Step 2: claim
cynic_coord_claim("agent-A", "rest.rs")                # → CLAIMED

# Step 3: conflict detection
cynic_coord_claim("agent-B", "rest.rs")                # → CONFLICT: held by agent-A

# Step 4: who shows state
cynic_coord_who()  # → agent-A: rest.rs since <timestamp>

# Step 5: release
cynic_coord_release("agent-A", "rest.rs")              # → RELEASED

# Step 6: now available
cynic_coord_claim("agent-B", "rest.rs")                # → CLAIMED
```
All 6 steps must pass.

---

## Vocabulary Registry — Target (12 items after Phase 0)

| Trigger | Tool | Type |
|---------|------|------|
| Any code change | make check / /build | Universal |
| Session start | cynic_coord_register | Coord MCP |
| Before file edit | cynic_coord_who + cynic_coord_claim | Coord MCP |
| After ILC done | cynic_coord_release | Coord MCP |
| See active agents | make agents / GET /agents | Coord REST |
| Deploy to prod | make deploy / /deploy | Universal |
| Judge quality | cynic-judge skill | CYNIC |
| Simplify code | cynic-burn skill | CYNIC |
| Complex decisions | crystallize-truth skill | CYNIC |
| Architecture ref | cynic-kernel skill | CYNIC |
| Research | cynic-empirical skill | CYNIC |
| Workflow ref | cynic-workflow skill | CYNIC |

---

## Open Questions

| # | Question | Blocking? | Phase |
|---|----------|-----------|-------|
| OQ1 | cynic_coord_register latency — MCP judge ~15s; registration must be <500ms (DB write). Verify after deploy. | Non-blocking | After IMMEDIATE |
| OQ2 | Session expiry at 5 min — paused session could auto-expire mid-work. May need configurable TTL. | Non-blocking | Phase B |
| OQ3 | Gemini MCP config — cynic_coord_* tools must be in Gemini's MCP config for Phase B. Location unknown. | **Blocking Phase B** | Phase A |
| OQ4 | agent_id stability — claude-$(date +%s) is unique but not stable on session resume. Affects H7 (historical context). | **Blocking H7** | Phase A |
| OQ5 | S. machine scope — 1% case: API contract changes affecting kernel + UI need shared ILC protocol. | Non-blocking | Post Phase B |

---

## What This Document Is Not

- Not the implementation plan (that comes from writing-plans)
- Not a guarantee (confidence ≤ 61.8%)
- Not immutable (update as reality falsifies claims)
- The 12-item vocabulary table is the transition target — it will supersede the relevant section of CLAUDE.md after Phase 0, not coexist with it

---

*Crystallized through: brainstorming → crystallize-truth (3 layers, 10 modes) → cynic-burn (vocabulary audit) → live probe (ts_exec, Tailscale MCP) → cynic-judge (WAG 71.2). Five commits by another session during this design session — proof the problem is real. Confidence cap: 61.8%.*
