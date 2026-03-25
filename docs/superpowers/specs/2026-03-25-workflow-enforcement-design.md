# Workflow Enforcement — Design Spec

*2026-03-25. Crystallized through cynic-empirical + crystallize-truth.*

## Problem

CYNIC's 33 CLAUDE.md rules are enforced by LLM compliance (~60-70% adherence). Academic research confirms context rot degrades LLM reasoning as instruction count grows. The result: constant human recalibration, post-audit corrections, code that compiles but lacks industrial rigor.

## Principle

**Every rule that can be mechanically enforced below the LLM MUST be. Only judgment rules remain in context.**

## Architecture — 8 Layers

```
ENFORCEMENT (can't bypass):
  L0: Rust Compiler — [workspace.lints], clippy.toml, deny.toml
  L1: Makefile — make check = build + test + clippy + audit + lint-rules
  L2: Git Hooks — pre-commit → cargo clippy --release, commit-msg → conventional format
  L3: Claude Code Hooks — PostToolUse rustfmt, Stop git-status, PreToolUse commit-feedback

CAPABILITY (tools available):
  L4: MCP Servers — cynic, tailscale, context7, playwright (standard MCP, cross-LLM)
  L5: Skills + Plugins — cynic-skills, superpowers, code-review (Claude-specific, adaptable)

INSTRUCTION (guides judgment):
  L6: .claude/rules/ — universal.md, kernel.md, workflow.md (scoped, organized)
  L7: CLAUDE.md — identity only (~80 lines: security, axioms, φ, dogs, infra, build notes)
```

**Agnosticism property:** L0-L2 survive any LLM client change. L4 survives via MCP standard. L3, L5-L7 are Claude-specific but adaptable.

## Crystallized Truths (from crystallize-truth analysis)

| # | Truth | Confidence | Impact |
|---|-------|-----------|--------|
| T1 | Compiler lints enforce ~30% of rules at 100% reliability | 55% | [workspace.lints] Axum-level |
| T2 | Stop hook fires every response. Heavy checks = bad UX | 58% | Stop = git status only |
| T3 | PreToolUse(git commit) is the only reliable hard gate | 55% | Primary enforcement point |
| T4 | Path scoping minimal value for single-language project | 48% | Split for organization |
| T5 | Simple hooks survive. Complex hooks become dead infra | 52% | Max 10 lines per script |
| T6 | Industry puts heavy gate at pre-commit, not Stop | 55% | Git hooks = primary |
| T7 | Subagents don't trigger project hooks | 52% | Commit gate catches all |
| T8 | Signal/noise ratio is the better frame | 55% | Less text, more mechanics |

## L0 — Rust Compiler

### [workspace.lints] in root Cargo.toml

Cherry-picked from Axum + Embark Studios, targeting AI-generated code mistakes:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
rust_2018_idioms = { level = "warn", priority = -1 }
unreachable_pub = "warn"
missing_debug_implementations = "warn"

[workspace.lints.clippy]
# Groups
correctness = { level = "deny", priority = -1 }
performance = { level = "deny", priority = -1 }
style = { level = "warn", priority = -1 }
suspicious = { level = "warn", priority = -1 }
# AI-specific (patterns LLMs frequently produce)
unwrap_used = "deny"
expect_used = "deny"
dbg_macro = "deny"
todo = "warn"
print_stdout = "warn"
print_stderr = "warn"
map_err_ignore = "warn"
fallible_impl_from = "warn"
mem_forget = "warn"
rc_mutex = "warn"
# Industrial quality
enum_glob_use = "warn"
implicit_clone = "warn"
inefficient_to_string = "warn"
manual_let_else = "warn"
needless_pass_by_value = "warn"
redundant_clone = "warn"
uninlined_format_args = "warn"
unused_self = "warn"
```

### clippy.toml

```toml
cognitive-complexity-threshold = 25
too-many-lines-threshold = 100
disallowed-methods = [
    { path = "std::process::exit", reason = "use proper shutdown via CancellationToken" },
]
```

### deny.toml (cargo-deny)

```toml
[advisories]
unmaintained = "warn"
unsound = "deny"

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

## L1 — Makefile

Harden cargo audit: remove conditional, make it a hard gate.

```makefile
# Replace conditional cargo-audit block with:
cargo audit --deny warnings
```

## L2 — Git Hooks

### .git/hooks/pre-commit

```bash
#!/bin/sh
# Fast gate: clippy (includes compile check + lints). Full tests at make check.
exec cargo clippy -p cynic-kernel --release -- -D warnings
```

Note: Full `make check` (tests + audit) is too slow for pre-commit (~2-5min with jobs=1).
Pre-commit runs clippy only (~30-60s). Full test suite runs via `make check` before deploy.

### .git/hooks/commit-msg

```bash
#!/bin/sh
MSG=$(cat "$1")
PATTERN='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|release)(\([a-z0-9._-]+\))?(!)?: .{1,100}'
if ! echo "$MSG" | grep -qE "$PATTERN"; then
  echo "ERROR: Not conventional commits format."
  echo "Expected: type(scope): description"
  exit 1
fi
```

**Setup:** Git hooks aren't tracked. Add `scripts/install-hooks.sh` that copies them.

## L3 — Claude Code Hooks

### PostToolUse: rustfmt on .rs edits

```bash
#!/bin/bash
INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[[ "$FILE" == *.rs ]] || exit 0
rustfmt "$FILE" 2>&1 || echo "WARNING: rustfmt failed on $FILE" >&2
```

Matcher: `Edit|Write`. Sync (default). Fast (<1s).

### Stop: git status warning (Rule #30)

```bash
#!/bin/bash
INPUT=$(cat)
MODIFIED=$(git -C "$CLAUDE_PROJECT_DIR" diff --name-only 2>/dev/null | head -5)
if [ -n "$MODIFIED" ]; then
  echo "WARNING: Uncommitted changes — Rule #30. Files: $MODIFIED"
fi
```

Informational only (no block). Fast (<1s).
IMPORTANT: This is ADDED to session-stop.sh, not replacing it. Coord/release must survive.

### PreToolUse: commit feedback

```bash
#!/bin/bash
INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
echo "$CMD" | grep -qE '^git commit' || exit 0
# Git pre-commit hook handles enforcement. This just gives Claude feedback.
echo "Commit gate: git pre-commit hook will run make check."
```

Note: The REAL gate is the git pre-commit hook. This Claude hook is informational.

## L6 — .claude/rules/

### universal.md (no paths — always loaded)

Rules 1-3, 6-7, 12, 14-16, 20-21, 24-31, 33 from current CLAUDE.md.
Judgment rules only. ~50 lines.

### kernel.md (paths: ["cynic-kernel/**"])

Rules 4-5, 10-11, 13, 17-19, 22-23, 32.
Rust-specific, architecture rules. ~25 lines.
Note: Rules 8, 9, 33 moved to universal.md (architectural, not Rust-specific).

### workflow.md (no paths — always loaded)

Current "Mandatory Tool Use" section with improved trigger clarity. ~30 lines.

## L7 — CLAUDE.md (~80 lines)

Keep: Security, Ownership Zones, Live Infrastructure, API Essentials, Axioms, φ Constants, Dogs, Canonical References, Build Notes.
Remove: Development Principles (→ rules/), Mandatory Tool Use (→ rules/).
Add: Pointer to .claude/rules/.

## Agent — kernel-auditor.md

```yaml
name: kernel-auditor
description: "CYNIC kernel auditor. Use PROACTIVELY after modifying cynic-kernel/ code."
model: sonnet
tools: Read, Grep, Glob
memory: project
```

## Future — Skill Router (NOT this session)

Design for embedding-based skill routing:
- CYNIC kernel endpoint: `POST /route-skill` with prompt text
- Sovereign embedding model encodes prompt + skill descriptions
- Cosine similarity → top skill suggestion
- Delivered via UserPromptSubmit HTTP hook → additionalContext
- Connects to session analysis pipeline (transcript mining)

## Review Fixes (from code-reviewer audit)

- C1: PostToolUse hooks APPENDED to existing array, not replaced. observe-tool.sh survives.
- C2: Pre-commit = `cargo clippy --release` only (~30-60s), not full `make check` (~2-5min).
- C3: Sweep for dbg!() before enabling workspace lints. Document test-exemption pattern.
- C4: cargo-deny deferred (not installed, guard needed). deny.toml is design reference.
- I1: Rules 8, 9, 33 → universal.md (architectural scope, not Rust-specific).
- I2: Stop git-status warning ADDED to session-stop.sh, not replacing coord/release.
- I3: kernel-auditor = manual @-mention only. No auto-trigger pretense.
- I4: exit removed from workspace deny. Kept in clippy.toml disallowed-methods only.
- S1: cognitive-complexity 30 → 25.
- S3: make check warns if .git/hooks/pre-commit absent (bootstrap gap).
- S4: rustfmt failure logged, not swallowed.

## Implementation Order

1. L0: [workspace.lints] + clippy.toml (sweep dbg!/unwrap first, then enable)
2. L1: Makefile cargo audit hard gate + hook-check warning
3. L2: Git hooks (pre-commit + commit-msg + install script)
4. L3: Claude hooks (rustfmt, git-status in session-stop.sh, commit-feedback) — APPEND to existing
5. L6-L7: CLAUDE.md slim + .claude/rules/ (universal.md, kernel.md, workflow.md)
6. Agent: kernel-auditor.md (manual @-mention)
7. Docs reorg + permissions cleanup
