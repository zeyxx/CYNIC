# Multi-Session Development Guide

A comprehensive guide for developing CYNIC across multiple sessions, maintaining isolation, and coordinating work through dashboards.

---

## 1. Overview

Multi-session development allows multiple developers (or the same developer across time) to work on independent features, priorities, and bug fixes without blocking each other. The CYNIC project uses:

- **Git branches** for feature isolation
- **Git worktrees** for parallel work in the same codebase
- **Event-driven architecture** for safe concurrent operations
- **Status dashboards** (docs/status.md) for coordination
- **Priority system** for sequencing delivery

Each session is independent but coordinated through:
- Documented blockers
- Shared memory files
- Dashboard synchronization
- Commit message conventions

### Why Multi-Session Development?

- ✅ Developers work independently without branch conflicts
- ✅ Feature branches are isolated and mergeable at any time
- ✅ Historical context is preserved in docs/ and MEMORY.md
- ✅ Unknown blockers surface early (before pushing)
- ✅ Long-running tasks don't block others

---

## 2. Quick Start (5 minutes)

### Option A: Start a New Feature/Priority Session

```bash
# 1. Pull latest from master
cd ~/cynic  # or your repo path
git fetch origin
git checkout master
git pull origin master

# 2. Create a new branch
git checkout -b feature/my-feature

# 3. (Optional) Create an isolated worktree
git worktree add .claude/worktrees/my-feature -b feature/my-feature

# 4. Start developing
cd .claude/worktrees/my-feature  # if using worktree
# ... make changes ...

# 5. Commit and push when ready
git add files/you/changed
git commit -m "feat(my-priority): Description of what you did"
git push -u origin feature/my-feature
```

### Option B: Clone an Existing Session/Branch

```bash
# 1. List available branches
git branch -a

# 2. Check out the branch
git checkout --track origin/feature/existing-feature

# 3. (Optional) Create worktree for this branch
git worktree add .claude/worktrees/existing -b feature/existing-feature

# 4. View prior session's context
cat docs/plans/YYYY-MM-DD-priority-N-description.md

# 5. Continue development
# ... make your changes ...

# 6. Push your changes
git push origin feature/existing-feature
```

### ℹ️ Key Points

- Always pull latest master before starting
- Branch names follow: `feature/{priority-number}-{short-description}`
- Worktrees live in `.claude/worktrees/` (git-ignored)
- Check docs/status.md for what's in progress

---

## 3. Workflow: From Task to Merge

A typical session follows 6 steps from isolated development to merged code.

### Step 1: Isolate

Ensure your session has no conflicts with ongoing work.

```bash
# Check current branch
git branch

# If needed, create new branch from latest master
git checkout master
git pull origin master
git checkout -b feature/priority-N-description

# List session isolation checklist:
# ✅ On correct branch
# ✅ No uncommitted changes
# ✅ No upstream changes to merge (git log master..HEAD is empty before you start)
```

**Verification:**
```bash
git status  # Should be clean
git log master..HEAD  # Should be empty (or show your new commits only)
```

### Step 2: Develop

Make changes, test, commit incrementally.

```bash
# Make changes to your files
vim src/module.py

# Run tests frequently
pytest tests/

# Commit with semantic messages
git add src/module.py
git commit -m "feat(priority-N-p1): Add new feature component"

# Repeat: make changes → test → commit
```

**Best Practices:**
- ✅ Commit frequently (logical units, not monolithic)
- ✅ Run tests before each push
- ✅ Check lint/format if applicable (pre-commit hooks)
- ❌ Don't commit half-finished features
- ❌ Don't push directly to master

### Step 3: Push

Push your branch to origin for visibility and backup.

```bash
# First push (creates tracking)
git push -u origin feature/priority-N-description

# Subsequent pushes
git push

# Verify pushed
git log origin/feature/priority-N-description..HEAD  # Should be empty
```

**Expected Outcome:**
- Branch appears in `git branch -a`
- GitHub shows branch with your commits
- CI/CD pipeline starts (if configured)

### Step 4: Review

If applicable, create a pull request and get feedback.

```bash
# Create PR via GitHub/GitLab (or gh CLI)
gh pr create \
  --title "feat: Add new feature component" \
  --body "Fixes #123. Adds X functionality to Y module."

# Address review feedback
# (if needed: make changes, commit, push)

# Get approval before merge
```

**No PR Needed If:**
- Direct merge to master is your workflow
- Feature is small and hotfix-like
- You're working solo in an experimental branch

### Step 5: Merge

Merge to master (or target branch) when approved.

```bash
# Switch to target (usually master)
git checkout master
git pull origin master

# Merge your branch
git merge feature/priority-N-description

# Or rebase (cleaner history)
git rebase origin/master
git checkout master
git merge feature/priority-N-description

# Push merged result
git push origin master
```

**Verify Merge Success:**
```bash
git log master -1  # Should show your commit
git log master | grep "priority-N"  # Should find your commits
```

### Step 6: Cleanup

Remove your branch to keep repository tidy.

```bash
# Delete local branch
git branch -d feature/priority-N-description

# Delete remote branch
git push origin --delete feature/priority-N-description

# Remove worktree (if created)
git worktree remove .claude/worktrees/my-feature

# Verify cleanup
git branch -a  # Your branch should not appear
```

**Documentation Cleanup:**
- Update `docs/status.md` to mark feature complete
- Archive relevant plan docs if needed
- Update MEMORY.md with session summary

---

## 4. Handling Blockers

### Scenario 1: Unknown Dependency (External Blocker)

**Symptom:** You're blocked on something outside your control (e.g., another priority, external API, waiting for review).

**Solution:**

```bash
# 1. Document the blocker
cat >> docs/status.md << 'EOF'

## Blocker: Priority-N waiting on Priority-M

- **Task:** Add feature X
- **Blocker:** Depends on Priority-M's API redesign (not yet complete)
- **Estimated Resolution:** 2026-03-05
- **Workaround:** None (hard dependency)

EOF

# 2. Push blocker doc for visibility
git add docs/status.md
git commit -m "docs: Document blocker for priority-N"
git push origin feature/priority-N-description

# 3. Switch to a different task while waiting
git checkout master
git checkout -b feature/priority-M-alternative

# 4. Return to original task when blocker resolves
# (check docs/status.md for updates)
```

### Scenario 2: Circular Dependency (Two Priorities Block Each Other)

**Symptom:** Priority-A needs Priority-B's code, but Priority-B needs Priority-A's interface.

**Solution:**

```bash
# 1. Identify the minimal subset needed by both
#    Example: Define shared interface/contract

# 2. Create intermediate branch with just the interface
git checkout master
git checkout -b feature/priority-shared-interface
# ... add interface only, no implementation ...
git push -u origin feature/priority-shared-interface

# 3. Both priorities merge this branch first
git merge feature/priority-shared-interface

# 4. Each priority continues with implementation
git commit -m "feat(priority-A-p1): Implement using shared interface"

# 5. Merge both when ready
git checkout master
git merge feature/priority-A-description
git merge feature/priority-B-description
```

**Prevention:**
- ✅ Review task dependencies before starting
- ✅ Communicate interfaces early (docs/TASK_NAME.md)
- ✅ Break circular deps into smaller, independent pieces

### Scenario 3: Merge Conflict

**Symptom:** `git merge` fails with conflicts in files you both modified.

**Solution:**

```bash
# 1. Start merge (may fail)
git merge origin/feature/other-priority

# 2. View conflicts
git status  # Shows conflicted files
git diff    # Shows conflict markers (<<<<, ====, >>>>)

# 3. Resolve manually
vim src/conflicted_file.py
# ... remove <<<<, ====, >>>> markers ...
# ... keep your logic + their logic (or choose one) ...

# 4. Mark resolved and continue merge
git add src/conflicted_file.py
git commit -m "merge: Resolve conflicts with priority-X"

# 5. Verify result
git log --oneline -5
```

**Prevention:**
- ✅ Pull latest master frequently
- ✅ Communicate when modifying shared files
- ✅ Use feature branches (not long-lived master edits)
- ❌ Don't force-push after merge (breaks collaboration)

---

## 5. Tips & Best Practices

### ✅ DO

| Practice | Example | Benefit |
|----------|---------|---------|
| Commit frequently | `git commit -m "feat: Add parser logic"` | Easy to revert bad changes; clear history |
| Use semantic commit messages | `feat(priority-5): ...` `fix(priority-8): ...` | Readers understand what changed and why |
| Test before pushing | `pytest && git push` | Catch failures early; no broken master |
| Document blockers | Update docs/status.md | Others unblocked; you remember context later |
| Keep branches focused | One feature per branch | Easy to review, merge, and revert |
| Pull before pushing | `git pull && git push` | Fewer merge conflicts |
| Use worktrees for isolation | `git worktree add .claude/worktrees/feat` | Work on multiple features in parallel |
| Review your own PR first | Read your changes before requesting review | Catch typos and logic errors early |

### ❌ DON'T

| Antipattern | Why It's Bad | Better Alternative |
|-------------|------------|-------------------|
| Commit to master directly | No isolation; blocks others; hard to revert | Use feature branches always |
| `git push --force` on shared branches | Loses others' work; breaks history | Use `git push` (normal) or rebase locally first |
| Huge commits with many changes | Hard to review; hard to bisect; scary to merge | Break into logical, testable commits |
| Merge master into feature (repeat) | Creates complex history | Use `git rebase master` once, or merge feature→master once |
| Leave uncommitted changes long-term | Easy to lose; hard to remember what they were | Commit frequently (or stash if truly WIP) |
| Work directly on master | Blocks others; no undo if you push broken code | Always use feature branches |
| Ignore test failures | Bad code in master; breaks others' work | Fix tests before pushing; use CI/CD gates |
| Forget to document blockers | Others waste time investigating; you forget context | Update docs/status.md immediately |

---

## 6. Dashboard: docs/status.md

The status dashboard is your hub for coordinating multi-session work. It answers:
- What's currently in progress?
- What's blocked and why?
- When will each priority finish?
- Which branch should I review?

### Structure

```markdown
# CYNIC Status Dashboard — [Current Date]

## Legend
- 🟢 Complete
- 🟡 In Progress
- 🔴 Blocked
- ⚪ Not Started

## Summary

| Priority | Task | Status | Owner | Branch | ETA | Notes |
|----------|------|--------|-------|--------|-----|-------|
| P1 | Unified error handling | 🟢 | Alice | merged | 2026-02-15 | In master |
| P2 | Recovery protocols | 🟢 | Bob | merged | 2026-02-20 | In master |
| P5 | Event protocol | 🟡 | Charlie | feature/priority-5-events | 2026-03-03 | 80% done |
| P9 | Metrics bridge | 🔴 | Diana | feature/priority-9-metrics | 2026-03-08 | Blocked on P8 |

## Active Blockers

### Blocker: P9 waiting on P8

- **Task:** Metrics bridge (analyze aggregate metrics across services)
- **Dependency:** P8's SelfProber proposal generation (needs metrics payload structure)
- **Status:** P8 in final review; merging today (2026-03-02)
- **Unblock:** Once P8 PR #456 merges, P9 can proceed
- **Workaround:** None (hard dependency)

## Recent Changes

- 2026-03-01: P8 SelfProber metrics integration complete (16 tests passing)
- 2026-03-01: Stabilization priority complete (5 phases, 53 tests)
- 2026-02-28: P7 event metrics + anomaly detection merged
```

### How to Update It

1. **At start of session:** Check docs/status.md for blockers or in-progress work
   ```bash
   cat docs/status.md | head -30
   ```

2. **When you begin work:** Update your task's status
   ```bash
   # Edit docs/status.md
   # Change: | P5 | Your task | ⚪ Not Started | - | - | - |
   # To:     | P5 | Your task | 🟡 In Progress | YourName | feature/priority-5-task | 2026-03-05 |

   git add docs/status.md
   git commit -m "docs: Mark priority-5 as in progress"
   git push
   ```

3. **When blocked:** Add blocker section with ETA
   ```bash
   # Add section to "Active Blockers"
   # Document:
   # - What you're blocked on
   # - Why (dependency, external issue, etc.)
   # - Estimated unblock date
   # - Any workarounds

   git add docs/status.md
   git commit -m "docs: Document blocker for priority-N"
   git push
   ```

4. **When complete:** Mark as done, merge to master
   ```bash
   # Update table: 🟡 → 🟢, add merged date
   # Remove from "Active Blockers" if listed

   git add docs/status.md
   git commit -m "docs: Mark priority-N as complete"
   git push
   ```

### Reading the Dashboard

- **By status:** What needs attention? (🔴 blockers come first)
- **By owner:** Who's working on what? (avoid duplicate effort)
- **By ETA:** What finishes soonest? (prioritize unblocking tasks)
- **By branch:** Which PR should I review? (use branch link to check GitHub)

---

## 7. Troubleshooting

### Issue 1: "Your branch is ahead of 'origin/master' by 5 commits"

**Cause:** You have local commits not yet pushed.

**Fix:**
```bash
# Option A: Push (if commits are ready)
git push origin feature/my-feature

# Option B: Check commits first (if unsure)
git log origin/feature/my-feature..HEAD
# Review commits before pushing
git push origin feature/my-feature
```

**Prevention:** Push frequently (after each test pass).

---

### Issue 2: "branch has diverged from remote-tracking branch"

**Cause:** Someone else pushed to your branch (or you force-pushed locally then pulled).

**Fix:**
```bash
# Rebase onto remote version
git fetch origin
git rebase origin/feature/my-feature

# Or merge (if rebase isn't preferred)
git merge origin/feature/my-feature

# Push your version
git push origin feature/my-feature
```

**Prevention:** Don't force-push to shared branches; communicate with collaborators.

---

### Issue 3: "Merge conflict in setup.py"

**Cause:** Multiple people edited the same file.

**Fix:**
```bash
# 1. Start merge
git merge origin/feature/other-priority

# 2. Open conflicted file
vim setup.py
# Look for: <<<<< HEAD ... ===== ... >>>>> origin/feature/other-priority
# Keep correct version (usually both changes, merged manually)

# 3. Resolve
git add setup.py
git commit -m "merge: Resolve setup.py conflicts"

# 4. Test to ensure nothing broke
pytest

# 5. Push
git push
```

**Prevention:**
- Communicate before modifying shared files (setup.py, __init__.py, etc.)
- Keep changes small and focused
- Pull frequently before pushing

---

### Issue 4: "I accidentally committed to master"

**Cause:** Forgot to create a feature branch; made commits directly to master.

**Fix:**
```bash
# 1. Find the commit hash before your changes
git log master -10  # Find the "bad" commit

# 2. Create a new branch from that commit
git branch feature/priority-N abc1234  # abc1234 = hash before your changes

# 3. Reset master to clean state
git reset --hard origin/master

# 4. Switch to your feature branch
git checkout feature/priority-N

# 5. Push feature branch
git push -u origin feature/priority-N

# Now create PR from feature/priority-N to master
```

**Prevention:** Always work on feature branches; protect master with branch rules.

---

### Issue 5: "tests pass locally, but CI/CD failed"

**Cause:** Environment differences (Python version, dependencies, OS).

**Fix:**
```bash
# 1. Check CI/CD log for specific error
# (on GitHub: Actions tab → your workflow → see error message)

# 2. Match environment locally
python --version  # Should match CI's Python
pip list | grep pytest  # Should match requirements-dev.txt

# 3. Run exact CI test command
# (copy from .github/workflows/ci.yml)
pytest --cov=cynic tests/

# 4. Fix in code
# Make changes to handle the error

# 5. Commit and push
git add <files>
git commit -m "fix: Handle CI failure in test"
git push
```

**Prevention:**
- Run CI test command before pushing
- Use virtual environments (match CI exactly)
- Check CI logs early (don't ignore red ❌)

---

### Issue 6: "how do I roll back a bad merge?"

**Cause:** You merged a bad commit to master; need to undo.

**Fix:**
```bash
# 1. Find the merge commit
git log master --oneline -10
# Example: abc1234 Merge pull request #123

# 2. Create revert commit (undoes the merge)
git revert -m 1 abc1234

# 3. Review what revert did
git log master -1 --stat  # Shows files that reverted

# 4. Push the revert
git push origin master

# ℹ️ Note: The bad commit is still in history (good for audit)
# If you want to hide it: git reset --hard origin/master (⚠️ destructive!)
```

**Prevention:**
- Require PR reviews before merge
- Run CI/CD before merging
- Use branch protection rules

---

## 8. Communication

### Updating Dashboard When You Need Help

If you're blocked and need input from others:

```bash
# 1. Document clearly in docs/status.md
cat >> docs/status.md << 'EOF'

### Blocker: Priority-N needs design review

- **Task:** Implement feature X
- **Blocker:** Uncertain on architecture (event vs. polling model)
- **Status:** Waiting for team discussion
- **Details:** See docs/plans/YYYY-MM-DD-priority-N-design.md
- **Unblock:** Design decision needed by 2026-03-05
- **Workaround:** Can proceed with polling model (less optimal but unblocks)

EOF

# 2. Commit and push (make it visible)
git add docs/status.md
git commit -m "docs: Request design review for priority-N"
git push

# 3. Notify team (Slack, email, GitHub issue)
# Link to PR or branch, mention blocker doc
```

### Providing Status Updates to Team

At end of session or when merging:

```bash
# 1. Update MEMORY.md with session summary
cat >> docs/MEMORY.md << 'EOF'

## SESSION 6L: PRIORITY-N FEATURE [DATE]

**Status:** Priority-N [COMPLETE/IN PROGRESS]
**Scope:** [1-2 sentence description]
**Achievement:** [What you accomplished]
**Tests:** [X passing, Y new]
**Files:** [N created, M modified]
**Commits:** [Brief summary of commits]

### Key Accomplishments

- [Bullet 1]
- [Bullet 2]

### Blockers/Next Steps

- [Any remaining work or dependencies]

EOF

# 2. Commit with clear message
git add docs/MEMORY.md
git commit -m "docs(session-6L): Priority-N feature summary"
git push

# 3. Communicate merge to team
# (PR description, commit message, or team message)
```

### Asking for Code Review

```bash
# 1. Push your branch
git push -u origin feature/priority-N-description

# 2. Create PR (or mention branch in chat)
gh pr create \
  --title "feat(priority-N): Add feature description" \
  --body "## Summary

Adds X functionality to Y module.

## What Changed
- Added A
- Modified B
- Fixed C

## Tests
- 10 new tests passing
- All 88 existing tests passing

## Blockers
None

## Reviewers
@alice @bob (please review)
"

# 3. Link in status dashboard
# Update docs/status.md with PR link
```

---

## Next Steps

1. **Read this guide:** Done! ✅
2. **Create your first branch:** Follow "Quick Start" (Option A or B)
3. **Check docs/status.md:** Know what's in progress and blocked
4. **Make your first commit:** Follow "Workflow: Step 2"
5. **Push and coordinate:** Update dashboard, get feedback
6. **Merge confidently:** You have a full workflow now

**Questions?** Check "Troubleshooting" or update docs/status.md with blocker.

---

## Glossary

| Term | Definition |
|------|-----------|
| **Branch** | Isolated copy of code; lives on both local machine and remote (GitHub) |
| **Worktree** | Lightweight checkout of a branch; allows working on multiple branches simultaneously |
| **Merge** | Combine one branch into another (e.g., feature → master) |
| **Rebase** | Re-apply commits on top of a different base; cleaner history than merge |
| **Conflict** | Two commits modified the same lines; manual resolution needed |
| **CI/CD** | Automated tests and deployment pipeline (runs on every push) |
| **Blocker** | External dependency preventing progress; documented in status.md |
| **Feature branch** | Branch for one feature/priority; merged once complete |
| **Master** | Main branch; always production-ready (hopefully!) |

---

## References

- [CYNIC Architecture Overview](../README.md)
- [Priority System & Roadmap](../docs/status.md)
- [Memory & Session Context](../docs/MEMORY.md)
- [Git Basics](https://git-scm.com/book/en/v2)

---

**Last Updated:** 2026-03-02
**Version:** 1.0
**Maintainer:** CYNIC Development Team
