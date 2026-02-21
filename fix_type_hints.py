#!/usr/bin/env python3
"""Fix Python 3.9 incompatible type hints (str | None -> Optional[str])"""

import re
from pathlib import Path

def fix_file(file_path: Path) -> bool:
    """Fix type hints in a single file. Returns True if changes made."""
    content = file_path.read_text(encoding='utf-8')
    original = content

    # Replace X | None with Optional[X]
    # Handle: str | None, int | None, bool | None, dict[...] | None, list[...] | None, etc.

    # Pattern 1: Simple types (str, int, bool, etc.)
    content = re.sub(r'(\w+)\s*\|\s*None', r'Optional[\1]', content)

    # Pattern 2: Generic types (dict[...], list[...], etc.)
    # This is trickier - need to match balanced brackets
    def replace_generic(match):
        generic_type = match.group(1)
        return f'Optional[{generic_type}]'

    # Match: dict[...] | None or list[...] | None
    content = re.sub(r'((?:dict|list|tuple|set)\[[^\]]*\])\s*\|\s*None', replace_generic, content)

    # Add Optional import if it doesn't exist and we made changes
    if content != original:
        if 'Optional' not in content:
            # Add Optional import at very top after __future__ imports
            lines = content.split('\n')
            insert_idx = 0

            # Skip __future__ imports
            for i, line in enumerate(lines):
                if line.startswith('from __future__'):
                    insert_idx = i + 1
                elif line.startswith('"""') or line.startswith("'''"):
                    # Skip docstrings
                    continue
                elif line and not line.startswith('#'):
                    break

            # Find existing typing import to add Optional to
            found_typing = False
            for i in range(insert_idx, len(lines)):
                if 'from typing import' in lines[i] and ')' in lines[i]:
                    # Single line import
                    if 'Optional' not in lines[i]:
                        lines[i] = lines[i].replace(')', ', Optional)')
                    found_typing = True
                    break

            if not found_typing:
                lines.insert(insert_idx, 'from typing import Optional')

            content = '\n'.join(lines)

    if content != original:
        file_path.write_text(content, encoding='utf-8')
        return True
    return False

def main():
    cynic_dir = Path('cynic/cynic')
    fixed_files = []

    for py_file in cynic_dir.rglob('*.py'):
        if fix_file(py_file):
            fixed_files.append(py_file)
            print(f"[OK] Fixed: {py_file.relative_to('.')}")

    print(f"\n[OK] Total files fixed: {len(fixed_files)}")
    return len(fixed_files) > 0

if __name__ == '__main__':
    import sys
    sys.exit(0 if main() else 1)
