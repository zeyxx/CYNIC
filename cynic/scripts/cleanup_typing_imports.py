"""
Cleanup legacy typing imports after ruff UP migration.

ruff UP fixed all USAGES (Dict[X] → dict[X], Optional[X] → X | None, etc.)
but left behind the import lines with now-unused symbols.

This script removes deprecated typing symbols (Dict, List, Optional, Set, Tuple,
FrozenSet, Deque, Type) from import lines where they are no longer used in the file body.

Safe to run multiple times. Does NOT touch:
  - Any (still from typing)
  - TYPE_CHECKING (still from typing)
  - Callable (still from typing unless moved to collections.abc)
  - Union (still from typing in some older patterns)
  - overload, cast, TypeVar, Protocol, etc.

Usage:
    python scripts/cleanup_typing_imports.py
    python scripts/cleanup_typing_imports.py --dry-run
"""
from __future__ import annotations

import ast
import re
import sys
from pathlib import Path

# These are the deprecated typing aliases we want to clean up
DEPRECATED = {"Dict", "List", "Optional", "Set", "Tuple", "FrozenSet", "Deque", "Type"}

# These are KEPT from typing (no builtin/stdlib replacement)
KEEP_ALWAYS = {"Any", "TYPE_CHECKING", "TypeVar", "Protocol", "overload", "cast",
               "Union", "Callable", "ClassVar", "Final", "Literal", "TypedDict",
               "NamedTuple", "Generator", "Iterator", "Iterable", "AsyncIterator",
               "AsyncGenerator", "Awaitable", "Coroutine", "Sequence", "Mapping",
               "MutableMapping", "MutableSequence", "AbstractSet", "IO", "TextIO",
               "BinaryIO", "Pattern", "Match", "AnyStr", "NoReturn", "Never",
               "LiteralString", "TypeAlias", "TypeGuard", "ParamSpec", "Concatenate",
               "Deque",  # keep Deque if collections.deque not used as annotation
               "DefaultDict", "OrderedDict", "Counter", "ChainMap",
               "SupportsInt", "SupportsFloat", "SupportsComplex", "SupportsBytes",
               "SupportsAbs", "SupportsRound",
               }

DRY_RUN = "--dry-run" in sys.argv

root = Path(__file__).parent.parent / "cynic"
files = list(root.rglob("*.py"))

changed = 0
for path in sorted(files):
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    # Find typing import lines
    modified = False
    new_lines = []
    for line in lines:
        m = re.match(r'^(\s*from\s+typing\s+import\s+)(.*?)(\s*)$', line, re.DOTALL)
        if not m:
            new_lines.append(line)
            continue

        prefix, imports_str, suffix = m.groups()

        # Parse imported names
        names = [n.strip() for n in imports_str.split(",") if n.strip()]

        # Check which DEPRECATED names are actually used in the file (beyond the import line)
        body = text  # Check against whole file (import line check is conservative)
        to_remove = set()
        for name in names:
            if name not in DEPRECATED:
                continue
            # Check if the name appears anywhere in the file body OTHER than in typing imports
            # Use word-boundary check to avoid matching substrings
            pattern = re.compile(r'(?<![.\w])' + re.escape(name) + r'(?![.\w])')
            # Count occurrences in body excluding the import line itself
            body_without_imports = re.sub(r'^from typing import.*$', '', body, flags=re.MULTILINE)
            if not pattern.search(body_without_imports):
                to_remove.add(name)

        if not to_remove:
            new_lines.append(line)
            continue

        # Remove deprecated unused names
        kept = [n for n in names if n not in to_remove]
        if not kept:
            # Remove the entire import line
            modified = True
            print(f"  REMOVE line: {line.rstrip()!r}")
            continue  # Don't add this line
        else:
            # Rebuild import line
            new_import = f"{prefix}{', '.join(kept)}{suffix}\n"
            if new_import != line:
                modified = True
                print(f"  UPDATE: {line.rstrip()!r}")
                print(f"       → {new_import.rstrip()!r}")
            new_lines.append(new_import)

    if modified:
        rel = path.relative_to(root.parent)
        print(f"\n[{rel}]")
        changed += 1
        if not DRY_RUN:
            path.write_text("".join(new_lines), encoding="utf-8")

print(f"\n{'DRY RUN: ' if DRY_RUN else ''}Fixed {changed} files.")
