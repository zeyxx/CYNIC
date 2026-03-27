---
description: Core development principles — always active
globs: ["**"]
---

## Development Principles

1. **Diagnose before fixing.** Read errors, trace data, one hypothesis, test minimally.
2. **2 fix attempts max.** Obvious → alternative → escalate. Never brute-force.
5. **Port contracts first.** New dependency → trait → adapter → test.
6. **Bounded everything.** Channels, retries, confidence. Unbounded = debt.
7. **Zero hardcoded paths.** Use `$(git rev-parse --show-toplevel)` or `${CYNIC_REST_ADDR}`. Never absolute paths in skills, hooks, or configs.
8. **Handle all fallible I/O.** Propagate with `?` or log + retry/skip. No silent `.ok()`. Enforced by `make lint-rules`.
9. **Wire or delete.** Every public symbol must have a caller AND a test. Zero-caller = delete.
12. **Fix the class, not the instance.** `grep` the entire codebase for the same pattern before closing a fix.
14. **One value, one source.** `backends.toml` = Dogs. `~/.cynic-env` = secrets. Never duplicate config.
15. **HTTP status codes are the contract.** Monitoring checks status codes, never parses JSON.
16. **Scripts are thin.** Bash = `curl + status code`. Logic lives in the kernel.
20. **Research testing patterns.** Before adding a module, cover how to TEST it (not just build it).
21. **No dead architecture.** Every feature must DO what it CLAIMS — no relabeling.
24. **Name things for what they ARE.** Code names match code behavior, not aspirations.
25. **Fix → Test → Gate → Verify.** Code fix + regression test + mechanical gate + gate verification. Enforcement must be mechanical, never LLM compliance.
26. **Strong > no > weak foundation.** Prove E2E with real data before building on a subsystem.
27. **Compound organically.** Map downstream connections — find the seed that feeds the most subsystems.
28. **Agents use the platform.** Agents delegate persistence/judgment/learning to the kernel. No agent-owned DBs.
29. **Deploy from main only.** Feature branches never touch production binaries.
30. **Commit before completing.** `git status --short` at session end must show 0 modified files.
31. **Measure before AND after.** Every "improved X" claim needs before/after numbers.
33. **Every producer needs a consumer.** `store_*` without a read path = invisible waste.
34. **Falsify before adopting.** Architectural decisions require: hypothesis, falsification test, evidence from production systems. No structural choice without attempting to disprove it.
