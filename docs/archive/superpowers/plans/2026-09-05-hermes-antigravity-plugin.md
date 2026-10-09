# Hermes Agent Antigravity Provider Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire et installer le plugin d'inférence Antigravity / Cloud Code Assist pour Hermes Agent (`nousresearch/hermes-agent`), permettant d'utiliser les modèles Gemini et Claude avec streaming, tool calling et thinking en s'appuyant sur l'architecture prouvée dans `pi-antigravity`.

**Architecture:** Le plugin est un module Python autonome déployé dans `~/.hermes/plugins/model-providers/antigravity/`. Il enregistre un `ProviderProfile` via le système de plugins officiel de Hermes, fournit une authentification hybride (OAuth 2.0 PKCE + détection des jetons locaux existants), traduit à la volée les requêtes de `AIAgent` vers l'API Cloud Code Assist Google (`/v1internal:streamGenerateContent`) et convertit les événements SSE en chunks OpenAI-compatibles pour la TUI et la Gateway Hermes.

**Tech Stack:** Python 3.10+, `urllib` / `http.server` (bibliothèque standard pour minimiser les dépendances externes), `pytest` pour les tests unitaires et d'intégration, compatible `nousresearch/hermes-agent`.

---

## Structure des Fichiers

Cible d'installation du plugin :
```
/home/user/.hermes/plugins/model-providers/antigravity/
├── plugin.yaml                    # Manifeste du plugin pour la découverte par Hermes
├── __init__.py                    # Profil AntigravityProfile & register_provider
├── auth.py                        # Gestion OAuth PKCE, loadCodeAssist, tokens locaux
├── client.py                      # Interface client.chat.completions.create pour AIAgent
├── translator.py                  # Traducteur de requêtes/outils et parser SSE
├── models.py                      # Catalogue statique de repli & fetch_available_models
├── README.md                      # Guide d'utilisation et configuration
└── tests/
    ├── __init__.py
    ├── test_models.py             # Tests catalogue & fallbacks
    ├── test_auth.py               # Tests PKCE, parsing de tokens, discovery
    ├── test_translator.py         # Tests mapping messages, tools, thinking, SSE
    ├── test_client.py             # Tests HTTP transport, headers, retries
    └── test_provider_registration.py # Tests intégration Hermes loader
```

---

### Task 1: Scaffolding, Manifeste & Catalogue de Modèles (`plugin.yaml`, `models.py`)

**Files:**
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/plugin.yaml`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/models.py`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/tests/__init__.py`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_models.py`

- [ ] **Step 1: Créer le répertoire du plugin et le manifeste `plugin.yaml`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/plugin.yaml` :
```yaml
name: antigravity
kind: model-provider
version: 1.0.0
description: Google Antigravity / Cloud Code Assist inference provider for Hermes Agent
author: Sovereign Operator
```

- [ ] **Step 2: Écrire le test unitaire pour `models.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_models.py` :
```python
import pytest
from unittest.mock import patch, MagicMock
from models import (
    FALLBACK_MODELS,
    DEFAULT_MODEL,
    resolve_runtime_model,
    get_thinking_config,
    fetch_available_models,
)


def test_fallback_models_contains_expected_architectures():
    assert "gemini-3.8-flash" in FALLBACK_MODELS
    assert "gemini-2.5-pro" in FALLBACK_MODELS
    assert "claude-3-7-sonnet" in FALLBACK_MODELS
    assert DEFAULT_MODEL == "gemini-3.8-flash"


def test_resolve_runtime_model_mapping():
    # Canonical IDs
    assert resolve_runtime_model("gemini-3.8-flash") == "gemini-3.8-flash"
    assert resolve_runtime_model("antigravity/gemini-3.8-flash") == "gemini-3.8-flash"
    assert resolve_runtime_model("claude-3-7-sonnet") == "claude-3-7-sonnet"
    assert resolve_runtime_model("unknown-model") == "unknown-model"


def test_get_thinking_config():
    # Thinking enabled for gemini
    cfg = get_thinking_config("gemini-3.8-flash", reasoning_effort="high")
    assert cfg is not None
    assert "thinkingBudget" in cfg
    assert cfg["thinkingBudget"] > 0

    # Thinking disabled
    cfg_off = get_thinking_config("gemini-3.8-flash", reasoning_effort=None)
    assert cfg_off is None


def test_fetch_available_models_fallback_on_failure():
    with patch("urllib.request.urlopen", side_effect=Exception("network down")):
        models = fetch_available_models("fake-token", "fake-project")
        assert models == list(FALLBACK_MODELS)
```

- [ ] **Step 3: Lancer le test pour vérifier qu'il échoue**

Run : `pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_models.py -v`  
Expected : FAIL (`ModuleNotFoundError: No module named 'models'`)

- [ ] **Step 4: Implémenter `models.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/models.py` :
```python
"""Model catalog and discovery for Antigravity provider in Hermes Agent."""

from __future__ import annotations

import json
import logging
import urllib.request
from typing import Any

logger = logging.getLogger(__name__)

FALLBACK_MODELS: tuple[str, ...] = (
    "gemini-3.8-flash",
    "gemini-3.7-flash",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "claude-3-7-sonnet",
    "claude-3-5-sonnet",
)

DEFAULT_MODEL = "gemini-3.8-flash"

ENDPOINT_FALLBACKS: list[str] = [
    "https://daily-cloudcode-pa.googleapis.com",
    "https://daily-cloudcode-pa.sandbox.googleapis.com",
    "https://cloudcode-pa.googleapis.com",
]

DEFAULT_USER_AGENT = (
    "antigravity/cli/1.1.23 (aidev_client; os_type=linux; arch=amd64; cl=974125021; auth_method=consumer)"
)


def resolve_runtime_model(model_id: str) -> str:
    """Strip provider prefix and return clean model identifier."""
    clean = model_id.strip()
    if clean.startswith("antigravity/"):
        clean = clean[len("antigravity/") :]
    return clean


def get_thinking_config(model: str, reasoning_effort: str | None) -> dict[str, Any] | None:
    """Return thinkingConfig structure if reasoning is enabled for this model."""
    if not reasoning_effort:
        return None

    budgets = {
        "low": 2048,
        "medium": 8192,
        "high": 16384,
        "max": 32768,
    }
    budget = budgets.get(str(reasoning_effort).lower(), 8192)

    # Gemini 2.5/3.x and Claude 3.7 support thinking
    if any(m in model for m in ("gemini-2.5", "gemini-3", "claude-3-7")):
        return {"thinkingBudget": budget}

    return None


def fetch_available_models(
    token: str,
    project_id: str,
    timeout: float = 6.0,
    endpoints: list[str] | None = None,
) -> list[str]:
    """Query v1internal:fetchAvailableModels across candidate endpoints."""
    candidates = endpoints or ENDPOINT_FALLBACKS
    discovered: set[str] = set()

    for endpoint in candidates:
        url = f"{endpoint}/v1internal:fetchAvailableModels"
        req = urllib.request.Request(
            url,
            data=json.dumps({"project": project_id}).encode("utf-8"),
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "User-Agent": DEFAULT_USER_AGENT,
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                models_dict = data.get("models", {})
                for m_id in models_dict.keys():
                    if any(prefix in m_id for prefix in ("gemini-", "claude-", "gpt-oss-")):
                        discovered.add(m_id)
                if discovered:
                    break
        except Exception as exc:
            logger.debug("fetchAvailableModels failed on %s: %s", endpoint, exc)
            continue

    if discovered:
        return sorted(discovered)
    return list(FALLBACK_MODELS)
```

- [ ] **Step 5: Relancer le test pour vérifier qu'il passe**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_models.py -v`  
Expected : PASS (4 tests passed)

---

### Task 2: Authentification Hybride & Gestion des Tokens (`auth.py`)

**Files:**
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/auth.py`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_auth.py`

- [ ] **Step 1: Écrire les tests unitaires pour `auth.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_auth.py` :
```python
import json
import os
import tempfile
from unittest.mock import patch, MagicMock
from auth import (
    generate_pkce,
    load_local_token,
    save_token_cache,
    extract_project_id,
    AntigravityAuthManager,
)


def test_generate_pkce():
    verifier, challenge = generate_pkce()
    assert len(verifier) >= 43
    assert len(challenge) >= 43
    assert verifier != challenge


def test_extract_project_id():
    assert extract_project_id({"projectId": "proj-123"}) == "proj-123"
    assert extract_project_id({"cloudaicompanionProject": "proj-456"}) == "proj-456"
    assert extract_project_id({"antigravityProjectId": "proj-789"}) == "proj-789"
    assert extract_project_id({"nested": {"id": "p1"}}) is None


def test_save_and_load_token_cache():
    with tempfile.TemporaryDirectory() as tmpdir:
        cache_path = os.path.join(tmpdir, "antigravity-auth.json")
        data = {
            "access_token": "ya29.test_access",
            "refresh_token": "1//test_refresh",
            "expires_at": 9999999999.0,
            "project_id": "test-project-001",
        }
        save_token_cache(data, cache_path=cache_path)

        loaded = load_local_token(cache_paths=[cache_path])
        assert loaded is not None
        assert loaded["access_token"] == "ya29.test_access"
        assert loaded["project_id"] == "test-project-001"


def test_auth_manager_get_valid_token_cached():
    manager = AntigravityAuthManager(
        credentials={
            "access_token": "ya29.valid_token",
            "refresh_token": "1//refresh",
            "expires_at": 9999999999.0,
            "project_id": "proj-abc",
        }
    )
    token, proj = manager.get_credentials()
    assert token == "ya29.valid_token"
    assert proj == "proj-abc"
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run : `pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_auth.py -v`  
Expected : FAIL (`ModuleNotFoundError: No module named 'auth'`)

- [ ] **Step 3: Implémenter `auth.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/auth.py` :
```python
"""Hybrid Authentication & Project Resolution for Google Antigravity in Hermes Agent."""

from __future__ import annotations

import base64
import hashlib
import json
import logging
import os
import secrets
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

AUTH_URL = "https://accounts.google.com/o/oauth2/v2/auth"
TOKEN_URL = "https://oauth2.googleapis.com/token"
REDIRECT_URI = "http://localhost:51121/oauth-callback"

# Google Antigravity Desktop Client Credentials (proven in pi-antigravity)
DEFAULT_CLIENT_ID = (
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
)
DEFAULT_CLIENT_SECRET = "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"

SCOPES = [
    "https://www.googleapis.com/auth/aicode",
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/cclog",
    "https://www.googleapis.com/auth/experimentsandconfigs",
]

DEFAULT_CACHE_FILE = Path.home() / ".hermes" / "antigravity-auth.json"


def generate_pkce() -> tuple[str, str]:
    """Generate PKCE verifier and S256 challenge."""
    verifier = secrets.token_urlsafe(64)[:128]
    digest = hashlib.sha256(verifier.encode("utf-8")).digest()
    challenge = base64.urlsafe_b64encode(digest).decode("utf-8").rstrip("=")
    return verifier, challenge


def extract_project_id(data: dict[str, Any]) -> str | None:
    """Extract Project ID from dictionary keys."""
    if not isinstance(data, dict):
        return None
    for key in (
        "projectId",
        "antigravityProjectId",
        "cloudaicompanionProject",
        "userDefinedCloudaicompanionProject",
        "project",
    ):
        val = data.get(key)
        if isinstance(val, str) and val.strip():
            return val.strip()
        if isinstance(val, dict) and "id" in val:
            return str(val["id"]).strip()
    return None


def load_local_token(cache_paths: list[str | Path] | None = None) -> dict[str, Any] | None:
    """Check known local token locations on this machine."""
    candidates = cache_paths or [
        DEFAULT_CACHE_FILE,
        Path.home() / ".gemini" / "antigravity-cli" / "antigravity-oauth-token",
        Path.home() / ".pi" / "agent" / "auth.json",
    ]

    # Check env vars first
    env_token = os.environ.get("ANTIGRAVITY_ACCESS_TOKEN") or os.environ.get("GOOGLE_OAUTH_TOKEN")
    if env_token:
        return {
            "access_token": env_token,
            "refresh_token": os.environ.get("ANTIGRAVITY_REFRESH_TOKEN", ""),
            "expires_at": time.time() + 3600,
            "project_id": os.environ.get("ANTIGRAVITY_PROJECT_ID", "antigravity-env-project"),
        }

    for path in candidates:
        p = Path(path).expanduser()
        if not p.is_file():
            continue
        try:
            with open(p, "r", encoding="utf-8") as f:
                content = f.read().strip()
                if not content:
                    continue
                data = json.loads(content)
                if isinstance(data, dict):
                    # Direct format
                    if "access_token" in data or "token" in data:
                        return {
                            "access_token": data.get("access_token") or data.get("token"),
                            "refresh_token": data.get("refresh_token", ""),
                            "expires_at": data.get("expires_at", time.time() + 3600),
                            "project_id": extract_project_id(data) or "antigravity-default",
                        }
                    # Pi auth.json format: {"antigravity": {"token": ...}}
                    if "antigravity" in data and isinstance(data["antigravity"], dict):
                        sub = data["antigravity"]
                        return {
                            "access_token": sub.get("access_token") or sub.get("token"),
                            "refresh_token": sub.get("refresh_token", ""),
                            "expires_at": sub.get("expires_at", time.time() + 3600),
                            "project_id": extract_project_id(sub) or "antigravity-default",
                        }
        except Exception as exc:
            logger.debug("Failed reading credential candidate %s: %s", p, exc)
            continue

    return None


def save_token_cache(data: dict[str, Any], cache_path: Path | str = DEFAULT_CACHE_FILE) -> None:
    """Save credentials to disk with 0600 permissions."""
    p = Path(cache_path).expanduser()
    p.parent.mkdir(parents=True, exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
    os.chmod(p, 0o600)


class _OAuthCallbackHandler(BaseHTTPRequestHandler):
    auth_code: str | None = None
    state: str | None = None

    def do_GET(self) -> None:
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path == "/oauth-callback":
            qs = urllib.parse.parse_qs(parsed.query)
            _OAuthCallbackHandler.auth_code = qs.get("code", [None])[0]
            _OAuthCallbackHandler.state = qs.get("state", [None])[0]

            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(
                b"<html><body><h1>Authentification Antigravity reussie !</h1><p>Vous pouvez fermer cet onglet.</p></body></html>"
            )
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format: str, *args: Any) -> None:
        pass


class AntigravityAuthManager:
    """Manages Google Antigravity OAuth tokens and project resolution."""

    def __init__(
        self,
        credentials: dict[str, Any] | None = None,
        client_id: str = DEFAULT_CLIENT_ID,
        client_secret: str = DEFAULT_CLIENT_SECRET,
    ) -> None:
        self.credentials = credentials or load_local_token()
        self.client_id = os.environ.get("ANTIGRAVITY_CLIENT_ID", client_id)
        self.client_secret = os.environ.get("ANTIGRAVITY_CLIENT_SECRET", client_secret)

    def refresh_access_token(self) -> str:
        """Exchange refresh_token for a new access_token."""
        if not self.credentials or not self.credentials.get("refresh_token"):
            raise ValueError("No refresh_token available for Antigravity provider")

        refresh_token = self.credentials["refresh_token"]
        payload = urllib.parse.urlencode(
            {
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "refresh_token": refresh_token,
                "grant_type": "refresh_token",
            }
        ).encode("utf-8")

        req = urllib.request.Request(
            TOKEN_URL,
            data=payload,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=10.0) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        new_access_token = data["access_token"]
        expires_in = data.get("expires_in", 3600)
        self.credentials["access_token"] = new_access_token
        self.credentials["expires_at"] = time.time() + expires_in - 60
        save_token_cache(self.credentials)
        return new_access_token

    def login_interactive(self) -> dict[str, Any]:
        """Perform interactive OAuth 2.0 PKCE flow."""
        verifier, challenge = generate_pkce()
        state = secrets.token_hex(16)
        params = {
            "client_id": self.client_id,
            "redirect_uri": REDIRECT_URI,
            "response_type": "code",
            "scope": " ".join(SCOPES),
            "code_challenge": challenge,
            "code_challenge_method": "S256",
            "state": state,
            "access_type": "offline",
            "prompt": "consent",
        }
        auth_url = f"{AUTH_URL}?{urllib.parse.urlencode(params)}"

        print("\n=== Antigravity Authentication ===")
        print(f"Ouvrez cette URL dans votre navigateur pour vous connecter :\n\n{auth_url}\n")

        # Try starting local server
        auth_code = None
        try:
            server = HTTPServer(("127.0.0.1", 51121), _OAuthCallbackHandler)
            server.timeout = 120.0
            server.handle_request()
            if _OAuthCallbackHandler.auth_code:
                auth_code = _OAuthCallbackHandler.auth_code
            server.server_close()
        except Exception:
            pass

        if not auth_code:
            auth_code = input("Collez ici le code d'autorisation (ou URL redirigée) : ").strip()
            if "code=" in auth_code:
                parsed = urllib.parse.urlparse(auth_code)
                auth_code = urllib.parse.parse_qs(parsed.query).get("code", [auth_code])[0]

        # Exchange code for tokens
        token_payload = urllib.parse.urlencode(
            {
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "code": auth_code,
                "code_verifier": verifier,
                "grant_type": "authorization_code",
                "redirect_uri": REDIRECT_URI,
            }
        ).encode("utf-8")

        req = urllib.request.Request(
            TOKEN_URL,
            data=token_payload,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=10.0) as resp:
            token_data = json.loads(resp.read().decode("utf-8"))

        creds = {
            "access_token": token_data["access_token"],
            "refresh_token": token_data.get("refresh_token", ""),
            "expires_at": time.time() + token_data.get("expires_in", 3600) - 60,
            "project_id": "antigravity-default",
        }

        # Resolve projectId via loadCodeAssist
        try:
            proj = self.discover_project_id(creds["access_token"])
            if proj:
                creds["project_id"] = proj
        except Exception:
            pass

        self.credentials = creds
        save_token_cache(creds)
        print("Authentification Antigravity enregistrée avec succès !\n")
        return creds

    def discover_project_id(self, token: str) -> str | None:
        """Call /v1internal:loadCodeAssist to discover the user's project ID."""
        req = urllib.request.Request(
            "https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist",
            data=json.dumps({"metadata": {"ideType": "ANTIGRAVITY"}}).encode("utf-8"),
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "User-Agent": DEFAULT_USER_AGENT,
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=8.0) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                return extract_project_id(data)
        except Exception as exc:
            logger.debug("loadCodeAssist discovery error: %s", exc)
            return None

    def get_credentials(self) -> tuple[str, str]:
        """Return (access_token, project_id), auto-refreshing if expired."""
        if not self.credentials or not self.credentials.get("access_token"):
            self.credentials = self.login_interactive()

        # Check expiration (< 5 min)
        if time.time() > self.credentials.get("expires_at", 0) - 300:
            if self.credentials.get("refresh_token"):
                try:
                    self.refresh_access_token()
                except Exception as exc:
                    logger.warning("Token refresh failed: %s, falling back to cached token", exc)

        return self.credentials["access_token"], self.credentials.get(
            "project_id", "antigravity-default"
        )
```

- [ ] **Step 4: Relancer les tests pour vérifier qu'ils passent**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_auth.py -v`  
Expected : PASS (4 tests passed)

---

### Task 3: Traducteur Bidirectionnel & Parser de Flux SSE (`translator.py`)

**Files:**
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/translator.py`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_translator.py`

- [ ] **Step 1: Écrire les tests unitaires pour `translator.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_translator.py` :
```python
import json
from translator import (
    to_antigravity_payload,
    parse_sse_event,
    ChatCompletionChunk,
)


def test_to_antigravity_payload_messages_and_tools():
    messages = [
        {"role": "system", "content": "You are a helpful coding agent."},
        {"role": "user", "content": "Write hello world in python"},
        {
            "role": "assistant",
            "content": None,
            "tool_calls": [
                {
                    "id": "call_123",
                    "type": "function",
                    "function": {"name": "run_shell", "arguments": '{"cmd": "echo 1"}'},
                }
            ],
        },
        {
            "role": "tool",
            "tool_call_id": "call_123",
            "content": "1\n",
        },
    ]
    tools = [
        {
            "type": "function",
            "function": {
                "name": "run_shell",
                "description": "Execute a shell command",
                "parameters": {
                    "type": "object",
                    "properties": {"cmd": {"type": "string"}},
                    "required": ["cmd"],
                },
            },
        }
    ]

    payload = to_antigravity_payload(
        model="gemini-3.8-flash",
        messages=messages,
        project_id="proj-test",
        tools=tools,
        reasoning_effort="medium",
    )

    assert payload["project"] == "proj-test"
    assert payload["model"] == "gemini-3.8-flash"
    assert "request" in payload

    req = payload["request"]
    assert "systemInstruction" in req
    assert req["systemInstruction"]["parts"][0]["text"] == "You are a helpful coding agent."

    contents = req["contents"]
    assert len(contents) == 3
    assert contents[0]["role"] == "user"
    assert contents[1]["role"] == "model"
    assert "functionCall" in contents[1]["parts"][0]
    assert contents[1]["parts"][0]["functionCall"]["name"] == "run_shell"
    assert contents[2]["role"] == "user"
    assert "functionResponse" in contents[2]["parts"][0]

    assert "tools" in req
    assert len(req["tools"][0]["functionDeclarations"]) == 1
    assert req["tools"][0]["functionDeclarations"][0]["name"] == "run_shell"


def test_parse_sse_event_text_and_thinking():
    data = {
        "response": {
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {"text": "Thinking step...", "thought": True},
                            {"text": "Final answer."},
                        ]
                    },
                    "finishReason": "STOP",
                }
            ]
        }
    }
    raw_event = json.dumps(data)
    chunks = parse_sse_event(raw_event, model_id="gemini-3.8-flash")
    assert len(chunks) == 2

    # Thinking chunk
    assert chunks[0].choices[0]["delta"].get("reasoning_content") == "Thinking step..."
    assert "content" not in chunks[0].choices[0]["delta"]

    # Text chunk
    assert chunks[1].choices[0]["delta"].get("content") == "Final answer."
    assert chunks[1].choices[0].get("finish_reason") == "stop"


def test_parse_sse_event_function_call():
    data = {
        "response": {
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {
                                "functionCall": {
                                    "id": "call_abc",
                                    "name": "bash",
                                    "args": {"command": "ls"},
                                }
                            }
                        ]
                    },
                    "finishReason": "STOP",
                }
            ]
        }
    }
    raw_event = json.dumps(data)
    chunks = parse_sse_event(raw_event, model_id="gemini-3.8-flash")
    assert len(chunks) == 1
    delta = chunks[0].choices[0]["delta"]
    assert "tool_calls" in delta
    tc = delta["tool_calls"][0]
    assert tc["function"]["name"] == "bash"
    assert json.loads(tc["function"]["arguments"]) == {"command": "ls"}
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run : `pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_translator.py -v`  
Expected : FAIL (`ModuleNotFoundError: No module named 'translator'`)

- [ ] **Step 3: Implémenter `translator.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/translator.py` :
```python
"""Bidirectional translator between Hermes OpenAI structures and Google Antigravity payloads."""

from __future__ import annotations

import json
import uuid
from typing import Any
from models import get_thinking_config


class ChatCompletionChunk:
    """Mock OpenAI ChatCompletionChunk compatible with Hermes AIAgent."""

    def __init__(
        self,
        chunk_id: str,
        model: str,
        choices: list[dict[str, Any]],
        usage: dict[str, Any] | None = None,
    ) -> None:
        self.id = chunk_id
        self.object = "chat.completion.chunk"
        self.model = model
        self.choices = choices
        self.usage = usage

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "object": self.object,
            "model": self.model,
            "choices": self.choices,
            "usage": self.usage,
        }


def to_antigravity_payload(
    model: str,
    messages: list[dict[str, Any]],
    project_id: str,
    tools: list[dict[str, Any]] | None = None,
    tool_choice: Any = None,
    temperature: float | None = None,
    max_tokens: int | None = None,
    reasoning_effort: str | None = None,
    session_id: str | None = None,
) -> dict[str, Any]:
    """Convert Hermes messages and parameters into Google Antigravity wire payload."""
    contents: list[dict[str, Any]] = []
    system_parts: list[dict[str, str]] = []

    # Map tool_call_id to function name for tool responses
    tool_call_names: dict[str, str] = {}

    for msg in messages:
        role = msg.get("role")
        content = msg.get("content")

        if role in ("system", "developer"):
            if isinstance(content, str) and content.strip():
                system_parts.append({"text": content})
            continue

        if role == "user":
            parts: list[dict[str, Any]] = []
            if isinstance(content, str):
                parts.append({"text": content})
            elif isinstance(content, list):
                for item in content:
                    if isinstance(item, str):
                        parts.append({"text": item})
                    elif isinstance(item, dict):
                        if item.get("type") == "text":
                            parts.append({"text": item.get("text", "")})
                        elif item.get("type") == "image_url":
                            url = item.get("image_url", {}).get("url", "")
                            if url.startswith("data:"):
                                mime_data = url.split(";base64,")
                                mime = mime_data[0].replace("data:", "")
                                b64 = mime_data[1] if len(mime_data) > 1 else ""
                                parts.append({"inlineData": {"mimeType": mime, "data": b64}})
            contents.append({"role": "user", "parts": parts or [{"text": ""}]})

        elif role == "assistant":
            parts = []
            if content and isinstance(content, str):
                parts.append({"text": content})

            tool_calls = msg.get("tool_calls") or []
            for tc in tool_calls:
                fn = tc.get("function", {})
                fn_name = fn.get("name", "tool")
                tc_id = tc.get("id", f"call_{uuid.uuid4().hex[:8]}")
                tool_call_names[tc_id] = fn_name

                raw_args = fn.get("arguments", "{}")
                args_dict = json.loads(raw_args) if isinstance(raw_args, str) else raw_args
                parts.append(
                    {
                        "functionCall": {
                            "id": tc_id,
                            "name": fn_name,
                            "args": args_dict or {},
                        }
                    }
                )
            contents.append({"role": "model", "parts": parts or [{"text": ""}]})

        elif role == "tool":
            tc_id = msg.get("tool_call_id", "")
            fn_name = tool_call_names.get(tc_id, "tool")
            res_content = content if isinstance(content, str) else json.dumps(content)
            contents.append(
                {
                    "role": "user",
                    "parts": [
                        {
                            "functionResponse": {
                                "name": fn_name,
                                "response": {"content": res_content},
                            }
                        }
                    ],
                }
            )

    request_body: dict[str, Any] = {"contents": contents}

    if system_parts:
        request_body["systemInstruction"] = {
            "role": "user",
            "parts": system_parts,
        }

    # Generation config
    gen_config: dict[str, Any] = {}
    if temperature is not None:
        gen_config["temperature"] = temperature
    if max_tokens is not None:
        gen_config["maxOutputTokens"] = max_tokens

    thinking = get_thinking_config(model, reasoning_effort)
    if thinking:
        gen_config["thinkingConfig"] = thinking

    if gen_config:
        request_body["generationConfig"] = gen_config

    # Tools conversion
    if tools:
        declarations = []
        for t in tools:
            if t.get("type") == "function":
                fn = t.get("function", {})
                declarations.append(
                    {
                        "name": fn.get("name"),
                        "description": fn.get("description", ""),
                        "parameters": fn.get("parameters", {}),
                    }
                )
        if declarations:
            request_body["tools"] = [{"functionDeclarations": declarations}]

    req_id = f"{uuid.uuid4().hex[:12]}-0"
    sess_id = session_id or f"sess-{uuid.uuid4().hex[:16]}"

    return {
        "project": project_id,
        "model": model,
        "request": request_body,
        "requestType": "agent",
        "userAgent": "antigravity",
        "requestId": req_id,
        "sessionId": sess_id,
    }


def parse_sse_event(json_str: str, model_id: str) -> list[ChatCompletionChunk]:
    """Parse one SSE data JSON payload into ChatCompletionChunk objects."""
    chunks: list[ChatCompletionChunk] = []
    try:
        data = json.loads(json_str)
    except Exception:
        return chunks

    resp = data.get("response") or data
    candidates = resp.get("candidates") or []
    if not candidates:
        return chunks

    candidate = candidates[0]
    parts = candidate.get("content", {}).get("parts", [])
    finish_reason_raw = candidate.get("finishReason")
    finish_reason = (
        "stop"
        if finish_reason_raw == "STOP"
        else "length"
        if finish_reason_raw == "MAX_TOKENS"
        else None
    )

    chunk_id = f"chatcmpl-{uuid.uuid4().hex[:12]}"

    for idx, part in enumerate(parts):
        delta: dict[str, Any] = {}
        is_last = idx == len(parts) - 1

        if "text" in part:
            if part.get("thought") is True:
                delta["reasoning_content"] = part["text"]
            else:
                delta["content"] = part["text"]

        if "functionCall" in part:
            fc = part["functionCall"]
            tc_id = fc.get("id") or f"call_{uuid.uuid4().hex[:8]}"
            args_str = (
                json.dumps(fc.get("args", {}))
                if isinstance(fc.get("args"), dict)
                else str(fc.get("args", "{}"))
            )
            delta["tool_calls"] = [
                {
                    "index": 0,
                    "id": tc_id,
                    "type": "function",
                    "function": {
                        "name": fc.get("name", ""),
                        "arguments": args_str,
                    },
                }
            ]
            if is_last and not finish_reason:
                finish_reason = "tool_calls"

        if delta:
            choice = {
                "index": 0,
                "delta": delta,
                "finish_reason": finish_reason if is_last else None,
            }
            chunks.append(ChatCompletionChunk(chunk_id=chunk_id, model=model_id, choices=[choice]))

    return chunks
```

- [ ] **Step 4: Relancer les tests pour vérifier qu'ils passent**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_translator.py -v`  
Expected : PASS (3 tests passed)

---

### Task 4: Client Adaptateur & Transport Réseau (`client.py`)

**Files:**
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/client.py`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_client.py`

- [ ] **Step 1: Écrire les tests unitaires pour `client.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_client.py` :
```python
from unittest.mock import patch, MagicMock
from client import AntigravityClient


def test_client_chat_completions_streaming_generator():
    mock_auth = MagicMock()
    mock_auth.get_credentials.return_value = ("fake-token", "fake-project")

    sse_lines = [
        b'data: {"response": {"candidates": [{"content": {"parts": [{"text": "Hello"}]}}]}}\n\n',
        b'data: {"response": {"candidates": [{"content": {"parts": [{"text": " world"}]},"finishReason": "STOP"]}}\n\n',
        b"data: [DONE]\n\n",
    ]

    mock_resp = MagicMock()
    mock_resp.__iter__.return_value = sse_lines
    mock_resp.status = 200

    with patch("urllib.request.urlopen", return_value=mock_resp):
        client = AntigravityClient(auth_manager=mock_auth)
        stream = client.chat.completions.create(
            model="gemini-3.8-flash",
            messages=[{"role": "user", "content": "hi"}],
            stream=True,
        )
        chunks = list(stream)
        assert len(chunks) == 2
        assert chunks[0].choices[0]["delta"]["content"] == "Hello"
        assert chunks[1].choices[0]["delta"]["content"] == " world"
        assert chunks[1].choices[0]["finish_reason"] == "stop"
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run : `pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_client.py -v`  
Expected : FAIL (`ModuleNotFoundError: No module named 'client'`)

- [ ] **Step 3: Implémenter `client.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/client.py` :
```python
"""Antigravity HTTP client adapter matching OpenAI client contract in Hermes AIAgent."""

from __future__ import annotations

import json
import logging
import urllib.request
from typing import Any, Iterator
from auth import AntigravityAuthManager
from models import ENDPOINT_FALLBACKS, DEFAULT_USER_AGENT, resolve_runtime_model
from translator import to_antigravity_payload, parse_sse_event, ChatCompletionChunk

logger = logging.getLogger(__name__)


class _CompletionsAdapter:
    def __init__(self, client: AntigravityClient) -> None:
        self._client = client

    def create(
        self,
        model: str,
        messages: list[dict[str, Any]],
        stream: bool = True,
        tools: list[dict[str, Any]] | None = None,
        tool_choice: Any = None,
        temperature: float | None = None,
        max_tokens: int | None = None,
        **kwargs: Any,
    ) -> Iterator[ChatCompletionChunk] | Any:
        return self._client.generate(
            model=model,
            messages=messages,
            stream=stream,
            tools=tools,
            tool_choice=tool_choice,
            temperature=temperature,
            max_tokens=max_tokens,
            **kwargs,
        )


class _ChatAdapter:
    def __init__(self, client: AntigravityClient) -> None:
        self.completions = _CompletionsAdapter(client)


class AntigravityClient:
    """Client implementing the client.chat.completions interface for Hermes Agent."""

    def __init__(
        self,
        auth_manager: AntigravityAuthManager | None = None,
        endpoints: list[str] | None = None,
    ) -> None:
        self.auth = auth_manager or AntigravityAuthManager()
        self.endpoints = endpoints or ENDPOINT_FALLBACKS
        self.chat = _ChatAdapter(self)

    def generate(
        self,
        model: str,
        messages: list[dict[str, Any]],
        stream: bool = True,
        tools: list[dict[str, Any]] | None = None,
        tool_choice: Any = None,
        temperature: float | None = None,
        max_tokens: int | None = None,
        **kwargs: Any,
    ) -> Iterator[ChatCompletionChunk]:
        runtime_model = resolve_runtime_model(model)
        token, project_id = self.auth.get_credentials()

        reasoning_effort = kwargs.get("reasoning_effort")
        if not reasoning_effort and isinstance(kwargs.get("extra_body"), dict):
            reasoning_effort = kwargs["extra_body"].get("reasoning_effort")

        payload = to_antigravity_payload(
            model=runtime_model,
            messages=messages,
            project_id=project_id,
            tools=tools,
            tool_choice=tool_choice,
            temperature=temperature,
            max_tokens=max_tokens,
            reasoning_effort=reasoning_effort,
        )

        body_bytes = json.dumps(payload).encode("utf-8")

        response = None
        last_error = None

        for endpoint in self.endpoints:
            url = f"{endpoint}/v1internal:streamGenerateContent?alt=sse"
            req = urllib.request.Request(
                url,
                data=body_bytes,
                headers={
                    "Authorization": f"Bearer {token}",
                    "Content-Type": "application/json",
                    "User-Agent": DEFAULT_USER_AGENT,
                },
                method="POST",
            )
            try:
                response = urllib.request.urlopen(req, timeout=30.0)
                break
            except urllib.error.HTTPError as err:
                last_error = err
                if err.code == 401:
                    # Token expired -> refresh once and retry
                    try:
                        token = self.auth.refresh_access_token()
                        req.add_header("Authorization", f"Bearer {token}")
                        response = urllib.request.urlopen(req, timeout=30.0)
                        break
                    except Exception as refresh_exc:
                        last_error = refresh_exc
                        continue
                elif err.code in (404, 500, 502, 503, 504):
                    # Candidate endpoint unavailable -> failover
                    continue
                else:
                    raise RuntimeError(f"Antigravity API HTTP {err.code}: {err.read().decode('utf-8', errors='replace')}") from err
            except Exception as exc:
                last_error = exc
                continue

        if response is None:
            raise RuntimeError(f"Antigravity request failed on all endpoints: {last_error}")

        def sse_generator() -> Iterator[ChatCompletionChunk]:
            with response:
                for line in response:
                    decoded = line.decode("utf-8").strip()
                    if not decoded.startswith("data:"):
                        continue
                    payload_part = decoded[len("data:") :].strip()
                    if not payload_part or payload_part == "[DONE]":
                        continue
                    chunks = parse_sse_event(payload_part, model_id=runtime_model)
                    for c in chunks:
                        yield c

        return sse_generator()
```

- [ ] **Step 4: Relancer les tests pour vérifier qu'ils passent**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_client.py -v`  
Expected : PASS (1 test passed)

---

### Task 5: Enregistrement du ProviderProfile (`__init__.py`)

**Files:**
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/__init__.py`
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_provider_registration.py`

- [ ] **Step 1: Écrire le test d'intégration pour Hermes Agent**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/tests/test_provider_registration.py` :
```python
import sys
import pytest

# Ensure Hermes agent is in sys.path
sys.path.insert(0, "/home/user/.hermes/hermes-agent")

from providers import get_provider_profile, list_providers
import __init__ as antigravity_module


def test_antigravity_profile_registered():
    profile = get_provider_profile("antigravity")
    assert profile is not None
    assert profile.name == "antigravity"
    assert "google-antigravity" in profile.aliases
    assert "agy" in profile.aliases
    assert profile.supports_vision is True
    assert profile.default_aux_model == "gemini-3.8-flash"


def test_antigravity_create_client_hook():
    profile = get_provider_profile("antigravity")
    client = profile.create_client()
    assert client is not None
    assert hasattr(client, "chat")
    assert hasattr(client.chat, "completions")
    assert hasattr(client.chat.completions, "create")
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity:/home/user/.hermes/hermes-agent pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_provider_registration.py -v`  
Expected : FAIL (`__init__.py is empty or not defining profile`)

- [ ] **Step 3: Implémenter `__init__.py`**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/__init__.py` :
```python
"""Google Antigravity inference provider plugin for Hermes Agent."""

from __future__ import annotations

import logging
from typing import Any

from providers import register_provider
from providers.base import ProviderProfile

from .auth import AntigravityAuthManager
from .client import AntigravityClient
from .models import FALLBACK_MODELS, fetch_available_models

logger = logging.getLogger(__name__)


class AntigravityProfile(ProviderProfile):
    """Profile for Google Antigravity / Cloud Code Assist."""

    def create_client(self, **client_kwargs: Any) -> Any:
        """Instantiate and return AntigravityClient for Hermes AIAgent."""
        auth_mgr = AntigravityAuthManager()
        return AntigravityClient(auth_manager=auth_mgr)

    def fetch_models(
        self,
        *,
        api_key: str | None = None,
        base_url: str | None = None,
        timeout: float = 8.0,
    ) -> list[str]:
        """Fetch available models dynamically from Antigravity API or return fallbacks."""
        auth_mgr = AntigravityAuthManager()
        try:
            token, project_id = auth_mgr.get_credentials()
            return fetch_available_models(token, project_id, timeout=timeout)
        except Exception as exc:
            logger.debug("fetch_models failed: %s", exc)
            return list(FALLBACK_MODELS)


antigravity = AntigravityProfile(
    name="antigravity",
    aliases=("google-antigravity", "agy", "cloudcode", "google-code-assist"),
    display_name="Google Antigravity",
    description="Google Antigravity / Cloud Code Assist (Gemini 2.5/3.x, Claude Sonnet)",
    signup_url="https://antigravity.google/",
    env_vars=("ANTIGRAVITY_ACCESS_TOKEN", "ANTIGRAVITY_REFRESH_TOKEN", "GOOGLE_OAUTH_TOKEN"),
    base_url="https://daily-cloudcode-pa.googleapis.com",
    auth_type="oauth_external",
    supports_vision=True,
    supports_vision_tool_messages=True,
    default_max_tokens=65536,
    default_aux_model="gemini-3.8-flash",
    fallback_models=FALLBACK_MODELS,
)

register_provider(antigravity)
```

- [ ] **Step 4: Relancer les tests pour vérifier qu'ils passent**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity:/home/user/.hermes/hermes-agent pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/test_provider_registration.py -v`  
Expected : PASS (2 tests passed)

---

### Task 6: Documentation & Validation Système Complète

**Files:**
- Create: `/home/user/.hermes/plugins/model-providers/antigravity/README.md`

- [ ] **Step 1: Rédiger le fichier `README.md` du plugin**

Créer `/home/user/.hermes/plugins/model-providers/antigravity/README.md` documentant :
- Présentation du provider Antigravity
- Prise en charge des modèles (`gemini-3.8-flash`, `claude-3-7-sonnet`, etc.)
- Configuration des variables d'environnement (`ANTIGRAVITY_ACCESS_TOKEN`, `ANTIGRAVITY_CLIENT_ID`, etc.)
- Utilisation avec Hermes CLI (`hermes --provider antigravity --model gemini-3.8-flash`)
- Dépannage et gestion des quotas.

- [ ] **Step 2: Exécuter la suite complète de tests unitaires du plugin**

Run : `PYTHONPATH=/home/user/.hermes/plugins/model-providers/antigravity:/home/user/.hermes/hermes-agent pytest /home/user/.hermes/plugins/model-providers/antigravity/tests/ -v`  
Expected : PASS (14 tests passed)

- [ ] **Step 3: Vérifier la reconnaissance du plugin par Hermes CLI**

Run : `python3 -c "import sys; sys.path.insert(0, '/home/user/.hermes/hermes-agent'); from providers import list_providers; print([p.name for p in list_providers() if p.name == 'antigravity'])"`  
Expected : Output `['antigravity']`
