# Branch Protection & Coverage Gates Setup

This document outlines the DevOps infrastructure for maintaining code quality on the master branch.

## Overview

**Branch Protection Rules** ensure that:
- All tests must pass before PRs can merge
- Coverage gates enforce minimum code coverage
- Code review is required
- Linear history is maintained (no merge commits)
- Force pushes are disabled

**Coverage Gates** ensure that:
- Minimum 75% code coverage is maintained
- Coverage reports are posted on PRs
- Failing coverage blocks PR merges

## Setup Instructions

### 1. Enable Branch Protection via GitHub Web UI

Navigate to: **Settings → Branches → Add rule**

Configure the following:

#### Basic Settings
```
Branch name pattern: master
```

#### Require status checks to pass before merging
- ✅ **Require branches to be up to date before merging** (strict mode)
- ✅ **Require status checks to pass before merging**

#### Required status checks:
```
- tests (3.11)
- tests (3.12)
- tests (3.13)
- Code Quality
- Coverage Gate
- Security Scan
```

#### Require a pull request before merging
- ✅ **Require pull request reviews before merging**
- ✅ **Require code owner reviews**
- Number of required reviews: **1**
- ✅ **Dismiss stale pull request approvals when new commits are pushed**

#### Other enforcement rules
- ✅ **Require conversation resolution before merging** (if desired)
- ✅ **Require signed commits**
- ✅ **Require linear history**
- ✅ **Restrict who can push to matching branches** (optional)

#### Restrictions (if applicable)
```
Allow force pushes: ❌ Do not allow
Allow deletions: ❌ Do not allow
```

## CI/CD Configuration

### Tests Workflow (`.github/workflows/tests.yml`)

**Multi-version Python Testing:**
```yaml
strategy:
  matrix:
    python-version: ['3.11', '3.12', '3.13']
```

**Coverage Reporting:**
```yaml
- name: Test with pytest
  run: |
    pytest tests/ \
      --cov=cynic \
      --cov-report=xml \
      --cov-report=term-missing \
      -v --tb=short
```

**Coverage Gate Job:**
- Runs independently
- Fails if coverage < 75%
- Posts results to PRs
- Blocks merge if coverage gate fails

### Code Quality Workflow (`.github/workflows/code-quality.yml`)

**Tools:**
- **Black** (code formatting)
- **isort** (import sorting)
- **flake8** (linting)
- **mypy** (type checking)
- **pylint** (code analysis)
- **Bandit** (security scanning)

### Multi-Platform Testing (`.github/workflows/multi-platform.yml`)

Tests across:
- Ubuntu (Linux)
- Windows
- macOS

## Pre-Commit Hooks

Local development validation before committing:

```bash
# Install pre-commit
pip install pre-commit

# Install the hook
pre-commit install

# Test on all files
pre-commit run --all-files
```

**Hooks configured (`.pre-commit-config.yaml`):**
- Trailing whitespace fixes
- End-of-file fixes
- YAML/JSON/TOML validation
- Merge conflict detection
- Private key detection
- Black formatting (line-length=120)
- isort import sorting
- flake8 linting
- Bandit security scanning
- pyupgrade (Python 3.11+ syntax)

## Required Status Checks Explained

| Check | Purpose | Triggers |
|-------|---------|----------|
| `tests (3.11)` | Test suite on Python 3.11 | On push/PR to master |
| `tests (3.12)` | Test suite on Python 3.12 | On push/PR to master |
| `tests (3.13)` | Test suite on Python 3.13 | On push/PR to master |
| `Code Quality` | Linting, formatting, security | On push/PR to master |
| `Coverage Gate` | Minimum 75% code coverage | On push/PR to master |
| `Security Scan` | SAST + dependency scanning | On schedule + PR/push |

## Enforcing Coverage

### Coverage Threshold: 75%

**Current Status (Session 6):**
```
✅ 1058/1109 tests passing (95.4%)
```

**Why 75%?**
- Aggressive enough to catch untested code
- Realistic for legacy/integration code
- Leaves room for legitimate exceptions

### Exemptions from Coverage

Add to `.coveragerc` if needed:
```ini
[run]
omit =
    */tests/*
    */venv/*
    setup.py

[report]
exclude_lines =
    pragma: no cover
    def __repr__
    raise AssertionError
    raise NotImplementedError
    if __name__ == .__main__.:
    if TYPE_CHECKING:
```

## Typical Workflow

### Developer Perspective

```bash
# 1. Clone repo
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC

# 2. Create feature branch
git checkout -b feature/my-feature

# 3. Make changes
# ...

# 4. Pre-commit runs automatically (or manually)
pre-commit run --all-files

# 5. Commit
git commit -m "feat: Add my feature"

# 6. Push
git push origin feature/my-feature

# 7. Create PR
gh pr create --title "feat: Add my feature"
```

### GitHub Actions Runs

1. **Tests** (3 parallel jobs for Python 3.11/3.12/3.13)
   - ~2-3 minutes

2. **Code Quality**
   - ~1-2 minutes

3. **Coverage Gate**
   - ~2-3 minutes
   - Posts coverage report to PR

4. **Multi-Platform**
   - ~5-10 minutes (scheduled daily)

5. **Security Scan**
   - ~3-5 minutes

### PR Merge Requirements

All of these must be true:
- ✅ All required status checks pass
- ✅ At least 1 approval
- ✅ Branch is up to date with master
- ✅ Linear history (no merge commits)
- ✅ No stale reviews (auto-dismissed on new pushes)

## Monitoring & Reporting

### Coverage Trends

**Tracked in:**
- Codecov integration
- GitHub PR comments
- Workflow artifacts

### Test Results

**Tracked in:**
- GitHub Actions UI
- PR status checks
- Email notifications

### Performance Metrics

**Workflow duration:**
- Target: < 15 minutes for full validation
- Current: ~10-12 minutes total

## Common Issues & Solutions

### "Coverage gate failed"
```
Problem: Coverage dropped below 75%
Solution:
1. Write tests for new code
2. Verify coverage locally: pytest --cov=cynic
3. Aim for 80%+ to have buffer
```

### "Code Quality failed"
```
Problem: Black/isort/flake8 violations
Solution:
1. Run pre-commit locally: pre-commit run --all-files
2. Fix issues: black ., isort .
3. Commit and push again
```

### "Tests timeout"
```
Problem: Tests take > 30 minutes
Solution:
1. Check for hanging tests (network I/O, infinite loops)
2. Mark slow tests: @pytest.mark.slow
3. Consider test parallelization
```

## Best Practices

1. **Keep master always green**
   - All PRs must pass checks before merge
   - No manual overrides

2. **Improve coverage incrementally**
   - Add 1-2% per release
   - Focus on critical paths first

3. **Run pre-commit locally**
   - Catch issues before CI
   - Faster feedback loop

4. **Review PR diffs carefully**
   - Check coverage impact
   - Verify no risky changes

5. **Monitor workflow performance**
   - Track CI times
   - Optimize slow tests

## References

- [GitHub Branch Protection Docs](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Coverage.py Docs](https://coverage.readthedocs.io/)
- [pytest-cov Plugin](https://pytest-cov.readthedocs.io/)
