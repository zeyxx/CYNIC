from __future__ import annotations

from pathlib import Path

from organs.gasdf.src.adapters.runtime_membrane import OrganOwnedRuntimeMembrane
from organs.gasdf.src.core.policy import evaluate_policy


def test_runtime_membrane_is_owned_and_non_signing():
    evidence = OrganOwnedRuntimeMembrane(Path("organs/gasdf")).evidence()

    assert evidence.runtime_owner == "organ-gasdf"
    assert not evidence.external_checkout_required
    assert not evidence.signer_adapter_present


def test_policy_does_not_enable_signing_without_signer_adapter():
    status = evaluate_policy(
        has_fee_payer_env=True,
        runtime_owned_by_organ=True,
        signer_adapter_present=False,
        lifecycle_approved=True,
    )

    assert not status.can_sign
    assert status.runtime_owned_by_organ
    assert "signer adapter" in status.reason
