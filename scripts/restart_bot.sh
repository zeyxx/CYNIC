#!/bin/bash
# Restart bot with command signature fix
# This ensures all Discord commands are properly synchronized

set -e

cd "$(dirname "$0")" || exit 1

echo "╔═════════════════════════════════════════════════════╗"
echo "║  CYNIC Governance Bot - Restart with Sig Fix        ║"
echo "╚═════════════════════════════════════════════════════╝"
echo ""

# Kill any existing bot processes
echo "→ Stopping any existing bot processes..."
pkill -f "python.*bot.py" || true
pkill -f "python.*governance_bot" || true
sleep 2

# Clean up old pid files
echo "→ Cleaning up old process files..."
rm -f governance_bot/bot.pid
rm -f governance_bot/bot_stability.pid
rm -f bot.pid
rm -f bot_stability.pid

# Start fresh bot instance
echo "→ Starting bot with command signature fix..."
echo "→ Bot will automatically sync all commands on startup"
echo ""

cd governance_bot
python bot.py &
BOT_PID=$!
echo $BOT_PID > bot.pid

echo "✓ Bot started with PID $BOT_PID"
echo "→ Commands syncing... (check logs in 10 seconds)"
echo ""
echo "📋 Log files:"
echo "  - governance_bot/startup.log"
echo "  - governance_bot/bot.log"
echo ""
echo "✓ Setup complete!"
echo "  The bot will now:"
echo "  1. Sync all 10 commands with Discord"
echo "  2. Fix signature mismatches"
echo "  3. Handle errors gracefully"
