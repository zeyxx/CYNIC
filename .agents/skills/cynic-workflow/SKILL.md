---
name: cynic-workflow
description: "Git/Forgejo/deploy workflow for CYNIC V2. Agent boundaries, branch protection, commit conventions, pipeline stages, escalation. Use when pushing, committing, deploying, creating PRs, or when agent actions need guardrails. Triggers on: 'push', 'deploy', 'commit', 'PR', 'pipeline', 'branch', 'merge', 'forgejo'."
---

# CYNIC Workflow — The Pipeline Is the Guardrail

*"No shortcut survives production."*

## Source of Truth

**Forgejo** (self-hosted, forge:3000) is the single source of truth. GitHub is a read-only mirror.

```
Local (Windows/Linux) ──git push forgejo──▶ Forgejo (forge:3000)
                                              │
                                              ├── post-receive hook
                                              │     └── systemd-run → validate + deploy
                                              │
                                              └── GitHub mirror (read-only)
```

## Access Tiers

| User | Can push to | Can merge PRs | Can push main |
|------|------------|---------------|---------------|
| **kairos** (human) | Any branch | Yes | Yes |
| **claude-agent** (AI) | `feature/*` only | No | No |

**Rule:** Agents propose via feature branches + PRs. Humans review and merge. No exceptions.

## Branch Strategy

```
main ─────────────────────────────────────────── production
  └── feature/description ──── agent or human work
        └── PR → review → merge to main
```

- `main` = always deployable. Protected: only kairos can push/merge.
- `feature/*` = all work happens here. Named descriptively: `feature/circuit-breaker`, `feature/probe-refactor`.
- No `develop` branch. No `release/*`. Trunk-based with feature branches.

## Commit Conventions

Format: `type(scope): description`

```
feat(probe): add GPU detection via GpuDetector trait
fix(router): circuit breaker cooldown timer reset
docs(spec): tailscale-mcp design document
refactor(hal): extract inference pipeline from MuscleHAL
test(backend): port contract tests for InferencePort
chore(deps): bump rmcp to latest for MCP stability
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`

**Rules:**
- One logical change per commit
- Message explains WHY, not WHAT (the diff shows what)
- Never `--no-verify` — if hooks fail, fix the cause
- Never `--force-push` to main

## Pipeline Stages

Triggered by push to main via Forgejo post-receive hook:

```
1. VALIDATE
   ├── cargo clippy --all-targets -- -D warnings
   ├── cargo test
   └── cargo audit (if available)

2. BUILD
   └── cargo build --release -p cynic-kernel

3. DEPLOY (atomic)
   ├── systemctl --user stop cynic-kernel
   ├── cp binary.new → mv -f binary (atomic replacement)
   └── systemctl --user start cynic-kernel

4. HEALTH CHECK
   └── curl -s http://localhost:3030/health (with retries)

5. MIRROR
   └── git push github main (read-only mirror)
```

**If any stage fails:** pipeline stops, service stays on previous version, error logged to journalctl.

## Deploy Safety

| Problem | Prevention |
|---------|-----------|
| Text file busy (ETXTBSY) | Stop service BEFORE binary copy. Atomic cp→mv. |
| Port already in use (AddrInUse) | Stop service with TimeoutStopSec=30. RestartSec=3 prevents race. |
| Broken binary deployed | Health check after start. If fails, previous binary is gone — investigate via journalctl. |
| Secrets in code | EnvironmentFile for secrets. Never commit `.env` or passwords. |

## Agent Guardrails

When operating as an AI agent in this repo:

### BLOCKED commands (never execute)
- `git push --force` (any variant)
- `git reset --hard` on shared branches
- `git commit --no-verify`
- Direct push to `main`
- `rm -rf` on repo directories
- Any command that modifies Forgejo admin settings

### REQUIRED before committing
1. Run `cargo clippy` — fix all warnings
2. Run `cargo test` — all tests pass
3. Verify commit message matches convention
4. Stage specific files (not `git add -A`)

### Escalation triggers
Stop and ask the human when:
- 2 fix attempts failed on the same issue
- Change affects 3+ unrelated files
- Pre-commit validation fails for unclear reasons
- A fix in file A breaks tests in file B
- Tempted to use `--force` anything

## Environment

```
Forge (kairos):
  - Forgejo: http://forge:3000
  - cynic-kernel: http://localhost:3030 (REST) — gRPC on [::1]:50051 feature-gated, off by default
  - SurrealDB: ws://localhost:8000
  - Deploy: ~/.config/systemd/user/cynic-kernel.service
  - Logs: journalctl --user -u cynic-kernel

Windows (desktop):
  - llama-server: http://desktop:11435 (OpenAI-compatible)
  - Development: git remote forgejo → http://forge:3000/kairos/CYNIC.git

Network: Tailscale VPN between nodes
```

## Quick Reference

```bash
# Push feature branch
git push -u forgejo feature/my-change

# Check pipeline status (on forge)
journalctl --user -u cynic-validate-* --since "5 min ago"

# Check kernel health
curl -s http://localhost:3030/health

# View deploy logs
journalctl --user -u cynic-kernel -f

# Mirror to GitHub manually
git push github main
```
