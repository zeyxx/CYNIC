#!/usr/bin/env python3
"""
Discover all Gemini models available on authenticated account + quota status.

Queries the Google Generative AI API to get full model list, then probes
each model to detect quota status. Outputs JSON for consumption by model_selector.

Output: ~/.cynic/organs/hermes/x/gemini_models.json
{
  "account": "authenticated",
  "timestamp": "2026-04-30T08:30:00",
  "models": [
    {
      "name": "gemini-2.5-flash",
      "status": "quota_exhausted",
      "reset_seconds": 15120,
      "reset_time": "2026-04-30T12:32:00"
    },
    {
      "name": "gemini-2.0-flash",
      "status": "available"
    },
    ...
  ]
}
"""

__version__ = "0.1.0"

import json
import os
import subprocess
import sys
import urllib.request
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Dict, Any


def get_auth_token() -> str:
    """Get Gemini auth token from environment or gcloud auth."""
    # Try environment variable first
    if os.environ.get("GEMINI_API_KEY"):
        return os.environ["GEMINI_API_KEY"]

    # Try gcloud auth (for authenticated accounts)
    try:
        result = subprocess.run(
            ["gcloud", "auth", "application-default", "print-access-token"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except Exception:
        pass

    raise RuntimeError("No GEMINI_API_KEY set and gcloud auth not available")


def get_available_models(token: str) -> List[str]:
    """Query Google Generative AI API for list of available models."""
    try:
        url = f"https://generativelanguage.googleapis.com/v1beta/models?key={token}"
        with urllib.request.urlopen(url, timeout=10) as response:
            data = json.loads(response.read())

        models = []
        for model in data.get("models", []):
            name = model.get("name", "").replace("models/", "")
            # Filter to text-generation models (skip embeddings, audio-only, etc.)
            if name and not any(
                x in name
                for x in ["embedding", "-vision-only", "-tts", "-audio"]
            ):
                models.append(name)

        return sorted(models)
    except Exception as e:
        print(f"Error querying models API: {e}", file=sys.stderr)
        return []


def probe_model_quota(model: str, token: str) -> Dict[str, Any]:
    """Probe a model's quota status via the REST API (faster than CLI)."""
    try:
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={token}"
        payload = json.dumps({"contents": [{"parts": [{"text": "test"}]}]})

        request = urllib.request.Request(
            url,
            data=payload.encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        try:
            with urllib.request.urlopen(request, timeout=2) as response:
                return {"name": model, "status": "available"}
        except urllib.error.HTTPError as e:
            status_info = {"name": model}

            if e.code == 429:
                status_info["status"] = "quota_exhausted"
                # Try to extract reset time from response body
                try:
                    body = e.read().decode("utf-8")
                    error_data = json.loads(body)
                    if "error" in error_data:
                        error_msg = str(error_data.get("error", {}))
                        if "reset after" in error_msg:
                            parts = error_msg.split("reset after ")
                            if len(parts) > 1:
                                time_str = parts[1].split("s")[0].strip()
                                # Try to parse time
                                hours = minutes = seconds = 0
                                try:
                                    if "h" in time_str:
                                        h_parts = time_str.split("h")
                                        hours = int(h_parts[0])
                                        time_str = h_parts[1]
                                    if "m" in time_str:
                                        m_parts = time_str.split("m")
                                        minutes = int(m_parts[0])
                                        time_str = m_parts[1]
                                    if "s" in time_str:
                                        s_parts = time_str.split("s")
                                        seconds = int(s_parts[0])

                                    reset_seconds = hours * 3600 + minutes * 60 + seconds
                                    status_info["reset_seconds"] = reset_seconds
                                    status_info["reset_time"] = (
                                        datetime.now() + timedelta(seconds=reset_seconds)
                                    ).isoformat()
                                except (ValueError, IndexError):
                                    pass
                except Exception:
                    pass
            elif e.code == 404:
                status_info["status"] = "not_available"
            elif e.code == 401:
                status_info["status"] = "unauthorized"
            else:
                status_info["status"] = "error"
                status_info["error_code"] = e.code

            return status_info

    except urllib.error.URLError as e:
        return {"name": model, "status": "network_error", "error": str(e)}
    except Exception as e:
        return {"name": model, "status": "error", "error": str(e)}


def main():
    print(f"Gemini Model Discovery v{__version__}")
    print("=" * 60)

    # Get auth token
    try:
        token = get_auth_token()
        print(f"✓ Auth token loaded")
    except RuntimeError as e:
        print(f"✗ {e}", file=sys.stderr)
        sys.exit(1)

    # Get list of models
    print("Querying available models...")
    models = get_available_models(token)
    if not models:
        print("✗ No models found", file=sys.stderr)
        sys.exit(1)

    print(f"✓ Found {len(models)} models")
    print()

    # Probe each model (limit to first 15 for speed, REST API is faster than CLI)
    models_to_probe = models[:15]
    print(f"Probing quota status ({len(models_to_probe)} of {len(models)} models)...")
    model_statuses = []
    for i, model in enumerate(models_to_probe):
        status = probe_model_quota(model, token)
        model_statuses.append(status)
        status_str = status.get("status", "unknown")
        reset_info = ""
        if status.get("reset_seconds"):
            h = status["reset_seconds"] // 3600
            m = (status["reset_seconds"] % 3600) // 60
            reset_info = f" (reset in {h}h{m}m)"
        print(f"  [{i+1:2d}/{len(models)}] {model:30s} {status_str:15s}{reset_info}")

    print()

    # Save to file
    output_dir = (
        Path.home() / ".cynic" / "organs" / "hermes" / "x"
    )
    output_dir.mkdir(parents=True, exist_ok=True)
    output_file = output_dir / "gemini_models.json"

    result = {
        "account": "authenticated",
        "timestamp": datetime.now().isoformat(),
        "models": model_statuses,
        "summary": {
            "total_available_in_account": len(models),
            "probed": len(model_statuses),
            "available": sum(1 for m in model_statuses if m.get("status") == "available"),
            "quota_exhausted": sum(
                1 for m in model_statuses if m.get("status") == "quota_exhausted"
            ),
            "not_available": sum(
                1 for m in model_statuses if m.get("status") == "not_available"
            ),
            "timeout": sum(1 for m in model_statuses if m.get("status") == "timeout"),
        },
    }

    with open(output_file, "w") as f:
        json.dump(result, f, indent=2)

    print(f"✓ Saved to {output_file}")
    print()
    print(f"Summary:")
    print(f"  Total models in account: {result['summary']['total_available_in_account']}")
    print(f"  Probed: {result['summary']['probed']}")
    print(f"  Available: {result['summary']['available']}")
    print(f"  Quota exhausted: {result['summary']['quota_exhausted']}")
    print(f"  Not available: {result['summary']['not_available']}")
    if result['summary']['timeout'] > 0:
        print(f"  Timeout: {result['summary']['timeout']}")


if __name__ == "__main__":
    main()
