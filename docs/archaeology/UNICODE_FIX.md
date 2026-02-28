# Unicode Encoding Fix for Windows

## Problem
Python on Windows defaults to the system code page (usually cp1252), which cannot display Unicode characters like emojis or special symbols. This causes `UnicodeEncodeError` when printing.

## Solution: Already Applied

The CYNIC codebase now includes automatic UTF-8 encoding fixes in:
- `cynic/mcp/__main__.py` - Sets UTF-8 on Windows startup
- All MCP files - Unicode characters removed from code

## If You Still Get Unicode Errors

### Option 1: Set Environment Variable (Recommended)
```bash
# Command Prompt
set PYTHONIOENCODING=utf-8

# PowerShell
$env:PYTHONIOENCODING = "utf-8"

# Or permanently in Windows Settings > Environment Variables
PYTHONIOENCODING=utf-8
```

### Option 2: Use Python Flag
```bash
python -X utf8 -m cynic.interfaces.mcp
```

### Option 3: Modify Python Startup
Add to your Python installation's `pyvenv.cfg`:
```
PYTHONIOENCODING=utf-8
```

## Why This Matters
- MCP server needs UTF-8 for JSON-RPC protocol
- Claude Code expects UTF-8 output
- Windows console is notoriously problematic for Unicode

## Files Modified
- `cynic/mcp/__main__.py` - Added automatic UTF-8 fix on Windows
- `cynic/mcp/stdio_server.py` - Removed unicode characters from docstrings
- All MCP docstrings now use ASCII equivalents

## Status
All critical code paths now handle UTF-8 properly. No more Unicode errors!
