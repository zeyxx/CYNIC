"""
Discord UI Components (Modals, Buttons, Views)
"""

import logging
from datetime import datetime

import discord

from cynic.interfaces.bots.governance.core.config import CYNIC_MCP_ENABLED
from cynic.interfaces.bots.governance.core.database import (
    create_community,
    create_proposal,
    create_vote,
    get_community,
    get_proposal,
    get_user_vote,
    is_voting_active,
    session_context,
    update_vote_counts,
)
from cynic.interfaces.bots.governance.core.error_handler import (
    cynic_circuit_breaker,
)
from cynic.interfaces.bots.governance.integration.cynic_integration import ask_cynic, learn_cynic
from cynic.interfaces.bots.governance.utils.formatting import build_proposal_embed

logger = logging.getLogger(__name__)


class ProposalModal(discord.ui.Modal, title="Submit Governance Proposal"):
    """Modal form for submitting a new proposal"""

    proposal_title = discord.ui.TextInput(
        label="Proposal Title",
        placeholder="What should the proposal be about?",
        max_length=200,
        required=True
    )
    description = discord.ui.TextInput(
        label="Description",
        placeholder="Provide detailed description of the proposal...",
        style=discord.TextStyle.paragraph,
        max_length=2000,
        required=True
    )
    category = discord.ui.TextInput(
        label="Category (optional)",
        placeholder="COMMUNITY_DECISION, BUDGET_ALLOCATION, PARTNERSHIP, etc.",
        max_length=100,
        required=False
    )

    async def on_submit(self, interaction: discord.Interaction):
        """Handle modal submission"""
        await interaction.response.defer(thinking=True)

        try:
            async with session_context() as session:
                # Get or create community
                community_id = f"discord_{interaction.guild.id}"
                community = await get_community(session, community_id)

                if not community:
                    community = await create_community(session, {
                        "community_id": community_id,
                        "platform": "discord",
                        "community_name": interaction.guild.name or "Unknown"
                    })

                # Normalize category
                category = str(self.category).strip() or "COMMUNITY_DECISION"
                valid_categories = ["COMMUNITY_DECISION", "BUDGET_ALLOCATION", "GOVERNANCE_CHANGE", "PARTNERSHIP"]
                if category not in valid_categories:
                    category = "COMMUNITY_DECISION"

                # Create proposal
                import uuid
                from datetime import timedelta
                proposal_id = f"prop_{datetime.utcnow().strftime('%Y%m%d')}_{str(uuid.uuid4())[:8]}"
                now = datetime.utcnow()

                proposal = await create_proposal(session, {
                    "proposal_id": proposal_id,
                    "community_id": community_id,
                    "proposer_id": str(interaction.user.id),
                    "title": str(self.proposal_title),
                    "description": str(self.description),
                    "category": category,
                    "impact_level": "MEDIUM",
                    "voting_start_time": now,
                    "voting_end_time": now + timedelta(hours=community.voting_period_hours),
                    "voting_status": "ACTIVE"
                })

                # Ask CYNIC for judgment (with circuit breaker check)
                if CYNIC_MCP_ENABLED:
                    if not cynic_circuit_breaker.is_available():
                        logger.warning(f"CYNIC unavailable (circuit breaker open): {cynic_circuit_breaker.get_status()}")
                        await interaction.followup.send(
                            " CYNIC is temporarily unavailable. Proposal created but judgment deferred.",
                            ephemeral=True
                        )
                    else:
                        try:
                            judgment = await ask_cynic(
                                question=str(self.proposal_title),
                                context=str(self.description),
                                reality="GOVERNANCE"
                            )

                            if judgment.get("verdict") != "PENDING":
                                from cynic.interfaces.bots.governance.core.database import (
                                    update_proposal_judgment,
                                )
                                await update_proposal_judgment(session, proposal_id, judgment)
                                proposal.judgment_verdict = judgment.get("verdict")
                                proposal.judgment_q_score = judgment.get("q_score")

                            # Record success
                            cynic_circuit_breaker.record_success()

                        except Exception as cynic_err:
                            logger.error(f"CYNIC judgment failed: {cynic_err}")
                            cynic_circuit_breaker.record_failure()
                            await interaction.followup.send(
                                " CYNIC judgment failed. Continuing without verdict.",
                                ephemeral=True
                            )

                # Build embed and send with voting buttons
                embed = build_proposal_embed(proposal)
                view = VotingView(proposal_id=proposal_id)

                await interaction.followup.send(embed=embed, view=view)
                logger.info(f"Proposal created: {proposal_id}")

        except Exception as e:
            logger.error(f"Error creating proposal: {e}", exc_info=True)
            await interaction.followup.send(f" Error creating proposal: {e}", ephemeral=True)

    async def on_error(self, interaction: discord.Interaction, error: Exception):
        """Handle modal errors"""
        logger.error(f"Modal error: {error}", exc_info=True)
        await interaction.response.send_message(f" Error: {error}", ephemeral=True)


class VotingView(discord.ui.View):
    """Persistent view with voting buttons for a proposal"""

    def __init__(self, proposal_id: str):
        super().__init__(timeout=None)  # persistent
        self.proposal_id = proposal_id
        self._build_buttons()

    def _build_buttons(self):
        """Build the voting buttons"""
        for choice, style, label in [
            ("YES", discord.ButtonStyle.green, " YES"),
            ("NO", discord.ButtonStyle.red, " NO"),
            ("ABSTAIN", discord.ButtonStyle.grey, " ABSTAIN"),
        ]:
            btn = discord.ui.Button(
                label=label,
                style=style,
                custom_id=f"vote_{choice}_{self.proposal_id}",
                row=0
            )
            btn.callback = self._vote_callback_factory(choice)
            self.add_item(btn)

    def _vote_callback_factory(self, vote_choice):
        """Factory to create vote callback for a specific choice"""
        async def callback(interaction):
            await self._handle_vote(interaction, vote_choice)
        return callback

    async def _handle_vote(self, interaction: discord.Interaction, vote_choice: str):
        """Handle a vote button click"""
        try:
            async with session_context() as session:
                # Check if voting is active
                if not await is_voting_active(session, self.proposal_id):
                    await interaction.response.send_message("- Voting is closed.", ephemeral=True)
                    return

                voter_id = str(interaction.user.id)

                # Check for existing vote
                existing = await get_user_vote(session, self.proposal_id, voter_id)
                if existing and existing.vote == vote_choice:
                    await interaction.response.send_message(
                        f"You already voted {vote_choice}.", ephemeral=True
                    )
                    return

                # If changing vote, ask for confirmation
                if existing:
                    view = ChangeVoteConfirmView(self.proposal_id, existing.vote, vote_choice)
                    await interaction.response.send_message(
                        f"You voted **{existing.vote}** " change to **{vote_choice}**?",
                        view=view, ephemeral=True
                    )
                    return

                # Record the vote
                vote_id = f"vote_{self.proposal_id}_{voter_id}"
                await create_vote(session, {
                    "vote_id": vote_id,
                    "proposal_id": self.proposal_id,
                    "voter_id": voter_id,
                    "vote": vote_choice,
                    "vote_weight": 1.0,
                    "reasoning": ""
                })

                # Update vote counts
                await update_vote_counts(session, self.proposal_id)
                proposal = await get_proposal(session, self.proposal_id)

                # Show confirmation
                await interaction.response.send_message(
                    f"{vote_choice} recorded! YES: {proposal.yes_votes:.0f} | NO: {proposal.no_votes:.0f} | ABSTAIN: {proposal.abstain_votes:.0f}",
                    ephemeral=True
                )

        except Exception as e:
            logger.error(f"Error handling vote: {e}", exc_info=True)
            await interaction.response.send_message(f" Error: {e}", ephemeral=True)


class ChangeVoteConfirmView(discord.ui.View):
    """View for confirming a vote change"""

    def __init__(self, proposal_id: str, old_vote: str, new_vote: str):
        super().__init__(timeout=60)
        self.proposal_id = proposal_id
        self.old_vote = old_vote
        self.new_vote = new_vote

    @discord.ui.button(label=" Confirm", style=discord.ButtonStyle.green)
    async def confirm(self, interaction: discord.Interaction, button: discord.ui.Button):
        """Confirm vote change"""
        try:
            async with session_context() as session:
                voter_id = str(interaction.user.id)
                vote_id = f"vote_{self.proposal_id}_{voter_id}"

                # Record new vote (old vote is auto-deleted by create_vote)
                await create_vote(session, {
                    "vote_id": vote_id,
                    "proposal_id": self.proposal_id,
                    "voter_id": voter_id,
                    "vote": self.new_vote,
                    "vote_weight": 1.0,
                    "reasoning": ""
                })

                # Update vote counts
                await update_vote_counts(session, self.proposal_id)
                proposal = await get_proposal(session, self.proposal_id)

                await interaction.response.edit_message(
                    content=f" Vote changed to **{self.new_vote}**! YES: {proposal.yes_votes:.0f} | NO: {proposal.no_votes:.0f} | ABSTAIN: {proposal.abstain_votes:.0f}",
                    view=None
                )

        except Exception as e:
            logger.error(f"Error confirming vote: {e}", exc_info=True)
            await interaction.response.edit_message(content=f" Error: {e}", view=None)

    @discord.ui.button(label=" Cancel", style=discord.ButtonStyle.grey)
    async def cancel(self, interaction: discord.Interaction, button: discord.ui.Button):
        """Cancel vote change"""
        await interaction.response.edit_message(
            content=f"Keeping your **{self.old_vote}** vote.",
            view=None
        )


class ProposalListView(discord.ui.View):
    """Paginated view for browsing proposals"""

    PAGE_SIZE = 5

    def __init__(self, proposals: list, community_id: str):
        super().__init__(timeout=120)
        self.proposals = proposals
        self.community_id = community_id
        self.current_page = 0
        self._refresh_buttons()

    def _refresh_buttons(self):
        """Refresh pagination and view buttons"""
        self.clear_items()

        # Add [View #N] buttons for current page
        start = self.current_page * self.PAGE_SIZE
        end = start + self.PAGE_SIZE
        page_proposals = self.proposals[start:end]

        for idx, proposal in enumerate(page_proposals):
            btn = discord.ui.Button(
                label=f"View #{idx + 1}",
                style=discord.ButtonStyle.blurple,
                custom_id=f"view_prop_{proposal.proposal_id}",
                row=0
            )
            btn.callback = self._make_view_callback(proposal.proposal_id)
            self.add_item(btn)

        # Add pagination buttons
        prev_btn = discord.ui.Button(
            label="< Prev",
            style=discord.ButtonStyle.grey,
            custom_id=f"prev_page_{self.current_page}",
            row=1
        )
        prev_btn.callback = self._prev_page
        if self.current_page == 0:
            prev_btn.disabled = True
        self.add_item(prev_btn)

        # Page counter (disabled)
        total_pages = (len(self.proposals) + self.PAGE_SIZE - 1) // self.PAGE_SIZE
        page_btn = discord.ui.Button(
            label=f"{self.current_page + 1}/{total_pages}",
            style=discord.ButtonStyle.grey,
            disabled=True,
            row=1
        )
        self.add_item(page_btn)

        # Next button
        next_btn = discord.ui.Button(
            label="Next >",
            style=discord.ButtonStyle.grey,
            custom_id=f"next_page_{self.current_page}",
            row=1
        )
        next_btn.callback = self._next_page
        if end >= len(self.proposals):
            next_btn.disabled = True
        self.add_item(next_btn)

    def _make_view_callback(self, proposal_id):
        """Factory to create view callback for a proposal"""
        async def callback(interaction):
            async with session_context() as session:
                proposal = await get_proposal(session, proposal_id)
                if proposal:
                    embed = build_proposal_embed(proposal)
                    await interaction.response.send_message(
                        embed=embed,
                        view=VotingView(proposal_id=proposal_id),
                        ephemeral=True
                    )
                else:
                    await interaction.response.send_message("Proposal not found.", ephemeral=True)
        return callback

    async def _prev_page(self, interaction: discord.Interaction):
        """Go to previous page"""
        if self.current_page > 0:
            self.current_page -= 1
            self._refresh_buttons()
            await interaction.response.edit_message(embed=self._build_list_embed(), view=self)

    async def _next_page(self, interaction: discord.Interaction):
        """Go to next page"""
        total_pages = (len(self.proposals) + self.PAGE_SIZE - 1) // self.PAGE_SIZE
        if self.current_page < total_pages - 1:
            self.current_page += 1
            self._refresh_buttons()
            await interaction.response.edit_message(embed=self._build_list_embed(), view=self)

    def _build_list_embed(self) -> discord.Embed:
        """Build the proposal list embed for current page"""
        embed = discord.Embed(
            title="" Governance Proposals",
            color=0x3498db
        )

        start = self.current_page * self.PAGE_SIZE
        end = start + self.PAGE_SIZE
        page_proposals = self.proposals[start:end]

        for idx, proposal in enumerate(page_proposals, 1):
            yes = proposal.yes_votes or 0
            no = proposal.no_votes or 0
            abstain = proposal.abstain_votes or 0
            verdict = proposal.judgment_verdict or "Pending"

            field_value = f"ID: `{proposal.proposal_id}`\nVerdict: **{verdict}** | Status: **{proposal.voting_status}**\nVotes: YES {yes:.0f} | NO {no:.0f} | ABSTAIN {abstain:.0f}"
            embed.add_field(name=f"{idx}. {proposal.title}", value=field_value, inline=False)

        total_pages = (len(self.proposals) + self.PAGE_SIZE - 1) // self.PAGE_SIZE
        embed.set_footer(text=f"Page {self.current_page + 1}/{total_pages} ({len(self.proposals)} total)")

        return embed


class OutcomeRatingView(discord.ui.View):
    """View for rating proposal outcomes with star buttons"""

    def __init__(self, proposal_id: str):
        super().__init__(timeout=3600)  # 1 hour
        self.proposal_id = proposal_id
        self._build_star_buttons()

    def _build_star_buttons(self):
        """Build the 5-star rating buttons"""
        star_configs = [
            (1, "", discord.ButtonStyle.red),
            (2, "", discord.ButtonStyle.red),
            (3, "", discord.ButtonStyle.grey),
            (4, "", discord.ButtonStyle.green),
            (5, "", discord.ButtonStyle.green),
        ]

        for stars, label, style in star_configs:
            btn = discord.ui.Button(
                label=label,
                style=style,
                custom_id=f"rate_{stars}_{self.proposal_id}",
                row=0
            )
            btn.callback = self._rating_callback_factory(stars)
            self.add_item(btn)

    def _rating_callback_factory(self, stars):
        """Factory to create rating callback for a specific star count"""
        async def callback(interaction):
            await self._handle_rating(interaction, stars)
        return callback

    async def _handle_rating(self, interaction: discord.Interaction, stars: int):
        """Handle outcome rating"""
        try:
            async with session_context() as session:
                proposal = await get_proposal(session, self.proposal_id)
                if not proposal:
                    await interaction.response.send_message("Proposal not found.", ephemeral=True)
                    return

                # Update community satisfaction rating
                proposal.community_satisfaction_rating = float(stars)
                await session.commit()

                # Learn from outcome (with circuit breaker check)
                verdict = proposal.judgment_verdict or "PENDING"
                approved = proposal.approval_status == "APPROVED"
                comment = f"Community rated {stars}/5 stars"

                star_display = "" * stars
                status_msg = ""

                if cynic_circuit_breaker.is_available():
                    try:
                        result = await learn_cynic(
                            judgment_id=proposal.judgment_id,
                            verdict=verdict,
                            approved=approved,
                            satisfaction=float(stars),
                            comment=comment
                        )

                        learning_status = result.get("learning_status", "skipped")
                        status_msg = "CYNIC is learning from this outcome." if learning_status == "completed" else ""
                        cynic_circuit_breaker.record_success()

                        logger.info(f"Outcome rated: {self.proposal_id} = {stars}/5 stars, learning_status={learning_status}")

                    except Exception as cynic_err:
                        logger.error(f"CYNIC learning failed: {cynic_err}")
                        cynic_circuit_breaker.record_failure()
                        status_msg = "Rating saved, but CYNIC learning skipped."
                else:
                    logger.warning(f"CYNIC unavailable (circuit breaker open): {cynic_circuit_breaker.get_status()}")
                    status_msg = "Rating saved, but CYNIC learning deferred (system unavailable)."

                # Respond with confirmation
                await interaction.response.send_message(
                    f"Rated {star_display} ({stars}/5) " {status_msg}",
                    ephemeral=True
                )

        except Exception as e:
            logger.error(f"Error handling outcome rating: {e}", exc_info=True)
            await interaction.response.send_message(f" Error: {e}", ephemeral=True)
