"""
CYNIC Governance Bot - Main Discord Bot Implementation
"""

# Windows event loop compatibility fix for aiohttp/aiodns
import asyncio
import sys

if sys.platform == 'win32':
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

import logging

import discord
from discord.ext import commands, tasks

from cynic.interfaces.bots.governance.core.config import (
    DISCORD_PREFIX,
    DISCORD_TOKEN,
)
from cynic.interfaces.bots.governance.core.database import (
    backup_database,
    check_voting_closed,
    close_db,
    create_learning_outcome,
    db_health_check,
    get_community,
    get_or_create_e_score,
    get_proposal,
    get_proposals_needing_outcome,
    init_db,
    list_proposals,
    mark_outcome_determined,
    session_context,
    verify_data_consistency,
)
from cynic.interfaces.bots.governance.core.error_handler import (
    cynic_circuit_breaker,
    health_check,
)
from cynic.interfaces.bots.governance.integration.cynic_integration import (
    get_cynic_status,
    learn_cynic,
)
from cynic.interfaces.bots.governance.utils.formatting import (
    build_outcome_embed,
    build_proposal_embed,
    format_cynic_verdict,
    format_error,
    format_help,
    format_voting_status,
)
from cynic.interfaces.bots.governance.utils.views import (
    OutcomeRatingView,
    ProposalListView,
    ProposalModal,
    VotingView,
)

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

def _find_announcement_channel(guild: discord.Guild) -> discord.TextChannel:
    """Find an appropriate text channel for announcements"""
    # Priority: franais/gnral, governance, proposals, general
    channel_names = ["franais", "gnral", "governance", "proposals", "general"]

    for name in channel_names:
        for channel in guild.text_channels:
            if channel.name.lower() == name.lower() and channel.permissions_for(guild.me).send_messages:
                if channel.permissions_for(guild.me).embed_links:
                    return channel

    # Fallback: first writable text channel
    for channel in guild.text_channels:
        if channel.permissions_for(guild.me).send_messages and channel.permissions_for(guild.me).embed_links:
            return channel

    return None


async def _process_closed_proposal(proposal):
    """Process a newly-closed proposal: create outcome record, update E-Score, post embed"""
    try:
        async with session_context() as session:
            # 1. Create learning outcome audit record
            await create_learning_outcome(
                session,
                proposal.proposal_id,
                proposal.approval_status,
                satisfaction_rating=None  # placeholder, will be updated by community
            )

            # 2. Update CYNIC E-Score correctness
            community_id = proposal.community_id
            cynic_correct = (proposal.judgment_verdict in {"HOWL", "WAG"}) == (proposal.approval_status == "APPROVED")

            # Get current E-Score stats for update
            from cynic.interfaces.bots.governance.core.database import update_e_score
            e_score = await get_or_create_e_score(session, community_id, "CYNIC", "cynic_main")
            new_total = (e_score.total_judgments or 0) + 1
            new_correct = (e_score.correct_predictions or 0) + (1 if cynic_correct else 0)
            new_accuracy = new_correct / new_total if new_total > 0 else 0.0

            await update_e_score(session, community_id, "CYNIC", "cynic_main", {
                "total_judgments": new_total,
                "correct_predictions": new_correct,
                "judgment_accuracy": new_accuracy,
                "last_update_source": "PROPOSAL_OUTCOME"
            })

            # 3. Mark outcome as determined (prevents re-processing)
            await mark_outcome_determined(session, proposal.proposal_id, proposal.approval_status)

        # 4. Find announcement channel and post outcome embed
        for guild in bot.guilds:
            if guild.id == int(proposal.community_id.replace("discord_", "")):
                channel = _find_announcement_channel(guild)
                if channel:
                    embed = build_outcome_embed(proposal)
                    try:
                        await channel.send(embed=embed, view=OutcomeRatingView(proposal.proposal_id))
                        logger.info(f"Outcome embed posted for {proposal.proposal_id} in {guild.name}")
                    except Exception as e:
                        logger.error(f"Failed to post outcome embed: {e}")
                break

        # 5. Auto-learn with placeholder rating (with circuit breaker check)
        verdict = proposal.judgment_verdict or "PENDING"
        approved = proposal.approval_status == "APPROVED"

        if cynic_circuit_breaker.is_available():
            try:
                await learn_cynic(
                    judgment_id=proposal.judgment_id,
                    verdict=verdict,
                    approved=approved,
                    satisfaction=3.0,
                    comment="Auto-learning from proposal outcome"
                )
                cynic_circuit_breaker.record_success()
                logger.info(f"Processed closed proposal: {proposal.proposal_id}")

            except Exception as cynic_err:
                logger.error(f"CYNIC auto-learning failed for {proposal.proposal_id}: {cynic_err}")
                cynic_circuit_breaker.record_failure()
        else:
            logger.warning(f"CYNIC unavailable for {proposal.proposal_id}: {cynic_circuit_breaker.get_status()}")

    except Exception as e:
        logger.error(f"Error processing closed proposal {proposal.proposal_id}: {e}", exc_info=True)


# Bot setup
intents = discord.Intents.default()
intents.message_content = True
intents.guilds = True

bot = commands.Bot(command_prefix=DISCORD_PREFIX, intents=intents)


@bot.event
async def on_ready():
    """Bot ready event"""
    logger.info(f"Bot logged in as {bot.user}")
    logger.info(f"Circuit breaker initialized: {cynic_circuit_breaker.get_status()}")

    # Sync slash commands with Discord
    try:
        synced = await bot.tree.sync()
        logger.info(f"Synced {len(synced)} slash commands to Discord")
    except Exception as e:
        logger.error(f"Failed to sync slash commands: {e}")

    await init_db()

    # Register persistent views for active proposals
    async with session_context() as session:
        for guild in bot.guilds:
            community_id = f"discord_{guild.id}"
            active = await list_proposals(session, community_id, status="ACTIVE")
            for proposal in active:
                bot.add_view(VotingView(proposal_id=proposal.proposal_id))
                logger.info(f"Registered persistent view: {proposal.proposal_id}")

    try:
        if not check_voting_status.is_running():
            check_voting_status.start()
            logger.info("Background task check_voting_status started")
    except Exception as e:
        logger.error(f"Failed to start background task: {e}", exc_info=True)

    # Log health status
    try:
        health = await health_check(bot)
        logger.info(f"Health check on ready: {health['bot_status']}, latency={health['discord_latency_ms']}ms")
    except Exception as e:
        logger.warning(f"Failed to get health status on startup: {e}")


# ============================================================================
# PROPOSAL COMMANDS
# ============================================================================

@bot.tree.command(
    name="propose",
    description="Submit a new governance proposal"
)
async def cmd_propose(interaction: discord.Interaction):
    """Submit a new proposal via modal"""
    await interaction.response.send_modal(ProposalModal())


@bot.tree.command(
    name="proposal_details",
    description="View full proposal details"
)
async def cmd_proposal_details(
    interaction: discord.Interaction,
    proposal_id: str
):
    """View proposal details with voting"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            proposal = await get_proposal(session, proposal_id)

            if not proposal:
                await interaction.followup.send(format_error(f"Proposal {proposal_id} not found"))
                return

            embed = build_proposal_embed(proposal)
            await interaction.followup.send(embed=embed, view=VotingView(proposal_id=proposal_id))

    except Exception as e:
        logger.error(f"Error fetching proposal: {e}")
        await interaction.followup.send(format_error(str(e)))


@bot.tree.command(
    name="proposals",
    description="List governance proposals"
)
async def cmd_proposals(
    interaction: discord.Interaction,
    status: str = "ACTIVE"
):
    """List proposals with pagination"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            community_id = f"discord_{interaction.guild.id}"
            proposals = await list_proposals(session, community_id, status)

            if not proposals:
                await interaction.followup.send(f"No {status} proposals found.")
                return

            list_view = ProposalListView(proposals=proposals, community_id=community_id)
            await interaction.followup.send(embed=list_view._build_list_embed(), view=list_view)

    except Exception as e:
        logger.error(f"Error listing proposals: {e}")
        await interaction.followup.send(format_error(str(e)))


# ============================================================================
# VOTING COMMANDS (voting via buttons only " slash command /vote removed)
# ============================================================================

@bot.tree.command(
    name="voting_status",
    description="View voting progress on a proposal"
)
async def cmd_voting_status(
    interaction: discord.Interaction,
    proposal_id: str
):
    """View voting status"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            proposal = await get_proposal(session, proposal_id)

            if not proposal:
                await interaction.followup.send(format_error(f"Proposal {proposal_id} not found"))
                return

            response = await format_voting_status(proposal, None)
            await interaction.followup.send(response)

    except Exception as e:
        logger.error(f"Error fetching voting status: {e}")
        await interaction.followup.send(format_error(str(e)))


# ============================================================================
# CYNIC JUDGMENT COMMANDS
# ============================================================================

@bot.tree.command(
    name="cynic_verdict",
    description="Get CYNIC's judgment on a proposal"
)
async def cmd_cynic_verdict(
    interaction: discord.Interaction,
    proposal_id: str
):
    """Get CYNIC verdict"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            proposal = await get_proposal(session, proposal_id)

            if not proposal:
                await interaction.followup.send(format_error(f"Proposal {proposal_id} not found"))
                return

            if not proposal.judgment_verdict:
                await interaction.followup.send("CYNIC judgment not yet available for this proposal.")
                return

            # Format judgment
            judgment_data = {
                "verdict": proposal.judgment_verdict,
                "q_score": proposal.judgment_q_score or 0.0,
                "confidence": proposal.judgment_confidence or 0.618,
                "reasoning": proposal.judgment_data.get("reasoning", "") if proposal.judgment_data else ""
            }

            response = format_cynic_verdict(judgment_data)
            await interaction.followup.send(response)

    except Exception as e:
        logger.error(f"Error fetching CYNIC verdict: {e}")
        await interaction.followup.send(format_error(str(e)))


@bot.tree.command(
    name="cynic_status",
    description="Check CYNIC organism status"
)
async def cmd_cynic_status(interaction: discord.Interaction):
    """Get CYNIC status"""
    await interaction.response.defer(thinking=True)

    try:
        status = await get_cynic_status()

        if status.get("status") == "online":
            response = f" **CYNIC STATUS**: Online\n\n{status.get('data', 'No data')}"
        else:
            response = " **CYNIC STATUS**: Offline\n\nCYNIC kernel is not responding."

        await interaction.followup.send(response[:2000])  # Discord message limit

    except Exception as e:
        logger.error(f"Error getting CYNIC status: {e}")
        await interaction.followup.send(format_error(str(e)))


@bot.tree.command(
    name="cynic_stats",
    description="View CYNIC learning statistics"
)
async def cmd_cynic_stats(interaction: discord.Interaction):
    """Get CYNIC learning statistics"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            community_id = f"discord_{interaction.guild.id}"
            proposals = await list_proposals(session, community_id)

            # Count proposal statuses
            total = len(proposals)
            closed = len([p for p in proposals if p.voting_status == "CLOSED"])
            learned = len([p for p in proposals if p.outcome_determined])
            approved = len([p for p in proposals if p.approval_status == "APPROVED"])
            rejected = len([p for p in proposals if p.approval_status == "REJECTED"])

            # Count ratings
            rated = len([p for p in proposals if p.community_satisfaction_rating is not None])
            if rated > 0:
                avg_rating = sum(p.community_satisfaction_rating for p in proposals if p.community_satisfaction_rating is not None) / rated
            else:
                avg_rating = 0.0

            # Get CYNIC E-Score
            e_score = await get_or_create_e_score(session, community_id, "CYNIC", "cynic_main")
            accuracy = (e_score.judgment_accuracy or 0.0) * 100

            text = f"""
 **CYNIC LEARNING STATISTICS**

**Proposals:**
 Total: {total}
 Closed: {closed}
 Learned from: {learned}

**Outcomes:**
 Approved: {approved}
 Rejected: {rejected}

**Community Feedback:**
 Rated: {rated}/{learned} outcomes
 Average rating: {avg_rating:.1f}/5 

**CYNIC Accuracy:**
 Total judgments: {e_score.total_judgments or 0}
 Correct predictions: {e_score.correct_predictions or 0}
 Accuracy: {accuracy:.1f}%

**Learning Signal:**
Each outcome trains CYNIC via Q-Table TD(0) update through /learn endpoint.
Community ratings (1-5 stars) inform reward signal for next judgment cycle.
"""
            await interaction.followup.send(text)

    except Exception as e:
        logger.error(f"Error fetching CYNIC stats: {e}")
        await interaction.followup.send(format_error(str(e)))


# ============================================================================
# COMMUNITY COMMANDS
# ============================================================================

@bot.tree.command(
    name="community_info",
    description="View community governance settings"
)
async def cmd_community_info(interaction: discord.Interaction):
    """View community info"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            community_id = f"discord_{interaction.guild.id}"
            community = await get_community(session, community_id)

            if not community:
                await interaction.followup.send("This community hasn't set up governance yet.")
                return

            text = f"""
 **GOVERNANCE SETTINGS**

**Community:** {community.community_name}
**Token:** {community.community_token}

**Voting:**
 Period: {community.voting_period_hours} hours
 Quorum: {community.quorum_percentage}%
 Approval Threshold: {community.approval_threshold_percentage}%
 Method: {community.voting_method}

**Integration:**
 GASdf: {'Enabled' if community.gasdf_enabled else 'Disabled'}
 NEAR: {'Enabled' if community.near_contract_address else 'Disabled'}
 CYNIC: {'Enabled' if community.cynic_enabled else 'Disabled'}
"""
            await interaction.followup.send(text)

    except Exception as e:
        logger.error(f"Error fetching community info: {e}")
        await interaction.followup.send(format_error(str(e)))


@bot.tree.command(
    name="governance_stats",
    description="View community governance statistics"
)
async def cmd_governance_stats(interaction: discord.Interaction):
    """View governance statistics"""
    await interaction.response.defer(thinking=True)

    try:
        async with session_context() as session:
            community_id = f"discord_{interaction.guild.id}"

            proposals = await list_proposals(session, community_id)

            total = len(proposals)
            approved = len([p for p in proposals if p.approval_status == "APPROVED"])
            rejected = len([p for p in proposals if p.approval_status == "REJECTED"])

            text = f"""
" **GOVERNANCE STATISTICS**

**Proposals:**
 Total: {total}
 Approved: {approved}
 Rejected: {rejected}
 Approval Rate: {(approved/total*100 if total > 0 else 0):.1f}%

**Participation:**
 Total Votes Cast: {sum(p.yes_votes + p.no_votes for p in proposals):.0f}
"""
            await interaction.followup.send(text)

    except Exception as e:
        logger.error(f"Error fetching governance stats: {e}")
        await interaction.followup.send(format_error(str(e)))


# ============================================================================
# HELP COMMAND
# ============================================================================

@bot.tree.command(
    name="help",
    description="Get help with governance bot commands"
)
async def cmd_help(interaction: discord.Interaction):
    """Get help"""
    await interaction.response.defer(thinking=True)
    help_text = format_help()
    await interaction.followup.send(help_text)


@bot.tree.command(
    name="health",
    description="Check bot and CYNIC system health"
)
async def cmd_health(interaction: discord.Interaction):
    """Get health status"""
    await interaction.response.defer(thinking=True)

    try:
        health_status = await health_check(bot)

        health_text = f"""
 **SYSTEM HEALTH CHECK**

**Bot Status:** {health_status['bot_status']}
**Discord Latency:** {health_status['discord_latency_ms']}ms
**Circuit Breaker:** {health_status['circuit_breaker']}

**Error Metrics:**
 Total Errors: {health_status['error_metrics']['total_errors']}
 Last Error: {health_status['error_metrics']['last_error']}

**Timestamp:** {health_status['timestamp']}
"""
        await interaction.followup.send(health_text)

    except Exception as e:
        logger.error(f"Error checking health: {e}")
        await interaction.followup.send(format_error(f"Health check failed: {e}"))


@bot.tree.command(
    name="database_status",
    description="Check database health and consistency"
)
async def cmd_database_status(interaction: discord.Interaction):
    """Check database health"""
    await interaction.response.defer(thinking=True)

    try:
        # Database health
        db_health = await db_health_check.check_health()

        # Data consistency
        consistency = await verify_data_consistency()

        db_text = f"""
' **DATABASE STATUS**

**Health:** {db_health['status']}
**Tables Found:** {db_health['tables_found']}
**Connectivity:** {db_health['connectivity']}

**Consistency Check:** {consistency['status']}
 Issues Found: {consistency['issue_count']}

**Last Check:** {db_health['timestamp']}
"""
        await interaction.followup.send(db_text)

    except Exception as e:
        logger.error(f"Error checking database status: {e}")
        await interaction.followup.send(format_error(f"Database check failed: {e}"))


@bot.tree.command(
    name="database_backup",
    description="Create manual database backup"
)
async def cmd_database_backup(interaction: discord.Interaction):
    """Create database backup"""
    await interaction.response.defer(thinking=True)

    try:
        backup_file = await backup_database()
        if backup_file:
            await interaction.followup.send(f" Database backed up to `{backup_file}`")
        else:
            await interaction.followup.send(" Backup failed. Check logs for details.")

    except Exception as e:
        logger.error(f"Error creating backup: {e}")
        await interaction.followup.send(format_error(f"Backup failed: {e}"))


# ============================================================================
# BACKGROUND TASKS
# ============================================================================

@tasks.loop(minutes=5)
async def check_voting_status():
    """Background task: close voting on past-deadline proposals and process outcomes"""
    try:
        logger.info("Background task: check_voting_status starting...")

        # Pass 1: Close ACTIVE proposals for all communities that have passed their voting deadline
        try:
            async with session_context() as session:
                # Get all ACTIVE proposals across all communities
                from datetime import datetime
                now = datetime.utcnow()

                # Query all ACTIVE proposals and check if any are past deadline
                await list_proposals(session, "", status="ACTIVE") if False else []
                # Fallback: check all communities from bot.guilds
                for guild in bot.guilds:
                    community_id = f"discord_{guild.id}"
                    active = await list_proposals(session, community_id, status="ACTIVE")
                    for proposal in active:
                        if proposal.voting_end_time < now:
                            await check_voting_closed(session, proposal.proposal_id)
            logger.debug("Pass 1 complete: voting closed for overdue proposals")
        except Exception as e:
            logger.error(f"Error in Pass 1: {e}", exc_info=True)

        # Pass 2: Process newly-closed proposals that need outcome determination
        try:
            async with session_context() as session:
                proposals_needing_outcome = await get_proposals_needing_outcome(session)
                logger.info(f"Found {len(proposals_needing_outcome)} proposals needing outcome processing")

            for proposal in proposals_needing_outcome:
                try:
                    await _process_closed_proposal(proposal)
                except Exception as e:
                    logger.error(f"Failed to process proposal {proposal.proposal_id}: {e}", exc_info=True)
            if proposals_needing_outcome:
                logger.debug("Pass 2 complete: outcomes processed")
        except Exception as e:
            logger.error(f"Error in Pass 2: {e}", exc_info=True)

    except Exception as e:
        logger.error(f"Critical error in check_voting_status: {e}", exc_info=True)


# ============================================================================
# ERROR HANDLER
# ============================================================================

@bot.tree.error
async def on_command_error(interaction: discord.Interaction, error: discord.app_commands.AppCommandError):
    """Handle command errors"""
    logger.error(f"Command error: {error}")
    try:
        # After any defer(), we MUST use followup.send() - this avoids race conditions
        if not interaction.response.is_done():
            # Interaction not yet responded to - defer first, then respond
            try:
                await interaction.response.defer(ephemeral=True)
            except (discord.errors.HTTPException, discord.errors.NotFound):
                # Already deferred/expired - use followup instead
                pass

        # Always use followup for safety (works whether deferred or already responded)
        await interaction.followup.send(
            format_error(f"An error occurred: {error}"),
            ephemeral=True
        )
    except discord.errors.NotFound:
        # Interaction no longer valid (expired after 3 seconds)
        logger.warning("Could not send error response: interaction expired")
    except discord.errors.HTTPException as e:
        # HTTP errors (rate limit, already acknowledged, etc.)
        if "already been acknowledged" in str(e):
            logger.warning("Interaction already acknowledged, cannot send error response")
        else:
            logger.error(f"HTTP error sending error response: {e}")
    except Exception as e:
        logger.error(f"Error sending error response: {e}", exc_info=True)


# ============================================================================
# BOT STARTUP
# ============================================================================

async def main():
    """Start the bot with proper cleanup."""
    try:
        await bot.start(DISCORD_TOKEN)
    except KeyboardInterrupt:
        logger.info("Bot stopping via KeyboardInterrupt...")
    except Exception as e:
        logger.error(f"Failed to start bot: {e}")
    finally:
        # Final cleanup: close database connections and other resources
        try:
            logger.info("Closing database connections...")
            await close_db()
            logger.info("Cleanup complete")
        except Exception as cleanup_err:
            logger.error(f"Error during cleanup: {cleanup_err}")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
