Build, deploy, and restart the CYNIC kernel. Run this after ANY code change to cynic-kernel/.

Steps:
1. `source ~/.cargo/env && source ~/.cynic-env`
2. `cargo build -p cynic-kernel --release`
3. `cargo test -p cynic-kernel --release`
4. `cargo clippy -p cynic-kernel --release -- -D warnings`
5. Backup DB before deploy: `surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 --username root --password "${SURREALDB_PASS}" ~/.surrealdb/backups/cynic_v2_pre_deploy_$(date +%Y%m%d_%H%M%S).surql`
6. `systemctl --user stop cynic-kernel && cp target/release/cynic-kernel ~/bin/cynic-kernel && systemctl --user start cynic-kernel`
7. Wait 4 seconds, then verify: `curl -s http://${CYNIC_REST_ADDR}/health | python3 -m json.tool`
8. Show how many Dogs are active and storage status from the health response

If any step fails, stop and diagnose. Do NOT skip tests or clippy.
