---
description: Workflow triggers and tool use discipline — always active
globs: ["**"]
---

## Workflow Triggers

BEFORE triggers — invoke PROACTIVELY before acting:
- **Before adding a module, dependency, or Cargo.toml change:** `/cynic-skills:cynic-empirical`
- **Before choosing between architecturally different approaches:** `/cynic-skills:crystallize-truth`
- **Before designing a new subsystem:** `/cynic-skills:engineering-stack-design`

AFTER triggers — MUST execute, no exceptions:
- **After ANY code change to cynic-kernel/:** `/build`
- **After modifying >5 files or >100 lines in a session:** `/cynic-skills:distill`
- **After changes to scoring, prompts, or crystal injection:** `/test-chess` (before AND after)

ON triggers — invoke when the situation matches:
- **Evaluate quality:** `/cynic-skills:cynic-judge`
- **Simplify/burn code:** `/cynic-skills:cynic-burn`
- **Touching kernel source:** `/cynic-kernel` (read architecture reference first)
- **Deploy to production:** `/deploy` (build + test + backup + deploy + restart + verify)
- **System status:** `/status`
- **Troubleshooting:** `/cynic-workflow`

## Automatic Enforcement (hooks — no LLM action needed)

These are handled mechanically. Do NOT invoke them manually:
- **Coord claims:** `protect-files.sh` auto-claims on Edit/Write to `cynic-kernel/src/*`. Blocks on CONFLICT (409). Graceful degradation if kernel down.
- **Coord release:** `session-stop.sh` releases all claims at session end.
- **Rust formatting:** `rustfmt-rs.sh` auto-formats `.rs` files after Edit/Write.
- **Tool observation:** `observe-tool.sh` records Edit/Write/Bash/Read/Grep/Glob to `/observe` (async, fire-and-forget).
- **File protection:** `protect-files.sh` blocks access to `.ssh/`, `.env`, secret configs.
- **Dirty tree warning:** `session-stop.sh` warns on uncommitted changes at session end.

## Crystal Feedback Loop

Crystal injection changes Dog prompts — the core value proposition.
Any change to `format_crystal_context`, `observe_crystal`, or `search_crystals_semantic` MUST be benchmarked with `/test-chess` before AND after.
