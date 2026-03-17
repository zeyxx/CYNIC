# CYNIC Workflow — Troubleshooting & Process Detail

Use when things go wrong, when coordinating with other agents, or when you need the full session lifecycle detail.

**The routing (BEFORE/AFTER/ON triggers) lives in CLAUDE.md. This skill covers the HOW and the WHAT-IF.**

---

## Session Lifecycle (detail)

Agent ID is in the session banner: `Agent: claude-XXXXXXXXXX`

```
1. SessionStart    → hook auto-registers (verify in output: "Agent: claude-XXXX")
2. Coord claim     → cynic_coord_who() + cynic_coord_claim(agent_id="claude-XXX", target="file.rs")
3. Worktree        → make scope SLUG=<name> → creates ../cynic-<slug>/ + branch session/<user>/<slug>
4. Work            → cd ../cynic-<slug>/ — main checkout stays clean
5. Validate        → make check (build + test + clippy --release)
6. Ship            → git commit + git push (L0 gates: gitleaks, build, test)
7. Release         → cynic_coord_release(agent_id, target="file.rs") — one file at a time as work completes
8. Clean up        → make done SLUG=<name> — removes worktree + branch
9. SessionEnd      → cynic_coord_release(agent_id) — no target = releases ALL claims
```

**Two enforcement layers:**
- **Git branch uniqueness** = hard enforcement. Duplicate branch names are physically rejected.
- **Coord claims** = soft enforcement (visibility). Prevents wasted work, not merge conflicts.

---

## Coordination Conflicts

If `cynic_coord_claim` returns `CONFLICT`:
1. `cynic_coord_who()` — identify the holder
2. Check TTL — claims auto-expire after 5 minutes of inactivity
3. If expired → retry claim (auto-succeeds)
4. If active → coordinate: split scope, serialize work, or handoff
5. Never override silently

If two agents edit the same file without coordination:
→ Merge conflict at push time. The coordination layer prevents this BEFORE writing.

---

## Escalation Protocol

**2 fix attempts max. Then escalate.**

| Attempt | Action |
|---------|--------|
| 1st | Obvious fix based on error message |
| 2nd | Alternative approach (different strategy, not retry) |
| 3rd | **STOP.** Ask the human. Explain what you tried and why it failed. |

### Escalation triggers (stop and ask immediately)
- Same error after 2 different fix attempts
- Change affects 3+ unrelated files
- Pre-commit hook fails for unclear reasons
- Fix in file A breaks tests in file B
- Tempted to use `--force` anything
- Unsure if a change is safe to deploy

---

## Troubleshooting

| Symptom | Check | Fix |
|---------|-------|-----|
| Kernel down | `/status` or `systemctl --user status cynic-kernel` | `/run` to restart |
| Dogs missing | Check session-init output (`Dogs: X/Y`) | Check backend health, restart llama-server |
| DB unreachable | `surreal is-ready --endpoint http://localhost:8000` | `systemctl --user restart surrealdb` |
| Build LLVM crash | SIGSEGV during compilation | `rustup toolchain uninstall <ver> && rustup toolchain install <ver>` |
| Tests fail after change | `cargo test -p cynic-kernel --release 2>&1` | Read the error. Diagnose before fixing. |
| Push rejected | L0 gates (gitleaks, build) | Fix the issue. Never `--no-verify`. |
| Stale coordination claims | `cynic_coord_who()` shows old agents | Claims auto-expire (5min TTL). Wait or investigate. |

---

## Commit Format

```
type(scope): description

Types: feat, fix, docs, refactor, test, chore, perf
One logical change per commit. Message explains WHY, not WHAT.
```

---

## Commands Quick Reference

| Command | What |
|---------|------|
| `make check` | Build + test + clippy (--release) |
| `make deploy` | Build + ship + deploy binary + restart + verify |
| `make scope SLUG=<name>` | Create ILC worktree + branch |
| `make done SLUG=<name>` | Clean up worktree + branch |
| `make agents` | Show active coordinated agents |
| `make backup` | Backup SurrealDB |
| `make status` | Full system dashboard |
| `make e2e` | End-to-end test against running kernel |
