#!/usr/bin/env python3
import sys
import os
from pathlib import Path
try:
    from organs.browser.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


_ROOT = bootstrap_repo_path()
SRC_ROOT = _ROOT / "organs" / "browser" / "src"
if str(SRC_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_ROOT))

from core.cli_runtime import main


if __name__ == "__main__":
    main()
