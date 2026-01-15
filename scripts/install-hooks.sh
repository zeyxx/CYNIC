#!/bin/bash
#
# CYNIC Git Hooks Installer
#
# "Le chien s'installe" - The dog installs itself
#
# This script installs CYNIC's git hooks for security scanning.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GIT_HOOKS_DIR="$PROJECT_ROOT/.git/hooks"
CYNIC_HOOKS_DIR="$PROJECT_ROOT/scripts/git-hooks"

echo "═══════════════════════════════════════════════════════════"
echo "🐕 CYNIC Git Hooks Installer"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Check if .git directory exists
if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo "❌ Not a git repository. Run this from a git project root."
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$GIT_HOOKS_DIR"

# Install pre-commit hook
echo "Installing pre-commit hook..."
cp "$CYNIC_HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
chmod +x "$GIT_HOOKS_DIR/pre-commit"
echo "✅ pre-commit hook installed"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "*tail wag* CYNIC hooks installed successfully!"
echo ""
echo "Security scanning will run automatically before each commit."
echo "To bypass (not recommended): git commit --no-verify"
echo "═══════════════════════════════════════════════════════════"
