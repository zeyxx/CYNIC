"""Core boundary for organ-slack."""

CAPABILITY = {
    "organ": "organ-slack",
    "domain": "slack",
    "role": "transitional_message_relay",
    "owns": ("routing_policy", "delivery_result_mapping"),
}
