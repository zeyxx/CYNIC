#!/usr/bin/env python3
"""
Load available models from gemini_models.json discovery output.

This is the source of truth for which models are available on this account.
Run discover_gemini_models.py to refresh.
"""

import json
from pathlib import Path
from typing import List, Dict, Any


def load_available_models() -> List[str]:
    """Load available models from gemini_models.json discovery output.

    Returns list of model names with "available" status.
    Falls back to hardcoded list if file not found.
    """
    import sys
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))
    from hermes_paths import HERMES_X_DIR
    discovery_file = HERMES_X_DIR / "gemini_models.json"

    if discovery_file.exists():
        try:
            with open(discovery_file, 'r') as f:
                data = json.load(f)
                available = [m['name'] for m in data.get('models', []) if m.get('status') == 'available']
                if available:
                    return available
        except Exception:
            pass

    # Fallback to known good models
    return [
        "gemini-2.5-flash-lite",
        "gemini-3-flash-preview",
        "gemini-2.5-flash",
    ]


def get_model_priority() -> List[str]:
    """Return models in priority order (fastest/cheapest first)."""
    priority = [
        "gemini-2.5-flash-lite",     # Fastest, lowest cost
        "gemini-3-flash-preview",     # Newest, very fast
        "gemini-2.5-flash",           # Balanced
        "gemini-2.5-flash-image",     # With vision
        "gemini-2.5-pro",             # Best quality
    ]

    available = load_available_models()

    # Return priority list filtered to only available models
    return [m for m in priority if m in available]
