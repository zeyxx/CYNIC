# CYNIC Discord Bot

**Ask collective consciousness in Discord.**

A Discord bot that connects to the CYNIC HTTP API, allowing your community to ask CYNIC questions, provide feedback, and see the system learn in real-time.

## Features

- **`/ask_cynic`** — Ask CYNIC a question, get judgment with Q-Score + verdict
- **`/teach_cynic`** — Provide feedback to improve CYNIC's Q-Table
- **`/cynic_status`** — Check CYNIC system health and metrics
- **`/cynic_empirical`** — Run autonomous empirical tests (100-10,000 judgments)
- **`/cynic_test_results`** — Get results from empirical tests

## Quick Start

### 1. Prerequisites

- Python 3.10+
- CYNIC API running (see main README)
- Discord server with bot permissions

### 2. Get Discord Token

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application"
3. Go to "Bot" tab, click "Add Bot"
4. Copy the token
5. Go to OAuth2 → URL Generator
6. Select scopes: `bot`
7. Select permissions:
   - Send Messages
   - Embed Links
   - Read Message History
   - Use Slash Commands
8. Copy the generated URL and invite bot to your server

### 3. Setup Bot

```bash
# Navigate to discord directory
cd cynic/discord

# Install dependencies
pip install -r requirements.txt

# Create .env file
cp .env.example .env

# Edit .env with your token and CYNIC API URL
nano .env
```

### 4. Run Bot

```bash
# Terminal 1: Start CYNIC API
cd ../..
docker-compose up cynic

# Terminal 2: Start Discord bot
cd cynic/discord
python bot.py
```

You should see:
```
Logged in as YourBotName#1234
Commands synced with Discord
✓ CYNIC health check passed
```

### 5. Try It in Discord

In any channel where the bot has permission:

```
/ask_cynic question:"Should we ship this feature?" context:"High demand from users" reality:"GENERAL"
```

## Commands Reference

### `/ask_cynic`

Ask CYNIC a question and get a judgment.

**Parameters:**
- `question` (required): The question to ask
- `context` (optional): Background information
- `reality` (optional): Type of reality — `GENERAL`, `CODE`, `MARKET`, `SOCIAL` (default: `GENERAL`)

**Example:**
```
/ask_cynic question:"Is this code maintainable?" context:"Complex recursive function" reality:"CODE"
```

**Response:**
```
🐕 CYNIC Judgment
Question: Is this code maintainable?
Q-Score: 52.3/100
Verdict: GROWL (caution)
Confidence: 54% (φ-bounded)
Explanation: Code is functional but lacks documentation...
Judgment ID: j-2026-02-24-abc123
```

---

### `/teach_cynic`

Provide feedback on a judgment to improve CYNIC's learning.

**Parameters:**
- `judgment_id` (required): ID from judgment response (e.g., `j-2026-02-24-abc123`)
- `rating` (required): Quality score 0.0-1.0 (1.0 = excellent)
- `comment` (optional): Why you rated it this way

**Example:**
```
/teach_cynic judgment_id:"j-2026-02-24-abc123" rating:0.95 comment:"Great analysis! Exactly what I thought."
```

**Response:**
```
✅ CYNIC Learned
Your Rating: 95%
New Q-Score: 54.1/100
Learning Rate: 0.0009 (Thompson-tuned)
Q-Table Size: 2048 states learned
Your Comment: Great analysis! Exactly what I thought.
```

---

### `/cynic_status`

Check CYNIC system health and metrics.

**Example:**
```
/cynic_status
```

**Response:**
```
🐕 CYNIC System Status
Status: 🟢 Online
Consciousness Level: MACRO
Q-Table Entries: 2048 states
Total Judgments: 12500
Dog Consensus: 87%
Uptime: 8h 34m
Active Validators: 11/11 Dogs online
```

---

### `/cynic_empirical`

Run autonomous empirical test with CYNIC.

**Parameters:**
- `count` (optional): Number of judgments (1-10,000, default 100)

**Example:**
```
/cynic_empirical count:500
```

**Response:**
```
🧪 Empirical Test Started
Test ID: test-2026-02-24-xyz789
Judgments: 500 (running autonomously)
Status: ⏳ In Progress
Run /cynic_test_results test-2026-02-24-xyz789 in 2-5 minutes to get results
```

---

### `/cynic_test_results`

Get results from an empirical test.

**Parameters:**
- `job_id` (required): Test ID from empirical command

**Example:**
```
/cynic_test_results job_id:"test-2026-02-24-xyz789"
```

**Response:**
```
📊 Empirical Test Results
Judgments Completed: 500
Average Q-Score: 54.3/100
Learning Improvement: 3.2x baseline
Q-Score Range: 45.1 → 61.4
Emergence Events: 5 patterns detected
Progression: 45.2 → 61.4
```

---

## Architecture

### Data Flow

```
Discord Server
    ↓ (slash command)
CYNIC Discord Bot
    ↓ (HTTP request)
CYNIC REST API (:8765)
    ↓
Consciousness Engine
    ├─ 11 Dogs vote (PBFT consensus)
    ├─ Q-Learning updates (Thompson sampling)
    └─ ResidualDetector finds patterns
    ↓
Response back to Discord
    └─ Embed message with judgment
```

### Bot Components

**`bot.py`**
- `CYNICClient`: Main Discord client with CYNIC integration
- `check_cynic_health()`: Periodic health checks (every 5 minutes)
- Command handlers for all slash commands
- Error handling and logging

**Configuration**
- `DISCORD_TOKEN`: From .env
- `CYNIC_API_URL`: URL to CYNIC API (default: `http://localhost:8765`)
- `CYNIC_API_TIMEOUT`: Request timeout in seconds (default: 30)

---

## Usage Examples

### Example 1: DAO Governance Vote

In a governance channel:

```
/ask_cynic question:"Should we increase treasury allocation to development?"
           context:"Proposal: 30% of treasury to dev team for Q2"
           reality:"GENERAL"
```

CYNIC analyzes from 11 perspectives (safety, feasibility, strategy, etc.) and provides judgment.

The DAO can use this to inform voting decisions.

---

### Example 2: Code Review

In a dev channel:

```
/ask_cynic question:"Is this PR ready to merge?"
           context:"Adds new authentication layer with unit tests"
           reality:"CODE"
```

CYNIC judges: Q-Score 78, Verdict WAG (worth acting on).

```
/teach_cynic judgment_id:"j-2026-02-24-code-pr1"
             rating:1.0
             comment:"Merged successfully, no issues in production!"
```

CYNIC learns: "PRs with unit tests + auth checks are high quality"

---

### Example 3: Community Decision

In general channel:

```
/ask_cynic question:"Should we host a hackathon next month?"
           context:"Community interest poll showed 73% support"
           reality:"GENERAL"
```

Community votes on the judgment. If approved:

```
/teach_cynic judgment_id:"j-2026-02-24-hack1"
             rating:0.9
             comment:"Hackathon was huge success! 200+ participants"
```

CYNIC learns: "Community-driven initiatives with >70% support succeed"

---

## Troubleshooting

### Bot not responding

1. **Check bot is online**:
   ```bash
   # In Discord
   /cynic_status
   ```

2. **Check CYNIC API**:
   ```bash
   # Terminal
   curl http://localhost:8765/health
   ```
   Should return `{"status": "ok"}`

3. **Check logs**:
   ```bash
   # Look for errors in bot.py terminal output
   tail -f /tmp/cynic_discord.log
   ```

### Timeout errors

- Increase `CYNIC_API_TIMEOUT` in .env (default 30s)
- CYNIC API might be overloaded, try again

### "CYNIC is not available"

- CYNIC API is down
- Start CYNIC: `docker-compose up cynic`
- Bot checks health every 5 minutes

### Judgment ID not found

- Test might still be running
- Wait 2-5 minutes and try again
- Or run smaller empirical test (100 instead of 1000)

---

## Advanced Configuration

### Change CYNIC API URL

Edit `.env`:
```
CYNIC_API_URL=http://cynic-api.example.com:8765
```

### Change Timeout

Edit `.env`:
```
CYNIC_API_TIMEOUT=60
```

For slow networks or large tests, increase timeout.

### Logging

Control log level in `.env`:
```
LOG_LEVEL=DEBUG  # More verbose
LOG_LEVEL=WARNING  # Less verbose
```

---

## Deployment

### Docker

```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY cynic/discord/requirements.txt .
RUN pip install -r requirements.txt

COPY cynic/discord/bot.py .
COPY cynic/discord/.env .

CMD ["python", "bot.py"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cynic-discord-bot
spec:
  replicas: 1
  selector:
    matchLabels:
      app: cynic-discord-bot
  template:
    metadata:
      labels:
        app: cynic-discord-bot
    spec:
      containers:
      - name: bot
        image: cynic-discord-bot:latest
        env:
        - name: DISCORD_TOKEN
          valueFrom:
            secretKeyRef:
              name: discord-secrets
              key: token
        - name: CYNIC_API_URL
          value: http://cynic-api:8765
```

---

## Monitoring

### Health Checks

Bot performs automatic health checks every 5 minutes:
```
✓ CYNIC health check passed
```

### Metrics

Track in your monitoring system:
- Bot uptime
- Command latency (CYNIC API response time)
- Error rate
- Active judgments

---

## Contributing

Found a bug? Want to add a feature?

1. Report issue on GitHub
2. Fork and create feature branch
3. Submit PR

---

## License

MIT

---

## FAQ

**Q: Can I run the bot on multiple servers?**
A: Yes. One bot can be in unlimited Discord servers.

**Q: Can I customize the embed colors?**
A: Yes. Edit the `verdict_colors` dict in `bot.py`:
```python
verdict_colors = {
    "HOWL": 0xFF0000,      # Change these hex colors
    "WAG": 0x00FF00,
    ...
}
```

**Q: How many commands can the bot handle?**
A: Limited by Discord rate limits (~50 commands per minute). CYNIC API throughput may be the limiting factor.

**Q: Can I use slash commands in threads?**
A: Yes, bot works in threads, DMs, and channels.

**Q: What if CYNIC makes a wrong judgment?**
A: Use `/teach_cynic` to provide feedback. CYNIC learns and improves next time.

---

**Status**: Ready to deploy
**Last Updated**: February 24, 2026
**Version**: 1.0.0

*The dog awaits. What shall we discover?* 🐕
