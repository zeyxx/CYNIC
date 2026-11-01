"""Detect available models on Hermes-Agent, including free ones.

Usage:
    python detect.py [--json] [--free-only]
"""
import argparse
import json
import os
import sys

# Ensure hermes-agent is importable
HERMES_REPO = os.path.expanduser("~/.hermes/hermes-agent")
if HERMES_REPO not in sys.path:
    sys.path.insert(0, HERMES_REPO)

def detect_models():
    """Detect available models via Hermes CLI internals."""
    models = []
    try:
        from hermes_cli.models import _PROVIDER_MODELS
        for provider, model_list in _PROVIDER_MODELS.items():
            for m in model_list:
                models.append({"provider": provider, "model": m, "free": False})
    except Exception as e:
        print(f"Warning: could not import _PROVIDER_MODELS: {e}", file=sys.stderr)

    # Try to get nous models (free tier)
    try:
        from hermes_cli.model_setup_flows import _nous_model_catalog
        # Nous free tier models
        free_models = _nous_model_catalog(True, "https://portal.nousresearch.com", [], {})
        if isinstance(free_models, tuple) and len(free_models) >= 2:
            model_ids = free_models[0] if isinstance(free_models[0], list) else []
            for mid in model_ids:
                models.append({"provider": "nous", "model": mid, "free": True})
    except Exception as e:
        print(f"Warning: could not get nous catalog: {e}", file=sys.stderr)

    return models

def main():
    parser = argparse.ArgumentParser(description="Detect available Hermes-Agent models")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    parser.add_argument("--free-only", action="store_true", help="Show only free models")
    args = parser.parse_args()

    models = detect_models()

    if args.free_only:
        models = [m for m in models if m.get("free")]

    if args.json:
        print(json.dumps(models, indent=2))
    else:
        # Group by provider
        by_provider = {}
        for m in models:
            by_provider.setdefault(m["provider"], []).append(m)
        for provider, ms in sorted(by_provider.items()):
            free_count = sum(1 for m in ms if m.get("free"))
            print(f"\n{provider} ({len(ms)} models, {free_count} free):")
            for m in ms:
                tag = " [FREE]" if m.get("free") else ""
                print(f"  - {m['model']}{tag}")

if __name__ == "__main__":
    main()
