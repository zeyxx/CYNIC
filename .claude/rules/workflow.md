---
description: Workflow triggers and tool use discipline — always active
globs: ["**"]
---

## Mandatory Workflow

IMPORTANT: These are not suggestions. Skipping any trigger is a workflow violation.

BEFORE triggers — invoke PROACTIVELY before acting:
- **Before adding a module, dependency, or Cargo.toml change:** `/cynic-skills:cynic-empirical`
- **Before choosing between architecturally different approaches:** `/cynic-skills:crystallize-truth`
- **Before designing a new subsystem:** `/cynic-skills:engineering-stack-design`
- **Before editing any file:** `cynic_coord_who()` then `cynic_coord_claim()`

AFTER triggers — MUST execute, no exceptions:
- **After ANY code change to cynic-kernel/:** `/build`
- **After modifying >5 files or >100 lines:** `/cynic-skills:distill`
- **After changes to scoring, prompts, or crystal injection:** `/test-chess` (before AND after)
- **After work complete on a file:** `cynic_coord_release()`

ON triggers — invoke when the situation matches:
- **Evaluate quality:** `/cynic-skills:cynic-judge`
- **Simplify/burn code:** `/cynic-skills:cynic-burn`
- **Touching kernel source:** `/cynic-kernel` (read architecture reference first)
- **Deploy:** `/deploy` (build + test + backup + deploy + restart + verify)
- **System status:** `/status`
- **Troubleshooting:** `/cynic-workflow`

## Feedback Loop (Rule 23)

Crystal injection changes Dog prompts — the core value proposition.
Any change to `format_crystal_context`, `observe_crystal`, or `search_crystals_semantic` MUST be benchmarked with `/test-chess` before AND after.

## Environment

All tools use `${CYNIC_REST_ADDR}` and `${CYNIC_API_KEY}` from `~/.cynic-env`.
MCP servers: **cynic** (kernel), **tailscale** (fleet), **context7** (docs), **playwright** (browser).
