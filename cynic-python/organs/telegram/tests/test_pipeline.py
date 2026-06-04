"""Tests for the Telegram signal pipeline classifier."""
import pytest
from ..pipeline import classify_block, ClassifiedBlock


# ── Buy-bot detection ─────────────────────────────────────────────────────────

def test_classify_buy_bot_sol_trending():
    text = (
        "[Dog Mars] Buy!\n"
        "🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢\n"
        "🔀 0.1243 SOL ($10)\n"
        "🔀 56 968.381 DOGM\n"
        "👤 B7QjAG...Kz2B | Txn: [link]\n"
        "⬆️ New Holder!\n"
        "💸 Market Cap $176 622\n"
        "📈 Chart https://dexscreener.com/solana/CENNjjm7cgHsA1d79xxxX3dhKqmeapGRKefbJKgXpump"
    )
    result = classify_block(-1002066575222, "block_1", text, 1)
    assert result.message_type == "buy-bot"
    assert result.domain in ("token-analysis", "twitter")
    assert len(result.mints) > 0


def test_classify_buy_bot_extracts_pump_mint():
    text = (
        "💸 Market Cap $207 477\n"
        "🔀 0.1603 SOL ($12.87)\n"
        "dexscreener.com/solana/AyLn7YdHqh3pGSNC1YrdEtEdSCVjG7BxQedxBvkTpump\n"
        "solscan.io/address/4v5z4siS1ZwrVQgz4XUqLkh6k4EG3Y5wVBKepMfrchjm"
    )
    result = classify_block(-9999, "block_2", text, 1)
    assert result.message_type == "buy-bot"
    assert any("pump" in m for m in result.mints)


# ── Shill/spam detection ──────────────────────────────────────────────────────

def test_classify_shill_service():
    text = (
        "VECTOR. A very organic Shiller, with an experience team.\n"
        "💠 TWITTER(X) RAiDS: Non-stop.\n"
        "💠 TELEGRAM SHILLING\n"
        "💠 LISTING/UPVOTING\n"
        "Message @vectorthesector for more info"
    )
    result = classify_block(-1001996676648, "block_3", text, 1)
    assert result.message_type == "shill"
    assert result.domain == "drop"


def test_classify_raid_coordination():
    text = (
        "🎊 Raid Ended - Targets Reached!\n"
        "🟩 Likes 10 | 10 [100%]\n"
        "🟩 Retweets 5 | 5 [100%]\n"
        "🟩 Replies 5 | 5 [100%]\n"
        "Duration: 31 minutes"
    )
    result = classify_block(-1002460104630, "block_4", text, 1)
    assert result.message_type == "raid"
    assert result.domain == "drop"


# ── Ops channels ──────────────────────────────────────────────────────────────

def test_classify_ops_channel():
    text = "Let's setup the group properly, configure buybots and setup raid"
    result = classify_block(-1003732138279, "block_5", text, 1)
    assert result.message_type == "ops"
    assert result.domain == "ops"


# ── Signal routing ────────────────────────────────────────────────────────────

def test_classify_trading_signal():
    text = (
        "Strong rejection from the 48k-50k resistance zone. "
        "Big players are selling there. "
        "Could still touch 50k before reversal."
    )
    result = classify_block(-1001864215962, "block_6", text, 1)
    assert result.message_type == "signal"
    assert result.domain == "trading"


def test_classify_dev_content():
    text = "this is what i see `Module '@pump-fun/pump-swap-sdk' has no exported member 'PumpFunClient'.ts`"
    result = classify_block(-1002634233787, "block_7", text, 1)
    assert result.message_type == "signal"
    assert result.domain == "dev"


def test_classify_twitter_signal():
    text = (
        "Watcher Guru: JUST IN: Robinhood gives away $500,000 worth of Dogecoin $DOGE."
    )
    result = classify_block(-1001556054753, "block_8", text, 1)
    assert result.message_type == "signal"
    assert result.domain == "twitter"


def test_classify_governance():
    text = "Don't worry meta will easily goes 20% below because of sell pressure"
    result = classify_block(-1002392373515, "block_9", text, 1)
    assert result.message_type == "signal"
    assert result.domain == "governance"


def test_unknown_channel_defaults_to_twitter():
    text = "Some random signal content without obvious patterns"
    result = classify_block(-9999999, "block_10", text, 1)
    assert result.domain == "twitter"
