#!/usr/bin/env python3
"""
Semi-automated bare exception handler fixer.

Usage:
    python3 scripts/fix_bare_exceptions.py cynic/cynic/api/routers/nervous.py

This script:
1. Finds all `except Exception` clauses
2. Analyzes context to identify exception type
3. Suggests specific exception types
4. Can apply fixes (with --apply flag)
"""

import re
import sys
import argparse
from pathlib import Path


# Exception patterns by context
PATTERNS = {
    "database": {
        "keywords": ["db.", "query", "asyncpg", "postgres", "save", "fetch", "update", "delete"],
        "exceptions": ["asyncpg.Error", "PersistenceError"],
        "priority": 1,
    },
    "json": {
        "keywords": ["json.", "parse", "loads", "dumps", "decode"],
        "exceptions": ["json.JSONDecodeError", "ValidationError"],
        "priority": 2,
    },
    "http": {
        "keywords": ["http", "request", "response", "fetch", "post", "get"],
        "exceptions": ["httpx.RequestError", "asyncio.TimeoutError"],
        "priority": 2,
    },
    "llm": {
        "keywords": ["llm", "ollama", "complete", "inference", "prompt"],
        "exceptions": ["asyncio.TimeoutError", "LLMError"],
        "priority": 2,
    },
    "file": {
        "keywords": ["open", "write", "read", "unlink", "mkdir", "os."],
        "exceptions": ["OSError", "IOError"],
        "priority": 2,
    },
    "event": {
        "keywords": ["emit", "subscribe", "event_bus", "bus.", "handler"],
        "exceptions": ["EventBusError"],
        "priority": 3,
    },
    "validation": {
        "keywords": ["validate", "check", "assert", "raise", "verify"],
        "exceptions": ["ValidationError"],
        "priority": 3,
    },
}


def identify_pattern(context):
    """Identify exception pattern from code context."""
    context_lower = context.lower()
    scores = {}

    for pattern_name, info in PATTERNS.items():
        score = sum(
            info["priority"]
            for keyword in info["keywords"]
            if keyword in context_lower
        )
        if score > 0:
            scores[pattern_name] = score

    if not scores:
        return None, None

    best_pattern = max(scores.items(), key=lambda x: x[1])[0]
    return best_pattern, PATTERNS[best_pattern]["exceptions"][0]


def find_exceptions(filepath):
    """Find all bare exception handlers in file."""
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()
        lines = content.split('\n')

    exceptions = []

    for i, line in enumerate(lines):
        if "except Exception" in line:
            # Get context (try block + surrounding lines)
            start = max(0, i - 10)
            end = min(len(lines), i + 3)
            context = '\n'.join(lines[start:end])

            pattern, exception_type = identify_pattern(context)

            exceptions.append({
                'line': i + 1,
                'content': line.strip(),
                'pattern': pattern,
                'suggested_exception': exception_type,
                'context': context,
            })

    return exceptions


def suggest_fix(exception_info):
    """Suggest a fix for the exception."""
    line = exception_info['content']
    suggested = exception_info['suggested_exception'] or "Exception"

    # Replace 'except Exception' with specific type
    if 'as e:' in line:
        fixed = line.replace('except Exception as e:', f'except {suggested} as e:')
    else:
        fixed = line.replace('except Exception:', f'except {suggested}:')

    return fixed


def apply_fixes(filepath, exceptions, apply=False):
    """Apply fixes to the file."""
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        lines = f.readlines()

    # Sort by line number descending (so replacements don't affect line numbers)
    sorted_exceptions = sorted(exceptions, key=lambda x: -x['line'])

    for exc in sorted_exceptions:
        line_idx = exc['line'] - 1
        old_line = lines[line_idx]
        suggested_fix = suggest_fix(exc)

        print(f"\nLine {exc['line']}:")
        print(f"  Pattern: {exc['pattern']}")
        print(f"  Before: {old_line.strip()}")
        print(f"  After:  {suggested_fix}")

        if apply:
            # Replace the line
            lines[line_idx] = old_line.replace(exc['content'], suggested_fix)
            print("  [APPLIED]")
        else:
            print("  (use --apply to make changes)")

    if apply:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        print(f"\n[OK] File updated: {filepath}")


def main():
    parser = argparse.ArgumentParser(description="Fix bare exception handlers")
    parser.add_argument("filepath", help="Python file to analyze")
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Apply fixes (default: dry-run, just show suggestions)",
    )
    parser.add_argument(
        "--pattern",
        help="Only fix exceptions matching this pattern (e.g., 'http', 'json')",
    )

    args = parser.parse_args()

    filepath = Path(args.filepath)
    if not filepath.exists():
        print(f"ERROR: {filepath} not found")
        sys.exit(1)

    exceptions = find_exceptions(filepath)

    if not exceptions:
        print(f"[OK] No bare exception handlers found in {filepath}")
        return

    print(f"\nFound {len(exceptions)} bare exception handlers in {filepath}\n")

    if args.pattern:
        exceptions = [e for e in exceptions if e['pattern'] == args.pattern]
        print(f"Filtered to {len(exceptions)} matching pattern '{args.pattern}'\n")

    apply_fixes(filepath, exceptions, apply=args.apply)


if __name__ == "__main__":
    main()
