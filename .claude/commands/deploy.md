Build, deploy, and restart the CYNIC kernel. Run this after ANY code change to cynic-kernel/.

Steps:
1. `source ~/.cargo/env && source ~/.cynic-env`
2. `make check`
3. `cargo build -p cynic-kernel --release`
4. Backup DB before deploy: `surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 --username root --password "${SURREALDB_PASS}" ~/.surrealdb/backups/cynic_v2_pre_deploy_$(date +%Y%m%d_%H%M%S).surql`
5. Deploy binary (mv, not cp — MCP instances hold the old inode open):
   ```
   systemctl --user stop cynic-kernel
   mv ~/bin/cynic-kernel ~/bin/cynic-kernel.prev 2>/dev/null
   cp target/release/cynic-kernel ~/bin/cynic-kernel
   systemctl --user start cynic-kernel
   ```
   Why mv+cp instead of cp-over: `cp` overwrites the inode in-place, which fails with ETXTBSY when any process (MCP) has the binary open. `mv` renames the old inode (running processes keep it), then `cp` creates a fresh inode at the path.
6. Verify health (K9: status codes are the contract):
   - Retry up to 15 times (3s apart): `curl -sf -o /dev/null http://${CYNIC_REST_ADDR}/ready`
   - If `/ready` returns 200 within retries, deploy succeeded
   - If all retries fail (non-200), rollback: `mv ~/bin/cynic-kernel ~/bin/cynic-kernel.bad 2>/dev/null; mv ~/bin/cynic-kernel.prev ~/bin/cynic-kernel && systemctl --user restart cynic-kernel`
7. Show Dog count: `curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" http://${CYNIC_REST_ADDR}/dogs | jq length`

If any step fails, stop and diagnose. Do NOT skip `make check`.
