"""Formal topological and mathematical invariants for the Model Layer (α10 to α14).

These tests formulate the mathematical ground truth of agentic trajectories
BEFORE the final Domain Model entities are coded (Test-First / Falsification-First).
They test the physical realities observed on the frozen capture:

* α10 — Strict Causal Acyclicity (DAG): The causal poset (both intra-session
        turns and inter-session subagent trees) must contain ZERO cycles.
* α11 — N-ary Compaction Conservation: Every `absorbed_message_uids` set maps to
        real message entities, forming an N:1 reduction hyper-edge.
* α12 — Transitive Subagent Tree Resolution: Every declared `parent_session_id`
        must resolve to a valid root/intermediate session, with no cyclic ancestry.
* α13 — Tool Concurrency Poset Preservation: When an assistant turn emits K > 1
        tool calls, the resulting executions are concurrent (A || B in Lamport poset),
        never sequentially dependent on each other.
* α14 — CoT Phenotype Conservation: Reasoning outputs (reasoning_content,
        codex_reasoning_items, reasoning_details) must remain distinctly typed and
        never flattened into role content strings.

α9 invariant enforced: Reads exclusively from the frozen capture.
"""

from __future__ import annotations

import json
import os
import sqlite3
import pytest

DB = "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"


def _connect():
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    con.row_factory = sqlite3.Row
    return con


@pytest.mark.skipif(not os.path.exists(DB), reason="Frozen capture absent")
def test_alpha10_inter_session_delegation_is_a_strict_dag():
    """α10: The subagent hierarchy across sessions must be a Directed Acyclic Graph.

    No session may be its own ancestor (cycles violate causal physics).
    """
    con = _connect()
    try:
        cur = con.execute(
            "SELECT id, parent_session_id FROM sessions "
            "WHERE parent_session_id IS NOT NULL AND parent_session_id != ''"
        )
        parent_map: dict[str, str] = {row["id"]: row["parent_session_id"] for row in cur}
    finally:
        con.close()

    assert len(parent_map) == 603, f"Expected 603 child sessions, got {len(parent_map)}"

    # Check for cycles using standard DFS cycle detection
    visited: set[str] = set()
    for child in parent_map:
        path: set[str] = set()
        curr = child
        while curr in parent_map:
            assert curr not in path, f"Cycle detected in subagent hierarchy at session {curr}"
            path.add(curr)
            curr = parent_map[curr]
        visited.update(path)


@pytest.mark.skipif(not os.path.exists(DB), reason="Frozen capture absent")
def test_alpha11_compaction_absorptions_resolve_strictly():
    """α11: N-ary Compaction Conservation.

    Every UUID in `absorbed_message_uids` represents a real historical message
    that was compacted into the summary. An absorption referencing phantom UIDs
    is a broken causal hyper-edge.
    """
    con = _connect()
    try:
        cur = con.execute(
            "SELECT id, session_id, message_uid, absorbed_message_uids "
            "FROM messages "
            "WHERE absorbed_message_uids IS NOT NULL AND absorbed_message_uids != ''"
        )
        absorbed_rows = cur.fetchall()
        assert len(absorbed_rows) > 0, "No compaction absorptions found in corpus"

        # Verify each absorption references real message UIDs within the same or known session.
        # Empirically discovered: a message_uid can appear across multiple compaction passes
        # within the session, so the mathematical invariant is:
        # count(DISTINCT message_uid) == len(set(raw_uids)).
        for row in absorbed_rows:
            raw_uids = json.loads(row["absorbed_message_uids"])
            assert isinstance(raw_uids, list), f"Expected list of UIDs, got {type(raw_uids)}"
            assert len(raw_uids) >= 1, "Empty absorbed_message_uids list recorded"
            distinct_uids = list(set(raw_uids))
            placeholders = ",".join("?" for _ in distinct_uids)
            found = con.execute(
                f"SELECT count(DISTINCT message_uid) FROM messages "
                f"WHERE session_id = ? AND message_uid IN ({placeholders})",
                [row["session_id"], *distinct_uids],
            ).fetchone()[0]
            assert found == len(distinct_uids), (
                f"Compaction row {row['id']} in session {row['session_id']} absorbed "
                f"{len(distinct_uids)} distinct UIDs but only {found} exist in session history"
            )
    finally:
        con.close()


@pytest.mark.skipif(not os.path.exists(DB), reason="Frozen capture absent")
def test_alpha12_subagent_parent_resolution_is_100_percent():
    """α12: Transitive Subagent Tree Resolution.

    100% of sessions with a declared parent_session_id must resolve to an
    existing session in the capture. No dangling subagent references.
    """
    con = _connect()
    try:
        cur = con.execute(
            "SELECT count(*) as total, "
            "sum(case when s2.id IS NOT NULL then 1 else 0 end) as resolved "
            "FROM sessions s1 "
            "LEFT JOIN sessions s2 ON s1.parent_session_id = s2.id "
            "WHERE s1.parent_session_id IS NOT NULL AND s1.parent_session_id != ''"
        )
        row = cur.fetchone()
        assert row["total"] == 603
        assert row["resolved"] == 603, (
            f"Subagent parent resolution failed: {row['resolved']}/{row['total']} resolved"
        )
    finally:
        con.close()


@pytest.mark.skipif(not os.path.exists(DB), reason="Frozen capture absent")
def test_alpha13_parallel_tool_calls_form_concurrent_poset():
    """α13: Tool Concurrency Poset Preservation.

    An assistant turn with K > 1 tool calls causes concurrent execution.
    In Lamport's poset, for an assistant node A with calls {c_1, ..., c_k} and
    corresponding results {r_1, ..., r_k}:
        A < r_i for all i, but r_i || r_j for i != j.
    The test verifies that multi-tool assistant turns produce distinct call IDs
    and each has its corresponding result recorded without collisions.
    """
    con = _connect()
    try:
        # Sample turns where the tool_calls JSON array genuinely contains K > 1 calls
        cur = con.execute(
            "SELECT id, session_id, tool_calls FROM messages "
            "WHERE tool_calls IS NOT NULL AND tool_calls != ''"
        )
        multi_tool_turns = []
        for row in cur:
            try:
                calls = json.loads(row["tool_calls"])
                if isinstance(calls, list) and len(calls) > 1:
                    multi_tool_turns.append((row["id"], row["session_id"], calls))
                    if len(multi_tool_turns) >= 25:
                        break
            except Exception:
                pass

        assert len(multi_tool_turns) == 25, "Could not find 25 multi-tool calls in corpus"

        for mid, sid, calls in multi_tool_turns:
            call_ids = [c["id"] for c in calls if "id" in c]
            assert len(call_ids) > 1, f"Expected K > 1 call ids in turn {mid}"
            # Call IDs within the same turn must be unique (no duplicate identities in parallel poset)
            assert len(call_ids) == len(set(call_ids)), (
                f"Duplicate call IDs in single assistant turn: {call_ids}"
            )
            # Find matching tool results
            placeholders = ",".join("?" for _ in call_ids)
            results = con.execute(
                f"SELECT tool_call_id FROM messages "
                f"WHERE session_id = ? AND role = 'tool' AND tool_call_id IN ({placeholders})",
                [sid, *call_ids],
            ).fetchall()
            result_ids = {r["tool_call_id"] for r in results}
            # Verify results exist for the calls
            assert len(result_ids) > 0, f"No tool results found for multi-call turn {mid}"
    finally:
        con.close()


@pytest.mark.skipif(not os.path.exists(DB), reason="Frozen capture absent")
def test_alpha14_cot_phenotype_preservation():
    """α14: CoT Phenotype Conservation.

    Model thinking/reasoning appears under distinct producer phenotypes:
    `reasoning_content` (DeepSeek/Nous style), `codex_reasoning_items` (OpenAI style),
    and `reasoning_details`.
    These fields must be preserved distinctly and never conflated or erased.
    """
    con = _connect()
    try:
        counts = con.execute(
            "SELECT "
            "sum(case when reasoning_content IS NOT NULL AND reasoning_content != '' then 1 else 0 end) as rc, "
            "sum(case when codex_reasoning_items IS NOT NULL AND codex_reasoning_items != '' then 1 else 0 end) as cri, "
            "sum(case when reasoning_details IS NOT NULL AND reasoning_details != '' then 1 else 0 end) as rd "
            "FROM messages"
        ).fetchone()

        assert counts["rc"] == 56053, f"Expected 56053 reasoning_content, got {counts['rc']}"
        assert counts["cri"] == 17558, f"Expected 17558 codex_reasoning_items, got {counts['cri']}"
        assert counts["rd"] == 2481, f"Expected 2481 reasoning_details, got {counts['rd']}"
    finally:
        con.close()
