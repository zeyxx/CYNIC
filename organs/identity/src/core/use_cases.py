from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from .entities import Credential, CredentialAttestation, SmokecheckResult
from .ports import CredentialStorePort, SmokecheckPort


# ---------------------------------------------------------------------------
# Known credential groups (providers)
# ---------------------------------------------------------------------------

KNOWN_GROUPS: dict[str, list[str]] = {
    "porkbun": ["PORKBUN_API_KEY", "PORKBUN_SECRET_API_KEY"],
    "cloudflare": ["CF_API_TOKEN", "CF_ACCOUNT_ID"],
    "kernel": ["CYNIC_API_KEY", "CYNIC_REST_ADDR"],
    "surrealdb": ["SURREALDB_URL", "SURREALDB_PASS"],
    "solana": ["SOLANA_RPC_URL", "HELIUS_API_KEY"],
    "telegram": ["CYNIC_TELEGRAM_BOT_TOKEN", "CYNIC_TELEGRAM_CHAT_ID"],
    "slack": ["SLACK_BOT_TOKEN"],
}


@dataclass
class OperationResult:
    ok: bool
    message: str
    details: dict = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Use case: ListCredentials
# ---------------------------------------------------------------------------

class ListCredentialsUseCase:
    """
    Non-mutating. Returns attestations for all keys — never values.
    Safe for agents to call via MCP.
    """

    def __init__(self, store: CredentialStorePort) -> None:
        self._store = store

    def execute(self, group: Optional[str] = None) -> list[CredentialAttestation]:
        keys_to_check = (
            KNOWN_GROUPS.get(group, []) if group else self._store.list_keys()
        )
        result = []
        for key in keys_to_check:
            cred = self._store.read_key(key)
            if cred:
                result.append(CredentialAttestation(
                    key=key,
                    present=True,
                    shape=cred.shape,
                    length=cred.length,
                    commitment=cred.commitment,
                    commitment_schema=cred.commitment_schema,
                    source=self._store.env_file_path(),
                ))
            else:
                result.append(CredentialAttestation(key=key, present=False))
        return result


# ---------------------------------------------------------------------------
# Use case: AttestCredentials
# ---------------------------------------------------------------------------

class AttestCredentialsUseCase:
    """
    Non-mutating. Returns a structured attestation for all known groups.
    This is what MCP tools surface to agents — presence + commitment, no values.
    """

    def __init__(self, store: CredentialStorePort) -> None:
        self._store = store

    def execute(self) -> dict:
        groups: dict[str, list[dict]] = {}
        all_present = True

        for group_name, keys in KNOWN_GROUPS.items():
            attestations = []
            for key in keys:
                cred = self._store.read_key(key)
                present = cred is not None
                if not present:
                    all_present = False
                attestations.append({
                    "key": key,
                    "present": present,
                    "shape": cred.shape if cred else None,
                    "commitment": cred.commitment if cred else None,
                    "commitment_schema": cred.commitment_schema if cred else None,
                })
            groups[group_name] = attestations

        return {
            "schema": "cynic.identity.attestation.v1",
            "source": self._store.env_file_path(),
            "all_known_present": all_present,
            "groups": groups,
        }


# ---------------------------------------------------------------------------
# Use case: AddCredential
# ---------------------------------------------------------------------------

class AddCredentialUseCase:
    """
    Mutating — human CLI only.
    Writes a credential to the env file.
    The value is read from the caller (stdin, not argv) to avoid shell history exposure.
    """

    def __init__(self, store: CredentialStorePort) -> None:
        self._store = store

    def execute(self, key: str, value: str) -> OperationResult:
        if not key or not key.replace("_", "").isalnum():
            return OperationResult(ok=False, message=f"Invalid key name: {key}")
        if not value:
            return OperationResult(ok=False, message="Value cannot be empty")

        cred = Credential(key=key, value=value)
        ok = self._store.write_key(cred)
        return OperationResult(
            ok=ok,
            message=f"{'Written' if ok else 'Failed to write'}: {key}",
            details={"key": key, "shape": cred.shape, "length": cred.length},
        )


# ---------------------------------------------------------------------------
# Use case: RemoveCredential
# ---------------------------------------------------------------------------

class RemoveCredentialUseCase:
    """Mutating — human CLI only. Removes a key from the env file."""

    def __init__(self, store: CredentialStorePort) -> None:
        self._store = store

    def execute(self, key: str) -> OperationResult:
        existing = self._store.read_key(key)
        if existing is None:
            return OperationResult(ok=False, message=f"Key not found: {key}")
        ok = self._store.delete_key(key)
        return OperationResult(
            ok=ok,
            message=f"{'Removed' if ok else 'Failed to remove'}: {key}",
        )


# ---------------------------------------------------------------------------
# Use case: SmokeCheck
# ---------------------------------------------------------------------------

class SmokeCheckUseCase:
    """
    Non-mutating (read-only API calls).
    Verifies that credentials are not just present but actually valid.
    Returns SmokecheckResult per provider — no credential values in output.
    """

    def __init__(self, store: CredentialStorePort, smokecheck: SmokecheckPort) -> None:
        self._store = store
        self._smokecheck = smokecheck

    def execute(self, providers: Optional[list[str]] = None) -> list[SmokecheckResult]:
        targets = providers or list(KNOWN_GROUPS.keys())
        results = []

        if "porkbun" in targets:
            api_key = self._store.read_key("PORKBUN_API_KEY")
            secret = self._store.read_key("PORKBUN_SECRET_API_KEY")
            if api_key and secret:
                results.append(self._smokecheck.check_porkbun(api_key.value, secret.value))
            else:
                results.append(SmokecheckResult(
                    provider="porkbun", ok=False,
                    message="Missing: PORKBUN_API_KEY or PORKBUN_SECRET_API_KEY",
                ))

        if "cloudflare" in targets:
            token = self._store.read_key("CF_API_TOKEN")
            if token:
                results.append(self._smokecheck.check_cloudflare(token.value))
            else:
                results.append(SmokecheckResult(
                    provider="cloudflare", ok=False,
                    message="Missing: CF_API_TOKEN",
                ))

        return results
