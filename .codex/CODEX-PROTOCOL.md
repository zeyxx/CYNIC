# CYNIC — Multi-Agent Protocol for Codex

> **Date:** 2026-04-15  
> **For:** Codex CLI (OpenAI's Claude Code equivalent)  
> **Mission:** Align inter-agent workflow via cynic-coord MCP proxy

---

## Quick Start

1. **Verify setup**
   ```bash
   codex --version  # Should be recent
   cd /home/user/Bureau/CYNIC
   echo $CYNIC_REST_ADDR  # Should be set (e.g., localhost:3030 or Tailscale IP)
   echo $CYNIC_API_KEY    # Should be set
   ```

2. **Launch Codex** with MCP coordination
   ```bash
   export CYNIC_REST_ADDR="<TAILSCALE_CORE>:3030"
   export CYNIC_API_KEY="$(grep CYNIC_API_KEY ~/.cynic-env | cut -d= -f2)"
   codex  # Loads .codex/config.toml automatically
   ```

3. **Verify MCP is live**
   ```
   /mcp
   Expected: cynic-coord - Ready (7 tools)
   ```

---

## The Protocol (3 Layers)

### Layer 1: Session Lifecycle

**At session start:**
```bash
/cynic-coord register --agent-id codex-<YOUR-IDENTIFIER> --intent "<what-you-are-working-on>"
/cynic-coord who  # See who else is active
/cynic-coord read-handoff  # Load context from .handoff.md
```

**Before editing a file in `cynic-kernel/src/`:**
```bash
/cynic-coord claim --target "cynic-kernel/src/path/to/file.rs"
# If CONFLICT (409): Check who has the file, coordinate with them
```

**During work:**
- Run `/cynic-coord who` periodically if session > 1 hour
- Use `/cynic-coord observe --discovery "what I learned"` when you hit a significant finding

**At session end:**
```bash
/cynic-coord write-handoff --message "what I did, what's next"
/cynic-coord release --agent-id codex-<YOUR-IDENTIFIER>  # Release ALL claims
```

### Layer 2: File Coordination (Automatic via Hooks)

The `.codex/config.toml` has a pre-tool hook that auto-claims on writes:
- **Before you edit** `cynic-kernel/src/*`: auto-claim via MCP
- **If CONFLICT:** Hook pauses, you decide: release file, wait, or escalate
- **At session end:** Auto-release via hook

This is transparent — you don't need to invoke tools manually. But understanding what's happening helps debug contention.

### Layer 3: Consensus & State Machine

When 2+ agents touch the same file simultaneously:
1. First claim wins (FIFO queue in kernel)
2. Second agent gets 409 CONFLICT
3. Agents must resolve: release, wait, or split the work
4. No force-overwrite; no silent merges

---

## Your Tasks (TODO-ROBUSTNESS #3 + #4)

### TODO #3: Structured Commits (Clean Dirty Files)

**Current state:** 22 modified files (11 modified, 11 untracked)

**What to do:**
1. Review each modified file (list below)
2. Commit with one logical change per commit
3. Preserve Hermes' stimulus.rs + token docs (untracked)
4. Add junk to .gitignore

**Modified files to review:**
```
M  .cargo/config.toml            → Already aligned (build defaults)
M  .claude/rules/workflow.md     → Already aligned (stack docs)
M  cynic-kernel/Cargo.toml       → Already aligned (license metadata)
M  cynic-kernel/src/dogs/inference.rs  → Already aligned (phi^-1 + JSON)
M  Makefile                      → Already aligned (stack value)
M  cynic-kernel/src/api/mcp/build_tools.rs  → Already aligned (subprocess env)
M  scripts/git-hooks/pre-commit  → Already aligned (L1 validation)
M  .gitignore                    → Already aligned (categorized artifacts)
```

**Status:** ✅ COMPLETE (Claude session committed all)

**Untracked to preserve:**
```
?? cynic-kernel/src/domain/stimulus.rs         (Hermes work — DO NOT TOUCH)
?? cynic-kernel/domains/token-analysis.md      (Hermes — DO NOT TOUCH)
?? docs/reference/token-43-dimensions.md       (Hermes — DO NOT TOUCH)
?? docs/reference/token-calibration-baseline.md  (Hermes — DO NOT TOUCH)
?? docs/reference/token-stimulus-roadmap.md    (Hermes — DO NOT TOUCH)
?? .claude/session-convention-caplogy.md       (Research doc — preserve)
```

**Untracked to ignore:**
```
?? ASDFBurnTracker/, ASDev/, CultScreener/, HolDex/, asdf_grinder/  (Dirs)
?? scripts/coord-claim-gemini.sh, cynic-api.py, experiment_phi.py   (Scripts)
?? scripts/x-consumer.sh, x-interceptor.py, web/                    (Exploratory)
```

All already added to .gitignore by Claude session.

---

### TODO #4: K15 Wiring (6 Producers → Acting Consumers)

**From organism audit, 6 violations:**

| ID | Producer | Issue | Fix |
|----|----------|-------|-----|
| #16 | dream-trigger.sh counter | Increments, nobody reads | Route to Slack #cynic channel (daily scorecard) |
| #17 | session-stop.sh compliance | Printed, no action taken | Gate: fail pre-commit if compliance < 0.55 |
| #18 | observe-tool.sh observations | CCM ignores non-chess | Extend CCM domain intake logic |
| #19 | organ_quality metrics | Published, never gated | Wire to /judge circuit breaker or alert |
| #20 | session summaries | Stored, never read | Create Slack summary post per session |
| #21 | event bus /events SSE | health-watcher not wired | Subscribe health-watcher to /events |

**Your responsibility (Codex):**
- You may continue one of these (#16–#21) if you choose
- Or leave for Claude DevOps (T.'s next agent role)
- Document your choice in .handoff.md

**Each requires:**
1. Probe live state (`/observe` data, kernel health, DB state)
2. Hypothesis: "If I wire X to Y, then Z changes"
3. Experiment: Implement + test on dev
4. Measure: Before/after metrics
5. Document: Why this producer/consumer pair matters

---

## MCP Tools Available

Via `/cynic-coord` (7 tools):

| Tool | What | When |
|------|------|------|
| `cynic_coord_register` | Register yourself | Session start |
| `cynic_coord_who` | See active agents + claims | Before start work, periodically |
| `cynic_coord_claim` | Claim a file | Before every file write to kernel/ |
| `cynic_coord_release` | Release your claims | Session end |
| `cynic_coord_read_handoff` | Load context | Session start |
| `cynic_coord_write_handoff` | Update .handoff.md | Session end |
| `cynic_coord_observe` | Record a discovery | When something material happens |

---

## Real Conflict Resolution

**Scenario:** You want to edit `cynic-kernel/src/judge.rs` but it's claimed by Gemini.

**Option A: Wait**
```bash
/cynic-coord who
# Shows: Gemini claimed judge.rs, timeout in 23 min
# → Check what Gemini is doing (ask in Slack, read .handoff.md)
# → Pause your work, pick another file
```

**Option B: Coordinate offline**
```bash
# Check .handoff.md to find Gemini's session ID
# Message them in Slack: "I need to touch judge.rs, can you release?"
# They release: /cynic-coord release --target judge.rs
# You claim: /cynic-coord claim --target judge.rs
# Both edit sequentially (not parallel)
```

**Option C: Split the work**
```bash
# You edit judge.rs lines 1–100 (e.g., type defs)
# Gemini edits lines 101–500 (e.g., logic)
# Coordinate this in .handoff.md BEFORE you start
# Both claim the same file with a note: "editing different sections"
```

**Option D: Escalate (last resort)**
```bash
# If stuck: reach out to T. in Slack
# They may force-release (only for gridlock, not casual conflicts)
```

---

## Session Continuity Example

**Start of session:**
```
codex
/cynic-coord register --agent-id codex-20260415-k15 --intent "wire dream counter to Slack"
/cynic-coord read-handoff
# Reads .handoff.md, sees what Claude just did (build + Dog prompt)
# Decides: I'll do #16 (dream counter → Slack)
```

**During work:**
```
# Edit src/api/mcp/dream_tools.rs
/cynic-coord claim --target "cynic-kernel/src/api/mcp/dream_tools.rs"
# ✅ Claim succeeds
# Edit the file, commit when ready
git add src/api/mcp/dream_tools.rs
git commit -m "fix(k15): wire dream counter to Slack scorecard posting"
/cynic-coord release --target "cynic-kernel/src/api/mcp/dream_tools.rs"
```

**End of session:**
```
/cynic-coord write-handoff --message "Completed #16: dream counter routed to Slack. Next: #17 (compliance gate). Testing passed locally."
/cynic-coord release --agent-id codex-20260415-k15
git status  # Should be clean
```

---

## Epistemic Honesty

Per CLAUDE.md § II:

- **φ⁻¹ = 0.618**: Max confidence on any claim. If you can't state what falsifies your approach, you don't have one.
- **Label all claims**: *observed* (probed), *deduced* (from observed), *inferred* (pattern), *conjecture* (hypothesis).
- **Instrument your work**: Every "improved X" needs before/after numbers.
- **Falsifiable hypotheses**: "If I wire X, then Y metric changes by Z" — state what would make you reject the approach.

---

## Success Criteria

When you finish your session:

1. **TODO #3:** `git status --short` shows ONLY untracked Hermes files + research docs
2. **TODO #4:** One producer is wired to an acting consumer (or deferred with clear reason)
3. **Build:** `cargo test --lib --release` passes (476+ tests)
4. **Coordination:** All claims released, .handoff.md updated
5. **Documentation:** Your work explains the ONE problem solved per commit

---

## Don't

- ❌ Edit stimulus.rs or token-* files (Hermes work, preserve)
- ❌ Force-claim a file without coordination
- ❌ Commit without running `make check` locally
- ❌ Assert confidence > 0.618 on any diagnosis
- ❌ Add a feature without a hypothesis, experiment, and measurement
- ❌ Skip the Scientific Protocol (CLAUDE.md § Workflow)

---

## Timeline

**Now:** You (Codex) start TODO #3 + #4 work
**After you:** Claude DevOps (T.'s next agent) starts #4 remainder + #5 (peer review gates)
**Week 2:** Autonomous task delegation & multi-agent negotiation (post-robustness)

---

*The organism doesn't flow. It's waiting for YOU to wire its nervous system.*

*Good luck. Ask questions.*
