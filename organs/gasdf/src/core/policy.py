"""Safety policy for the GASdf gas delegation surface."""

from __future__ import annotations

from dataclasses import dataclass


REQUIRED_GATES = (
    "no_committed_fee_payer_secret",
    "registry_lifecycle_authority",
    "human_approval_for_signing_runtime",
)


@dataclass(frozen=True)
class GasdfPolicyStatus:
    can_sign: bool
    required_gates: tuple[str, ...]
    reason: str
    runtime_owned_by_organ: bool


def evaluate_policy(
    *,
    has_fee_payer_env: bool,
    runtime_owned_by_organ: bool,
    signer_adapter_present: bool,
    lifecycle_approved: bool,
) -> GasdfPolicyStatus:
    if not runtime_owned_by_organ:
        return GasdfPolicyStatus(
            False,
            REQUIRED_GATES,
            "runtime is still outside the organ membrane",
            runtime_owned_by_organ=False,
        )
    if not signer_adapter_present:
        return GasdfPolicyStatus(
            False,
            REQUIRED_GATES,
            "organ-owned runtime is present, but signer adapter is intentionally absent",
            runtime_owned_by_organ=True,
        )
    if not has_fee_payer_env:
        return GasdfPolicyStatus(
            False,
            REQUIRED_GATES,
            "fee payer credential is absent from operator environment",
            runtime_owned_by_organ=True,
        )
    if not lifecycle_approved:
        return GasdfPolicyStatus(
            False,
            REQUIRED_GATES,
            "signing remains disabled until registry lifecycle approval is explicit",
            runtime_owned_by_organ=True,
        )
    return GasdfPolicyStatus(
        False,
        REQUIRED_GATES,
        "signing remains disabled until a dedicated signer adapter is implemented and audited",
        runtime_owned_by_organ=True,
    )
