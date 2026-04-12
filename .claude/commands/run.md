---
name: run
description: Start the CYNIC kernel via systemd (build if needed).
disable-model-invocation: true
allowed-tools: Bash(cargo *) Bash(systemctl *) Bash(curl *) Bash(cp *) Bash(source *)
---

Start the CYNIC kernel via systemd (build if needed).

Steps:
1. `source ~/.cargo/env && source ~/.cynic-env`
2. Build: `cargo build -p cynic-kernel --release 2>&1 | tail -3`
3. Deploy binary: `systemctl --user stop cynic-kernel 2>/dev/null; cp target/release/cynic-kernel ~/bin/cynic-kernel`
4. Start: `systemctl --user start cynic-kernel`
5. Wait 4 seconds, then health check: `curl -s http://${CYNIC_REST_ADDR}/health | python3 -m json.tool`
6. Report status: which Dogs are active, storage status, uptime
