#!/usr/bin/env python3
"""
Setup CYNIC as a Claude Code plugin.

Copies the plugin to ~/.claude/plugins/cynic and verifies the setup.
"""
import json
import os
import platform
import shutil
import sys
from pathlib import Path

# Force UTF-8 encoding on Windows
if sys.stdout.encoding != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')


def get_plugin_dir():
    """Get Claude Code plugins directory based on OS."""
    if platform.system() == "Windows":
        return Path(os.path.expandvars(r"%USERPROFILE%\.claude\plugins"))
    else:
        return Path.home() / ".claude" / "plugins"


def setup_plugin():
    """Setup CYNIC as Claude Code plugin."""

    # Get paths
    cynic_root = Path(__file__).parent  # CYNIC-clean directory
    plugin_source = cynic_root / "claude_plugin" / "cynic"
    plugin_target = get_plugin_dir() / "cynic"


    # Verify CYNIC directory
    if not cynic_root.exists():
        return False

    # Verify plugin source
    if not plugin_source.exists():
        # For now, we'll use .claude/plugins/cynic directly
        plugin_source = None

    # Create .claude/plugins directory if needed
    get_plugin_dir().mkdir(parents=True, exist_ok=True)

    # Copy or link plugin
    if plugin_source and plugin_source.exists():
        if plugin_target.exists():
            response = input("   Overwrite? (y/n): ").strip().lower()
            if response == 'y':
                shutil.rmtree(plugin_target)
            else:
                return False

        shutil.copytree(plugin_source, plugin_target)
    else:
        pass

    # Verify plugin.json
    plugin_json = plugin_target / "plugin.json"
    if not plugin_json.exists():
        return False

    with open(plugin_json) as f:
        json.load(f)


    # Verify .mcp.json
    mcp_json = plugin_target / ".mcp.json"
    if not mcp_json.exists():
        return False

    with open(mcp_json) as f:
        json.load(f)


    # Verify CYNIC installation
    try:
        import cynic
    except ImportError:
        return False


    return True


if __name__ == "__main__":
    try:
        success = setup_plugin()
        sys.exit(0 if success else 1)
    except Exception:
        import traceback
        traceback.print_exc()
        sys.exit(1)
