# CYNIC Multi-Session Coordination

## Quick Reference

- **Worktrees:** Use `claude code --worktree feature/priority-X-pY` to start isolated sessions
- **Pre-commit Gates:** Automatic validation (encoding, imports, factory, tests, message format) on commit
- **CI/CD:** GitHub Actions runs full test suite on PR (3-4 min), blocks merge if failed
- **Dashboard:** Check `docs/status.md` for active sessions and blockers
- **Merge:** Requires 1 code review approval + CI/CD passing + all gates

## Session Workflow (TL;DR)

1. **Start:** `claude code --worktree cli/priority-10-p4`
2. **Develop:** Make changes, commit (gates validate)
3. **Push:** `git push -u origin cli/priority-10-p4`
4. **Wait:** CI/CD runs (~3 min)
5. **Review:** Ask for 1 approval
6. **Merge:** Click merge button (on GitHub) or `git merge` + `git push`
7. **Cleanup:** Optional worktree cleanup

## Pre-Commit Gates (What Gets Validated)

| Gate | Time | What | Failure |
|------|------|------|---------|
| Encoding | 45s | UTF-8 integrity, no BOM | Block commit |
| Imports | 30s | No circular deps | Block commit |
| Factory | 20s | Wiring consistency | Block commit |
| Tests | 90s | Unit + coverage | Block commit |
| Message | 10s | Commit format | Block commit |

## CI/CD Gates (GitHub Actions)

| Job | Time | What | Failure |
|-----|------|------|---------|
| Test | 3min | Unit + integration tests | Block merge |
| Architecture | 2min | No new debt, no cycles | Block merge |
| Performance | 1min | TPS, memory, latency | Warn (don't block) |
| Build | 30s | Import validation | Block merge |

## Decision Logging

Every merge commit must include:

```
Impacts: Which other priorities are affected
Breaking: Yes/No (and migration path if yes)
Tech Debt: -1 (improved) / 0 (neutral) / +1 (introduced)
Tested: List of new tests
```

This helps future sessions understand the impact of your changes.

## Handling Conflicts

**Scenario:** Two sessions modify the same file.

Git will detect this when merging. To resolve:

1. **Pull main locally:**
   ```bash
   git pull origin main
   ```

2. **Rebase on main:**
   ```bash
   git rebase origin/main
   ```
   Git will show conflicts.

3. **Edit conflicted files:**
   ```bash
   vim cynic/path/to/conflicted_file.py
   ```
   Look for `<<<<< HEAD` and `>>>>> main` markers. Resolve them.

4. **Continue rebase:**
   ```bash
   git add cynic/path/to/conflicted_file.py
   git rebase --continue
   ```

5. **Push:**
   ```bash
   git push --force-with-lease origin your-branch
   ```
   (Only on your own branch, never main!)

## When Blocked

If you're waiting for another session:

1. **Check dashboard:** `cat docs/status.md`
2. **See what's blocking:** "Waiting for PR #XX to merge"
3. **Help unblock:** Review that PR, provide feedback
4. **Pull and continue:**
   ```bash
   git pull origin main
   # Continue your work
   ```

## Resources

- **Guide:** `docs/MULTI_SESSION_GUIDE.md` (detailed walkthrough)
- **Design:** `docs/plans/2026-03-02-multi-session-coordination-design.md` (architecture details)
- **Dashboard:** `docs/status.md` (real-time status)
- **Scripts:** `scripts/` (validate_encoding, analyze_imports, audit_factory, etc.)
