"""Canonical contract tests — invariants and declared losses."""

from __future__ import annotations

import pytest

from cynic_data.canonical.domain.entities import (
    CorrectionKind,
    Edge,
    EdgeKind,
    EdgeProvenance,
    HumanSignal,
    Turn,
)
from cynic_data.canonical.domain.session import NormalizedSession
from cynic_data.kernel.contract import (
    CANONICAL_CONTRACT,
    CAPTURE_CONTRACT,
    PROJECTION_CONTRACT,
    Authority,
    Layer,
    all_layer_contracts,
)
from cynic_data.kernel.errors import ContractViolation


def _session() -> NormalizedSession:
    return NormalizedSession(session_uid="hermes:s1", harness="hermes", source_ref="x")


def test_four_layers_are_declared():
    layers = [c.layer for c in all_layer_contracts()]
    assert layers == [Layer.CAPTURE, Layer.CANONICAL, Layer.MODEL, Layer.PROJECTION]


def test_authority_split():
    assert CAPTURE_CONTRACT.authority is Authority.AUTHORITATIVE
    assert CANONICAL_CONTRACT.authority is Authority.AUTHORITATIVE
    assert PROJECTION_CONTRACT.authority is Authority.DERIVED


def test_capture_and_canonical_declare_no_semantic_loss():
    assert CAPTURE_CONTRACT.declared_losses == ()
    assert CANONICAL_CONTRACT.declared_losses == ()


def test_reads_only_from_layer_above():
    assert CAPTURE_CONTRACT.reads_from is None
    assert CANONICAL_CONTRACT.reads_from is Layer.CAPTURE


def test_projection_must_be_able_to_declare_loss():
    from cynic_data.kernel.contract import LossDeclaration

    contract = PROJECTION_CONTRACT
    assert not contract.declares_loss_of("human_text")
    contract2 = type(contract)(
        layer=Layer.PROJECTION,
        authority=Authority.DERIVED,
        guarantee="sft view",
        reads_from=Layer.MODEL,
        declared_losses=(LossDeclaration("human_text", "not needed for this view"),),
    )
    assert contract2.declares_loss_of("human_text")


def test_duplicate_turn_uid_is_a_violation():
    s = _session()
    s.turns.append(Turn("hermes:s1:t0000", "hermes:s1", 0, "user", "hi"))
    s.turns.append(Turn("hermes:s1:t0000", "hermes:s1", 1, "assistant", "yo"))
    with pytest.raises(ContractViolation):
        s.check()


def test_foreign_session_uid_is_a_violation():
    s = _session()
    s.turns.append(Turn("hermes:s1:t0000", "other:s9", 0, "user", "hi"))
    with pytest.raises(ContractViolation):
        s.check()


def test_edge_to_unknown_turn_is_a_violation():
    s = _session()
    s.turns.append(Turn("hermes:s1:t0000", "hermes:s1", 0, "user", "hi"))
    s.edges.append(
        Edge(
            "hermes:s1:e0000:reply",
            "hermes:s1",
            EdgeKind.REPLY,
            ("hermes:s1:t0000",),
            "hermes:s1:t9999",
            EdgeProvenance.DERIVED,
        )
    )
    with pytest.raises(ContractViolation):
        s.check()


def test_human_signal_must_reference_a_known_turn():
    s = _session()
    s.human.append(HumanSignal("h1", "hermes:s1", "hermes:s1:t0000", "no"))
    with pytest.raises(ContractViolation):
        s.check()


def test_valid_session_passes():
    s = _session()
    s.turns.append(Turn("hermes:s1:t0000", "hermes:s1", 0, "user", "hi"))
    s.turns.append(Turn("hermes:s1:t0001", "hermes:s1", 1, "assistant", "yo"))
    s.edges.append(
        Edge(
            "hermes:s1:e0000:sequence",
            "hermes:s1",
            EdgeKind.SEQUENCE,
            ("hermes:s1:t0000",),
            "hermes:s1:t0001",
            EdgeProvenance.DERIVED,
        )
    )
    s.human.append(
        HumanSignal("h1", "hermes:s1", "hermes:s1:t0000", "hi", CorrectionKind.UNKNOWN)
    )
    s.check()  # must not raise


def test_nary_edge_is_representable():
    """The hypergraph: one edge, many sources."""
    e = Edge(
        "e1",
        "hermes:s1",
        EdgeKind.COMPACTION,
        ("t1", "t2", "t3"),
        "t9",
        EdgeProvenance.NATIVE,
    )
    assert len(e.source_uids) == 3
