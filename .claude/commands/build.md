Build, test, and lint the CYNIC kernel in one shot. Run this after any code change.

Steps:
1. `source ~/.cargo/env 2>/dev/null; cargo build -p cynic-kernel --release 2>&1 | tail -5`
2. If build succeeds: `cargo test -p cynic-kernel --release 2>&1 | grep -E "^test |^test result"`
3. If tests pass: `cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
4. Report: BUILD ok/fail, TESTS pass/fail (count), CLIPPY clean/warnings
5. IMPORTANT: Always use `--release` flag (debug mode SIGSEGV on kernel 6.17)
