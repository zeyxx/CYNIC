#!/bin/bash
# CYNIC May 1-10 Launch Setup
# Prepares three-voice coherent frequency activation
# Run: bash scripts/may-1-launch.sh

set -e

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "=== CYNIC MAY 1 LAUNCH SETUP ==="
echo ""

# 1. Generate verdicts for posting
echo "[1] Generating verdicts_to_post.json from curator..."
mkdir -p ~/.cynic/organs/hermes/x
python3 scripts/hermes-x/x_verdict_curator.py --output ~/.cynic/organs/hermes/x/verdicts_to_post.json
VERDICT_COUNT=$(jq length ~/.cynic/organs/hermes/x/verdicts_to_post.json 2>/dev/null || echo "0")
echo "✓ Generated $VERDICT_COUNT verdicts ready for posting"
echo ""

# 2. Create observation log
echo "[2] Creating observation framework..."
mkdir -p .cynic/may-1-10-experiment
cat > .cynic/may-1-10-experiment/OBSERVATION_LOG.md << 'EOF'
# CYNIC May 1-10 Observation Log

**Hypothesis:** Coherent frequency across three voices (personal T., @CynicOracle, Telegram) establishes signal in noise.

**Voices:**
1. Personal T. (human, dataset tweets)
2. @CynicOracle (organism prophecy, verdicts)
3. Telegram bot (interactive witness, /judge queries)

**Daily log (update evening):**

## May 1
- [ ] @CynicOracle philosophy thread posted (time: ___)
- [ ] T. personal tweet posted (time: ___, engagement: ___)
- [ ] Telegram bot deployed (time: ___)
- Notes: ___

## May 2-10
(repeating structure)

**Weekly analysis:**

### Week 1 (May 1-7)
- Which voice got most engagement? (Twitter/Telegram/personal)
- What did people ask CYNIC to judge?
- Which verdicts resonated most?
- Coherence signal: voices reinforced each other? (Y/N)

### Week 2 (May 8-10)
- Truth check: Do Week 1 verdicts hold in Week 2 data?
- Final analysis: What did coherent frequency reveal?

EOF
echo "✓ Observation framework created at .cynic/may-1-10-experiment/"
echo ""

# 3. Verify Twitter/Telegram access
echo "[3] Checking external accounts..."
echo "  @CynicOracle Twitter: Ready? (check manually: https://twitter.com)"
echo "  Telegram: Ready? (check manually: https://telegram.org)"
echo ""

# 4. Create Telegram bot skeleton
echo "[4] Preparing Telegram bot skeleton..."
mkdir -p cynic-python/agents/telegram_bot
cat > cynic-python/agents/telegram_bot/bot.py << 'EOF'
#!/usr/bin/env python3
"""CYNIC Telegram Bot — Interactive Witness Voice

Responds to /judge queries with verdicts from live kernel.
Posts verdicts from curator on schedule.
Tracks what community wants CYNIC to judge.

Usage:
  export TELEGRAM_BOT_TOKEN="..."
  python3 bot.py
"""

__version__ = "0.1.0"

import asyncio
import json
import logging
from pathlib import Path
from datetime import datetime

# Telegram library
try:
    from telegram import Update
    from telegram.ext import Application, CommandHandler, ContextTypes, MessageHandler, filters
except ImportError:
    print("ERROR: python-telegram-bot not installed. Run: pip install python-telegram-bot")
    exit(1)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("cynic-telegram-bot")

KERNEL_URL = "http://localhost:3030"  # Set from env: CYNIC_REST_ADDR
BOT_TOKEN = None  # Set from env: TELEGRAM_BOT_TOKEN

async def start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handler for /start command"""
    await update.message.reply_text(
        "🐕 CYNIC — Decentralized epistemic consensus\n\n"
        "I judge content across 6 axioms with φ⁻¹ (61.8%) confidence cap.\n\n"
        "Try: `/judge chess e4` or `/judge token bonk`\n"
        "Questions? Check: https://github.com/cynic"
    )

async def judge(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handler for /judge command"""
    if not context.args or len(context.args) < 2:
        await update.message.reply_text("Usage: `/judge <domain> <content>`\nExample: `/judge chess e4`")
        return

    domain = context.args[0]
    content = " ".join(context.args[1:])

    # TODO: Call CYNIC kernel /judge endpoint
    # For now, placeholder response
    await update.message.reply_text(
        f"🔍 Judging: {domain}\nContent: {content}\n\n"
        f"Kernel integration: TBD\n"
        f"Status: Bot skeleton ready (awaiting kernel wiring)"
    )

async def main():
    """Start the bot"""
    if not BOT_TOKEN:
        logger.error("TELEGRAM_BOT_TOKEN not set in environment")
        return

    app = Application.builder().token(BOT_TOKEN).build()

    # Handlers
    app.add_handler(CommandHandler("start", start))
    app.add_handler(CommandHandler("judge", judge))

    logger.info("CYNIC Telegram bot started")
    logger.info(f"Kernel: {KERNEL_URL}")

    # Run
    await app.run_polling()

if __name__ == "__main__":
    asyncio.run(main())
EOF
echo "✓ Telegram bot skeleton created at cynic-python/agents/telegram_bot/"
echo ""

# 5. Summary
echo "=== SETUP COMPLETE ==="
echo ""
echo "✓ Verdicts: $VERDICT_COUNT ready in ~/.cynic/organs/hermes/x/verdicts_to_post.json"
echo "✓ Observation framework: .cynic/may-1-10-experiment/OBSERVATION_LOG.md"
echo "✓ Telegram bot skeleton: cynic-python/agents/telegram_bot/bot.py"
echo ""
echo "NEXT STEPS (for T., starting May 1):"
echo "1. Post @CynicOracle philosophy thread (4-6 tweets)"
echo "2. Share personal insight tweet from datasets"
echo "3. Deploy Telegram bot (requires: pip install python-telegram-bot)"
echo "4. Log daily observations (update OBSERVATION_LOG.md evening)"
echo "5. Monitor engagement (which voice resonates?)"
echo ""
echo "Hypothesis test: Are three voices coherent?"
echo "Falsification test: Do Week 1 verdicts hold in Week 2 data?"
