#!/usr/bin/env python3
"""Better type hint fixer: handle multi-line imports properly"""

import re
from pathlib import Path

def fix_file(file_path: Path) -> bool:
    """Fix type hints in a single file."""
    content = file_path.read_text(encoding='utf-8')
    original = content

    # Step 1: Replace X | None with Optional[X]
    content = re.sub(r'(\w+)\s*\|\s*None', r'Optional[\1]', content)
    content = re.sub(r'((?:dict|list|tuple|set)\[[^\]]*\])\s*\|\s*None', r'Optional[\1]', content)

    # Step 2: Ensure Optional is imported
    if 'Optional[' in content and 'from typing import' not in content:
        # Find last import line
        lines = content.split('\n')
        last_import_idx = -1
        for i, line in enumerate(lines):
            if line.startswith('from ') or line.startswith('import '):
                last_import_idx = i

        if last_import_idx >= 0:
            # Insert after last import
            lines.insert(last_import_idx + 1, 'from typing import Optional')
            content = '\n'.join(lines)

    if content != original:
        file_path.write_text(content, encoding='utf-8')
        return True
    return False

def main():
    cynic_dir = Path('cynic/cynic')
    fixed = 0

    for py_file in cynic_dir.rglob('*.py'):
        if fix_file(py_file):
            fixed += 1
            print(f"[FIX] {py_file.relative_to('.')}")

    print(f"\n[DONE] Fixed {fixed} files")
    return fixed > 0

if __name__ == '__main__':
    import sys
    sys.exit(0 if main() else 1)
