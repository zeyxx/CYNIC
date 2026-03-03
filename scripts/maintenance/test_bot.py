#!/usr/bin/env python
"""
Test the Discord bot for syntax and import errors.
"""

import asyncio
import os
import sys

# Mock the Discord token before importing bot
os.environ["DISCORD_TOKEN"] = "test_token_for_syntax_check"

try:
    # Get the actual module from sys.modules
    import sys

    import cynic.discord.bot as bot_module

    bot_mod = sys.modules["cynic.discord.bot"]
except Exception:
    sys.exit(1)

# Test that key functions and commands exist
try:
    assert hasattr(bot_module, "tree"), "tree commands not found"
    assert (
        len(bot_module.tree.get_commands()) >= 5
    ), f"Expected 5+ commands, got {len(bot_module.tree.get_commands())}"
    assert "cleanup" in bot_mod.__dict__, "cleanup function not found in module"
    assert "main" in bot_mod.__dict__, "main function not found in module"
    assert (
        "check_cynic_health" in bot_mod.__dict__
    ), "health check task not found in module"
except AssertionError:
    sys.exit(1)

# Test that the bot object has the right attributes
try:
    # bot_module is the actual Bot object (due to how it's defined)
    assert hasattr(bot_module, "cynic_session"), "cynic_session attribute not found"
    assert hasattr(bot_module, "cynic_ready"), "cynic_ready attribute not found"
    assert bot_module.cynic_session is None, "Session should be None before on_ready"
    assert bot_module.cynic_ready is False, "Should not be ready initially"
except AssertionError:
    sys.exit(1)

# Test cleanup function exists and is async
try:
    cleanup_fn = bot_mod.__dict__["cleanup"]
    assert asyncio.iscoroutinefunction(cleanup_fn), "cleanup should be async"
except (KeyError, AssertionError):
    sys.exit(1)
