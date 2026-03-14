Start the CYNIC kernel (build if needed, kill existing, launch).

Steps:
1. Kill any existing kernel: `pkill -f cynic-kernel 2>/dev/null; sleep 1`
2. Build: `source ~/.cargo/env; cargo build -p cynic-kernel --release 2>&1 | tail -3`
3. Launch: `source ~/.cynic-env; ./target/release/cynic-kernel` (run in background)
4. Wait 3 seconds, then health check: `curl -s http://localhost:3030/health | python3 -m json.tool`
5. Report status: which backends are HEALTHY, how many Dogs active
