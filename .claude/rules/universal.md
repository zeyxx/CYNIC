---
description: Core development principles — always active
globs: ["**"]
---

## Meta-Principle

**Fix → Test → Gate → Verify.** Every fix: code change + regression test + mechanical gate + gate verification. Enforcement must be mechanical, not LLM compliance.

## Enforced Rules (verified by make check / hooks)

1. **Zero hardcoded paths.** Use `$(git rev-parse --show-toplevel)` or `${CYNIC_REST_ADDR}`. Never absolute paths in skills, hooks, or configs. — `make lint-rules`
2. **Handle all fallible I/O.** Propagate with `?` or log + retry/skip. No silent `.ok()`. — `make lint-rules`
3. **Every producer needs a consumer.** `store_*` without a read path = invisible waste. — `make lint-drift`
4. **Commit before completing.** `git status --short` at session end must show 0 modified files. — `session-stop.sh` (warns)
5. **No dead architecture.** Commented modules need `DORMANT:` tag. Skills in CLAUDE.md must exist on disk. Hooks on disk must be wired in settings. — `make lint-drift`

## Design Principles (judgment guidance — no mechanical gate)

6. **Diagnose before fixing.** Read errors, trace data, one hypothesis, test minimally. 2 fix attempts max — obvious → alternative → escalate. Never brute-force.
7. **Measure before AND after.** Every "improved X" claim needs before/after numbers.
8. **Port contracts first.** New dependency → trait → adapter → test.
9. **Bounded everything.** Channels, retries, confidence. Unbounded = debt.
10. **Wire or delete.** Every public symbol must have a caller AND a test. Zero-caller = delete.
11. **Fix the class, not the instance.** `grep` the entire codebase for the same pattern before closing a fix.
12. **One value, one source.** `backends.toml` = Dogs. `~/.cynic-env` = secrets. Never duplicate config.
13. **Name things for what they ARE.** Code names match code behavior, not aspirations.
14. **Strong > no > weak foundation.** Prove E2E with real data before building on a subsystem.
15. **Falsify before adopting.** Architectural decisions require: hypothesis, falsification test, evidence from production systems.
16. **Gate at the lowest common caller.** Security gates live at the function ALL paths call, not at one convenient caller.
17. **Bugs before abstractions.** Data-corrupting SQL bug outranks a compile-time newtype. Fix what breaks data first.
18. **Deploy from main only.** Feature branches never touch production binaries.
19. **Scripts are thin.** Bash = `curl + status code`. Logic lives in the kernel.
20. **Every feature must DO what it CLAIMS.** No relabeling — if it says "enforced," there must be a gate.
21. **Verify gates catch violations.** Every new gate (lint target, hook, check) must be tested against a known violation before trusting. A gate that has never failed is not a gate. — `make lint-rules`, `make lint-security`
