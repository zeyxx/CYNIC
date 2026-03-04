#!/usr/bin/env python3
"""Check cyclomatic complexity in cynic/ — blocks commit if violations found."""
import os
import sys

try:
    from radon.complexity import cc_visit
except ImportError:
    print("⚠️  radon not installed. Skipping complexity check.")
    print("   Install with: pip install radon")
    sys.exit(0)

COMPLEXITY_THRESHOLD = 10
VIOLATIONS = []

for root, dirs, files in os.walk('cynic'):
    dirs[:] = [d for d in dirs if d not in ['__pycache__', '.pytest_cache']]

    for file in files:
        if not file.endswith('.py'):
            continue

        filepath = os.path.join(root, file)
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                source = f.read()

            results = cc_visit(source)
            for result in results:
                if hasattr(result, 'complexity') and result.complexity > COMPLEXITY_THRESHOLD:
                    VIOLATIONS.append({
                        'file': filepath,
                        'function': result.name,
                        'complexity': result.complexity,
                        'threshold': COMPLEXITY_THRESHOLD
                    })
        except Exception:
            pass

if VIOLATIONS:
    print(f"\n❌ COMPLEXITY VIOLATIONS ({len(VIOLATIONS)}):\n")
    for v in VIOLATIONS[:10]:
        print(f"  {v['file']}::{v['function']}")
        print(f"     Complexity: {v['complexity']} (threshold: {v['threshold']})")

    if len(VIOLATIONS) > 10:
        print(f"\n  ... and {len(VIOLATIONS) - 10} more")

    print(f"\n✋ ACTION: Refactor functions to reduce complexity")
    sys.exit(1)
else:
    print("✅ No complexity violations detected")
    sys.exit(0)
