# CYNIC Governance Bot Deployment Guide

## Overview
The governance bot is a Discord bot that integrates with CYNIC for memecoin community governance. It's fully containerized and ready for production deployment.

## Prerequisites

### 1. Discord Bot Setup
1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application (e.g., "CYNIC Governance")
3. Go to "Bot" section → "Add Bot"
4. Copy the token and add it to `.env`:
   ```bash
   DISCORD_TOKEN=your_token_here
   ```
5. Go to OAuth2 → URL Generator
6. Select scopes: `bot`
7. Select permissions: `Send Messages`, `Read Message History`, `Use Slash Commands`
8. Copy the generated URL and invite the bot to your server

### 2. Environment Setup
Update `.env` file with:
```bash
DISCORD_TOKEN=your_bot_token
DISCORD_PREFIX=/
LOG_LEVEL=INFO
```

## Deployment

### Option A: Deploy with Docker Compose (Recommended)

**Full Stack (CYNIC + Governance Bot):**
```bash
docker compose --profile governance up -d
```

**Check Status:**
```bash
docker compose logs governance-bot -f
```

### Option B: Deploy Governance Bot Only

**Prerequisites:** CYNIC API must be running on `http://localhost:8765`

```bash
docker compose --profile governance up -d governance-bot
```

## Verification

1. **Bot is Running:**
   ```bash
   docker ps | grep governance-bot
   ```

2. **Check Logs:**
   ```bash
   docker compose logs governance-bot --tail 50
   ```

3. **Discord Bot Status:**
   - Bot should appear online in your Discord server
   - Try `/help` command in Discord to see available commands

4. **Database Check:**
   ```bash
   docker exec cynic-governance-bot ls -la governance_bot.db
   ```

## Available Commands

Once deployed, users can interact with the bot via Discord slash commands:

- `/propose` - Submit a governance proposal
- `/proposals` - View active proposals
- `/vote` - Cast your vote on a proposal
- `/voting_status` - View voting progress
- `/cynic_verdict` - Get CYNIC's judgment on a proposal
- `/community_info` - View governance settings
- `/governance_stats` - View governance statistics

## Architecture

```
Discord Server
    ↓
Governance Bot (Discord.py)
    ↓
SQLite Database (governance_bot.db)
    ↓
CYNIC API Server (8765)
```

## Database Persistence

The governance bot database is stored in a Docker volume:
- **Volume:** `governance_data`
- **Location in container:** `/app/governance_bot.db`
- **Persists across:** container restarts and updates

## Performance Notes

- **Memory Usage:** Stable at 500-600MB (fixed with session leak resolution)
- **Health:** Continuously healthy with proper connection cleanup
- **Scalability:** Can handle 10+ communities simultaneously

## Troubleshooting

### Bot Not Responding
1. Check Discord token in `.env` is valid
2. Verify CYNIC API is running: `curl http://localhost:8765/health`
3. Check logs: `docker compose logs governance-bot`

### Database Errors
1. Delete old test database: `rm governance_bot/governance_bot_test.db`
2. Restart bot: `docker compose restart governance-bot`

### Memory Issues
- Fixed in commit `9d0ca7f` (session leak resolution)
- Monitor: `docker stats governance-bot`

## Testing

Run the governance flow tests:
```bash
cd governance_bot
python test_governance_flow.py
```

Expected output: All tests pass ✅

## Recent Changes

- **Session Leak Fix (2026-02-26):** Resolved memory leak with async context managers
  - Memory: 12.57GB → 372MB (94% improvement)
  - All database operations now properly cleanup sessions
  - Commit: `9d0ca7f`

## Next Steps

1. Add your DISCORD_TOKEN to `.env`
2. Run `docker compose --profile governance up`
3. Test bot commands in Discord
4. Monitor logs for any issues

---

For more information on CYNIC architecture, see [CYNIC README](../README.md)
