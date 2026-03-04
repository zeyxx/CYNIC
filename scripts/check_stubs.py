#!/usr/bin/env python3
"""Check for unimplemented stubs in core cynic/ code — blocks commit if found."""
import os
import re
import sys

STUB_PATTERNS = [
    (r'^\s*\.\.\.\s*$', 'Bare ellipsis'),
    (r'^\s*raise\s+NotImplementedError', 'NotImplementedError'),
    (r'pass\s*#\s*TODO', 'pass with TODO'),
]

STUBS = []

for root, dirs, files in os.walk('cynic'):
    dirs[:] = [d for d in dirs if d not in ['__pycache__', '.pytest_cache']]

    for file in files:
        if not file.endswith('.py'):
            continue

        filepath = os.path.join(root, file)
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                lines = f.readlines()

            for i, line in enumerate(lines, 1):
                for pattern, stub_type in STUB_PATTERNS:
                    if re.search(pattern, line):
                        STUBS.append({
                            'file': filepath,
                            'line': i,
                            'type': stub_type,
                            'code': line.strip()[:60]
                        })
        except Exception:
            pass

if STUBS:
    print(f"\n⚠️  STUBS FOUND ({len(STUBS)}):\n")
    for stub in STUBS[:10]:
        print(f"  {stub['file']}:{stub['line']}")
        print(f"     [{stub['type']}] {stub['code']}...")

    if len(STUBS) > 10:
        print(f"\n  ... and {len(STUBS) - 10} more")

    print(f"\n✋ ACTION: Implement stubs before commit")
    sys.exit(1)
else:
    print("✅ No unimplemented stubs detected")
    sys.exit(0)
