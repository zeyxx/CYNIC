#!/usr/bin/env python3
"""Bridge: canonical SQLite store -> teich-compatible trace format.

Teich expects OpenAI-style messages: assistant turns carry ``tool_calls``
(structured), and tool outputs arrive as ``role: "tool"`` messages linked by
``tool_call_id``. Our canonical store keeps turns and tools in separate tables
with an explicit parent pointer (``parent_exec_uid``) and an explicit
``tool_name == "tool_result"`` marker for outputs.

Lossless mapping rules:
  * turn with tool_result rows  -> one ``role: "tool"`` message per result,
    ``tool_call_id`` = the result's own call_id (native), content = raw_output
  * assistant turn with call rows -> ``tool_calls`` from command_or_args
  * every other turn -> passthrough {role, content}

Usage:
    python bridge_to_teich.py --store canonical.sqlite --out traces.jsonl
"""

from __future__ import annotations

import argparse
import json
import sqlite3


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--store", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    con = sqlite3.connect(f"file:{args.store}?mode=ro", uri=True)
    cur = con.cursor()

    sessions = cur.execute(
        "SELECT session_uid, harness FROM sessions ORDER BY session_uid"
    ).fetchall()

    n_traces = 0
    with open(args.out, "w", encoding="utf-8") as out:
        for session_uid, harness in sessions:
            turns = cur.execute(
                "SELECT turn_uid, role, content, native_model "
                "FROM turns WHERE session_uid = ? ORDER BY idx",
                (session_uid,),
            ).fetchall()
            if not turns:
                continue

            tools_by_turn: dict[str, list[tuple]] = {}
            for row in cur.execute(
                "SELECT turn_uid, tool_name, call_id, command_or_args, raw_output, is_error "
                "FROM tools WHERE session_uid = ? ORDER BY idx",
                (session_uid,),
            ):
                tools_by_turn.setdefault(row[0], []).append(row)

            # Declare exactly the tools observed in this session, name-only.
            # The raw store has no tool parameter schemas, so none are invented.
            observed_names = sorted({
                row[1]
                for rows in tools_by_turn.values()
                for row in rows
                if row[1] and row[1] != "tool_result"
            })
            tool_schemas = [
                {"type": "function", "function": {"name": name}}
                for name in observed_names
            ]

            messages: list[dict] = []
            for turn_uid, role, content, model in turns:
                turn_tools = tools_by_turn.get(turn_uid, [])
                results = [t for t in turn_tools if t[1] == "tool_result"]
                calls = [t for t in turn_tools if t[1] != "tool_result"]

                # Tool outputs become role:"tool" messages, one per result.
                for _, _name, call_id, _args, output, is_err in results:
                    msg = {"role": "tool", "content": output or ""}
                    if call_id:
                        msg["tool_call_id"] = call_id
                    if is_err:
                        msg["is_error"] = True
                    messages.append(msg)

                msg = {"role": role, "content": content or ""}
                if model:
                    msg["model"] = model

                if calls and role == "assistant":
                    tool_calls = []
                    for _, name, call_id, t_args, _output, _is_err in calls:
                        if not call_id:
                            continue
                        tool_calls.append({
                            "id": call_id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": t_args or "{}",
                            },
                        })
                    if tool_calls:
                        msg["tool_calls"] = tool_calls

                messages.append(msg)

            out.write(json.dumps({
                "id": session_uid,
                "harness": harness,
                "messages": messages,
                "tools": tool_schemas,
            }, ensure_ascii=False) + "\n")
            n_traces += 1

    con.close()
    print(f"[bridge] wrote {n_traces} traces to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
