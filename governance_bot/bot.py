"""
CYNIC Governance Bot - Main Discord Bot Implementation
"""

# Windows event loop compatibility fix for aiohttp/aiodns
import sys
import asyncio
if sys.platform == 'win32':
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

import logging
import discord
from discord.ext import commands, tasks
from datetime import datetime, timedelta
import uuid

from config import DISCORD_TOKEN, DISCORD_PREFIX, CYNIC_MCP_ENABLED
from database import (
    init_db, get_session, session_context, get_community, create_community,
    create_proposal, get_proposal, list_proposals, update_proposal_status,
    update_proposal_judgment, create_vote, count_votes, update_vote_counts,
    is_voting_active, check_voting_closed, get_user_vote
)
from cynic_integration import ask_cynic, learn_cynic, observe_cynic, get_cynic_status
from formatting import (
    format_proposal_embed, format_voting_status, format_cynic_verdict,
    format_proposal_created, format_vote_recorded, format_error, format_help,
    build_proposal_embed
)
from views import ProposalModal, VotingView, ProposalListView

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Bot setup
intents = discord.Intents.default()
intents.message_content = True
intents.guilds = True

bot = commands.Bot(command_prefix=DISCORD_PREFIX, intents=intents)


@bot.event
async def on_ready():
    """Bot ready event"""
    logger.info(f"Bot logged in as {bot.user}")

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

    check_voting_status.start()


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
# VOTING COMMANDS (voting via buttons only — slash command /vote removed)
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
            response = f"🧠 **CYNIC STATUS**: Online\n\n{status.get('data', 'No data')}"
        else:
            response = "🧠 **CYNIC STATUS**: Offline\n\nCYNIC kernel is not responding."

        await interaction.followup.send(response[:2000])  # Discord message limit

    except Exception as e:
        logger.error(f"Error getting CYNIC status: {e}")
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
🏛️ **GOVERNANCE SETTINGS**

**Community:** {community.community_name}
**Token:** {community.community_token}

**Voting:**
• Period: {community.voting_period_hours} hours
• Quorum: {community.quorum_percentage}%
• Approval Threshold: {community.approval_threshold_percentage}%
• Method: {community.voting_method}

**Integration:**
• GASdf: {'Enabled' if community.gasdf_enabled else 'Disabled'}
• NEAR: {'Enabled' if community.near_contract_address else 'Disabled'}
• CYNIC: {'Enabled' if community.cynic_enabled else 'Disabled'}
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
📊 **GOVERNANCE STATISTICS**

**Proposals:**
• Total: {total}
• Approved: {approved}
• Rejected: {rejected}
• Approval Rate: {(approved/total*100 if total > 0 else 0):.1f}%

**Participation:**
• Total Votes Cast: {sum(p.yes_votes + p.no_votes for p in proposals):.0f}
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


# ============================================================================
# BACKGROUND TASKS
# ============================================================================

@tasks.loop(minutes=5)
async def check_voting_status():
    """Check if voting periods have ended and update statuses"""
    try:
        async with session_context() as session:
            # This would iterate through active proposals and check voting status
            # Implementation details depend on database queries
            logger.debug("Checking voting status...")
    except Exception as e:
        logger.error(f"Error in check_voting_status: {e}")


# ============================================================================
# ERROR HANDLER
# ============================================================================

@bot.tree.error
async def on_command_error(interaction: discord.Interaction, error: discord.app_commands.AppCommandError):
    """Handle command errors"""
    logger.error(f"Command error: {error}")
    await interaction.response.send_message(
        format_error(f"An error occurred: {error}"),
        ephemeral=True
    )


# ============================================================================
# BOT STARTUP
# ============================================================================

async def main():
    """Start the bot"""
    try:
        await bot.start(DISCORD_TOKEN)
    except Exception as e:
        logger.error(f"Failed to start bot: {e}")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
