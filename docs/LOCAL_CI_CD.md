# Local CI/CD — Zero Cost Validation (Cost: $0/month)

## The Problem

GitHub Actions billing was consuming monthly budget unnecessarily. Solution: **shift CI/CD validation from GitHub to developer machines** (actually faster + cheaper).

## The Solution: Pre-commit Hooks

All code quality validation now runs **locally on your machine** before you even push to GitHub.

### What Gets Validated

| Check | Tool | Threshold | Cost |
|-------|------|-----------|------|
| **Unit Tests** | pytest | 1297 tests pass or fail | $0 |
| **Code Coverage** | pytest-cov | ≥75% minimum | $0 |
| **Complexity** | radon | ≤10 cyclomatic per function | $0 |
| **Stubs** | regex | None in core code | $0 |
| **Encoding** | custom | UTF-8 validated | $0 |
| **Imports** | custom | No circular dependencies | $0 |
| **Factory** | custom | Wiring consistency | $0 |
| **Commit Message** | custom | Proper format enforced | $0 |

**Total validation time:** 2-3 seconds (instant feedback)
**GitHub Actions time:** 3-5 minutes (waiting for remote)
**Speed improvement:** 60-100x faster

## Installation

### First Time Setup (One-time)

```bash
# From repo root
bash scripts/setup-local-ci.sh
```

This:
1. ✅ Installs pre-commit hook (runs on every commit)
2. ✅ Validates dependencies
3. ✅ Sets up helper scripts
4. ✅ Generates initial test health report

### Manual Installation

```bash
# Link the hook
ln -sf ../../git-hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

# Make scripts executable
chmod +x scripts/check_complexity.py
chmod +x scripts/check_stubs.py
chmod +x scripts/generate_test_health.py
```

## How It Works

### 1. Before Every Commit

When you run `git commit`, the hook runs automatically:

```bash
$ git commit -m "feat: add new feature"

🔍 Running pre-commit gates...
  [1/7] Validating UTF-8 encoding...
  ✅ All files valid UTF-8
  [2/7] Checking for circular imports...
  ✅ No circular dependencies
  [3/7] Auditing factory wiring...
  ✅ Factory wiring consistent
  [4/7] Running unit tests + coverage (75% minimum)...
  ✅ 1297 tests passed, coverage 82.3%
  [5/7] Checking cyclomatic complexity (max 10)...
  ✅ No complexity violations
  [6/7] Detecting unimplemented stubs...
  ✅ No stubs found
  [7/7] Validating commit message format...
  ✅ Message format valid

╔════════════════════════════════════════════════════════╗
║   ✅ ALL PRE-COMMIT GATES PASSED - SAFE TO COMMIT     ║
╚════════════════════════════════════════════════════════╝

[master abc1234] feat: add new feature
 5 files changed, 123 insertions(+)
```

### 2. Failing Commits Block Immediately

If any check fails, commit is blocked:

```bash
$ git commit -m "feat: incomplete work"

❌ COMPLEXITY VIOLATIONS (1):
  cynic/module.py::function_name
     Complexity: 15 (threshold: 10)

✋ ACTION: Refactor functions to reduce complexity

# Commit is BLOCKED until you fix it
```

### 3. Test Health Tracking

Update the test health report before pushing:

```bash
# Run test health generation
python scripts/generate_test_health.py

# Commit the report
git add .github/test-health.json
git commit -m "chore: update test health report"
```

This creates `.github/test-health.json` with current metrics:

```json
{
  "timestamp": "2026-03-04T12:30:00.000000",
  "status": "healthy",
  "tests": {
    "total": 1297,
    "passed": 1297,
    "failed": 0,
    "skipped": 0
  },
  "coverage": {
    "percentage": 82,
    "minimum": 75,
    "status": "✅ above minimum"
  }
}
```

## Branch Protection (GitHub)

GitHub still blocks merges without branch protection rules. Enable in GitHub UI:

**Settings → Branches → Branch protection rules**

Required:
- ✅ Require pull request reviews before merging (≥1 review)
- ✅ Dismiss stale pull request approvals
- ✅ Require status checks to pass (if you want GitHub to validate)

**Note:** With local pre-commit hooks, you don't need GitHub status checks anymore. The review requirement is enough to catch peer issues.

## Benefits vs GitHub Actions

| Aspect | GitHub Actions | Local Pre-commit |
|--------|---|---|
| **Cost** | $0.008/min ($50-200/month if heavy use) | $0 |
| **Feedback time** | 3-5 minutes | 2-3 seconds |
| **Early detection** | After push (too late) | Before commit (immediate) |
| **Developer experience** | Delayed | Instant |
| **Failed builds** | Hidden initially | Caught immediately |
| **Iteration speed** | Slower (wait for GitHub) | Faster (instant feedback) |
| **Email spam** | Yes (notifications) | No (blocked before push) |

## Integration Testing

For integration tests that need SurrealDB + Vault:

```bash
# Option 1: Run locally with Docker Compose
docker-compose -f infra/docker-compose.yml up -d
python -m pytest tests/integrations/ -v
docker-compose -f infra/docker-compose.yml down

# Option 2: On demand (skip for daily development)
# Integration tests run in CI when infrastructure is available
```

## FAQ

**Q: What if I want to skip validation?**
A: You can't (by design). This prevents silent failures. If you have a legitimate reason, modify the hook temporarily, but it's a red flag.

**Q: What if validation is too slow?**
A: On modern machines, the full suite runs in 2-3 seconds. If slower:
1. Check Python version (3.11+ is faster)
2. Install optional `pytest-xdist` for parallel test runs
3. Check disk speed (SSD recommended)

**Q: Can I run just specific checks?**
A: Yes, manually:
```bash
# Just coverage
python -m pytest tests/ --cov=cynic --cov-report=term

# Just complexity
python scripts/check_complexity.py

# Just stubs
python scripts/check_stubs.py
```

**Q: What if the hook breaks?**
A: Disable temporarily:
```bash
# Uninstall hook
rm .git/hooks/pre-commit

# Fix the issue, then reinstall
bash scripts/setup-local-ci.sh
```

**Q: How do I update test-health.json?**
A: Before pushing a major change:
```bash
python scripts/generate_test_health.py
git add .github/test-health.json
git commit -m "chore: update test health report"
```

## Cost Analysis

**Old Setup (GitHub Actions):**
- Minutes per build: ~5 min
- Builds per day: ~10 (dev iteration)
- Cost: ~50 min/day × $0.008 = **$0.40/day = $12/month minimum**
- Actual usage likely 2-3x higher with all workflows

**New Setup (Local Pre-commit):**
- Validation time: ~2-3 sec (local only)
- Cost: **$0/month**
- Savings: **$12+ per month**
- Annual savings: **$144+**
- Plus: 60-100x faster feedback loop

## Conclusion

**Local pre-commit hooks provide:**
- ✅ Zero cost
- ✅ Faster feedback (2-3 sec vs 3-5 min)
- ✅ No silent failures (commit blocked if tests fail)
- ✅ No email spam (validated before push)
- ✅ Better developer experience (instant iteration)

All 100% test validation is still enforced — just locally instead of on GitHub.
