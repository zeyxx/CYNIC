"""
CYNIC Vocal Awakening â€” Telegram Validation Script.

Checks if Telegram environment variables are correctly loaded and
sends a test 'HEARTBEAT' signal. Supports .env files.
"""

import os
import asyncio
import sys

# Try to load .env if python-dotenv is installed
try:
    from dotenv import load_dotenv

    load_dotenv()
except ImportError:
    pass

# Add root to path
sys.path.append(os.getcwd())


async def test_telegram():
    print("\n--- ðŸ“£ CYNIC TELEGRAM AWAKENING TEST ---")

    token = os.environ.get("TELEGRAM_BOT_TOKEN")
    chat_id = os.environ.get("TELEGRAM_CHAT_ID")

    if not token or not chat_id:
        print("âŒ CONFIG ERROR: TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID missing.")
        print(
            "   Current values: TOKEN="
            + ("Set" if token else "MISSING")
            + ", CHAT_ID="
            + ("Set" if chat_id else "MISSING")
        )
        print(
            "   Please ensure you have a .env file or set your environment variables."
        )
        return

    print(
        f"Step 1: Found credentials (Token ending in ...{token[-4:] if token else '????'})"
    )

    from cynic.interfaces.bots.telegram_bridge import TelegramBridge

    bridge = TelegramBridge(token=token, chat_id=chat_id)

    print("Step 2: Sending Heartbeat message...")
    success = await bridge.notify(
        "â¤ï¸ <b>CYNIC HEARTBEAT</b>\n\nI am awake and my voice is connected to your device."
    )

    if success:
        print("\nâœ… SUCCESS: Message sent to Telegram. Check your phone!")
    else:
        print(
            "\nâŒ FAILURE: Bridge could not send message. Check network or token validity."
        )


if __name__ == "__main__":
    asyncio.run(test_telegram())
