# B&C CLAUDE.md — additions proposées — 2026-06-03

À ajouter dans `/home/user/Bureau/ASDFASDFA/ragnar-no-sleep/blitz-and-chill/CLAUDE.md`
après la section "Anti-Heresy Rules".

---

## Multi-Cortex Rules (MC1-MC5)

**MC1 — One branch = one session = one scope.**
Never two open sessions on the same branch. Branch naming: `feat/<scope>-YYYY-MM-DD`

**MC2 — Push before ending a session.**
At session end: `git branch -v` — any branch showing `ahead` must be pushed and a draft PR opened before closing. Unpushed commits in a parallel session = active collision risk.

**MC3 — Pull main before branching.**
`git pull origin main` before any new branch. After any merge: rebase active branches.

**MC4 — Check overlap before working.**
At session start with another session open: compare modified files (`git diff --name-only`). Any file appearing in both sessions → STOP, re-partition scope before proceeding.

**MC5 — Atomic scope.**
Each branch = 1 coherent feature or fix. Scope creep → new branch.

**Session end gate:**
```bash
git status --short   # must show 0 modified files
git branch -v        # any "ahead" → push now
```

---

## Current State (update each session)

| Item | Status |
|---|---|
| Auth (Google/Discord OAuth) | BROKEN in prod — `AUTH_GOOGLE_ID`/`AUTH_GOOGLE_SECRET` not set on Vercel |
| `feat/poh-step1` | S. scope — timing/behavioral signals, NOT merged to main |
| `worktree-feat+gamification-p0` | 5 commits local on cynic-gpu, NOT pushed |
| PR #11 `feat/poh-verify-endpoint` | T. scope — verify API endpoint, open |

---

## PoH Scope Partition (T. ↔ S.)

- `feat/poh-step1` = **S. owns** — behavioral/timing signal collection
- `feat/poh-verify-endpoint` = **T. owns** — verification API endpoint (PR #11)
- **Merge order:** `poh-step1` first (signals), then `poh-verify` enriches it
- MC4 applies: if both branches touch the same file → surface conflict, stop
