#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: Hermes X backend selector.
"""Hermes X backend selector.

Contract-first selector: infra/registry.json defines organ-x backend order,
backends.toml defines the concrete endpoints. The legacy CLI path is only used when
HERMES_X_ALLOW_ANTIGRAVITY_LEGACY is enabled.
"""

from __future__ import annotations

import json
import os
import subprocess
from typing import Any, Dict, Optional, Tuple
from urllib import error, request

from model_selector_config import (
    get_backend_config,
    resolve_backend_state,
)


class BackendSelector:
    """Select the best available inference backend for Hermes X."""

    def __init__(self):
        state = resolve_backend_state()
        self.backends: dict[str, dict[str, Any]] = state["backends"]
        self.contract: dict[str, Any] = state["contract"]
        self.legacy_backend: str = state["legacy_backend"]
        self.legacy_command: str = state["legacy_command"]
        self.legacy_model: str = state["legacy_model"]
        self.legacy_enabled: bool = bool(state["legacy_enabled"])
        self.priority: list[str] = state["priority"]
        self.policy = state["policy"]

    def discover_all_backends(self) -> Dict[str, Dict[str, Any]]:
        """Return the resolved backend table and a lightweight status summary."""
        summary: Dict[str, Dict[str, Any]] = {}
        for name, cfg in self.backends.items():
            summary[name] = {
                "enabled": cfg.get("enabled", True) is not False,
                "provider": cfg.get("provider", "unknown"),
                "model": cfg.get("model", ""),
                "base_url": cfg.get("base_url", ""),
                "json_mode": bool(cfg.get("json_mode", False)),
                "disable_thinking": bool(cfg.get("disable_thinking", False)),
            }
        return summary

    def select_best_backend(self) -> Tuple[Optional[str], Optional[Dict[str, Any]], Dict[str, Any]]:
        """Select the best available backend in contract-first order."""
        for name in self.priority:
            cfg = self.backends.get(name) or get_backend_config(name)
            if not cfg:
                continue
            if cfg.get("enabled") is False:
                continue
            if name == self.legacy_backend and not self.legacy_enabled:
                continue
            return name, cfg, {
                "selected_backend": name,
                "selected_model": cfg.get("model"),
                "selected_account": cfg.get("provider", name),
                "status": "available",
            }

        return None, None, {
            "selected_backend": None,
            "selected_model": None,
            "status": "degraded",
            "reason": "No inference backend available",
            "policy": self.policy,
        }

    def query_with_fallback(self, prompt: str) -> Tuple[Optional[str], Dict[str, Any]]:
        """Query the first healthy backend and return its completion text."""
        backend_name, backend_cfg, selection = self.select_best_backend()

        if not backend_name or not backend_cfg:
            return None, selection

        if backend_name == self.legacy_backend:
            return self._query_legacy_cli(prompt)

        try:
            result = self._query_openai_backend(backend_name, backend_cfg, prompt)
            if result.get("ok"):
                return result["content"], {
                    "selected_backend": backend_name,
                    "selected_model": backend_cfg.get("model"),
                    "status": "success",
                }
            return None, {
                "selected_backend": backend_name,
                "selected_model": backend_cfg.get("model"),
                "status": result.get("status", "error"),
                "error": result.get("error", "backend request failed"),
            }
        except Exception as exc:
            return None, {
                "selected_backend": backend_name,
                "selected_model": backend_cfg.get("model"),
                "status": "error",
                "error": str(exc),
            }

    def _query_openai_backend(
        self,
        backend_name: str,
        backend_cfg: Dict[str, Any],
        prompt: str,
    ) -> Dict[str, Any]:
        base_url = str(backend_cfg.get("base_url", "")).rstrip("/")
        model = str(backend_cfg.get("model", "default"))
        if not base_url:
            return {"ok": False, "status": "missing_base_url", "error": f"{backend_name}: no base_url configured"}

        url = f"{base_url}/chat/completions"
        timeout = int(backend_cfg.get("timeout_secs", backend_cfg.get("timeout", 60)))
        max_tokens = int(backend_cfg.get("max_tokens", 1024))
        temperature = float(backend_cfg.get("temperature", 0.3))
        json_mode = bool(backend_cfg.get("json_mode", False))
        disable_thinking = bool(backend_cfg.get("disable_thinking", False))
        api_key_env = str(backend_cfg.get("api_key_env", "")).strip()
        api_key = os.environ.get(api_key_env, "") if api_key_env else ""

        messages = [{"role": "user", "content": prompt}]
        if disable_thinking and "qwen" in model.lower():
            messages[0]["content"] = "/no_think\n" + messages[0]["content"]

        body: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }
        if json_mode:
            body["response_format"] = {"type": "json_object"}

        headers = {
            "Content-Type": "application/json",
            "User-Agent": "CYNIC-HermesX-BackendSelector/1.0",
        }
        if api_key:
            headers["Authorization"] = f"Bearer {api_key}"

        data = json.dumps(body).encode("utf-8")
        req = request.Request(url, data=data, headers=headers, method="POST")

        try:
            with request.urlopen(req, timeout=timeout) as resp:
                raw = resp.read().decode("utf-8", errors="replace")
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError:
                return {"ok": False, "status": "unparseable_response", "error": raw[:200]}

            content = (
                payload.get("choices", [{}])[0]
                .get("message", {})
                .get("content")
            )
            if not isinstance(content, str) or not content.strip():
                return {"ok": False, "status": "empty_response", "error": raw[:200]}
            return {"ok": True, "content": content.strip(), "raw": payload}
        except error.HTTPError as exc:
            body_text = exc.read().decode("utf-8", errors="replace")[:200]
            return {"ok": False, "status": f"http_{exc.code}", "error": body_text}
        except Exception as exc:
            return {"ok": False, "status": "request_error", "error": str(exc)}

    def _query_legacy_cli(self, prompt: str) -> Tuple[Optional[str], Dict[str, Any]]:
        if not self.legacy_enabled:
            return None, {
                "selected_backend": self.legacy_backend,
                "status": "disabled",
                "error": "Legacy CLI path disabled; set HERMES_X_ALLOW_ANTIGRAVITY_LEGACY=1 if you must use it",
            }

        env = os.environ.copy()
        # Legacy CLI expects its own environment to be present already.
        try:
            result = subprocess.run(
                [self.legacy_command, "-m", self.legacy_model, "-p", prompt],
                capture_output=True,
                text=True,
                timeout=60,
                env=env,
            )
        except subprocess.TimeoutExpired:
            return None, {"selected_backend": self.legacy_backend, "status": "timeout"}
        except Exception as exc:
            return None, {"selected_backend": self.legacy_backend, "status": "error", "error": str(exc)}

        if result.returncode == 0:
            return result.stdout.strip(), {
                "selected_backend": self.legacy_backend,
                "status": "success",
            }

        return None, {
            "selected_backend": self.legacy_backend,
            "status": "error",
            "error": result.stderr[:200],
        }


def main() -> int:
    """Test contract-first backend discovery and selection."""
    print("Hermes X Backend Selector v1.0.0")
    print("=" * 60)
    print()

    selector = BackendSelector()

    print(f"Discovered {len(selector.backends)} backend(s):")
    for name in selector.priority:
        cfg = selector.backends.get(name, {})
        print(f"  - {name} -> {cfg.get('model', '')}")
    if selector.legacy_enabled and selector.legacy_backend not in selector.priority:
        print(f"  - {selector.legacy_backend} (legacy opt-in, command: {selector.legacy_command})")
    print()

    print("Inspecting backend configuration...")
    discovery = selector.discover_all_backends()
    for backend_name, info in discovery.items():
        print(f"\n{backend_name}:")
        print(f"  Enabled: {info['enabled']}")
        print(f"  Provider: {info['provider']}")
        print(f"  Model: {info['model']}")
        print(f"  Base URL: {info['base_url']}")

    print()
    print("Selecting best available backend...")
    backend, cfg, status = selector.select_best_backend()
    print(f"  Selected: {backend} (model: {cfg.get('model') if cfg else None})")
    print(f"  Status: {status.get('status')}")

    if backend:
        print()
        print("Testing query...")
        response, query_status = selector.query_with_fallback("Say 'hello' briefly")
        if response:
            print(f"  ✓ Success: {response[:100]}")
        else:
            print(f"  ✗ Failed: {query_status.get('status')}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
