Build, deploy, and restart the CYNIC kernel. Run this after ANY code change to cynic-kernel/.

Steps:
1. `source ~/.cargo/env && source ~/.cynic-env`
2. `make check`
3. `cargo build -p cynic-kernel --release`
4. Backup DB before deploy: `surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 --username root --password "${SURREALDB_PASS}" ~/.surrealdb/backups/cynic_v2_pre_deploy_$(date +%Y%m%d_%H%M%S).surql`
5. `systemctl --user stop cynic-kernel && cp ~/bin/cynic-kernel ~/bin/cynic-kernel.prev 2>/dev/null; cp target/release/cynic-kernel ~/bin/cynic-kernel && systemctl --user start cynic-kernel`
6. Verify health (K9: status codes are the contract):
   - Retry up to 15 times (3s apart): `curl -sf -o /dev/null http://${CYNIC_REST_ADDR}/ready`
   - If `/ready` returns 200 within retries, deploy succeeded
   - If all retries fail (non-200), rollback: `cp ~/bin/cynic-kernel.prev ~/bin/cynic-kernel && systemctl --user restart cynic-kernel`
7. Show Dog count: `curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" http://${CYNIC_REST_ADDR}/dogs | jq length`

If any step fails, stop and diagnose. Do NOT skip `make check`.
