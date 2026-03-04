# GitHub Actions Workflows

## Current Status: Local CI/CD Migration

**Situation:** GitHub Actions billing was consuming unnecessary resources. Solution: **all CI/CD validation moved to local pre-commit hooks** (0% cost, faster feedback).

## Active Workflows

| Workflow | Purpose | Cost |
|----------|---------|------|
| `auto-continue.yml` | Auto-continue workflows | Free (infrequent) |
| `ci.yml` | Legacy CI (minimal) | Minimal |
| `ci-gates.yml` | Encoding validation | Minimal |
| `code-quality.yml` | Linting (non-blocking) | Minimal |
| `dependencies.yml` | Dependency updates | Free (GitHub-hosted) |
| `documentation.yml` | Docs generation | Minimal |
| `multi-platform.yml` | Cross-platform validation | Minimal |
| `pr-validation.yml` | PR checks | Minimal |
| `release.yml` | Release automation | Minimal |
| `update-status.yml` | Status dashboard | Minimal |

**Total cost:** ~$0-2/month (minimal)

## Disabled Workflows

Moved to `disabled/` — no longer running (saved ~$15-20/month):

- `tests.yml` — Unit tests now run locally on developer machine (2-3 sec)
- `debt-prevention.yml` — Complexity checks now in pre-commit hook
- `integration-tests.yml` — Integration tests run locally when needed
- `test-health-dashboard.yml` — Health report generated locally

**Why disabled?**
1. **Cost savings:** ~$15-20/month
2. **Faster feedback:** Local validation (2-3 sec) vs GitHub (3-5 min)
3. **Better DX:** Developers catch issues before commit, not after push
4. **No email spam:** Pre-commit validation prevents broken pushes

## Local CI/CD Setup

All developers must run:

```bash
bash scripts/setup-local-ci.sh
```

This installs pre-commit hooks that validate:
- ✅ 1297 unit tests pass/fail
- ✅ Coverage ≥75%
- ✅ Complexity ≤10 per function
- ✅ No stubs/NotImplementedError
- ✅ UTF-8 encoding
- ✅ No circular imports
- ✅ Factory wiring consistency
- ✅ Commit message format

See `docs/LOCAL_CI_CD.md` for complete guide.

## Re-enabling Workflows

If you need GitHub Actions again (e.g., after paying bill), restore from `disabled/`:

```bash
cp disabled/*.yml .
```

Then update as needed and push.

## Cost Comparison

| Scenario | Monthly Cost | Feedback Time | Status |
|----------|---|---|---|
| All GitHub Actions | $15-20 | 3-5 min | ❌ Disabled |
| Local pre-commit only | $0 | 2-3 sec | ✅ Active |
| GitHub Actions + Local | $2-3 | 2-3 sec + async checks | Hybrid (future) |

**Current choice: Local only = $0 cost, faster feedback.**
