#!/bin/bash
# Setup local CI/CD validation (replaces GitHub Actions for cost savings)
set -e

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║   Setting up Local CI/CD Validation (Cost: $0)        ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Step 1: Install pre-commit hook
echo "📍 Installing pre-commit hook..."
if [ -f "git-hooks/pre-commit" ]; then
    ln -sf ../../git-hooks/pre-commit .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    echo "   ✅ Pre-commit hook installed"
else
    echo "   ❌ git-hooks/pre-commit not found"
    exit 1
fi

# Step 2: Check dependencies
echo ""
echo "📍 Checking dependencies..."
python -c "import radon" 2>/dev/null && echo "   ✅ radon installed" || echo "   ⚠️  radon not found: pip install radon"

# Step 3: Make helper scripts executable
echo ""
echo "📍 Setting up helper scripts..."
chmod +x scripts/check_complexity.py
chmod +x scripts/check_stubs.py
chmod +x scripts/generate_test_health.py
echo "   ✅ Helper scripts ready"

# Step 4: Generate initial health report
echo ""
echo "📍 Generating initial test health report..."
if python scripts/generate_test_health.py; then
    git add .github/test-health.json
    echo "   ✅ Test health report generated"
else
    echo "   ⚠️  First test run failed (might need dependencies)"
fi

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║      ✅ Local CI/CD Setup Complete!                   ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""
echo "📋 Summary:"
echo "  • Pre-commit hooks: ACTIVE (runs on every commit)"
echo "  • Coverage gate: 75% minimum (enforced)"
echo "  • Complexity check: ≤10 cyclomatic (enforced)"
echo "  • Stub detection: Active (enforced)"
echo "  • Test health: .github/test-health.json (commit after runs)"
echo ""
echo "🚀 Next steps:"
echo "  1. Try committing: validation runs automatically"
echo "  2. Update test health: python scripts/generate_test_health.py"
echo "  3. Branch protection: Requires reviews (GitHub UI)"
echo ""
