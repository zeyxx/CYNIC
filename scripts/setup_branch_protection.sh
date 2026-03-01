#!/bin/bash
# Automated Branch Protection Setup
# Sets up branch protection rules via GitHub API (no manual UI needed)
# Usage: ./scripts/setup_branch_protection.sh

set -e

echo "🔐 Setting up branch protection for master branch..."

REPO="zeyxx/CYNIC"
BRANCH="master"

# Required status checks
REQUIRED_CHECKS=(
  "tests (3.11)"
  "tests (3.12)"
  "tests (3.13)"
  "Code Quality"
  "Coverage Gate"
  "Security Scan"
)

# Convert to JSON array
CHECKS_JSON=$(printf '%s\n' "${REQUIRED_CHECKS[@]}" | jq -R . | jq -s .)

echo "📋 Configuring branch protection with:"
echo "   - Required checks: ${#REQUIRED_CHECKS[@]} workflows"
echo "   - PR reviews: 1 required"
echo "   - Strict mode: enabled"
echo "   - Linear history: required"
echo "   - Force push: disabled"
echo "   - Deletions: disabled"

# Create protection rule using GitHub API
gh api repos/$REPO/branches/$BRANCH/protection \
  -X PUT \
  -f required_status_checks="{
    \"strict\": true,
    \"contexts\": $CHECKS_JSON
  }" \
  -f required_pull_request_reviews="{
    \"dismiss_stale_reviews\": true,
    \"require_code_owner_reviews\": false,
    \"required_approving_review_count\": 1
  }" \
  -f enforce_admins=true \
  -f required_linear_history=true \
  -f allow_force_pushes=false \
  -f allow_deletions=false \
  -f restrictions=null

echo ""
echo "✅ Branch protection configured successfully!"
echo ""
echo "Protected branch: $BRANCH"
echo "Repository: $REPO"
echo ""
echo "Active rules:"
echo "  ✓ Require PR reviews (1 approval)"
echo "  ✓ Require status checks to pass:"
for check in "${REQUIRED_CHECKS[@]}"; do
  echo "    - $check"
done
echo "  ✓ Strict mode (must be up to date)"
echo "  ✓ Linear history (no merge commits)"
echo "  ✓ Force pushes disabled"
echo "  ✓ Deletions disabled"
echo ""
echo "🎯 Master branch is now protected!"
