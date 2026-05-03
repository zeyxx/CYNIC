#!/bin/bash
set -e

echo "=== Protocol Wiring Verification ==="
echo

# Check 1: Session hook exists and is executable
echo "1. Session Hook"
if [[ -x ~/.config/claude/hooks/session-start-protocol.sh ]]; then
    echo "   ✓ Hook exists and is executable"
else
    echo "   ✗ Hook missing or not executable"
    exit 1
fi

# Check 2: Cortex manifest exists
echo
echo "2. Cortex Manifest"
if [[ -f ~/.cynic/cortex-manifests/claude-code.json ]]; then
    echo "   ✓ Manifest exists"
    cortex_id=$(jq -r '.cortex_id' ~/.cynic/cortex-manifests/claude-code.json)
    consumer_count=$(jq '.consumes | length' ~/.cynic/cortex-manifests/claude-code.json)
    echo "   • Cortex: ${cortex_id}"
    echo "   • Consumers: ${consumer_count}"
else
    echo "   ✗ Manifest missing"
    exit 1
fi

# Check 3: Consumer registry exists
echo
echo "3. Consumer Registry"
if [[ -f ~/.cynic/organisms/consumers/consumer_registry.json ]]; then
    echo "   ✓ Registry exists"
    consumer_count=$(jq '.consumers | length' ~/.cynic/organisms/consumers/consumer_registry.json)
    echo "   • Total consumers: ${consumer_count}"
else
    echo "   ✗ Registry missing"
    exit 1
fi

# Check 4: Run the session hook to verify output
echo
echo "4. Session Hook Output"
echo "   Running hook..."
~/.config/claude/hooks/session-start-protocol.sh | sed 's/^/   /'

# Check 5: Test artifact loading
echo
echo "5. Artifact Loading"
echo "   Testing Python loader..."
python3 cynic-python/artifact_loader.py claude-code 2>&1 | sed 's/^/   /'

echo
echo "=== Verification Complete ==="
echo "✓ Protocol wiring is ready"
echo
echo "Next steps:"
echo "1. Await kernel recovery (kernel_health=true)"
echo "2. Wiring will activate automatically on kernel recovery"
echo "3. Phase 2 gate (May 5-6) will test signal improvement"
