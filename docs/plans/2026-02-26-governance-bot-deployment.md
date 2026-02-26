# Governance Bot Deployment Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deploy CYNIC governance bot to test Discord/Telegram, verify all functionality works, then prepare for production deployment to real memecoin communities.

**Architecture:**
- **Phase 1 (Test):** Deploy to isolated test Discord server + test Telegram group, verify governance flow (propose → judge → vote → execute)
- **Phase 2 (Production):** Register production bot tokens, setup separate production database, deploy to real memecoin communities
- **Integration:** Governance bot ↔ CYNIC organism (for judgments) + SQLite database (for proposals/votes) + Discord/Telegram APIs

**Tech Stack:** discord.py, python-telegram-bot, SQLAlchemy, SQLite, CYNIC MCP integration

---

## Phase 1: Test Environment Setup

### Task 1: Create test Discord server and register test bot

**Files:**
- Modify: `governance_bot/.env` (add test bot token)
- Reference: `governance_bot/config.py`

**Step 1: Create test Discord server**

If you don't have a test server yet:
1. Go to Discord (discord.com)
2. Click "Create a Server"
3. Name it "CYNIC Governance Test"
4. Copy the server ID (Settings → Copy Server ID)
5. Invite yourself as admin

**Step 2: Create Discord bot application**

1. Go to Discord Developer Portal (discord.com/developers/applications)
2. Click "New Application" → name it "CYNIC Governance Bot Test"
3. Go to "Bot" section → "Add Bot"
4. Under TOKEN, click "Copy" (this is your test bot token)
5. Enable these intents: Message Content, Guilds
6. Go to OAuth2 → URL Generator
7. Select scopes: `bot`, `applications.commands`
8. Select permissions: `Send Messages`, `Embed Links`, `Read Message History`, `Use Slash Commands`
9. Copy the generated URL, open it, select your test server, authorize

**Step 3: Update .env with test token**

```env
DISCORD_TOKEN=<your_test_bot_token>
CYNIC_URL=http://127.0.0.1:8765
DATABASE_URL=sqlite:///governance_bot_test.db
LOG_LEVEL=DEBUG
```

**Step 4: Verify token format**

Run this Python command to validate the token format:
```python
import re
token = "YOUR_TOKEN_HERE"
if re.match(r'^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$', token):
    print("✓ Token format valid")
else:
    print("✗ Token format invalid")
```

**Step 5: Commit**

```bash
git add governance_bot/.env
git commit -m "test: add Discord test bot token for governance bot testing"
```

---

### Task 2: Verify Discord bot starts and connects

**Files:**
- Reference: `governance_bot/bot.py`
- Test: `test_bot.py`

**Step 1: Start CYNIC kernel**

First, ensure CYNIC is running (it needs to be for the bot to work):
```bash
cd cynic
python -m cynic.core.kernel
```
Wait for: `[CYNIC] Kernel started on 127.0.0.1:8765`

**Step 2: Start the governance bot**

In a new terminal:
```bash
cd governance_bot
python bot.py
```

**Step 3: Verify bot is online**

Expected output:
```
INFO - Bot logged in as CYNIC Governance Bot Test#1234
INFO - Synced 10 commands to guilds
```

**Step 4: Test in Discord**

1. Go to your test server
2. Type `/help`
3. Bot should respond with help text

**Step 5: Verify database created**

Check that `governance_bot_test.db` exists:
```bash
ls -la governance_bot/governance_bot_test.db
```

---

### Task 3: Test core governance flow (propose → judge → vote)

**Files:**
- Reference: `governance_bot/bot.py` (commands: propose, vote, voting_status)
- Test: Manual testing in Discord

**Step 1: Create a test proposal**

In your Discord test server:
```
/propose
  title: "Increase community treasury allocation"
  description: "Allocate 5% of monthly revenue to community projects"
  category: COMMUNITY_DECISION
  impact_level: MEDIUM
```

**Step 2: Verify proposal created and CYNIC judged it**

Expected response:
```
✅ Proposal created: prop_20260226_abc12345
🧠 CYNIC Verdict: [HOWL/WAG/GROWL/BARK]
Q-Score: 0.618
```

Check database:
```bash
sqlite3 governance_bot/governance_bot_test.db "SELECT proposal_id, title, judgment_verdict FROM proposals LIMIT 1;"
```

**Step 3: Cast votes on the proposal**

As different users (or use multiple Discord accounts in test server):
```
/vote
  proposal_id: prop_20260226_abc12345
  vote: YES
  reasoning: "I agree with this allocation"
```

Record at least 3 votes (YES, NO, ABSTAIN)

**Step 4: Check voting status**

```
/voting_status
  proposal_id: prop_20260226_abc12345
```

Expected response shows vote counts and current tally.

**Step 5: View CYNIC verdict**

```
/cynic_verdict
  proposal_id: prop_20260226_abc12345
```

Expected: Full CYNIC judgment with reasoning.

---

### Task 4: Register and setup Telegram bot

**Files:**
- Modify: `governance_bot/.env` (add TELEGRAM_TOKEN)
- Reference: `cynic/telegram/bot.py`

**Step 1: Create Telegram bot via BotFather**

1. Open Telegram and search for @BotFather
2. Send `/newbot`
3. When asked for name: "CYNIC Governance Bot Test"
4. When asked for username: "cynic_governance_test_bot" (must be unique, may need to add timestamp)
5. Copy the token (format: `123456789:ABCDefGHijKLmnoPQRstUvWxyz`)

**Step 2: Update .env with Telegram token**

```env
DISCORD_TOKEN=<your_discord_test_token>
TELEGRAM_TOKEN=<your_telegram_bot_token>
CYNIC_URL=http://127.0.0.1:8765
DATABASE_URL=sqlite:///governance_bot_test.db
LOG_LEVEL=DEBUG
```

**Step 3: Create test Telegram group**

1. In Telegram, create a new group "CYNIC Governance Test"
2. Add your bot to the group
3. Copy the group ID (send a message and check logs, or use @userinfobot)

**Step 4: Verify Telegram bot token format**

Token should be: `123456789:ABCDefGHijKLmnoPQRstUvWxyz`
(Numbers:Letters pattern)

**Step 5: Commit**

```bash
git add governance_bot/.env
git commit -m "test: add Telegram test bot token for governance bot testing"
```

---

### Task 5: Test Telegram bot basic connectivity

**Files:**
- Reference: `cynic/telegram/bot.py`

**Step 1: Start Telegram bot (alongside Discord bot)**

In governance_bot directory, check if there's a unified bot launcher or start separately:

If unified:
```bash
cd governance_bot
python bot.py --platforms discord,telegram
```

If separate:
```bash
cd cynic/telegram
python bot.py
```

Expected output:
```
INFO - Telegram bot started
INFO - Polling for updates
```

**Step 2: Test Telegram bot in test group**

In your Telegram test group:
```
/help
```

Bot should respond with help message.

**Step 3: Create a test proposal in Telegram**

```
/propose "Telegram Test Proposal" "Testing governance bot on Telegram"
```

Expected: Proposal created and stored in database.

**Step 4: Verify database has Telegram proposal**

```bash
sqlite3 governance_bot/governance_bot_test.db \
  "SELECT proposal_id, title FROM proposals WHERE proposal_id LIKE '%telegram%' LIMIT 1;"
```

---

### Task 6: Run integration tests

**Files:**
- Reference: `test_bot.py`
- Test: Write comprehensive integration tests

**Step 1: Check existing tests**

```bash
python -m pytest test_bot.py -v
```

Expected: Should show all existing tests. If any fail, investigate and fix.

**Step 2: Add tests for core flow**

Update `test_bot.py` to add tests for:
- Creating a proposal
- Casting votes
- Checking voting status
- Getting CYNIC verdict

```python
import pytest
import asyncio
from governance_bot.bot import bot
from governance_bot.database import get_session, create_proposal

@pytest.mark.asyncio
async def test_proposal_creation():
    """Test creating a proposal"""
    session = await get_session()
    proposal = await create_proposal(session, {
        "proposal_id": "test_prop_001",
        "community_id": "test_community",
        "proposer_id": "test_user",
        "title": "Test Proposal",
        "description": "This is a test",
        "category": "COMMUNITY_DECISION",
        "impact_level": "MEDIUM",
        "voting_status": "PENDING"
    })
    assert proposal.proposal_id == "test_prop_001"
    assert proposal.title == "Test Proposal"

@pytest.mark.asyncio
async def test_voting_flow():
    """Test voting on a proposal"""
    # Create proposal, cast vote, verify
    pass
```

**Step 3: Run all tests**

```bash
python -m pytest test_bot.py -v --tb=short
```

Expected: All tests pass with coverage > 80%.

**Step 4: Commit**

```bash
git add test_bot.py
git commit -m "test: add integration tests for governance bot core flow"
```

---

## Phase 2: Production Environment Setup

### Task 7: Prepare production environment

**Files:**
- Create: `governance_bot/.env.production`
- Modify: `governance_bot/config.py` (add environment detection)

**Step 1: Create production config**

Create `governance_bot/.env.production`:
```env
# Production Discord Bot
DISCORD_TOKEN=<will_be_filled_from_production_app>

# Production Telegram Bot
TELEGRAM_TOKEN=<will_be_filled_from_production_app>

# CYNIC Integration (production instance)
CYNIC_URL=http://127.0.0.1:8765

# Production Database
DATABASE_URL=postgresql://user:pass@prod-db:5432/cynic_governance
# Or for SQLite: DATABASE_URL=sqlite:///governance_bot_production.db

# Production Settings
LOG_LEVEL=WARNING
ENVIRONMENT=production
```

**Step 2: Update config to support environments**

Modify `governance_bot/config.py`:

```python
import os
from dotenv import load_dotenv

# Load environment-specific .env
environment = os.getenv("ENVIRONMENT", "development")
if environment == "production":
    load_dotenv(".env.production")
else:
    load_dotenv(".env")  # test/dev

# Rest of config stays the same
```

**Step 3: Create production bot registration guide**

Create `docs/DEPLOYMENT.md`:

```markdown
# Production Deployment Guide

## Creating Production Bots

### Discord Production Bot

1. Discord Developer Portal (discord.com/developers/applications)
2. Create new application: "CYNIC Governance Bot Production"
3. Add Bot, copy token
4. Enable intents: Message Content, Guilds
5. OAuth2 → URL Generator → scopes: bot, applications.commands
6. Permissions: Send Messages, Embed Links, Use Slash Commands
7. Copy authorization URL
8. Invite to production Discord server
9. Add token to `.env.production`

### Telegram Production Bot

1. Message @BotFather
2. Send `/newbot`
3. Name: "CYNIC Governance Bot"
4. Username: "cynic_governance_bot"
5. Copy token to `.env.production`
6. Add bot to production Telegram channel

## Database Migration

If using PostgreSQL for production:

```bash
export DATABASE_URL=postgresql://user:pass@prod-db:5432/cynic_governance
alembic upgrade head
```

## Deployment

```bash
export ENVIRONMENT=production
python governance_bot/bot.py
```
```

**Step 4: Commit**

```bash
git add governance_bot/.env.production docs/DEPLOYMENT.md
git commit -m "docs: add production environment configuration and deployment guide"
```

---

### Task 8: Register production Discord and Telegram bots

**Files:**
- Modify: `governance_bot/.env.production` (add real tokens)

**Step 1: Create production Discord bot**

Follow the Discord Developer Portal steps in DEPLOYMENT.md, get the production token.

**Step 2: Create production Telegram bot**

Contact @BotFather, create production bot, get token.

**Step 3: Update .env.production**

```env
DISCORD_TOKEN=<production_discord_token>
TELEGRAM_TOKEN=<production_telegram_token>
ENVIRONMENT=production
```

**Step 4: Verify tokens**

Don't commit yet—verify both tokens work:

```bash
# Quick Discord token validation
python -c "from discord import Client; print('Discord token format valid')"

# Quick Telegram token validation
python -c "import requests; requests.get(f'https://api.telegram.org/bot{TOKEN}/getMe'); print('Telegram token valid')"
```

**Step 5: Commit (secrets protected)**

```bash
git add governance_bot/.env.production
git commit -m "prod: add production bot tokens (secrets protected in .env.production)"
```

---

### Task 9: Prepare first memecoin community deployment

**Files:**
- Create: `docs/MEMECOIN_INTEGRATION.md`
- Reference: `governance_bot/bot.py` (community setup code)

**Step 1: Document integration steps**

Create `docs/MEMECOIN_INTEGRATION.md`:

```markdown
# Deploying CYNIC Governance to a Memecoin Community

## Prerequisites

- Memecoin Discord server (community must own/control)
- Memecoin community token address
- Community treasury wallet address
- CYNIC governance bot invited to server

## Setup Steps

### 1. Invite Bot to Server

Send authorization link to community manager:
```
https://discord.com/api/oauth2/authorize?client_id=YOUR_APP_ID&permissions=274877906944&scope=bot%20applications.commands
```

### 2. Initialize Community in Bot

In your Discord server:
```
/community_info
```

This creates the community record and initializes default governance settings.

### 3. Configure Governance Settings

(Future command)
```
/configure_governance
  voting_period_hours: 72
  quorum_percentage: 25
  approval_threshold_percentage: 50
  community_token: "TOKEN_ADDRESS"
  treasury_wallet: "WALLET_ADDRESS"
```

### 4. Enable Integrations

- **GASdf**: (set gasdf_enabled: true)
- **NEAR**: (set near_contract_address once smart contract deployed)
- **CYNIC**: Already enabled by default

### 5. Announce Governance Launch

Post in community:
```
🎉 **CYNIC Governance is LIVE**

Your community can now:
✅ Submit governance proposals
✅ Vote with democratic influence
✅ Get AI-powered judgment from CYNIC (11 Dogs consensus)
✅ Execute decisions on-chain

/propose - Create a new proposal
/vote - Vote on active proposals
/voting_status - Check voting progress
```

## First Proposal

Help the community create their first proposal:

1. Moderator: "Let's propose increasing community rewards"
2. Community votes
3. CYNIC judges the proposal
4. Results execute on NEAR (once smart contracts deployed)

## Success Metrics

- ✅ Proposals created within first week
- ✅ Community participation (>25% quorum)
- ✅ CYNIC judgments useful to community
- ✅ Fees burned via GASdf
```

**Step 2: Create memecoin onboarding checklist**

Create `docs/MEMECOIN_CHECKLIST.md`:

```markdown
# Memecoin Community Onboarding Checklist

## Phase 1: Preparation (Before reaching out)
- [ ] Production Discord bot is registered and stable
- [ ] Production Telegram bot is registered
- [ ] Database is prepared (SQLite or PostgreSQL)
- [ ] CYNIC kernel is running stable
- [ ] Governance settings are documented

## Phase 2: Community Contact
- [ ] Identify memecoin community leader(s)
- [ ] Explain governance solution (GASdf + NEAR + CYNIC)
- [ ] Get Discord/Telegram server invite links
- [ ] Confirm community token address
- [ ] Confirm treasury wallet address

## Phase 3: Deployment
- [ ] Invite bot to Discord server
- [ ] Invite bot to Telegram group
- [ ] Initialize community via `/community_info`
- [ ] Test with a sample proposal
- [ ] Announce governance launch to community

## Phase 4: Support
- [ ] Monitor first 24-48 hours
- [ ] Answer community questions
- [ ] Ensure voting periods close correctly
- [ ] Verify CYNIC judgments are useful
- [ ] Log learnings for next community

## Post-Launch
- [ ] Community creates first real proposal
- [ ] Community votes participation >25%
- [ ] CYNIC verdict is trusted by community
- [ ] Learning loop improves community governance
```

**Step 3: Commit**

```bash
git add docs/MEMECOIN_INTEGRATION.md docs/MEMECOIN_CHECKLIST.md
git commit -m "docs: add memecoin community integration and onboarding guide"
```

---

### Task 10: Deploy to first test memecoin community

**Files:**
- Reference: `docs/MEMECOIN_INTEGRATION.md`
- Testing: Manual testing in real community

**Step 1: Contact test memecoin community**

Select a friendly memecoin community (could be one you know, or create a test community):
1. Send governance solution overview
2. Get Discord/Telegram invitation
3. Confirm they want to test CYNIC governance

**Step 2: Deploy bot to community server**

1. Invite production bot to their Discord
2. Invite production bot to their Telegram
3. Verify bot is online in their channels

**Step 3: Initialize community**

In their Discord, run:
```
/community_info
```

This registers their community and initializes voting settings.

**Step 4: Create first proposal**

Help them create their first real governance proposal:
```
/propose
  title: "Increase marketing budget"
  description: "Allocate 10% of treasury to marketing campaign"
  category: TREASURY_ALLOCATION
  impact_level: HIGH
```

**Step 5: Community votes**

Wait for community members to vote. Monitor:
- Number of votes cast
- Vote distribution (YES/NO/ABSTAIN)
- CYNIC judgment quality

**Step 6: Collect feedback**

1. Ask community: "Was CYNIC's judgment helpful?"
2. Record which Dogs agreed/disagreed
3. Note any issues or bugs
4. Update E-Score based on community feedback

**Step 7: Document learnings**

Create `docs/DEPLOYMENT_LEARNINGS.md`:

```markdown
# First Deployment Learnings

## Community: [Name]
## Date: 2026-02-26

### What Worked
- [ ] Bot connection stable
- [ ] Proposal creation smooth
- [ ] CYNIC judgments made sense
- [ ] Voting period timing correct

### What Needs Fixing
- [ ] (List issues)

### Community Feedback
- [ ] (What did they say about CYNIC?)

### Next Steps
- [ ] (What to improve before next community)
```

**Step 8: Commit**

```bash
git add docs/DEPLOYMENT_LEARNINGS.md
git commit -m "docs: record learnings from first memecoin community deployment"
```

---

## Success Criteria

✅ **Phase 1 (Test)**
- Discord bot creates proposals, votes work, CYNIC judges them
- Telegram bot functional in test group
- All integration tests passing
- Database stores all governance data correctly

✅ **Phase 2 (Production)**
- Production bots registered and running
- Environment configuration working
- First memecoin community deployed successfully
- Community can propose, vote, and see CYNIC judgments
- No crashes or memory leaks in production

---

## Execution Path

**This plan should be executed with:**
- `superpowers:executing-plans` (separate session with checkpoints)
- OR `superpowers:subagent-driven-development` (parallel agents, stay in session)

Choose your preferred approach below.
