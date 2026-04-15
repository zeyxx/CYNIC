---
description: Workflow triggers and tool use discipline — always active
globs: ["**"]
---

## Session Continuity

At session start, after reading injected context:
1. **Read TODO.md** — P1 items are injected by session-init.sh. Check if any are stale or completed.
2. **Read #cynic on Slack** — the human posts priorities/thoughts between sessions. Use `mcp__plugin_slack_slack__slack_read_channel` on `#cynic`, last 5 messages. Non-blocking: skip if unavailable.
3. **Probe live state** — don't trust memory. `curl /health`, `git status`, check what changed since last session.

At session end:
- Update TODO.md if any items were completed or discovered.
- session-stop.sh will warn if TODO.md was not touched.

## Pre-Commit Validation (MANDATORY for cynic-kernel/)

**BEFORE ANY `git commit` to cynic-kernel/**, run locally:

```bash
export RUST_MIN_STACK=16777216   # Required: Rust 1.94.1 compiler bug (A1, release needs 16MB)
cargo build --tests               # Must pass: 0 errors, 0 failing tests
cargo clippy --all -- -D warnings # Must pass: zero lint violations
```

If any step fails: fix root cause, re-validate, THEN commit.

**Cost:** ~2-3 min per commit. **Saves:** 4-5 failed commit attempts × (5-10 min debugging each).

**Why:** Pre-commit validation moves failures LEFT (local validation) not RIGHT (git hook rejection). A failed pre-commit validation costs 3 min to fix locally; a failed git hook costs 5-10 min (diagnose, iterate, retry). This discipline is load-bearing per kernel rules K6, K8.

**Falsifiable:** Track failed commits per session. Target: 0 failed commits post-validation.

## Workflow Triggers

BEFORE triggers — invoke PROACTIVELY before acting:
- **Before adding a module, dependency, or Cargo.toml change:** `/cynic-skills:cynic-empirical`
- **Before choosing between architecturally different approaches:** `/cynic-skills:crystallize-truth`
- **Before designing a new subsystem OR making a decision that constrains future options** (new port trait, new storage table, new API surface, new external dependency): `/cynic-skills:engineering-stack-design`
- **Before any technical decision with measurable impact:** apply the Scientific Protocol (see below)
- **Before building domain-specific features** (stimulus builders, prompts, screeners, domain endpoints): create a calibration corpus first — 3 easy-positive, 3 easy-negative, 3 ambiguous cases with expected verdicts. Run through `/cynic-skills:cynic-judge`, measure inter-Dog agreement (σ). If max_disagreement ≥ φ⁻² on easy cases, fix Dog prompts before building features.

AFTER triggers — MUST execute, no exceptions:
- **After ANY code change to cynic-kernel/:** `/build`
- **After modifying >5 files or >100 lines in a session:** `/cynic-skills:distill`
- **After changes to scoring, prompts, or crystal injection:** `/test-chess` (before AND after)

ON triggers — invoke when the situation matches:
- **Evaluate quality:** `/cynic-skills:cynic-judge`
- **Simplify/burn code:** `/cynic-skills:cynic-burn`
- **Touching kernel source:** `/cynic-kernel` (read architecture reference first)
- **Deploy to production:** `/deploy` (build + test + backup + deploy + restart + verify)
- **System status:** `/status`
- **Troubleshooting:** `/cynic-workflow`

## Scientific Protocol

Every technical decision with measurable impact follows this loop. No exceptions.
This is not a skill to invoke — it's a discipline to apply inline.

```
OBSERVE   → What is actually happening? (data, not vibes)
HYPOTHESIZE → "If I do X, then Y changes by Z" (falsifiable, one variable)
EXPERIMENT  → Do X. Measure Y before AND after.
ANALYZE   → Did Y change? By how much? Why or why not?
CONCLUDE  → Adopt / reject / modify the hypothesis.
            → Loop back to OBSERVE if inconclusive.
```

**Five questions before acting:**

1. **What do I observe?** — Concrete data. Not "it feels slow" but "p95 latency is 2.3s on /judge".
2. **What is my hypothesis?** — Falsifiable. "Adding a cache will reduce p95 below 500ms" not "caching would help".
3. **How do I test?** — The experiment. What changes, what stays constant.
4. **How do I measure?** — Before/after numbers. Same conditions. Rule 7: every "improved X" needs before/after.
5. **What would falsify this?** — The result that would make me reject the hypothesis and try something else.

**When to apply:** Any change that claims to "improve", "fix", "optimize", or "add capability". If you can't state the hypothesis, you don't understand the problem yet.

**When to skip:** Pure mechanical fixes (typos, lint, formatting), doc updates that match existing code, dependency bumps with no behavior change.

**Anti-patterns:**
- "Let's try X and see" without stating what "see" means → state the measurement first
- Measuring after but not before → worthless without baseline
- Changing 3 things at once → one variable per experiment
- "It works" as conclusion → works compared to what? By how much?

## Automatic Enforcement (hooks — no LLM action needed)

These are handled mechanically. Do NOT invoke them manually:
- **Coord claims:** `protect-files.sh` auto-claims on Edit/Write to `cynic-kernel/src/*`. Blocks on CONFLICT (409). Graceful degradation if kernel down.
- **Coord release:** `session-stop.sh` releases all claims at session end.
- **Rust formatting:** `rustfmt-rs.sh` auto-formats `.rs` files after Edit/Write.
- **Tool observation:** `observe-tool.sh` records Edit/Write/Bash/Read/Grep/Glob to `/observe` (async, fire-and-forget).
- **File protection:** `protect-files.sh` blocks access to `.ssh/`, `.env`, secret configs.
- **Dirty tree warning:** `session-stop.sh` warns on uncommitted changes at session end.

## Crystal Feedback Loop

Crystal injection changes Dog prompts — the core value proposition.
Any change to `format_crystal_context`, `observe_crystal`, or `search_crystals_semantic` MUST be benchmarked with `/test-chess` before AND after.
