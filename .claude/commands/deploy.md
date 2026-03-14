Build, deploy, and restart the CYNIC kernel. Run this after ANY code change to cynic-kernel/.

Steps:
1. cargo build -p cynic-kernel --release
2. cargo test -p cynic-kernel
3. cargo clippy --workspace -- -D warnings
4. cp target/release/cynic-kernel ~/bin/cynic-kernel (stop service first if "text file busy")
5. systemctl --user restart cynic-kernel
6. Wait 4 seconds, then verify: curl -s http://localhost:3030/health
7. Show how many Dogs are active from journalctl

If any step fails, stop and diagnose. Do NOT skip tests or clippy.
