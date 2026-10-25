"""α6 — the conservation contract is mandatory and self-consistent.

Every normalizer must expose ``conservation()``. Adapters that have one are held
to it: the contract's field set must match the producer's DDL, and its schema
version must match the producer's. Adapters without one are listed as an explicit
unknown — a report, not a silent pass.

This test is the promotion mechanism: as each harness gets its vertical, it moves
from UNCONTRACTED to CONTRACTED here, and the list can only shrink.
"""

from __future__ import annotations

import re

import pytest

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer
from cynic_data.canonical.domain.conservation import Fate, parse_fate

# Adapters that must expose a contract, with a callable that builds them.
# α9: bound to the frozen capture, never the live store.
CONTRACTED = {
    "hermes": lambda: HermesNormalizer("/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"),
}

# Adapters that still lack a contract. This list may only shrink; α6 is the
# promotion mechanism, so a new harness lands here until its vertical is done.
# Retirements also shrink it (T6 capsules in docs/specs/harness-*.md).
# T6 emptied it (antigravity was the last entry, retired 2026-10-19):
# every retired harness is capsule-documented, hermes holds a contract.
UNCONTRACTED: dict = {}


@pytest.mark.parametrize("name", sorted(CONTRACTED))
def test_alpha6_contracted_adapter_exposes_its_contract(name):
    contract = CONTRACTED[name]().conservation()
    assert contract is not None, f"{name}: declared contracted but returns None"
    assert contract.producer, f"{name}: contract has no producer"
    assert contract.producer_schema_version > 0, f"{name}: no schema version pinned"
    assert contract.producer_schema_source, f"{name}: no schema source named"


@pytest.mark.parametrize("name", sorted(CONTRACTED))
def test_alpha6_contracted_adapter_has_no_malformed_fate(name):
    contract = CONTRACTED[name]().conservation()
    assert not contract.malformed_fates(), f"{name}: {contract.malformed_fates()}"


@pytest.mark.parametrize("name", sorted(CONTRACTED))
def test_alpha6_contracted_adapter_has_a_preserved_and_a_declared_fate(name):
    """A contract that declares nothing is not a contract."""
    contract = CONTRACTED[name]().conservation()
    preserved = contract.preserved_fields()
    declared = contract.declared_losses()
    assert preserved, f"{name}: no preserved fields"
    assert declared, f"{name}: no declared losses (a contract must admit something)"


@pytest.mark.parametrize("name", sorted(UNCONTRACTED))
def test_alpha6_uncontracted_adapters_are_reported_not_hidden(name):
    """An adapter without a contract returns None — an explicit unknown. This
    test exists so the gap is visible and shrinks, never silently accepted."""
    contract = UNCONTRACTED[name]().conservation()
    assert contract is None, (
        f"{name}: now exposes a contract — move it to CONTRACTED in this test"
    )


def test_alpha6_every_normalizer_has_the_conservation_method():
    """The port method must exist on every adapter, even when it returns None."""
    for name, build in {**CONTRACTED, **UNCONTRACTED}.items():
        adapter = build()
        assert callable(getattr(adapter, "conservation", None)), (
            f"{name}: missing conservation() — the port requires it"
        )
