# CYNIC Discord Bot

Community interface for CYNIC governance decisions. Connect your Discord server to CYNIC's collective consciousness.

## Features

- **Ask CYNIC** — Submit questions and get multi-axiom judgments
- **Governance Proposals** — Evaluate proposals with CYNIC before community voting
- **Learning Feedback** — Teach CYNIC from past decisions to improve future judgments
- **System Status** — Monitor CYNIC's consciousness and health
- **Empirical Testing** — Run test suites to measure CYNIC's learning

## Setup

### 1. Get Discord Token

1. Go to Discord Developer Portal (https://discord.com/developers/applications)
2. Create a new application
3. Go to "Bot" section and create a bot
4. Copy the token

### 2. Configure Environment

Add to `.env` or `cynic/discord/.env`:

```env
DISCORD_TOKEN=your_token_here
CYNIC_API_URL=http://localhost:8765
CYNIC_API_TIMEOUT=30
LOG_LEVEL=INFO
```

### 3. Run Bot

**Standalone:**
```bash
cd cynic/discord
python bot.py
```

**With Docker Compose:**
```bash
docker compose --profile with-discord up
```

## Commands

- `/ask_cynic` — Ask a question and get a judgment
- `/proposal` — Submit governance proposal for evaluation
- `/cynic_status` — Check CYNIC health and consciousness
- `/teach_cynic` — Provide feedback to improve learning
- `/cynic_empirical` — Run empirical tests
- `/cynic_test_results` — Get test results
- `/cynic_help` — Show all commands

## Governance Workflow

1. Community member submits proposal with `/proposal`
2. CYNIC evaluates through 11 Dogs + 5 Axioms
3. CYNIC returns verdict (HOWL/WAG/GROWL/BARK)
4. Community votes with embed reactions
5. Results fed back to CYNIC for learning

## Verdicts

- **HOWL** (🔴) — Strong recommendation
- **WAG** (🟢) — Positive recommendation  
- **GROWL** (🟡) — Proceed with caution
- **BARK** (🔶) — Reject recommendation

Q-Score: 0-100 confidence (φ-bounded at 61.8% max)

## Support

See GitHub Issues or Discord community for help.
