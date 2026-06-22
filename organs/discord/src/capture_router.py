"""Endpoint router for Discord passive capture.

The router classifies observable Discord web API responses. Storage decisions
stay policy-driven: seeing an endpoint does not imply storing raw content.
"""
from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True)
class CaptureRoute:
    endpoint_type: str
    policy: str
    resource_id: int | None = None


_CHANNEL_MESSAGES = re.compile(r"/api/v\d+/(?:channels|guilds/\d+/channels)/(\d+)/messages")
_CHANNEL_MESSAGES_SEARCH = re.compile(r"/api/v\d+/channels/(\d+)/messages/search")
_CHANNEL = re.compile(r"/api/v\d+/channels/(\d+)(?:$|[/?])")
_GUILD_CHANNELS = re.compile(r"/api/v\d+/guilds/(\d+)/channels")
_GUILD = re.compile(r"/api/v\d+/guilds/(\d+)(?:$|[/?])")
_THREADS = re.compile(r"/api/v\d+/channels/(\d+)/threads")
_USERS = re.compile(r"/api/v\d+/users/(@me|\d+)(?:$|[/?])")
_RELATIONSHIPS = re.compile(r"/api/v\d+/users/@me/relationships")


def route_discord_path(path: str, allowed_channels: set[int]) -> CaptureRoute | None:
    """Classify a Discord API path into a normalized endpoint type and policy."""
    match = _CHANNEL_MESSAGES_SEARCH.search(path)
    if match:
        channel_id = int(match.group(1))
        policy = "store_messages" if channel_id in allowed_channels else "metadata_only"
        return CaptureRoute("channel_messages_search", policy, channel_id)

    match = _CHANNEL_MESSAGES.search(path)
    if match:
        channel_id = int(match.group(1))
        policy = "store_messages" if channel_id in allowed_channels else "metadata_only"
        return CaptureRoute("channel_messages", policy, channel_id)

    match = _THREADS.search(path)
    if match:
        return CaptureRoute("threads", "metadata_only", int(match.group(1)))

    match = _GUILD_CHANNELS.search(path)
    if match:
        return CaptureRoute("guild_channels", "metadata_only", int(match.group(1)))

    match = _GUILD.search(path)
    if match:
        return CaptureRoute("guild", "metadata_only", int(match.group(1)))

    match = _CHANNEL.search(path)
    if match:
        return CaptureRoute("channel", "metadata_only", int(match.group(1)))

    if _RELATIONSHIPS.search(path):
        return CaptureRoute("relationships", "metadata_only", None)

    if _USERS.search(path):
        return CaptureRoute("user", "metadata_only", None)

    return CaptureRoute("unknown_discord_api", "ignore", None)
