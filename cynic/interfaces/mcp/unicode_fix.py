"""
Unicode fix for Windows console output.

Add this import at the top of any module that prints to console.
"""
import os
import sys


def fix_unicode_on_windows():
    """Force UTF-8 encoding on Windows console output."""
    if sys.platform == "win32":
        # Set environment variable
        os.environ["PYTHONIOENCODING"] = "utf-8"
        
        # Try to reconfigure stdout/stderr for UTF-8
        if hasattr(sys.stdout, "reconfigure"):
            try:
                sys.stdout.reconfigure(encoding="utf-8", errors="replace")
            except Exception as _e:
            logger.debug(f'Silenced: {_e}')
        
        if hasattr(sys.stderr, "reconfigure"):
            try:
                sys.stderr.reconfigure(encoding="utf-8", errors="replace")
            except Exception as _e:
            logger.debug(f'Silenced: {_e}')

# Auto-fix on import
fix_unicode_on_windows()
