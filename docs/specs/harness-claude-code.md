# Harness `claude_code` — knowledge capsule (retired T6)

Claude Code CLI sessions (this operator's own history). Python chain
retired 2026-10-19 (T6, fourth harness, operator HITL) : no Rust vertical
(T7). The EASY harness — explicit tree pointers, native edges. Code
history : slice 3 era -> `2fe85682` -> `2f3d7e19`.
No FIELD_FATE table ; 5 contract tests + full sweep lived in
`test_normalizers_contract.py` (deleted with the chain).

## Sources

- Live (ACTIVE — operator's current Claude Code) :
  `/home/user/.claude/projects/<slug>/<session>.jsonl` (107 files).
- Frozen : `/var/lib/cynic/vault/corpus/sources/claude_code/<slug>/`
  (349 files : `-home-user` 8, `-home-user-Bureau-CYNIC` 333,
  `-home-user-Bureau-KAIROS` 7,
  `-home-user-Bureau-SOLANA-asdf-forge-zeyxx-CYNIC` 1).
- Live-vs-frozen gap (recorded) : live keeps growing (107 files and
  counting) while the freeze is static. Retiring stops ingestion —
  the operator's ongoing sessions will NOT be normalized until T7.
  Re-derivable at any time : frozen files + live dir, same format.

## Native shape (one JSON object per line, chronological)

- `type` : `user` | `assistant` | `file-history-snapshot`.
- `parentUuid` -> explicit tree pointer, so REPLY edges are NATIVE.
- `isSidechain` -> subagent turns explicitly flagged
  (EdgeKind.SIDECHAIN).
- `message.content` : LIST of typed blocks `text` | `tool_use` |
  `tool_result`.
- `message.usage` -> per-turn token counters (only harness inline).
- `toolUseResult` on the user turn : raw result payload.
- `message.model` kept on the Turn (operator invariant) ; unobserved
  fields stay None.

## Coverage at retirement

- Silver : 106 sessions.
- Guards removed : 5 contract tests (valid session, native REPLY edges,
  tool_result link-back, native model, full sweep over live archives).
- Registry entry removed : `UNCONTRACTED["claude_code"]`
  (`test_conservation_contract.py`) -> `~/.claude/projects`.
- Port doc updated (`ports.py` : ClaudeCode bullet retired).

## T7 rebuild pointers

- Scan : `<slug>/<session>.jsonl`, one file = one session tree.
- NATIVE edges : REPLY from parentUuid, SIDECHAIN from isSidechain.
- Per-turn usage counters inline — no aggregation pass needed.
- The deleted `test_normalizers_contract.py` (git history) encoded the
  contract : valid session, native REPLY, tool_result link-back,
  native model, full sweep. Rebuild it first, Rust side.
