# CYNIC Governance Bot

A Discord bot for memecoin community governance powered by CYNIC's 11-Dog consensus system.

## Features

✅ **Proposal Management**
- Submit governance proposals
- View proposal details with CYNIC judgments
- List proposals by status

✅ **Community Voting**
- Cast votes (YES/NO/ABSTAIN)
- Real-time voting status
- Vote weight based on token holdings (configurable)

✅ **CYNIC Integration**
- 11 Dogs evaluate every proposal
- Q-Score judgment (0-100)
- Verdict: HOWL/WAG/GROWL/BARK
- Detailed reasoning from each Dog

✅ **Learning Loop**
- Track proposal outcomes
- CYNIC learns from results
- E-Score reputation tracking
- Cross-community learning

✅ **Treasury & Execution**
- GASdf fee burning (100% non-extractive)
- NEAR smart contract execution
- Automatic proposal execution on approval

## Setup

### Prerequisites

- Python 3.10+
- Discord bot token (create at https://discord.com/developers/applications)
- CYNIC kernel running (http://127.0.0.1:8765)

### Installation

1. **Install dependencies**
```bash
cd governance_bot
pip install -r requirements.txt
```

2. **Configure environment**
```bash
cp .env.example .env
# Edit .env with your Discord token
```

3. **Create Discord bot**
- Go to https://discord.com/developers/applications
- Click "New Application"
- Create a bot user
- Copy the token to .env file
- Enable "Message Content Intent" in privileged gateway intents

4. **Invite bot to server**
- Go to OAuth2 → URL Generator
- Select scopes: `bot`, `applications.commands`
- Select permissions: `Send Messages`, `Embed Links`, `Read Message History`
- Copy the generated URL and open it to invite bot

5. **Start the bot**
```bash
python bot.py
```

## Commands

### Proposal Commands

```
/propose <title> <description> [category] [impact_level]
  Submit a new governance proposal

/proposal-details <proposal_id>
  View full proposal with CYNIC judgment

/proposals [status]
  List proposals (ACTIVE, CLOSED, APPROVED, REJECTED)
```

### Voting Commands

```
/vote <proposal_id> <yes|no|abstain> [reasoning]
  Cast your vote on a proposal

/voting-status <proposal_id>
  See current voting progress and time remaining

/my-votes
  View your voting history
```

### CYNIC Judgment Commands

```
/cynic-verdict <proposal_id>
  Get CYNIC's detailed judgment with all Dogs' votes

/cynic-status
  Check CYNIC organism status and health
```

### Community Commands

```
/community-info
  View governance settings (voting period, quorum, etc.)

/governance-stats
  View community statistics (proposals, approval rate, etc.)

/leaderboard [type]
  View reputation leaderboard (PROPOSERS, VOTERS, COMMUNITIES)

/help
  Get help with all commands
```

## Architecture

### Database Schema

```
Communities
  ├─ Proposals
  │  └─ Votes
  │  └─ CYNIC Judgments
  │  └─ Learning Outcomes
  ├─ E-Scores (Reputation)
  └─ Community Users
```

### Integration Points

1. **CYNIC MCP** → ask_cynic, learn_cynic tools
2. **GASdf** → Fee burning to community treasury
3. **NEAR** → Smart contract execution (optional)
4. **Learning Loop** → Outcome tracking and CYNIC improvement

### Bot Flow

```
User submits proposal
         ↓
Bot creates proposal in DB
         ↓
Bot calls ask_cynic → Get CYNIC judgment
         ↓
Bot posts to Discord with verdict
         ↓
Community votes for 72 hours
         ↓
If approved → Execute on NEAR
         ↓
After 30 days → Track outcome
         ↓
Bot calls learn_cynic → CYNIC learns
         ↓
E-Score updated (CYNIC reputation)
```

## Configuration

Edit `config.py` to customize:

- `DISCORD_PREFIX` - Command prefix
- `CYNIC_URL` - CYNIC kernel URL
- `DEFAULT_VOTING_PERIOD_HOURS` - How long voting lasts
- `DEFAULT_QUORUM_PERCENTAGE` - Minimum participation
- `DEFAULT_APPROVAL_THRESHOLD_PERCENTAGE` - Approval requirement
- `DEFAULT_PROPOSAL_SUBMISSION_FEE` - Fee to submit proposal
- `ENABLE_GASDF` - Enable fee burning
- `ENABLE_NEAR_EXECUTION` - Enable on-chain execution

## Development

### Project Structure

```
governance_bot/
├── bot.py                 # Main Discord bot
├── config.py              # Configuration
├── database.py            # Database operations
├── models.py              # SQLAlchemy models
├── cynic_integration.py   # CYNIC MCP integration
├── formatting.py          # Discord embed formatting
├── requirements.txt       # Python dependencies
├── .env.example          # Environment template
└── governance_bot.db     # SQLite database (created on first run)
```

### Adding New Commands

1. Create command function with `@bot.tree.command()` decorator
2. Use `get_session()` to get database session
3. Format response using functions from `formatting.py`
4. Call CYNIC via `cynic_integration.py` functions

Example:
```python
@bot.tree.command(name="my_command")
async def cmd_my_command(interaction: discord.Interaction, arg: str):
    await interaction.response.defer(thinking=True)

    try:
        session = await get_session()
        # Your logic here
        await interaction.followup.send("Response")
    except Exception as e:
        await interaction.followup.send(format_error(str(e)))
```

## Features Coming Soon

- ⏳ Proposal amendments during voting
- ⏳ Telegram bot variant
- ⏳ Multi-signature execution
- ⏳ Budget allocation tracking
- ⏳ Treasury management dashboard
- ⏳ Automated outcome detection
- ⏳ Webhook integration for live updates

## Troubleshooting

### Bot not responding
- Check Discord token in .env
- Verify bot has message permissions
- Check logs for errors

### CYNIC not responding
- Verify CYNIC_URL in config.py
- Check CYNIC kernel is running: `curl http://127.0.0.1:8765/health`
- Check bot logs for CYNIC errors

### Database errors
- Delete `governance_bot.db` and restart bot to reinitialize
- Check SQLite permissions

### No judgments appearing
- Ensure `CYNIC_MCP_ENABLED=True` in config
- Check CYNIC is responding: `/cynic-status` command
- Check bot logs for errors calling ask_cynic

## Performance

- 100+ proposals can be managed simultaneously
- Voting updates in real-time
- CYNIC judgments cached (5-10 seconds)
- Database queries optimized for Discord response times (<3s)

## Security

⚠️ **In Production:**
- Use environment variables for sensitive data
- Enable Discord permission restrictions
- Use NEAR contract audited by security firm
- Rate limit commands to prevent spam
- Validate all user inputs
- Use HTTPS for GASdf integration

## Contributing

This bot is part of the CYNIC governance project. For contributions:

1. Create a feature branch
2. Follow existing code style
3. Add tests for new features
4. Submit pull request with description

## License

MIT License - See LICENSE file

## Support

- 📖 Documentation: See GOVERNANCE_BOT_SCHEMA.md
- 💬 Discord: Join community server for support
- 🐛 Issues: Report bugs on GitHub

## Roadmap

**Week 1:** Core bot deployment (✅ done)
**Week 2:** Voting & NEAR execution
**Week 3:** Learning loop & E-Score
**Week 4:** Multi-community support & analytics

---

**Status:** Phase 1 Implementation Complete
**Version:** 0.1.0 (Beta)
**Last Updated:** 2026-02-25
