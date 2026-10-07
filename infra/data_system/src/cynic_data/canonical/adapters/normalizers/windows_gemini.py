"""Windows-Gemini normalizer.

Source: ``.../zeyxm_dot_gemini/tmp/<project>/chats/session-*.json`` — one JSON
document per file (``{sessionId, projectHash, startTime, lastUpdated, messages,
kind, summary}``), plus an optional sibling
``tool-outputs/session-<sessionId>/`` directory holding full tool outputs.

A session is a ``sessionId`` group: its chat files (disjoint message sets,
merged in timestamp order) plus its tool-outputs directory. The chat files
carry masked/truncated results (``<tool_output_masked>``, ``[TRUNCATED]``);
the ``*.txt`` files are the full outputs. There is NO reliable per-call key
between them (filenames carry no call id, counts do not align), so the adapter
does NOT fabricate links: tool entities come from the inline ``toolCalls``,
and the tool-outputs directory is inventoried (file count, bytes, manifest
digest) in ``native_meta``, re-derivable from the frozen capture.

Native shape:
  * message ``type`` is ``user`` | ``gemini`` | ``info`` | ``error``
  * ``user.content`` is a LIST of ``{text}`` blocks (rarely ``inlineData`` or a
    ``functionResponse`` result riding on the user turn, as in Claude Code)
  * ``gemini.content`` is a string; ``thoughts`` is a list of
    ``{subject, description, timestamp}`` -> ReasoningTrace
  * ``tokens`` is ``{input, output, cached, thoughts, tool, total}`` ->
    a genuine StateSnapshot
  * ``toolCalls`` items carry ``id`` | ``name`` | ``displayName`` |
    ``description`` | ``args`` | ``result`` (list of ``{functionResponse:
    {id, name, response: {output}}}``) | ``resultDisplay`` (str or dict) |
    ``status`` (``success`` | ``error`` | ``cancelled``) | ``timestamp`` |
    ``renderOutputAsMarkdown``
  * NO parent pointer -> sequence edges are DERIVED from timestamp order
  * one chat file (13.5 MB) is corrupt on disk (``2output":`` key typo plus
    bare Windows-path backslashes); ``_load_chat`` applies exactly two
    documented repairs and records ``repaired: true`` in ``native_meta``

Per the operator invariant: harnesses carry different models. The native
``gemini`` role is kept (it is the harness's word); unobserved fields stay
None.
"""

from __future__ import annotations

import glob
import hashlib
import json
import os
import re
from functools import lru_cache
from pathlib import Path

from typing import Any

from ....capture.adapters._hash import hash_tree
from ....kernel.ids import (
    SessionUid,
    edge_uid,
    human_uid,
    reasoning_uid,
    state_uid,
    tool_uid,
    turn_uid,
)
from ...domain.entities import (
    Edge,
    EdgeKind,
    EdgeProvenance,
    HumanSignal,
    ReasoningTrace,
    StateSnapshot,
    ToolCall,
    Turn,
)
from ...domain.session import NormalizedSession
from ._human import classify_human_turn

HARNESS = "windows_gemini"

# Exactly the repairs a corrupt chat file needs. Anything else fails loudly.
_REPAIR_LITERAL = "\n        2output\":"
_REPAIR_FIXED = '\n        "output":'
_BAD_ESCAPE = re.compile(r"\\\\|\\(u(?![0-9a-fA-F]{4})|U(?![0-9a-fA-F]{8})|(?![\"\\/bfnrtuU]))")


def _fix_escape(match: re.Match[str]) -> str:
    """Double a lone backslash; leave a valid \\ pair untouched.

    The alternation consumes valid pairs first, so a lone backslash is never
    the second half of an already-valid escape.
    """
    text = match.group(0)
    if len(text) == 2:
        return text
    return "\\" + text


def _repair_json(text: str) -> tuple[str, bool]:
    """Apply the two documented repairs. Returns (text, repaired)."""
    repaired = False
    if _REPAIR_LITERAL in text:
        text = text.replace(_REPAIR_LITERAL, _REPAIR_FIXED)
        repaired = True
    # The third alternative is a bare lookahead: _fix_escape doubles the lone
    # backslash and leaves valid pairs untouched.
    fixed, n = _BAD_ESCAPE.subn(_fix_escape, text)
    if n:
        # Count only effective changes: a sub that rewrites \X to \\X.
        text, repaired = fixed, True
    return text, repaired


@lru_cache(maxsize=None)
def _load_chat(path: str) -> tuple[dict[str, Any], bool]:
    """Load one chat file, applying documented repairs. Raises on failure.

    Cached per path per process (the 13 MB corrupt file must not be re-read
    and re-repaired on every scan).
    """
    with open(path, encoding="utf-8", errors="replace") as fh:
        text = fh.read()
    try:
        return json.loads(text), False
    except json.JSONDecodeError:
        pass
    repaired_text, _ = _repair_json(text)
    return json.loads(repaired_text), True


@lru_cache(maxsize=None)
def _tool_outputs_manifest(tool_dir: str) -> dict[str, Any]:
    """Inventory a session's tool-outputs directory (no content linkage).

    Cached: the frozen tree is immutable, so the manifest is computed once per
    directory per process instead of once per normalize call.
    """
    if not os.path.isdir(tool_dir):
        return {"files": 0, "bytes": 0, "manifest": None}
    digest, count, total = hash_tree(tool_dir)
    return {"files": count, "bytes": total, "manifest": digest}


# Every raw field path observed in the corpus maps to exactly one fate.
# See omp.py FIELD_FATE for the contract: the closed-world test fails on any
# observed path missing here.
FIELD_FATE: dict[str, str] = {
    # -- document envelope --
    "*.sessionId": "preserved: session_uid",
    "*.projectHash": "preserved: meta.project_hash",
    "*.startTime": "preserved: meta.start_time",
    "*.lastUpdated": "preserved: meta.last_updated",
    "*.kind": "preserved: meta.kind",
    "*.summary": "preserved: meta.summary (human-authored session summary)",
    "*.messages": "preserved: dispatch",
    # -- message envelope --
    "message.id": "preserved: turn.native.record_id",
    "message.timestamp": "preserved: timestamp_iso",
    "message.type": "preserved: turn.role (native: user|gemini|info|error)",
    # -- user blocks --
    "user_block.text": "preserved: turn.content",
    "user_block.inlineData": "preserved: turn.native.inline_data",
    "user_block.functionResponse": "preserved: result entity (user-carried tool result)",
    # -- gemini message --
    "message.content": "preserved: turn.content",
    "message.displayContent": "preserved: turn.native.display_content (alternate rendering)",
    "message.model": "preserved: turn.native_model",
    "message.thoughts": "preserved: reasoning entities",
    "thought.subject": "preserved: reasoning.subject",
    "thought.description": "preserved: reasoning.content",
    "thought.timestamp": "preserved: reasoning.timestamp",
    "message.tokens": "preserved: state (input/output/total) + native (cached/thoughts/tool)",
    "tokens.input": "preserved: state.context_prompt_tokens",
    "tokens.output": "preserved: state.native.output",
    "tokens.cached": "preserved: state.context_nonmessage_tokens",
    "tokens.thoughts": "preserved: state.native.thoughts_tokens",
    "tokens.tool": "preserved: state.native.tool_tokens",
    "tokens.total": "preserved: state.cumulative_tokens",
    # -- toolCall item --
    "toolCall.id": "preserved: tool.call_id",
    "toolCall.name": "preserved: tool.tool_name",
    "toolCall.displayName": "preserved: tool.native.display_name",
    "toolCall.description": "preserved: tool.native.description",
    "toolCall.args": "preserved: tool.command_or_args",
    "toolCall.result": "preserved: tool.raw_output (masked/truncated as harnessed)",
    "functionResponse.id": "preserved: tool.call_id (result side)",
    "functionResponse.name": "preserved: tool.native.result_name",
    "functionResponse.response": "preserved: tool.raw_output",
    "functionResponse.parts": "preserved: tool.native.result_parts (binary payloads, e.g. base64 PDFs)",
    # -- toolCall dispatch marker (items covered above) --
    "message.toolCalls": "preserved: dispatch",
    "toolCall.resultDisplay": "preserved: tool.native.result_display (str or dict)",
    "toolCall.status": "preserved: tool.is_error + exit_code (success|error|cancelled)",
    "toolCall.timestamp": "preserved: timestamp_iso",
    "toolCall.renderOutputAsMarkdown": "preserved: tool.native.render_markdown",
    # -- tool-outputs directory (no per-call key exists: inventory, not links) --
    "tool_outputs.files": "declared: inventoried in meta.tool_outputs (count/bytes/manifest); full bytes re-derivable from frozen capture, never content-linked (no reliable key)",
}


class WindowsGeminiNormalizer:
    harness = HARNESS

    def __init__(self, gemini_root: str = "/var/lib/cynic/vault/archive/windows_sessions_2026/zeyxm_dot_gemini") -> None:
        self.gemini_root = Path(gemini_root)

    def _tmp_root(self) -> Path:
        return self.gemini_root / "tmp"

    @lru_cache(maxsize=1)
    def _scan(self) -> tuple[dict[str, list[str]], list[str]]:
        """Group chat files by sessionId. Returns (session_id -> files, broken).

        Cached: the frozen tree is immutable, so the scan runs once per
        normalizer instance instead of once per can_read/session_refs/normalize.
        """
        groups: dict[str, list[str]] = {}
        broken: list[str] = []
        tmp = self._tmp_root()
        if not tmp.is_dir():
            return groups, broken
        for path in sorted(tmp.rglob("chats/session-*.json")):
            try:
                data, _ = _load_chat(str(path))
            except (json.JSONDecodeError, OSError):
                broken.append(str(path))
                continue
            sid = data.get("sessionId") if isinstance(data, dict) else None
            if not sid:
                broken.append(str(path))
                continue
            groups.setdefault(str(sid), []).append(str(path))
        return groups, broken

    def can_read(self, source_ref: str) -> bool:
        try:
            inside = Path(source_ref).resolve().is_relative_to(self.gemini_root.resolve())
        except OSError:
            inside = False
        if source_ref.endswith(".json") and "/chats/" in source_ref and inside:
            return True
        groups, _ = self._scan()
        return source_ref in groups

    def conservation(self):
        """α6: no contract yet — an explicit unknown, never an implicit no-loss.

        This harness's vertical is not done. Until it is, the audit reports this
        adapter as UNCONTRACTED rather than assuming conservation.
        """
        return None

    def session_refs(self) -> list[str]:
        """A session is a sessionId group (chat files + tool-outputs dir)."""
        groups, _ = self._scan()
        return sorted(groups)

    def normalize(self, session_ref: str) -> NormalizedSession:
        groups, _ = self._scan()
        if session_ref not in groups:
            raise KeyError(f"unknown windows_gemini session: {session_ref}")
        session_uid = str(SessionUid(HARNESS, session_ref))

        # Merge the group's chat files in timestamp order (disjoint sets).
        messages: list[dict[str, Any]] = []
        repaired = False
        project_hash: str | None = None
        start_time: str | None = None
        last_updated: str | None = None
        kind: str | None = None
        summary: str | None = None
        for path in groups[session_ref]:
            data, was_repaired = _load_chat(path)
            repaired = repaired or was_repaired
            project_hash = project_hash or data.get("projectHash")
            start_time = start_time or data.get("startTime")
            last_updated = data.get("lastUpdated", last_updated)
            kind = kind or data.get("kind")
            if data.get("summary") and not summary:
                summary = data["summary"]
            for m in data.get("messages", []):
                if isinstance(m, dict):
                    messages.append(m)
        messages.sort(key=lambda m: (m.get("timestamp") or "", m.get("id") or ""))

        tool_dir = str(self._tmp_root() / "*" / "tool-outputs" / f"session-{session_ref}")
        resolved_dirs = sorted(glob.glob(tool_dir))
        tool_manifest = _tool_outputs_manifest(resolved_dirs[0]) if resolved_dirs else {"files": 0, "bytes": 0, "manifest": None}

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=";".join(groups[session_ref]),
            native_meta={
                "ordering": "message timestamp order, merged across chat files",
                "parent_pointer": "none: sequence edges are DERIVED",
                "project_hash": project_hash,
                "start_time": start_time,
                "last_updated": last_updated,
                "kind": kind,
                "summary": summary,
                "repaired": repaired,
                "chat_files": len(groups[session_ref]),
                "tool_outputs": tool_manifest,
            },
        )

        call_id_to_exec: dict[str, str] = {}
        turn_idx = tool_idx = state_idx = edge_idx = reasoning_idx = 0
        prev_turn: str | None = None

        for m in messages:
            mtype = m.get("type", "")
            ts = m.get("timestamp")
            t_uid = turn_uid(session_uid, turn_idx)

            if prev_turn is not None:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "sequence", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.SEQUENCE,
                        source_uids=(prev_turn,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.DERIVED,
                        payload={"rule": "timestamp order (no parent pointer)"},
                        timestamp_iso=ts,
                    )
                )
                edge_idx += 1

            if mtype == "user":
                texts: list[str] = []
                inline_data: list[Any] = []
                for b in m.get("content") or []:
                    if not isinstance(b, dict):
                        continue
                    if b.get("text") is not None:
                        texts.append(b["text"])
                    if "inlineData" in b:
                        inline_data.append(b["inlineData"])
                    fr = b.get("functionResponse")
                    if isinstance(fr, dict):
                        resp = fr.get("response") or {}
                        exec_uid = tool_uid(session_uid, tool_idx)
                        sess.tools.append(
                            ToolCall(
                                exec_uid=exec_uid,
                                session_uid=session_uid,
                                turn_uid=t_uid,
                                index=tool_idx,
                                tool_name="tool_result",
                                call_id=fr.get("id"),
                                command_or_args=None,
                                raw_output=json.dumps(resp.get("output"), ensure_ascii=False)
                                if resp.get("output") is not None
                                else None,
                                exit_code=0,
                                is_error=False,
                                parent_exec_uid=None,
                                timestamp_iso=ts,
                                native_fields={"result_name": fr.get("name")},
                            )
                        )
                        sess.edges.append(
                            Edge(
                                edge_uid=edge_uid(session_uid, "tool_result", edge_idx),
                                session_uid=session_uid,
                                kind=EdgeKind.TOOL_RESULT,
                                source_uids=(t_uid,),
                                target_uid=None,
                                provenance=EdgeProvenance.DERIVED,
                                payload={"rule": "user-carried functionResponse, no call id link"},
                                timestamp_iso=ts,
                            )
                        )
                        edge_idx += 1
                        tool_idx += 1
                sess.turns.append(
                    Turn(
                        turn_uid=t_uid,
                        session_uid=session_uid,
                        index=turn_idx,
                        role="user",
                        content="".join(texts),
                        timestamp_iso=ts,
                        native_model=None,
                        native_fields={
                            k: v
                            for k, v in {
                                "record_id": m.get("id"),
                                "display_content": m.get("displayContent"),
                                "inline_data": inline_data or None,
                            }.items()
                            if v is not None
                        },
                    )
                )
                kind_m, markers = classify_human_turn("".join(texts))
                sess.human.append(
                    HumanSignal(
                        human_uid=human_uid(session_uid, t_uid),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        text="".join(texts),
                        kind=kind_m,
                        markers=markers,
                        timestamp_iso=ts,
                    )
                )

            elif mtype == "gemini":
                content = m.get("content")
                text = content if isinstance(content, str) else ""
                model = m.get("model")
                sess.turns.append(
                    Turn(
                        turn_uid=t_uid,
                        session_uid=session_uid,
                        index=turn_idx,
                        role="gemini",
                        content=text,
                        timestamp_iso=ts,
                        native_model=model,
                        native_fields={
                            k: v
                            for k, v in {
                                "record_id": m.get("id"),
                                "display_content": m.get("displayContent"),
                            }.items()
                            if v is not None
                        },
                    )
                )
                for th in m.get("thoughts") or []:
                    if not isinstance(th, dict):
                        continue
                    sess.reasoning.append(
                        ReasoningTrace(
                            trace_uid=reasoning_uid(session_uid, reasoning_idx),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            content=th.get("description", ""),
                            subject=th.get("subject"),
                            model=model,
                            timestamp_iso=th.get("timestamp"),
                        )
                    )
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "reasoning", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.REASONING,
                            source_uids=(t_uid,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={"subject": th.get("subject")},
                            timestamp_iso=ts,
                        )
                    )
                    edge_idx += 1
                    reasoning_idx += 1
                toks = m.get("tokens")
                if isinstance(toks, dict):
                    sess.states.append(
                        StateSnapshot(
                            snapshot_uid=state_uid(session_uid, state_idx),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            index=state_idx,
                            model=model,
                            context_prompt_tokens=toks.get("input"),
                            context_nonmessage_tokens=toks.get("cached"),
                            cumulative_tokens=toks.get("total"),
                            compacted=None,
                            native_fields={
                                k: v
                                for k, v in {
                                    "output": toks.get("output"),
                                    "thoughts_tokens": toks.get("thoughts"),
                                    "tool_tokens": toks.get("tool"),
                                }.items()
                                if v is not None
                            },
                            timestamp_iso=ts,
                        )
                    )
                    state_idx += 1
                for tc in m.get("toolCalls") or []:
                    if not isinstance(tc, dict):
                        continue
                    call_id = tc.get("id", "")
                    exec_uid = tool_uid(session_uid, tool_idx)
                    result = tc.get("result") or []
                    output = None
                    result_name = None
                    result_parts: list[Any] = []
                    for item in result:
                        if isinstance(item, dict) and isinstance(item.get("functionResponse"), dict):
                            fr = item["functionResponse"]
                            result_name = fr.get("name")
                            resp = fr.get("response") or {}
                            output = resp.get("output")
                            if isinstance(fr.get("parts"), list):
                                result_parts = fr["parts"]
                    status = tc.get("status", "success")
                    is_err = status != "success"
                    sess.tools.append(
                        ToolCall(
                            exec_uid=exec_uid,
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            index=tool_idx,
                            tool_name=tc.get("name", "tool"),
                            call_id=call_id or None,
                            command_or_args=json.dumps(tc.get("args", {}), ensure_ascii=False),
                            raw_output=json.dumps(output, ensure_ascii=False)
                            if output is not None
                            else None,
                            exit_code=1 if is_err else 0,
                            is_error=is_err,
                            parent_exec_uid=None,
                            timestamp_iso=tc.get("timestamp") or ts,
                            native_fields={
                                k: v
                                for k, v in {
                                    "display_name": tc.get("displayName"),
                                    "description": tc.get("description"),
                                    "result_name": result_name,
                                    "result_display": tc.get("resultDisplay"),
                                    "result_parts": result_parts or None,
                                    "render_markdown": tc.get("renderOutputAsMarkdown"),
                                }.items()
                                if v is not None
                            },
                        )
                    )
                    if call_id:
                        call_id_to_exec[call_id] = exec_uid
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "tool_call", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.TOOL_CALL,
                            source_uids=(t_uid,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={"tool_name": tc.get("name"), "call_id": call_id, "status": status},
                            timestamp_iso=ts,
                        )
                    )
                    edge_idx += 1
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "tool_result", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.TOOL_RESULT,
                            source_uids=(exec_uid,),
                            target_uid=t_uid,
                            provenance=EdgeProvenance.NATIVE,
                            payload={"call_id": call_id, "status": status},
                            timestamp_iso=ts,
                        )
                    )
                    edge_idx += 1
                    tool_idx += 1

            elif mtype in ("info", "error"):
                raw_content = m.get("content")
                sess.turns.append(
                    Turn(
                        turn_uid=t_uid,
                        session_uid=session_uid,
                        index=turn_idx,
                        role=mtype,
                        content=raw_content if isinstance(raw_content, str) else "",
                        timestamp_iso=ts,
                        native_model=None,
                        native_fields={"record_id": m.get("id")},
                    )
                )

            else:
                raw_content = m.get("content")
                sess.turns.append(
                    Turn(
                        turn_uid=t_uid,
                        session_uid=session_uid,
                        index=turn_idx,
                        role=mtype or "unknown",
                        content=raw_content if isinstance(raw_content, str) else "",
                        timestamp_iso=ts,
                        native_model=m.get("model"),
                        native_fields={"record_id": m.get("id")},
                    )
                )

            prev_turn = t_uid
            turn_idx += 1

        sess.check()
        return sess
