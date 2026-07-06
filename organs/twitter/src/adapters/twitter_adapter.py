"""Compatibility facade for older organ-twitter imports.

New code should import from the focused adapter modules directly.
"""

from .context_exporter import ContextBundleExporter
from .context_repository import JsonlContextRepository, count_jsonl, item_from_dict, read_jsonl, stable_item_id
from .twitter_perceiver import JsonStateStorage, SystemTwitterPerceiver
from .x_resolver import (
    BrowserHubRenderResolver,
    CompositeSocialSourceResolver,
    FallbackUnresolvedResolver,
    LocalHermesDatasetResolver,
    PublicEmbedResolver,
    RawCaptureResolver,
    extract_handle,
    extract_tweet_id,
)

__all__ = [
    "CompositeSocialSourceResolver",
    "BrowserHubRenderResolver",
    "ContextBundleExporter",
    "FallbackUnresolvedResolver",
    "JsonStateStorage",
    "JsonlContextRepository",
    "LocalHermesDatasetResolver",
    "PublicEmbedResolver",
    "RawCaptureResolver",
    "SystemTwitterPerceiver",
    "count_jsonl",
    "extract_handle",
    "extract_tweet_id",
    "item_from_dict",
    "read_jsonl",
    "stable_item_id",
]
