# Branch Protection Setup for CYNIC

## Manual GitHub UI Setup

Since branch protection rules are enforced at the repository level (not in code), follow these steps:

### Step 1: Go to Repository Settings
1. Navigate to https://github.com/your-org/CYNIC-clean/settings/branches
2. Click "Add rule" under "Branch protection rules"

### Step 2: Configure Protection for `main`

**Pattern:** `main`

**Require:**
- ✅ "Require a pull request before merging"
  - ✅ "Require approvals" (1 approval minimum)
- ✅ "Require status checks to pass before merging"
  - Check: `test` (Unit & Integration Tests)
  - Check: `architecture` (Architecture Integrity)
  - Check: `build` (Build & Import Validation)
  - NOTE: `performance` is NOT blocking (warning only)
- ✅ "Require branches to be up to date before merging"
- ✅ "Require code reviews before merging"
  - Dismiss stale PR approvals when new commits are pushed
- ✅ "Require conversation resolution before merging"

**Bypass for administrators:** Uncheck (enforce for everyone)

### Step 3: Verify Rules

Run:
```bash
gh api repos/your-org/CYNIC-clean/branches/main/protection
```

Expected output: JSON showing all rules in place.

## Automation

To automate this in the future, use:
```bash
gh api repos/your-org/CYNIC-clean/branches/main/protection \
  -f required_status_checks='{"strict":true,"contexts":["test","architecture","build"]}' \
  -f required_pull_request_reviews='{"dismiss_stale_reviews":true,"require_code_owner_reviews":false}' \
  -f enforce_admins=true
```

## For Each PR

Before merging to `main`:

1. ✅ All CI/CD jobs pass (test, architecture, build)
2. ✅ 1 approval from code reviewer
3. ✅ Branch is up-to-date with main
4. ✅ All conversations resolved

Example PR checklist message:
```markdown
## Pre-Merge Checklist

- [ ] CI gates passed (test, architecture, build)
- [ ] Code review complete (1 approval required)
- [ ] Architecture review (no new debt)
- [ ] Decision logged in merge commit
- [ ] Branch is up-to-date with main
```
