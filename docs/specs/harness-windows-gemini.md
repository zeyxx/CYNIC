# Harness `windows_gemini` — knowledge capsule (retired T6)

Gemini CLI on Windows (host zeyxm). Python chain retired 2026-10-19
(T6, first harness) : ingestion incomplete (34 sessions on ~69k source
files), no Rust vertical (T7). This capsule preserves the producer-format
knowledge for a future Rust vertical. Code history :
`48c5cd1b` (birth, slice 3) -> `2fe85682` -> `2f3d7e19`.

## Sources (all vault-local, immutable)

- Default root : `/var/lib/cynic/vault/archive/windows_sessions_2026/zeyxm_dot_gemini`
- Mirror : `/var/lib/cynic/vault/corpus/sources/windows_historical/zeyxm_dot_gemini` (69144 files)
- Live (never ingested) : `/home/user/.gemini`
- Sibling (Antigravity registry entry, same producer family) :
  `/var/lib/cynic/vault/archive/linux_sessions_2026/gemini_antigravity/conversations`

## Native shape

One JSON document per file `tmp/<project>/chats/session-*.json` :
`{sessionId, projectHash, startTime, lastUpdated, messages, kind, summary}`,
plus an optional sibling `tool-outputs/session-<sessionId>/` directory
with full tool outputs as `*.txt`.

- Message `type` : `user` | `gemini` | `info` | `error`. Native `gemini`
  role kept (operator invariant : harnesses carry different models).
- `user.content` is a LIST of `{text}` blocks (rarely `inlineData` or a
  `functionResponse` result riding on the user turn, as in Claude Code).
- `gemini.content` is a string ; `thoughts` is a list of
  `{subject, description, timestamp}` -> ReasoningTrace.
- `tokens` is `{input, output, cached, thoughts, tool, total}` ->
  a genuine StateSnapshot.
- `toolCalls` items : `id` | `name` | `displayName` | `description` |
  `args` | `result` (list of `{functionResponse: {id, name, response:
  {output}}}`) | `resultDisplay` (str or dict) | `status` (`success` |
  `error` | `cancelled`) | `timestamp` | `renderOutputAsMarkdown`.
- NO parent pointer -> sequence edges DERIVED from timestamp order.
- Chat files carry masked/truncated results (`<tool_output_masked>`,
  `[TRUNCATED]`) ; the `*.txt` files are the full outputs. NO reliable
  per-call key between them (filenames carry no call id, counts do not
  align) : tool entities come from inline `toolCalls`, the tool-outputs
  directory is inventoried (file count, bytes, manifest digest) in
  `native_meta`, never content-linked.
- One chat file (13.5 MB) is corrupt on disk (`2output":` key typo plus
  bare Windows-path backslashes) ; `_load_chat` applies exactly two
  documented repairs and records `repaired: true` in `native_meta`.
- Unobserved fields stay None.

## FIELD_FATE (closed world — every observed path maps to exactly one fate)

```
*.sessionId: preserved: session_uid
*.projectHash: preserved: meta.project_hash
*.startTime: preserved: meta.start_time
*.lastUpdated: preserved: meta.last_updated
*.kind: preserved: meta.kind
*.summary: preserved: meta.summary (human-authored session summary)
*.messages: preserved: dispatch
message.id: preserved: turn.native.record_id
message.timestamp: preserved: timestamp_iso
message.type: preserved: turn.role (native: user|gemini|info|error)
user_block.text: preserved: turn.content
user_block.inlineData: preserved: turn.native.inline_data
user_block.functionResponse: preserved: result entity (user-carried tool result)
message.content: preserved: turn.content
message.displayContent: preserved: turn.native.display_content (alternate rendering)
message.model: preserved: turn.native_model
message.thoughts: preserved: reasoning entities
thought.subject: preserved: reasoning.subject
thought.description: preserved: reasoning.content
thought.timestamp: preserved: reasoning.timestamp
message.tokens: preserved: state (input/output/total) + native (cached/thoughts/tool)
tokens.input: preserved: state.context_prompt_tokens
tokens.output: preserved: state.native.output
tokens.cached: preserved: state.context_nonmessage_tokens
tokens.thoughts: preserved: state.native.thoughts_tokens
tokens.tool: preserved: state.native.tool_tokens
tokens.total: preserved: state.cumulative_tokens
toolCall.id: preserved: tool.call_id
toolCall.name: preserved: tool.tool_name
toolCall.displayName: preserved: tool.native.display_name
toolCall.description: preserved: tool.native.description
toolCall.args: preserved: tool.command_or_args
toolCall.result: preserved: tool.raw_output (masked/truncated as harnessed)
functionResponse.id: preserved: tool.call_id (result side)
functionResponse.name: preserved: tool.native.result_name
functionResponse.response: preserved: tool.raw_output
functionResponse.parts: preserved: tool.native.result_parts (binary payloads, e.g. base64 PDFs)
message.toolCalls: preserved: dispatch
toolCall.resultDisplay: preserved: tool.native.result_display (str or dict)
toolCall.status: preserved: tool.is_error + exit_code (success|error|cancelled)
toolCall.timestamp: preserved: timestamp_iso
toolCall.renderOutputAsMarkdown: preserved: tool.native.render_markdown
tool_outputs.files: declared: inventoried in meta.tool_outputs (count/bytes/manifest); full bytes re-derivable from frozen capture, never content-linked (no reliable key)
```

## Coverage at retirement

- Silver : 34 sessions (`windows_gemini_chat` 21 + `windows_gemini_tools` 13),
  frozen, no timestamps on rows.
- Guards (deleted with the chain, not in the T5 harness) : capture test,
  closed-world field coverage, normalizer test.
- Registry entries removed : `UNCONTRACTED["windows_gemini"]`
  (`test_conservation_contract.py`), `~/.gemini` lambda.

## T7 rebuild pointers

Session = one `sessionId` group (chat files merged in timestamp order +
tool-outputs dir). Scan : `tmp/**/chats/session-*.json`, group by
`sessionId`, skip non-dict roots, `_load_chat` two-repair rule.
Contract : replicate the FIELD_FATE table above against the sibling
`omp.py` contract style ; malformed fates fail closed.
