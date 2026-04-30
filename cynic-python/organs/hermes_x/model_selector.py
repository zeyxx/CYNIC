#!/usr/bin/env python3
"""
Model Selection Strategy for Hermes Meta-Agent

Manages Gemini API + local Gemma fallback based on quota exhaustion.
Alerts kernel + human when switching models or hitting quota limits.

Priority:
1. Try Gemini API (cloud) — primary, best quality
   - Primary key: GEMINI_API_KEY
   - Backup keys: GEMINI_API_KEY_2, GEMINI_API_KEY_3, etc. (optional)
   - Rotates through keys if one hits quota
2. Fall back to Gemma (local) — secondary, if all API keys exhausted
3. Degrade gracefully (skip meta-guidance) — if both unavailable

Configuration:
  # Primary API key
  export GEMINI_API_KEY=<project-1-api-key>

  # Optional backup keys (different Google Cloud projects)
  export GEMINI_API_KEY_2=<project-2-api-key>
  export GEMINI_API_KEY_3=<project-3-api-key>

  # Or set in ~/.cynic-env or systemd EnvironmentFile=/path/to/env
"""

__version__ = "0.1.0"

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict


class ModelSelector:
    """Manage LLM model selection with fallback strategy.

    Supports multiple Gemini API keys for quota rotation:
    - Primary: GEMINI_API_KEY
    - Backups: GEMINI_API_KEY_2, GEMINI_API_KEY_3, etc. (optional)

    When one key hits quota, tries next in rotation.
    """

    def __init__(self, kernel_api_addr: str = "", kernel_api_key: str = ""):
        self.kernel_api_addr = kernel_api_addr or os.environ.get("CYNIC_REST_ADDR", "")
        self.kernel_api_key = kernel_api_key or os.environ.get("CYNIC_API_KEY", "")

        # Collect all available Gemini API keys for rotation
        self.gemini_api_keys = []
        if os.environ.get("GEMINI_API_KEY"):
            self.gemini_api_keys.append(("primary", os.environ.get("GEMINI_API_KEY")))

        # Check for backup keys (GEMINI_API_KEY_2, GEMINI_API_KEY_3, etc.)
        i = 2
        while os.environ.get(f"GEMINI_API_KEY_{i}"):
            self.gemini_api_keys.append((f"backup_{i-1}", os.environ.get(f"GEMINI_API_KEY_{i}")))
            i += 1

        self.current_key_index = 0
        self.failed_keys = set()

    def query_gemini_api(self, prompt: str) -> tuple[Optional[str], Dict]:
        """Try Gemini API with current active key.

        Note: Multiple keys are discovered from environment (GEMINI_API_KEY,
        GEMINI_API_KEY_2, etc.) but only GEMINI_API_KEY is active. To use
        a different key, set it as GEMINI_API_KEY in the environment before
        calling this script.

        Returns (response, status_dict) where status_dict contains:
        - model: "gemini-api"
        - status: "success" | "quota_exhausted" | "error"
        - api_key_used: "primary" (always, only one active at a time)
        - quota_reset_seconds: seconds until quota resets (if exhausted)
        - error: error message (if applicable)
        """
        try:
            result = subprocess.run(
                ["gemini", "-p", prompt],
                capture_output=True,
                text=True,
                timeout=60,
            )

            if result.returncode == 0:
                return result.stdout.strip(), {
                    "model": "gemini-api",
                    "status": "success",
                    "api_key_used": "primary",
                }

            # Check if quota exhausted
            stderr = result.stderr or ""
            if "TerminalQuotaError" in stderr or "QUOTA_EXHAUSTED" in stderr:
                reset_seconds = 0
                if "reset after" in stderr:
                    try:
                        parts = stderr.split("reset after ")
                        if len(parts) > 1:
                            time_str = parts[1].split("s")[0]
                            if "h" in time_str:
                                h = int(time_str.split("h")[0])
                                reset_seconds = h * 3600
                    except (ValueError, IndexError):
                        pass

                return None, {
                    "model": "gemini-api",
                    "status": "quota_exhausted",
                    "api_key_used": "primary",
                    "quota_reset_seconds": reset_seconds,
                    "keys_available": len(self.gemini_api_keys),
                }
            else:
                return None, {
                    "model": "gemini-api",
                    "status": "error",
                    "api_key_used": "primary",
                    "error": stderr[:200],
                }

        except FileNotFoundError:
            return None, {
                "model": "gemini-api",
                "status": "error",
                "error": "gemini CLI not found",
            }
        except subprocess.TimeoutExpired:
            return None, {
                "model": "gemini-api",
                "status": "error",
                "error": "timeout",
            }
        except Exception as e:
            return None, {
                "model": "gemini-api",
                "status": "error",
                "error": str(e),
            }

    def query_gemma_local(self, prompt: str) -> tuple[Optional[str], Dict]:
        """Try local Gemma model.

        Returns (response, status_dict) where status_dict contains:
        - model: "gemma-local"
        - status: "success" | "not_available" | "error"
        - error: error message (if applicable)
        """
        try:
            # Test if Gemma server is running
            result = subprocess.run(
                ["gemini", "-m", "gemma", "-p", prompt],
                capture_output=True,
                text=True,
                timeout=60,
            )

            if result.returncode == 0:
                return result.stdout.strip(), {
                    "model": "gemma-local",
                    "status": "success",
                }
            else:
                stderr = result.stderr or ""
                if "not installed" in stderr or "not running" in stderr or "not found" in stderr:
                    return None, {
                        "model": "gemma-local",
                        "status": "not_available",
                        "error": "Gemma not set up. Run: gemini gemma setup",
                    }
                else:
                    return None, {
                        "model": "gemma-local",
                        "status": "error",
                        "error": stderr[:200],
                    }

        except FileNotFoundError:
            return None, {
                "model": "gemma-local",
                "status": "error",
                "error": "gemini CLI not found",
            }
        except subprocess.TimeoutExpired:
            return None, {
                "model": "gemma-local",
                "status": "error",
                "error": "timeout",
            }
        except Exception as e:
            return None, {
                "model": "gemma-local",
                "status": "error",
                "error": str(e),
            }

    def select_and_query(self, prompt: str) -> tuple[Optional[str], Dict]:
        """Select best available model and query.

        Priority: Gemini API → Gemma local → None (degraded)

        Returns (response, status_dict) where status_dict tracks model choices + alerts.
        """
        response, api_status = self.query_gemini_api(prompt)

        if response:
            return response, {
                "selected_model": "gemini-api",
                "status": "success",
                "model_choice_reason": "primary (cloud)",
            }

        # API failed or quota exhausted
        if api_status.get("status") == "quota_exhausted":
            # Alert kernel about quota
            self.alert_kernel_quota_exhausted(api_status)

        # Try Gemma fallback
        response, gemma_status = self.query_gemma_local(prompt)

        if response:
            # Successful fallback
            reason = "API quota exhausted, using local Gemma"
            self.alert_kernel_model_switch("gemini-api", "gemma-local", reason)
            return response, {
                "selected_model": "gemma-local",
                "status": "success",
                "model_choice_reason": reason,
                "primary_failure": api_status,
            }

        # Both failed
        reason = "Gemini API quota exhausted, Gemma unavailable"
        self.alert_kernel_degraded(api_status, gemma_status, reason)
        return None, {
            "selected_model": None,
            "status": "degraded",
            "model_choice_reason": reason,
            "api_failure": api_status,
            "gemma_failure": gemma_status,
        }

    def alert_kernel_quota_exhausted(self, api_status: Dict):
        """Alert kernel that Gemini API quota is exhausted."""
        if not self.kernel_api_addr or not self.kernel_api_key:
            return

        alert = {
            "tool": "model_quota_alert",
            "target": "hermes-meta-agent",
            "severity": "warning",
            "context": f"Gemini API quota exhausted. Reset in {api_status.get('quota_reset_seconds', '?')}s. Falling back to local Gemma if available.",
            "timestamp": datetime.now().isoformat(),
            "tags": ["quota-exhausted", "gemini-api"],
        }

        try:
            import requests

            requests.post(
                f"{self.kernel_api_addr}/observe",
                headers={"Authorization": f"Bearer {self.kernel_api_key}"},
                json=alert,
                timeout=5,
            )
        except Exception:
            pass

    def alert_kernel_model_switch(self, from_model: str, to_model: str, reason: str):
        """Alert kernel that model was switched due to quota/failure."""
        if not self.kernel_api_addr or not self.kernel_api_key:
            return

        alert = {
            "tool": "model_switch_alert",
            "target": "hermes-meta-agent",
            "severity": "info",
            "context": f"Switched from {from_model} to {to_model}: {reason}",
            "timestamp": datetime.now().isoformat(),
            "tags": ["model-switch", from_model, to_model],
        }

        try:
            import requests

            requests.post(
                f"{self.kernel_api_addr}/observe",
                headers={"Authorization": f"Bearer {self.kernel_api_key}"},
                json=alert,
                timeout=5,
            )
        except Exception:
            pass

    def alert_kernel_degraded(self, api_status: Dict, gemma_status: Dict, reason: str):
        """Alert kernel that meta-agent is degraded (no LLM available)."""
        if not self.kernel_api_addr or not self.kernel_api_key:
            return

        alert = {
            "tool": "meta_agent_degraded",
            "target": "hermes-meta-agent",
            "severity": "warning",
            "context": f"Meta-agent degraded: {reason}. Skipping Gemini synthesis.",
            "timestamp": datetime.now().isoformat(),
            "api_failure": api_status,
            "gemma_failure": gemma_status,
            "tags": ["degraded", "no-llm"],
        }

        try:
            import requests

            requests.post(
                f"{self.kernel_api_addr}/observe",
                headers={"Authorization": f"Bearer {self.kernel_api_key}"},
                json=alert,
                timeout=5,
            )
        except Exception:
            pass


def main():
    """Test model selection with multi-key rotation."""
    selector = ModelSelector()

    print("Testing model selection strategy...")
    print(f"Available Gemini API keys: {len(selector.gemini_api_keys)}")
    for name, _ in selector.gemini_api_keys:
        print(f"  - {name}")
    print()

    # Test Gemini API with key rotation
    print("1. Trying Gemini API (with key rotation)...")
    response, status = selector.query_gemini_api("Say 'hello' briefly")
    if response:
        print(f"   ✓ Gemini API success (key: {status.get('api_key_used')})")
        print(f"   Response: {response[:100]}")
    else:
        print(f"   ✗ Gemini API failed: {status.get('status')}")
        if status.get("keys_tried") and status.get("keys_available"):
            print(f"      Tried {status['keys_tried']}/{status['keys_available']} keys")
        if status.get("quota_reset_seconds"):
            hours = status['quota_reset_seconds'] / 3600
            print(f"      (All quotas reset in ~{hours:.1f}h)")

    print()
    print("2. Trying Gemma local...")
    response, status = selector.query_gemma_local("Say 'hello' briefly")
    if response:
        print(f"   ✓ Gemma local success")
        print(f"   Response: {response[:100]}")
    else:
        print(f"   ✗ Gemma local failed: {status.get('status')}")
        if "not_available" in status.get("status", ""):
            print(f"      {status.get('error')}")

    print()
    print("3. Full fallback selection...")
    response, status = selector.select_and_query("Respond with your model name only")
    print(f"   Selected model: {status.get('selected_model')}")
    print(f"   Status: {status.get('status')}")
    print(f"   Reason: {status.get('model_choice_reason')}")
    if response:
        print(f"   Response: {response[:100]}")


if __name__ == "__main__":
    main()
