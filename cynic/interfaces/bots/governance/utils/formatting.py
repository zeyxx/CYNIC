"""
Discord message formatting helpers
"""

from datetime import datetime

import discord

from cynic.interfaces.bots.governance.logic.models import Community, Proposal


async def format_proposal_embed(proposal: Proposal, community: Community = None) -> dict:
    """Format proposal as Discord embed"""

    embed = {
        "title": proposal.title,
        "description": proposal.description[:500] if len(proposal.description) > 500 else proposal.description,
        "color": {
            "ACTIVE": 0x3498db,      # Blue
            "CLOSED": 0x95a5a6,      # Gray
            "APPROVED": 0x2ecc71,    # Green
            "REJECTED": 0xe74c3c     # Red
        }.get(proposal.voting_status, 0x3498db),
        "fields": [
            {
                "name": "Proposal ID",
                "value": proposal.proposal_id,
                "inline": True
            },
            {
                "name": "Category",
                "value": proposal.category or "General",
                "inline": True
            },
            {
                "name": "Impact Level",
                "value": proposal.impact_level or "Medium",
                "inline": True
            },
            {
                "name": "Status",
                "value": f"**{proposal.voting_status}** ({proposal.approval_status})",
                "inline": True
            }
        ],
        "footer": {
            "text": f"Created: {proposal.created_at.strftime('%Y-%m-%d %H:%M')} UTC"
        }
    }

    # Add CYNIC verdict if available
    if proposal.judgment_verdict:
        embed["fields"].append({
            "name": "CYNIC Verdict",
            "value": f"**{proposal.judgment_verdict}** | Q-Score: {proposal.judgment_q_score:.1f}",
            "inline": False
        })

    return embed


async def format_voting_status(proposal: Proposal, community: Community) -> str:
    """Format proposal voting status"""

    # Calculate time remaining
    now = datetime.utcnow()
    time_remaining = proposal.voting_end_time - now
    hours_remaining = max(0, int(time_remaining.total_seconds() / 3600))
    minutes_remaining = int((time_remaining.total_seconds() % 3600) / 60)

    status = f"""
- **VOTING STATUS**

**Proposal:** {proposal.title}
**ID:** {proposal.proposal_id}

**Time Remaining:** {hours_remaining}h {minutes_remaining}m
**Status:** {proposal.voting_status}

**Current Votes:**
 YES:     {proposal.yes_votes:,.0f} tokens
 NO:      {proposal.no_votes:,.0f} tokens
 ABSTAIN: {proposal.abstain_votes:,.0f} tokens

**Approval Status:** {project.approval_status if proposal.approval_status != "PENDING" else "Vote in progress..."}

' /vote {proposal.proposal_id} YES to support
' /cynic-verdict {proposal.proposal_id} for CYNIC judgment
"""
    return status.strip()


def format_cynic_verdict(judgment_data: dict) -> str:
    """Format CYNIC judgment as readable text"""

    verdict = judgment_data.get("verdict", "PENDING")
    q_score = judgment_data.get("q_score", 0.0)
    confidence = judgment_data.get("confidence", 0.0)

    # Verdict interpretation
    verdict_interpretations = {
        "HOWL": " **HOWL** " Highly Recommended",
        "WAG": "' **WAG** " Lean Toward Approval",
        "GROWL": " **GROWL** " Lean Toward Rejection",
        "BARK": " **BARK** " Not Recommended",
        "PENDING": " **PENDING** " Judgment in progress"
    }

    text = f"""
 **CYNIC JUDGMENT**

{verdict_interpretations.get(verdict, verdict)}

**Q-Score:** {q_score:.1f}/100
**Confidence:** {confidence:.1%} (-bounded)

**Reasoning:**
{judgment_data.get('reasoning', 'No details available')[:500]}
"""
    return text.strip()


def format_proposal_created(proposal_id: str, verdict: str, q_score: float) -> str:
    """Format proposal created announcement"""

    emoji = {
        "HOWL": "",
        "WAG": "'",
        "GROWL": "",
        "BARK": ""
    }.get(verdict, """)

    text = f"""
 **PROPOSAL SUBMITTED**

**ID:** {proposal_id}

 **CYNIC JUDGMENT**
{emoji} **{verdict}** | Q-Score: **{q_score:.1f}/100**

Voting Opens: in 1 minute
Voting Period: 72 hours
Approval Threshold: 50% + 1
Quorum Required: 25%

' /vote {proposal_id} YES to support
' /proposal-details {proposal_id} for full details
' /cynic-verdict {proposal_id} for CYNIC reasoning
"""
    return text.strip()


def format_voting_started(proposal_id: str, title: str) -> str:
    """Format voting started announcement"""

    text = f"""
- **VOTING STARTED**

**Proposal:** {title}
**ID:** {proposal_id}

Voting Period: 72 hours

' /vote {proposal_id} YES or NO
' /voting-status {proposal_id} to see current votes
' /cynic-verdict {proposal_id} for CYNIC guidance
"""
    return text.strip()


def format_vote_recorded(voter_id: str, proposal_id: str, vote: str) -> str:
    """Format vote recorded confirmation"""

    emoji = {"YES": "", "NO": "", "ABSTAIN": ""}.get(vote, """)

    text = f"""
{emoji} **VOTE RECORDED**

Your vote: **{vote}**
Proposal: {proposal_id}

Your vote has been recorded. You can change it anytime before voting closes.

' /voting-status {proposal_id} to see current standings
"""
    return text.strip()


def format_proposal_approved(proposal_id: str, title: str) -> str:
    """Format proposal approved announcement"""

    text = f"""
 **PROPOSAL APPROVED**

**Title:** {title}
**ID:** {proposal_id}

This proposal has been approved by the community!

Execution scheduled for 24 hours from now.

' /proposal-details {proposal_id} for details
"""
    return text.strip()


def format_proposal_rejected(proposal_id: str, title: str) -> str:
    """Format proposal rejected announcement"""

    text = f"""
 **PROPOSAL REJECTED**

**Title:** {title}
**ID:** {proposal_id}

This proposal did not receive enough support.

Community feedback:
- Consider addressing concerns raised during voting
- Submit a revised proposal if you wish to try again

' /proposal-details {proposal_id} for details and feedback
"""
    return text.strip()


def format_error(error_message: str) -> str:
    """Format error message"""

    text = f"""
 **ERROR**

{error_message}

If this error persists, please contact the governance administrator.
"""
    return text.strip()


def format_help() -> str:
    """Format help message"""

    text = """
 **CYNIC GOVERNANCE BOT " Help**

**Proposal Commands:**
 /propose <title> <description> " Submit a proposal
 /proposal-details <id> " View full proposal details
 /proposals [status] " List proposals

**Voting Commands:**
 /vote <proposal_id> <yes|no|abstain> " Cast your vote
 /voting-status <proposal_id> " See voting progress
 /my-votes " View your voting history

**CYNIC Judgment:**
 /cynic-verdict <proposal_id> " Get CYNIC's detailed judgment
 /cynic-status " Check CYNIC's health

**Community:**
 /community-info " Governance settings
 /governance-stats " Community metrics
 /leaderboard " Top proposers/voters

For more help: /help <command>
"""
    return text.strip()


def build_proposal_embed(proposal) -> discord.Embed:
    """Build a rich Discord embed for a proposal with vote counts and CYNIC verdict"""
    VERDICT_COLORS = {
        "HOWL": 0x00FF7F, "WAG": 0xFFD700,
        "GROWL": 0xFF8C00, "BARK": 0xFF4500,
    }
    STATUS_COLORS = {
        "ACTIVE": 0x3498db, "CLOSED": 0x95a5a6,
        "APPROVED": 0x2ecc71, "REJECTED": 0xe74c3c,
    }
    color = (VERDICT_COLORS.get(proposal.judgment_verdict)
             or STATUS_COLORS.get(proposal.voting_status, 0x3498db))

    embed = discord.Embed(
        title=proposal.title,
        description=proposal.description[:500] + ("..." if len(proposal.description) > 500 else ""),
        color=color
    )
    embed.add_field(name="Category", value=proposal.category or "General", inline=True)
    embed.add_field(name="Impact", value=proposal.impact_level or "MEDIUM", inline=True)
    embed.add_field(name="Status", value=proposal.voting_status, inline=True)

    if proposal.judgment_verdict:
        VERDICT_LABELS = {
            "HOWL": "HOWL " Highly Recommended",
            "WAG": "WAG " Lean Approve",
            "GROWL": "GROWL " Lean Reject",
            "BARK": "BARK " Not Recommended",
        }
        q = proposal.judgment_q_score or 0.0
        embed.add_field(
            name="CYNIC Verdict",
            value=f"{VERDICT_LABELS.get(proposal.judgment_verdict, proposal.judgment_verdict)}\nQ-Score: **{q:.1f}/100**",
            inline=False
        )
    else:
        embed.add_field(name="CYNIC Verdict", value="Pending...", inline=False)

    yes = proposal.yes_votes or 0
    no = proposal.no_votes or 0
    abstain = proposal.abstain_votes or 0
    total = yes + no + abstain
    if total > 0:
        yes_pct = (yes / total) * 100
        no_pct = (no / total) * 100
        bar = _build_vote_bar(yes_pct, no_pct)
        vote_value = f"YES {yes:.0f} ({yes_pct:.0f}%) | NO {no:.0f} ({no_pct:.0f}%) | ABSTAIN {abstain:.0f}\n{bar}"
    else:
        vote_value = "No votes yet"
    embed.add_field(name="Current Votes", value=vote_value, inline=False)

    if proposal.voting_end_time:
        remaining = proposal.voting_end_time - datetime.utcnow()
        if remaining.total_seconds() > 0:
            h = int(remaining.total_seconds() // 3600)
            m = int((remaining.total_seconds() % 3600) // 60)
            time_str = f"{h}h {m}m remaining"
        else:
            time_str = "Voting closed"
        embed.set_footer(text=f"ID: {proposal.proposal_id} | {time_str}")
    else:
        embed.set_footer(text=f"ID: {proposal.proposal_id}")

    return embed


def _build_vote_bar(yes_pct: float, no_pct: float) -> str:
    """Build a text progress bar showing YES/NO vote distribution"""
    bar_width = 20
    yes_blocks = round((yes_pct / 100) * bar_width)
    no_blocks = round((no_pct / 100) * bar_width)
    remaining = bar_width - yes_blocks - no_blocks
    return f"[{'|' * yes_blocks}{'.' * no_blocks}{' ' * remaining}]"


def build_outcome_embed(proposal) -> discord.Embed:
    """Build a rich Discord embed for a proposal outcome"""
    # Determine color based on outcome
    is_approved = proposal.approval_status == "APPROVED"
    color = 0x2ecc71 if is_approved else 0xe74c3c  # green or red

    # Title
    title_prefix = " APPROVED" if is_approved else " REJECTED"
    embed = discord.Embed(
        title=f"{title_prefix} " {proposal.title}",
        color=color
    )

    # Final vote bar
    yes = proposal.yes_votes or 0
    no = proposal.no_votes or 0
    abstain = proposal.abstain_votes or 0
    total = yes + no + abstain

    if total > 0:
        yes_pct = (yes / total) * 100
        no_pct = (no / total) * 100
        bar = _build_vote_bar(yes_pct, no_pct)
        vote_value = f"YES {yes:.0f} ({yes_pct:.0f}%) | NO {no:.0f} ({no_pct:.0f}%) | ABSTAIN {abstain:.0f}\n{bar}"
    else:
        vote_value = "No votes recorded"

    embed.add_field(name="Final Vote", value=vote_value, inline=False)

    # CYNIC verdict + correctness signal
    if proposal.judgment_verdict:
        cynic_approved = proposal.judgment_verdict in {"HOWL", "WAG"}
        matched = cynic_approved == is_approved
        correctness = " Correct" if matched else " Mismatch"

        verdict_text = f"**{proposal.judgment_verdict}** | Q-Score: **{proposal.judgment_q_score:.1f}/100**\n{correctness}"
        embed.add_field(name="CYNIC Verdict", value=verdict_text, inline=False)

    # Rate this outcome
    embed.add_field(
        name="Rate This Outcome",
        value="Click a star below to tell CYNIC how well it judged this proposal.",
        inline=False
    )

    # Footer
    embed.set_footer(text=f"ID: {proposal.proposal_id} | Outcome recorded at {datetime.utcnow().strftime('%Y-%m-%d %H:%M')} UTC")

    return embed
