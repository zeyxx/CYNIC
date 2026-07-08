"""Core boundary for organ-discord."""

CAPABILITY = {
    "organ": "organ-discord",
    "domain": "social",
    "role": "passive_discord_capture",
    "owns": ("message_normalization", "pipeline_extraction", "capture_policy"),
}
