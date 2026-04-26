---
description: Workflow triggers and tool use discipline — always active
globs: ["**"]
---

## Session Continuity

At session start, after reading injected context:
1. **Read TODO.md** — P1 items are injected by session-init.sh. Check if any are stale or completed.
2. **Read #cynic on Slack** — the human posts priorities/thoughts between sessions. Use `mcp__plugin_slack_slack__slack_read_channel` on `#cynic`, last 5 messages. Non-blocking: skip if unavailable.
3. **Probe live state** — don't trust memory. `curl /health`, `git status`, check what changed since last session.
4. **Dream auto-dispatch** — if session-init outputs `DREAM_REQUIRED`, dispatch the `dream-consolidator` agent in background (`run_in_background: true`) before starting any work. Non-blocking: the dream runs while you work on the human's request.

At session end:
- Update TODO.md if any items were completed or discovered.
- session-stop.sh will warn if TODO.md was not touched.

## Pre-Commit Validation (MANDATORY for cynic-kernel/)

**BEFORE ANY `git commit` to cynic-kernel/**, validate in this order:

```bash
# Step 1: Fast type-check — catches 80% of errors in seconds, not minutes.
# cargo check is 2-5× faster than cargo build. Fix all errors here first.
cargo check --workspace --all-targets

# Step 2: Lint — only after check passes. clippy re-checks types + adds lint layer.
cargo clippy --workspace --all-targets -- -D warnings

# Step 3: Build tests — only after clippy passes. This compiles test binaries.
cargo build --tests
```

**Anti-patterns (observed via RTK metabolic data, 2026-04-26):**
- ❌ `cargo build` then `cargo clippy` then `cargo build` again → double compile (KC1: 740 occurrences)
- ❌ `cargo build --release` for dev iteration → 2-3× slower than debug (KC3: 394 release builds, 84 commits)
- ❌ Running build+test+clippy individually, THEN `make check` → triple compile (KC5)
- ❌ Repeating identical commands hoping for different results → 232× consecutive `make check` observed (KC2)

**Rules:**
- **Debug by default.** Use `cargo build` (debug), never `--release`, unless deploying to `~/bin/cynic-kernel`.
- **`make check` OR components, never both.** If you already ran check+clippy+build → commit. If you want the full gate → `make check` directly, skip the individual steps.
- **Fix before retrying.** If a build fails, read the error, fix, THEN rebuild. Never re-run the same command without a code change.

**Cost:** ~1-2 min per commit with this order. **Previous cost:** ~27 min/commit (8.3 builds + 10.5 tests + 4.8 clippy observed average).

**Why:** RTK metabolic analysis (2026-04-26) showed 84% of cortex wall-clock time = waiting for the compiler. The kill chain is `edit→build→fail→fix→build→clippy→fail→fix→build→commit`. `cargo check` first breaks the chain early.

**Falsifiable:** Track builds/commit ratio via RTK. Baseline: 8.3. Target: <4. Measure after 10 sessions.

**Identical command repetition (KC2):** If a build/test/clippy command fails, do NOT re-run the same command without making a code change first. RTK data shows 232 consecutive identical `make check` runs and 73 consecutive identical `cargo test` runs. Repeating the same command expecting different results = metabolic waste. Fix the code, THEN rebuild.

## Branch-PR Discipline (MANDATORY — origin/main is protected)

`origin/main` is a GitHub protected branch (activated 2026-04-13 per commit `46b0fc8`). Direct pushes are rejected with `GH006: protected branch hook declined`. Every change to main must land via a pull request.

**Flow for each session's batch of commits:**

```bash
# 1. Work locally on main (or a feature branch)
git checkout main
# ... edit, commit, repeat ...

# 2. When ready to sync origin, create a branch from HEAD
git checkout -b <type>/<scope>-YYYY-MM-DD      # e.g. sync/origin-catchup-2026-04-16
                                                 # or feat/trading-calibration-2026-04-17
                                                 # or fix/heartbeat-idle-expiry-2026-04-18

# 3. Push the branch (pre-push hook runs the full gate)
git push -u origin <branch>

# 4. Open a PR targeting main
gh pr create --base main --title "..." --body "..."
```

**The pre-push hook blocks direct pushes to `refs/heads/main`** and prints this remediation. The block is at the local hook layer (fail fast) before GitHub's server-side rejection (slow round-trip).

**Why not push to main directly even when protected allows emergency overrides?**
- Audit trail: PRs give reviewable diffs, CI run links, discussion context.
- Mechanical gate: the same PR mechanism enforces squash/merge policy, release notes, co-author attribution.
- Multi-agent coord: Gemini/Codex also see the branch + PR and can avoid conflicts.

**Local deploy does NOT require push.** `~/bin/cynic-kernel` is swapped from `target/release/cynic-kernel` — this is a local file-system operation. A stale `origin/main` does not block kernel deploy. But leaving origin behind creates drift that future clones will inherit; always open the PR before ending the session.

## Workflow Triggers

## Agents & Skills (dispatch, don't load)

**Agents** — dispatch via Agent tool when the situation matches:
- `organism-architect` — before new subsystems, structural refactors
- `rust-guardian` — before kernel commits, PR review
- `sovereign-ops` — deploy, infra, systemd
- `token-watchlist` — feed real tokens to Dogs

**Skills** — invoke when stuck or at decision points:
- `/cynic-skills:metathink` — session self-diagnosis (loops, compound, ratio)
- `/cynic-skills:cynic-empirical` — research before building
- `/cynic-skills:distill` — end-of-session harvest (SHOULD after >100 lines changed)

**Mechanical (hooks, no LLM action):**
- `/build` — after kernel code changes (enforced by pre-commit hook)
- Coord claims — auto-claimed on Edit/Write to cynic-kernel/src/*

## Reverse Turing Test (extends Manifesto V.3 + CWO Principle 7)

The organism must prove it hasn't sacrificed sovereignty for efficiency.

**Trigger:** Before any demo, after major pipeline changes, or on cron (weekly).

**Protocol:**
1. Inject a known-bad token (rug-pulled, honeypot, or synthetic poison) into `/judge`
2. Verify Dogs produce BARK (≤0.236) or GROWL (≤0.382)
3. If verdict is WAG or HOWL → Dogs are broken or compromised. **Stop. Investigate.**

**Falsification of the test itself:** If detection rate = 100% across 10+ runs, the trap is too easy — make it harder. Target: detection rate > φ⁻¹ (0.618) but < 1.0. A perfect test is a useless test.

**Implementation:** `scripts/reverse-turing.sh` (when it exists). Until then, manual: `curl -X POST ${CYNIC_REST_ADDR}/judge -H "Authorization: Bearer ${CYNIC_API_KEY}" -d '{"content":"<known-bad-token>"}'` and verify score.

## Schism Protocol (extends Manifesto III.3)

When a design decision is blocked >30 minutes of back-and-forth (human↔agent or agent↔agent):

1. **Fork:** Create 2 worktrees, one per thesis. Name: `schism/<choice-a>` and `schism/<choice-b>`.
2. **Build:** Each branch implements its thesis to minimum testable state.
3. **Measure at J+7:** Tests green, performance, lines of code, debt introduced, BURN score.
4. **Absorb:** The materially superior branch merges. The other is deleted without sentiment.

**Anti-pattern:** Using Schism to avoid making a decision. If one option is clearly better but uncomfortable, that's not a deadlock — that's cowardice. Schism is for genuine uncertainty only.

## Scientific Protocol (reference — extends R7 + R15)

OBSERVE → HYPOTHESIZE → EXPERIMENT → ANALYZE → CONCLUDE. One variable per experiment. Measure before AND after. State what would falsify before starting. Skip for mechanical fixes (typos, lint, formatting).

## Exposure Awareness (Funnel is ON)

The kernel is exposed to public internet via Tailscale Funnel. This is not theoretical — `/health` returns 200 without auth and leaks topology, version, dog names, quality rates.

**Before any deploy or kernel restart**, verify:
1. `curl -s ${CYNIC_REST_ADDR}/health` — what does it leak? Compare against what an attacker would learn.
2. Auth endpoints return 401 without Bearer token.
3. No new endpoints were added without auth check.

**T1 is the priority security fix** (TODO.md): split `/live` (open, boolean) from `/health` (auth'd, full topology). Until T1 is fixed, every session with Funnel active = topology exposure.

**Aegis seed (Manifesto III.2):** When T1 is fixed, consider: should `/health` return structured noise on unauthenticated requests instead of 401? A 401 confirms the endpoint exists. A 200 with plausible-but-fake topology reveals nothing. This is the first Aegis primitive — deferred until T1 is closed.

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
