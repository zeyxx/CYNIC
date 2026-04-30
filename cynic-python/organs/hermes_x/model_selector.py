#!/usr/bin/env python3
"""
Advanced Model Selection for Hermes Meta-Agent

Multi-account + multi-model discovery and routing:
1. Discover all available accounts (GEMINI_API_KEY, GEMINI_API_KEY_2, etc.)
2. Per account, discover available models + quota status
3. Select best model by cost/speed trade-off
4. Route: Account1-Pro → Account1-Flash → Account2-Pro → Gemma → Degraded

Configuration:
  export GEMINI_API_KEY=<google-cloud-project-1-key>
  export GEMINI_API_KEY_2=<google-cloud-project-2-key>
  export GEMINI_API_KEY_3=<google-cloud-project-3-key>

Architecture:
  ModelSelector
  ├─ Discover Accounts (scan env for GEMINI_API_KEY_N)
  │  └─ Per account: test models + quota status
  │
  ├─ Model Tiers (priority order)
  │  ├─ Tier 1: gemini-2.0-flash (fastest, lowest quota cost)
  │  ├─ Tier 2: gemini-1.5-flash (balanced)
  │  ├─ Tier 3: gemini-1.5-pro (slowest, highest quality)
  │  └─ Fallback: gemma-local (no quota)
  │
  └─ Select Best Available
     ├─ Try each account in order: Pro → Flash
     └─ When account exhausted: skip to next account
"""

__version__ = "0.2.0"

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, List, Tuple


class Account:
    """Represents a Google Cloud project with Gemini API access."""

    def __init__(self, name: str, api_key: str):
        self.name = name  # "primary", "backup_1", "backup_2", etc.
        self.api_key = api_key
        self.available_models = []  # Discovered models
        self.quota_status = {}  # Per-model quota info
        self.discovered = False

    def discover_models(self) -> bool:
        """Test which models are available in this account.

        Returns True if discovery successful, False if account is invalid/inaccessible.
        """
        models_to_try = [
            "gemini-2.0-flash",
            "gemini-1.5-flash",
            "gemini-1.5-pro",
        ]

        for model in models_to_try:
            try:
                result = subprocess.run(
                    ["gemini", "-m", model, "-p", "test"],
                    capture_output=True,
                    text=True,
                    timeout=10,
                    env={**os.environ, "GEMINI_API_KEY": self.api_key},
                )

                if result.returncode == 0:
                    # Model available in this account
                    self.available_models.append(model)
                    self.quota_status[model] = {"status": "available"}
                elif "QUOTA_EXHAUSTED" in result.stderr or "TerminalQuotaError" in result.stderr:
                    # Model available but quota exhausted
                    reset_seconds = 0
                    if "reset after" in result.stderr:
                        try:
                            parts = result.stderr.split("reset after ")
                            if len(parts) > 1:
                                time_str = parts[1].split("s")[0]
                                if "h" in time_str:
                                    h = int(time_str.split("h")[0])
                                    reset_seconds = h * 3600
                        except (ValueError, IndexError):
                            pass
                    self.quota_status[model] = {
                        "status": "quota_exhausted",
                        "reset_seconds": reset_seconds,
                    }
                elif "ModelNotFoundError" in result.stderr or "Requested entity was not found" in result.stderr:
                    # Model not available in this account
                    self.quota_status[model] = {"status": "not_available"}
                else:
                    # Other error
                    self.quota_status[model] = {
                        "status": "error",
                        "error": result.stderr[:100],
                    }

            except subprocess.TimeoutExpired:
                self.quota_status[model] = {"status": "timeout"}
            except Exception as e:
                self.quota_status[model] = {"status": "error", "error": str(e)}

        self.discovered = True
        return len(self.available_models) > 0 or any(
            s.get("status") == "quota_exhausted"
            for s in self.quota_status.values()
        )


class ModelSelector:
    """Intelligent multi-account, multi-model selector with fallback strategy."""

    def __init__(self):
        self.accounts: List[Account] = []
        self.kernel_api_addr = os.environ.get("CYNIC_REST_ADDR", "")
        self.kernel_api_key = os.environ.get("CYNIC_API_KEY", "")
        self._discover_accounts()

    def _discover_accounts(self):
        """Scan environment for all available Gemini API keys."""
        # Primary account
        if os.environ.get("GEMINI_API_KEY"):
            self.accounts.append(
                Account("primary", os.environ.get("GEMINI_API_KEY"))
            )

        # Backup accounts
        i = 2
        while os.environ.get(f"GEMINI_API_KEY_{i}"):
            self.accounts.append(
                Account(f"backup_{i-1}", os.environ.get(f"GEMINI_API_KEY_{i}"))
            )
            i += 1

    def discover_all_models(self) -> Dict:
        """Discover available models across all accounts.

        Returns dict: {account_name: {model: quota_status}}
        """
        result = {}
        for account in self.accounts:
            account.discover_models()
            result[account.name] = {
                "available": account.available_models,
                "quota_status": account.quota_status,
            }
        return result

    def select_best_model(self) -> Tuple[Optional[str], Optional[str], Dict]:
        """Select the best available model across all accounts.

        Returns: (model_name, account_name, status_dict)

        Priority order (per account):
        1. gemini-2.0-flash (fastest, cheapest)
        2. gemini-1.5-flash (balanced)
        3. gemini-1.5-pro (slowest, best quality)

        Then moves to next account if current exhausted.
        """
        # Discover all models first if not already done
        for account in self.accounts:
            if not account.discovered:
                account.discover_models()

        # Try each account
        for account in self.accounts:
            # Try models in priority order
            for model in ["gemini-2.0-flash", "gemini-1.5-flash", "gemini-1.5-pro"]:
                if model in account.available_models:
                    return model, account.name, {
                        "selected_model": model,
                        "selected_account": account.name,
                        "status": "available",
                    }

        # No cloud model available, check Gemma local
        try:
            result = subprocess.run(
                ["gemini", "gemma", "status"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if result.returncode == 0 and "Running" in result.stdout:
                return "gemma-local", "local", {
                    "selected_model": "gemma-local",
                    "selected_account": "local",
                    "status": "available",
                }
        except Exception:
            pass

        # All exhausted
        return None, None, {
            "selected_model": None,
            "status": "degraded",
            "reason": "All models exhausted",
            "accounts_tried": len(self.accounts),
            "accounts_available": len(self.accounts),
        }

    def query_with_fallback(self, prompt: str) -> Tuple[Optional[str], Dict]:
        """Query with intelligent fallback across accounts/models.

        Returns: (response, status_dict)
        """
        model, account_name, selection = self.select_best_model()

        if not model:
            return None, selection

        try:
            env = os.environ.copy()

            # Set account if it's a cloud model
            if account_name != "local":
                account = next(a for a in self.accounts if a.name == account_name)
                env["GEMINI_API_KEY"] = account.api_key

            result = subprocess.run(
                ["gemini", "-m", model, "-p", prompt],
                capture_output=True,
                text=True,
                timeout=60,
                env=env,
            )

            if result.returncode == 0:
                return result.stdout.strip(), {
                    "selected_model": model,
                    "selected_account": account_name,
                    "status": "success",
                }
            else:
                return None, {
                    "selected_model": model,
                    "selected_account": account_name,
                    "status": "error",
                    "error": result.stderr[:200],
                }

        except subprocess.TimeoutExpired:
            return None, {
                "selected_model": model,
                "selected_account": account_name,
                "status": "timeout",
            }
        except Exception as e:
            return None, {
                "selected_model": model,
                "selected_account": account_name,
                "status": "error",
                "error": str(e),
            }


def main():
    """Test multi-account discovery and selection."""
    print("Advanced Model Selector v0.2.0")
    print("=" * 60)
    print()

    selector = ModelSelector()

    print(f"Discovered {len(selector.accounts)} account(s):")
    for account in selector.accounts:
        print(f"  - {account.name}")
    print()

    print("Discovering available models per account...")
    discovery = selector.discover_all_models()
    for account_name, info in discovery.items():
        print(f"\n{account_name}:")
        print(f"  Available: {info['available']}")
        for model, status in info["quota_status"].items():
            print(f"    {model}: {status.get('status', 'unknown')}")

    print()
    print("Selecting best available model...")
    model, account, status = selector.select_best_model()
    print(f"  Selected: {model} (account: {account})")
    print(f"  Status: {status.get('status')}")

    if model:
        print()
        print("Testing query...")
        response, query_status = selector.query_with_fallback("Say 'hello' briefly")
        if response:
            print(f"  ✓ Success: {response[:100]}")
        else:
            print(f"  ✗ Failed: {query_status.get('status')}")


if __name__ == "__main__":
    main()
