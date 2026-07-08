"""Domain orchestration for Freebox router authorization and port forwarding."""

from __future__ import annotations

from dataclasses import dataclass
import time
from typing import Callable, Final

from .config import EdgeExposureEvidence, FreeboxConfig, PortForwardIntent
from .ports import FileTokenStore, FreeboxProvider


AuthorizationOutput = dict[str, object]
OperationOutput = str
PrintFn = Callable[[str], None]
SleepFn = Callable[[float], None]


@dataclass(frozen=True)
class FreeboxRuntime:
    """Orchestrates provider and token persistence through the desired domain flow."""

    provider: FreeboxProvider
    token_store: FileTokenStore
    config: FreeboxConfig
    poll_interval: float = 2.0
    printer: PrintFn = print
    sleep: SleepFn = time.sleep

    def authorize(self) -> str | None:
        """Request authorization and block until grant/deny state."""

        self.printer("Demande d'autorisation à la Freebox...")
        response = self.provider.request_authorization(self.config.authorization_payload())
        if not isinstance(response, dict) or not response.get("success"):
            self.printer(f"Erreur: {response}")
            return None

        result = response.get("result")
        if not isinstance(result, dict):
            self.printer(f"Réponse d'autorisation invalide: {response}")
            return None

        app_token = result.get("app_token")
        track_id = result.get("track_id")
        if not isinstance(app_token, str) or not isinstance(track_id, str):
            self.printer(f"Réponse d'autorisation incomplète: {response}")
            return None

        self.printer("\n👉 VEUILLEZ VALIDER SUR L'ECRAN DE LA FREEBOX (Fleche Droite / Oui)")
        status = self._poll_authorization_status(track_id)
        if status != "granted":
            return None

        self.token_store.write_token(app_token)
        self.printer("✅ Autorisation accordée !")
        return app_token

    def _poll_authorization_status(self, track_id: str) -> str:
        while True:
            status = self.provider.read_authorization_status(track_id)
            if status == "granted":
                return status
            if status != "pending":
                self.printer(f"❌ Autorisation refusée ou expirée : {status}")
                return status
            self.sleep(self.poll_interval)

    def login(self, token: str | None = None) -> str | None:
        """Return a valid Freebox session token."""

        token_override = token or self.token_store.read_token()
        if token_override is None:
            token_override = self.authorize()
            if token_override is None:
                return None

        challenge = self.provider.get_challenge()
        password = self._build_password(token_override, challenge)
        return self.provider.request_session_token(self.config.app_id, password)

    def _build_password(self, token: str, challenge: str) -> str:
        import hashlib
        import hmac

        return hmac.new(token.encode(), challenge.encode(), hashlib.sha1).hexdigest()

    def open_port(
        self,
        session_token: str,
        intent: PortForwardIntent,
        approved: bool = False,
        evidence: EdgeExposureEvidence | None = None,
    ) -> str:
        """Request an explicitly approved port mapping and return response text."""

        if not approved:
            return "blocked: explicit router mutation approval required"
        if evidence is None or not evidence.approved:
            return "blocked: registry/Synapse edge exposure evidence required"

        result = self.provider.request_port_open(session_token, intent.to_payload())
        text = result.get("status") if isinstance(result, dict) else str(result)
        return str(text) if text is not None else "no-response"

    def run_bridge(
        self,
        intent: PortForwardIntent | None = None,
        approved: bool = False,
        evidence: EdgeExposureEvidence | None = None,
    ) -> bool:
        """Backward-compatible orchestration for explicit router mutation."""

        if intent is None:
            self.printer("No port-forward intent supplied; refusing router mutation.")
            return False
        if not approved:
            self.printer("Explicit router mutation approval required.")
            return False
        if evidence is None or not evidence.approved:
            self.printer("Registry/Synapse edge exposure evidence required.")
            return False

        session_token = self.login()
        if session_token is None:
            return False

        response = self.open_port(session_token, intent, approved=True, evidence=evidence)
        self.printer(f"Port opening response: {response}")
        return not response.startswith("blocked:")


def build_default_runtime(
    config: FreeboxConfig | None = None,
    printer: PrintFn = print,
    sleep: SleepFn = time.sleep,
) -> FreeboxRuntime:
    """Build a default runtime with configured filesystem token storage."""

    cfg = config or FreeboxConfig()
    return FreeboxRuntime(
        provider=NullProvider(cfg.freebox_url),
        token_store=FileTokenStore(cfg.token_file),
        config=cfg,
        printer=printer,
        sleep=sleep,
    )


class NullProvider:
    """Fallback provider used in tests to catch missing adapter wiring."""

    def __init__(self, base_url: str) -> None:
        self.base_url: Final[str] = base_url

    def request_authorization(self, payload: dict[str, str]) -> AuthorizationOutput:  # pragma: no cover - guardrail
        raise RuntimeError(f"Adapter not injected; attempt against {self.base_url}")

    def read_authorization_status(self, track_id: str) -> str:  # pragma: no cover
        raise RuntimeError(f"Adapter not injected; attempt against {self.base_url}")

    def get_challenge(self) -> str:  # pragma: no cover
        raise RuntimeError(f"Adapter not injected; attempt against {self.base_url}")

    def request_session_token(self, app_id: str, password: str) -> str:  # pragma: no cover
        raise RuntimeError(f"Adapter not injected; attempt against {self.base_url}")

    def request_port_open(self, session_token: str, payload: dict[str, object]) -> AuthorizationOutput:  # pragma: no cover
        raise RuntimeError(f"Adapter not injected; attempt against {self.base_url}")
