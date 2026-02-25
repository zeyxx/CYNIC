"""
CYNIC Discord Bot — Ask collective consciousness in Discord

Connects to CYNIC HTTP API and provides Discord commands for:
- /ask-cynic — Ask CYNIC a question
- /teach-cynic — Provide feedback on a judgment
- /cynic-test — Run empirical tests
- /cynic-status — Check CYNIC health

Usage: python bot.py
"""

import os
import sys
import asyncio
import logging
from typing import Optional
import platform
import aiohttp
import discord
from discord.ext import commands, tasks
from dotenv import load_dotenv

# Windows event loop fix
if platform.system() == "Windows":
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

# Load environment variables
load_dotenv()

# Logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s — %(message)s",
)
logger = logging.getLogger("cynic.discord")

# Configuration
DISCORD_TOKEN = os.getenv("DISCORD_TOKEN")
CYNIC_API_URL = os.getenv("CYNIC_API_URL", "http://localhost:8765")
CYNIC_API_TIMEOUT = int(os.getenv("CYNIC_API_TIMEOUT", "30"))


# Health check task
@tasks.loop(minutes=5)
async def check_cynic_health():
    """Check CYNIC API health periodically."""
    if bot.cynic_session is None or bot.cynic_session.closed:
        return

    try:
        async with bot.cynic_session.get(
            f"{CYNIC_API_URL}/health",
            timeout=aiohttp.ClientTimeout(total=5)
        ) as resp:
            if resp.status == 200:
                bot.cynic_ready = True
                logger.debug("✓ CYNIC health check passed")
            else:
                bot.cynic_ready = False
                logger.warning(f"✗ CYNIC health check failed: {resp.status}")
    except asyncio.TimeoutError:
        bot.cynic_ready = False
        logger.warning("✗ CYNIC health check timeout")
    except Exception as e:
        bot.cynic_ready = False
        logger.debug(f"✗ CYNIC health check error: {e}")

@check_cynic_health.before_loop
async def before_check_cynic():
    """Wait until bot is ready before checking health."""
    await bot.wait_until_ready()


# Initialize bot with intents (minimal for slash commands)
intents = discord.Intents.default()
intents.message_content = False  # Not needed for slash commands
bot = commands.Bot(command_prefix="/", intents=intents)

# Store CYNIC session on bot
bot.cynic_session = None
bot.cynic_ready = False


@bot.event
async def on_ready():
    """Bot connected and ready."""
    logger.info(f"Logged in as {bot.user}")

    # Initialize CYNIC session if not already done
    if bot.cynic_session is None:
        # Create session with connection limits to prevent leaks
        connector = aiohttp.TCPConnector(
            limit=10,
            limit_per_host=5,
            ttl_dns_cache=300,
            enable_cleanup_closed=True
        )
        timeout = aiohttp.ClientTimeout(
            total=CYNIC_API_TIMEOUT,
            connect=10,
            sock_read=10
        )
        bot.cynic_session = aiohttp.ClientSession(
            connector=connector,
            timeout=timeout
        )
        if not check_cynic_health.is_running():
            check_cynic_health.start()
        logger.info("CYNIC Discord bot ready (session initialized)")

    # Sync commands with Discord
    try:
        await bot.tree.sync()
        logger.info("Commands synced with Discord")
    except Exception as e:
        logger.error(f"Failed to sync commands: {e}")


@bot.event
async def on_error(event, *args, **kwargs):
    """Handle bot errors."""
    logger.error(f"Error in {event}: {sys.exc_info()}")


async def cleanup():
    """Graceful shutdown of bot and session."""
    logger.info("Shutting down CYNIC Discord bot...")

    # Stop health check task
    if check_cynic_health.is_running():
        check_cynic_health.cancel()
        logger.info("Health check stopped")

    # Close session
    if bot.cynic_session and not bot.cynic_session.closed:
        await bot.cynic_session.close()
        logger.info("Session closed")

    logger.info("Shutdown complete")


# Command group for CYNIC commands
@bot.tree.command(name="ask_cynic", description="Ask CYNIC a question")
async def ask_cynic(
    interaction: discord.Interaction,
    question: str,
    context: Optional[str] = None,
    reality: str = "GENERAL"
):
    """
    Ask CYNIC a question and get a judgment.

    Args:
        question: The question to ask CYNIC
        context: Optional context or background information
        reality: Type of reality (GENERAL, CODE, MARKET, SOCIAL)
    """
    if not bot.cynic_ready:
        await interaction.response.send_message(
            "❌ CYNIC is not available right now. Try again in a moment.",
            ephemeral=True
        )
        return

    # Defer response (might take a few seconds)
    await interaction.response.defer(thinking=True)

    try:
        payload = {
            "question": question,
            "context": context or question,
            "reality": reality,
        }

        async with bot.cynic_session.post(
            f"{CYNIC_API_URL}/judge",
            json=payload
        ) as resp:
            if resp.status != 200:
                await interaction.followup.send(
                    f"❌ CYNIC error: HTTP {resp.status}",
                    ephemeral=True
                )
                return

            result = await resp.json()

        # Format response
        q_score = result.get("q_score", 0)
        verdict = result.get("verdict", "UNKNOWN")
        confidence = result.get("confidence", 0)
        explanation = result.get("explanation", "No explanation provided")
        judgment_id = result.get("judgment_id", "unknown")

        # Color code by verdict
        verdict_colors = {
            "HOWL": 0xFF0000,      # Red (strong)
            "WAG": 0x00FF00,       # Green (yes)
            "GROWL": 0xFFFF00,     # Yellow (caution)
            "BARK": 0xFF6600,      # Orange (reject)
        }
        color = verdict_colors.get(verdict, 0x9933FF)

        # Create embed
        embed = discord.Embed(
            title="🐕 CYNIC Judgment",
            description=f"**Question:** {question}",
            color=color
        )

        embed.add_field(
            name="Q-Score",
            value=f"{q_score:.1f}/100",
            inline=True
        )
        embed.add_field(
            name="Verdict",
            value=f"`{verdict}`",
            inline=True
        )
        embed.add_field(
            name="Confidence",
            value=f"{confidence:.1%} (φ-bounded)",
            inline=True
        )

        if context:
            embed.add_field(
                name="Context",
                value=context[:256],
                inline=False
            )

        embed.add_field(
            name="Explanation",
            value=explanation[:512],
            inline=False
        )

        embed.add_field(
            name="Judgment ID",
            value=f"`{judgment_id}`",
            inline=False
        )

        embed.set_footer(
            text="Use /teach_cynic to provide feedback on this judgment"
        )

        await interaction.followup.send(embed=embed)
        logger.info(f"Judgment: {question[:50]}... → {verdict} ({q_score:.1f})")

    except asyncio.TimeoutError:
        await interaction.followup.send(
            "⏱️ CYNIC took too long to respond. Try again later.",
            ephemeral=True
        )
    except Exception as e:
        logger.error(f"Error calling CYNIC: {e}")
        await interaction.followup.send(
            f"❌ Error: {str(e)[:200]}",
            ephemeral=True
        )


@bot.tree.command(name="teach_cynic", description="Teach CYNIC from a judgment")
async def teach_cynic(
    interaction: discord.Interaction,
    judgment_id: str,
    rating: float,
    comment: Optional[str] = None
):
    """
    Provide feedback on a CYNIC judgment.

    Args:
        judgment_id: ID from the judgment response
        rating: Quality score (0.0 - 1.0, where 1.0 = excellent)
        comment: Optional comment on why you rated it this way
    """
    if not bot.cynic_ready:
        await interaction.response.send_message(
            "❌ CYNIC is not available right now.",
            ephemeral=True
        )
        return

    # Validate rating
    if not (0.0 <= rating <= 1.0):
        await interaction.response.send_message(
            "❌ Rating must be between 0.0 and 1.0",
            ephemeral=True
        )
        return

    await interaction.response.defer(thinking=True)

    try:
        payload = {
            "judgment_id": judgment_id,
            "rating": rating,
            "comment": comment or "",
        }

        async with bot.cynic_session.post(
            f"{CYNIC_API_URL}/learn",
            json=payload
        ) as resp:
            if resp.status != 200:
                await interaction.followup.send(
                    f"❌ CYNIC error: HTTP {resp.status}",
                    ephemeral=True
                )
                return

            result = await resp.json()

        # Format response
        new_q_score = result.get("new_q_score", 0)
        learning_rate = result.get("learning_rate", 0)
        q_table_entries = result.get("q_table_entries", 0)

        embed = discord.Embed(
            title="✅ CYNIC Learned",
            color=0x00FF00
        )
        embed.add_field(
            name="Your Rating",
            value=f"{rating:.1%}",
            inline=True
        )
        embed.add_field(
            name="New Q-Score",
            value=f"{new_q_score:.1f}/100",
            inline=True
        )
        embed.add_field(
            name="Learning Rate",
            value=f"{learning_rate:.4f} (Thompson-tuned)",
            inline=True
        )
        embed.add_field(
            name="Q-Table Size",
            value=f"{q_table_entries} states learned",
            inline=False
        )
        if comment:
            embed.add_field(
                name="Your Comment",
                value=comment[:256],
                inline=False
            )

        embed.set_footer(text="CYNIC improves with feedback")

        await interaction.followup.send(embed=embed)
        logger.info(f"Feedback: {judgment_id} rated {rating:.1%}")

    except asyncio.TimeoutError:
        await interaction.followup.send(
            "⏱️ CYNIC took too long. Try again later.",
            ephemeral=True
        )
    except Exception as e:
        logger.error(f"Error teaching CYNIC: {e}")
        await interaction.followup.send(
            f"❌ Error: {str(e)[:200]}",
            ephemeral=True
        )


@bot.tree.command(name="cynic_status", description="Check CYNIC system status")
async def cynic_status(interaction: discord.Interaction):
    """Get CYNIC system health and metrics."""
    if not bot.cynic_ready:
        await interaction.response.send_message(
            "❌ CYNIC is not available right now.",
            ephemeral=True
        )
        return

    await interaction.response.defer(thinking=True)

    try:
        # Get health
        async with bot.cynic_session.get(
            f"{CYNIC_API_URL}/health"
        ) as resp:
            if resp.status != 200:
                await interaction.followup.send(
                    "❌ CYNIC health check failed",
                    ephemeral=True
                )
                return
            health = await resp.json()

        # Get telemetry
        try:
            async with bot.cynic_session.get(
                f"{CYNIC_API_URL}/empirical/telemetry"
            ) as resp:
                if resp.status == 200:
                    telemetry = await resp.json()
                else:
                    telemetry = {}
        except:
            telemetry = {}

        # Format response
        embed = discord.Embed(
            title="🐕 CYNIC System Status",
            color=0x9933FF
        )

        # Health metrics
        embed.add_field(
            name="Status",
            value="🟢 Online" if bot.cynic_ready else "🔴 Offline",
            inline=True
        )

        # Consciousness
        consciousness = health.get("consciousness", {})
        embed.add_field(
            name="Consciousness Level",
            value=consciousness.get("current_level", "Unknown"),
            inline=True
        )

        # Learning
        learning = health.get("learning", {})
        q_table_entries = learning.get("q_table_entries", 0)
        embed.add_field(
            name="Q-Table Entries",
            value=f"{q_table_entries} states",
            inline=True
        )

        # Telemetry
        if telemetry:
            embed.add_field(
                name="Total Judgments",
                value=str(telemetry.get("total_judgments", "Unknown")),
                inline=True
            )
            embed.add_field(
                name="Dog Consensus",
                value=f"{telemetry.get('dog_consensus_rate', 0):.1%}",
                inline=True
            )
            uptime = telemetry.get("uptime_s", 0)
            hours = int(uptime // 3600)
            minutes = int((uptime % 3600) // 60)
            embed.add_field(
                name="Uptime",
                value=f"{hours}h {minutes}m",
                inline=True
            )

        # Dogs
        dogs = consciousness.get("dogs_active", 0)
        embed.add_field(
            name="Active Validators",
            value=f"{dogs}/11 Dogs online",
            inline=False
        )

        embed.set_footer(text="SONA heartbeat: Every 34 minutes")

        await interaction.followup.send(embed=embed)
        logger.info("Status check requested")

    except asyncio.TimeoutError:
        await interaction.followup.send(
            "⏱️ Status check timed out.",
            ephemeral=True
        )
    except Exception as e:
        logger.error(f"Error checking status: {e}")
        await interaction.followup.send(
            f"❌ Error: {str(e)[:200]}",
            ephemeral=True
        )


@bot.tree.command(name="cynic_empirical", description="Run empirical test")
async def cynic_empirical(
    interaction: discord.Interaction,
    count: int = 100
):
    """
    Run autonomous empirical test with CYNIC.

    Args:
        count: Number of judgments to run (default 100, max 10000)
    """
    if not bot.cynic_ready:
        await interaction.response.send_message(
            "❌ CYNIC is not available right now.",
            ephemeral=True
        )
        return

    # Validate count
    if not (1 <= count <= 10000):
        await interaction.response.send_message(
            "❌ Count must be between 1 and 10000",
            ephemeral=True
        )
        return

    await interaction.response.defer(thinking=True)

    try:
        # Start test
        payload = {"count": count, "seed": None}
        async with bot.cynic_session.post(
            f"{CYNIC_API_URL}/empirical/test/start",
            json=payload
        ) as resp:
            if resp.status != 200:
                await interaction.followup.send(
                    f"❌ Failed to start test: HTTP {resp.status}",
                    ephemeral=True
                )
                return

            result = await resp.json()

        job_id = result.get("job_id", "unknown")

        embed = discord.Embed(
            title="🧪 Empirical Test Started",
            color=0x00FF00
        )
        embed.add_field(
            name="Test ID",
            value=f"`{job_id}`",
            inline=False
        )
        embed.add_field(
            name="Judgments",
            value=f"{count} (running autonomously)",
            inline=True
        )
        embed.add_field(
            name="Status",
            value="⏳ In Progress",
            inline=True
        )

        embed.set_footer(
            text=f"Run /cynic_test_results {job_id} in 2-5 minutes to get results"
        )

        await interaction.followup.send(embed=embed)
        logger.info(f"Empirical test started: {job_id} ({count} judgments)")

    except asyncio.TimeoutError:
        await interaction.followup.send(
            "⏱️ Test startup timed out.",
            ephemeral=True
        )
    except Exception as e:
        logger.error(f"Error starting test: {e}")
        await interaction.followup.send(
            f"❌ Error: {str(e)[:200]}",
            ephemeral=True
        )


@bot.tree.command(name="cynic_test_results", description="Get empirical test results")
async def cynic_test_results(
    interaction: discord.Interaction,
    job_id: str
):
    """
    Get results from an empirical test.

    Args:
        job_id: Test ID from /cynic_empirical response
    """
    if not bot.cynic_ready:
        await interaction.response.send_message(
            "❌ CYNIC is not available right now.",
            ephemeral=True
        )
        return

    await interaction.response.defer(thinking=True)

    try:
        async with bot.cynic_session.get(
            f"{CYNIC_API_URL}/empirical/test/{job_id}/results"
        ) as resp:
            if resp.status == 404:
                await interaction.followup.send(
                    f"❌ Test `{job_id}` not found. Still running?",
                    ephemeral=True
                )
                return
            elif resp.status != 200:
                await interaction.followup.send(
                    f"❌ Error: HTTP {resp.status}",
                    ephemeral=True
                )
                return

            result = await resp.json()

        # Format results
        q_scores = result.get("q_scores", [])
        improvement = result.get("improvement_factor", 0)
        emergences = result.get("emergences_detected", 0)

        if not q_scores:
            await interaction.followup.send(
                "⏳ Test still running. Check again in a moment.",
                ephemeral=True
            )
            return

        avg_q = sum(q_scores) / len(q_scores)
        min_q = min(q_scores)
        max_q = max(q_scores)

        embed = discord.Embed(
            title="📊 Empirical Test Results",
            color=0x00FF00
        )
        embed.add_field(
            name="Judgments Completed",
            value=f"{len(q_scores)}",
            inline=True
        )
        embed.add_field(
            name="Average Q-Score",
            value=f"{avg_q:.1f}/100",
            inline=True
        )
        embed.add_field(
            name="Learning Improvement",
            value=f"{improvement:.2f}x baseline",
            inline=True
        )
        embed.add_field(
            name="Q-Score Range",
            value=f"{min_q:.1f} → {max_q:.1f}",
            inline=True
        )
        embed.add_field(
            name="Emergence Events",
            value=f"{emergences} patterns detected",
            inline=True
        )

        # Progression
        if len(q_scores) > 1:
            progress = f"{q_scores[0]:.1f} → {q_scores[-1]:.1f}"
            embed.add_field(
                name="Progression",
                value=f"`{progress}`",
                inline=False
            )

        embed.set_footer(text="CYNIC is continuously learning")

        await interaction.followup.send(embed=embed)
        logger.info(f"Test results: {job_id} avg_q={avg_q:.1f} improvement={improvement:.2f}x")

    except asyncio.TimeoutError:
        await interaction.followup.send(
            "⏱️ Results fetch timed out.",
            ephemeral=True
        )
    except Exception as e:
        logger.error(f"Error getting results: {e}")
        await interaction.followup.send(
            f"❌ Error: {str(e)[:200]}",
            ephemeral=True
        )


# Error handler
@bot.tree.error
async def on_app_command_error(interaction: discord.Interaction, error: Exception):
    """Handle command errors."""
    logger.error(f"Command error: {error}")
    if not interaction.response.is_done():
        await interaction.response.send_message(
            f"❌ Error: {str(error)[:200]}",
            ephemeral=True
        )


def main():
    """Run the bot."""
    if not DISCORD_TOKEN:
        logger.error("DISCORD_TOKEN not set. Set it in .env file.")
        sys.exit(1)

    logger.info("Starting CYNIC Discord bot...")
    logger.info(f"CYNIC API: {CYNIC_API_URL}")

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)

    try:
        loop.run_until_complete(bot.start(DISCORD_TOKEN))
    except KeyboardInterrupt:
        logger.info("Bot shutdown requested")
    finally:
        loop.run_until_complete(cleanup())
        loop.close()
        logger.info("Event loop closed")


if __name__ == "__main__":
    main()
