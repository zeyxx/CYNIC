#!/usr/bin/env python3
"""Remove wrongly-inserted typing imports and add them correctly"""

import re
from pathlib import Path

broken_files = [
    "cynic/cynic/api/routers/nervous.py",
    "cynic/cynic/api/routers/orchestration.py",
    "cynic/cynic/chat/formatter.py",
    "cynic/cynic/cli/tui_dashboard.py",
    "cynic/cynic/cognition/cortex/amplification_benchmark.py",
    "cynic/cynic/cognition/cortex/mcts_benchmark.py",
    "cynic/cynic/cognition/cortex/qtable_benchmark.py",
    "cynic/cynic/cognition/cortex/real_benchmark.py",
    "cynic/cynic/core/axioms.py",
    "cynic/cynic/core/trust_model.py",
    "cynic/cynic/tui/panels/orchestration.py",
]

def fix_file(file_path: Path):
    content = file_path.read_text(encoding='utf-8')

    # Remove any `from typing import Optional` that's in the middle of imports
    lines = content.split('\n')
    new_lines = []
    import_section_end = 0

    for i, line in enumerate(lines):
        # Skip wrongly-placed imports
        if line.strip() == 'from typing import Optional' and i < 50:
            # This is likely wrongly placed, skip for now
            continue
        new_lines.append(line)
        if line and (line.startswith('from ') or line.startswith('import ')) and not line.startswith('from typing'):
            import_section_end = len(new_lines)

    # Now add Optional import at the right place if needed
    if 'Optional[' in '\n'.join(new_lines) and 'from typing import Optional' not in '\n'.join(new_lines):
        if import_section_end > 0:
            new_lines.insert(import_section_end, 'from typing import Optional')
        else:
            new_lines.insert(0, 'from typing import Optional')

    result = '\n'.join(new_lines)
    file_path.write_text(result, encoding='utf-8')
    print(f"[FIXED] {file_path}")

for fpath in broken_files:
    fix_file(Path(fpath))

print("[DONE]")
