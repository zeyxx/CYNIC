#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Setup CYNIC as a Claude Code plugin.

Copies the plugin to ~/.claude/plugins/cynic and verifies the setup.
"""
import os
import sys
import json
import shutil
import platform
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

    print(f"🧬 CYNIC Claude Code Plugin Setup")
    print(f"━" * 50)
    print(f"CYNIC directory: {cynic_root}")
    print(f"Claude Code plugins: {get_plugin_dir()}")
    print(f"Plugin target: {plugin_target}")
    print()

    # Verify CYNIC directory
    if not cynic_root.exists():
        print(f"❌ Error: CYNIC root not found at {cynic_root}")
        return False

    # Verify plugin source
    if not plugin_source.exists():
        print(f"⚠️  Plugin source not found at {plugin_source}")
        print(f"   Creating plugin structure in place...")
        # For now, we'll use .claude/plugins/cynic directly
        plugin_source = None

    # Create .claude/plugins directory if needed
    get_plugin_dir().mkdir(parents=True, exist_ok=True)

    # Copy or link plugin
    if plugin_source and plugin_source.exists():
        if plugin_target.exists():
            print(f"⚠️  Plugin already exists at {plugin_target}")
            response = input("   Overwrite? (y/n): ").strip().lower()
            if response == 'y':
                shutil.rmtree(plugin_target)
            else:
                print("   Skipped")
                return False

        shutil.copytree(plugin_source, plugin_target)
        print(f"✅ Plugin copied to {plugin_target}")
    else:
        print(f"⚠️  Using existing plugin at {plugin_target}")

    # Verify plugin.json
    plugin_json = plugin_target / "plugin.json"
    if not plugin_json.exists():
        print(f"❌ Error: plugin.json not found")
        return False

    with open(plugin_json) as f:
        plugin_config = json.load(f)

    print(f"✅ Plugin manifest: {plugin_config['displayName']} v{plugin_config['version']}")

    # Verify .mcp.json
    mcp_json = plugin_target / ".mcp.json"
    if not mcp_json.exists():
        print(f"❌ Error: .mcp.json not found")
        return False

    with open(mcp_json) as f:
        mcp_config = json.load(f)

    print(f"✅ MCP configuration: {list(mcp_config.keys())[0]}")

    # Verify CYNIC installation
    try:
        import cynic
        print(f"✅ CYNIC package installed: {cynic.__file__}")
    except ImportError:
        print(f"❌ Error: CYNIC package not found")
        print(f"   Install with: pip install -e {cynic_root}")
        return False

    print()
    print(f"🎉 Setup complete!")
    print(f"━" * 50)
    print(f"Next steps:")
    print(f"1. Restart Claude Code to load the plugin")
    print(f"2. Run: /mcp")
    print(f"3. Verify 'cynic-organism' appears in the list")
    print()

    return True


if __name__ == "__main__":
    try:
        success = setup_plugin()
        sys.exit(0 if success else 1)
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
