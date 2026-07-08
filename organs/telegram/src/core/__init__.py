"""Core boundary for organ-telegram."""

CAPABILITY = {
    "organ": "organ-telegram",
    "domain": "social",
    "role": "telegram_channel_ingestion",
    "owns": ("message_buffering", "message_normalization", "pipeline_extraction"),
}
