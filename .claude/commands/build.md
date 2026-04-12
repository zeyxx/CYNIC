---
name: build
description: Build, test, and lint the CYNIC kernel in one shot. Run this after any code change.
disable-model-invocation: true
allowed-tools: Bash(make *) Bash(cargo *) Bash(source *)
---

Build, test, and lint the CYNIC kernel in one shot. Run this after any code change.

Run `make check` from the project root. This is the single entry point — never run cargo build/test/clippy manually.

`make check` runs: fmt → clippy → test → lint-rules → lint-drift → cargo audit → integration tests (if SurrealDB available).

Report the exit code and any FAIL lines from the output. If it passes, report test count from the output.

If `make check` fails with SIGSEGV, run `make clean` then retry.
