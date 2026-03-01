# Quick Branch Protection Setup (5 minutes)

**No code needed - just click through GitHub's Web UI**

## Step 1: Open Branch Protection Settings
```
https://github.com/zeyxx/CYNIC/settings/branches
```
Or:
1. Go to https://github.com/zeyxx/CYNIC
2. Click **Settings**
3. Click **Branches** (left sidebar)
4. Click **Add rule**

## Step 2: Configure Rule

### Pattern
```
Branch name pattern: master
```

## Step 3: Require Status Checks ✓

- [x] **Require branches to be up to date before merging**
- [x] **Require status checks to pass before merging**

### Add required status checks:
```
tests (3.11)
tests (3.12)
tests (3.13)
Code Quality
Coverage Gate
Security Scan
```

## Step 4: Require Pull Request Reviews ✓

- [x] **Require a pull request before merging**
- [x] **Require approvals**
  - Number: **1**
- [x] **Dismiss stale pull request approvals when new commits are pushed**
- [x] **Require review from Code Owners** (optional)

## Step 5: Enforce Rules ✓

- [x] **Require branches to be up to date before merging** (already checked above)
- [x] **Require status checks to pass before merging** (already checked above)
- [x] **Require conversation resolution before merging**
- [x] **Require signed commits**
- [x] **Require linear history**

## Step 6: Restrict Force Pushes ✓

- [ ] **Allow force pushes** (leave UNCHECKED)
- [ ] **Allow deletions** (leave UNCHECKED)

## Step 7: Save

Click **Create** button at the bottom.

---

## Verify It Worked

After saving:
1. Go back to **Settings → Branches**
2. You should see **master** branch with a shield icon
3. Click on it to view/edit the rule

---

## What This Protects

| Item | Requirement |
|------|-------------|
| Tests | 3 Python versions (3.11, 3.12, 3.13) |
| Code Quality | Linting, formatting, type checks |
| Coverage | Minimum 75% |
| Security | Vulnerability scanning |
| PR Review | 1 approval required |
| Branch Status | Must be up to date with base |
| History | Linear (no merge commits) |
| Force Push | ❌ Blocked |
| Deletions | ❌ Blocked |

---

## Testing Your Setup

Try creating a PR with:
```bash
git checkout -b test/check-protection
echo "test" >> README.md
git add README.md
git commit -m "test: verify branch protection"
git push origin test/check-protection
gh pr create --title "test: verify branch protection"
```

You should see:
- ✅ All status checks running
- ✅ Coverage gate checking
- ✅ Blocking merge until all pass

---

## Troubleshooting

**"Cannot find 'Code Quality' check"**
- Wait for first workflow run
- Check `.github/workflows/code-quality.yml` exists
- Push a commit to trigger workflows

**"Coverage Gate not found"**
- Workflows need to run first
- After first push/PR, all workflows will appear

**"Need to wait for X check"**
- This is normal - workflows take 5-10 minutes
- Let them complete before merging

---

## Done! 🎯

Your master branch is now protected. Every PR must pass:
- All tests ✅
- Coverage gate ✅
- Code quality ✅
- 1 approval ✅

**Now you can code fast with confidence!**
