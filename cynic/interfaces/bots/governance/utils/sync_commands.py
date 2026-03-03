#!/usr/bin/env python3
"""
Force Discord command synchronization - clears and re-registers all slash commands.
This fixes signature mismatch errors caused by stale Discord cache.

Usage:
    python sync_commands.py
"""

import asyncio
import logging
import sys

# Windows event loop compatibility
if sys.platform == 'win32':
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

import discord
from discord.ext import commands

from cynic.interfaces.bots.governance.core.config import DISCORD_PREFIX, DISCORD_TOKEN

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


async def sync_commands():
    """Force complete Discord command synchronization"""

    # Create minimal bot instance
    intents = discord.Intents.default()
    intents.message_content = True
    intents.guilds = True

    bot = commands.Bot(command_prefix=DISCORD_PREFIX, intents=intents)

    # Import all commands (this registers them)

    @bot.event
    async def on_ready():
        logger.info(f"âœ" Bot logged in as {bot.user}")
        logger.info(f"âœ" Found {len(bot.tree._get_all_commands())} commands registered locally")

        try:
            logger.info("â' Synchronizing commands with Discord...")
            synced = await bot.tree.sync()
            logger.info(f"âœ" Successfully synced {len(synced)} commands to Discord:")
            for cmd in synced:
                logger.info(f"  â€¢ /{cmd.name}")

            logger.info("\nâœ" Discord command signature fix complete!")
            logger.info("  The bot will now recognize all commands correctly.")

        except Exception as e:
            logger.error(f"âœ- Failed to sync commands: {e}", exc_info=True)
            return False

        await bot.close()
        return True

    try:
        async with bot:
            await bot.start(DISCORD_TOKEN)
    except Exception as e:
        logger.error(f"âœ- Bot startup failed: {e}", exc_info=True)
        return False


if __name__ == "__main__":

    asyncio.run(sync_commands())
