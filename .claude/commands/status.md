Show full CYNIC system status — kernel, DB, backends, network.

First: `source ~/.cynic-env 2>/dev/null`

Run these in parallel and report:
1. Kernel health: `curl -s http://${CYNIC_REST_ADDR}/health | python3 -m json.tool`
2. SurrealDB: `surreal is-ready --endpoint http://localhost:8000 2>&1`
3. Tailscale: `tailscale status 2>&1 | head -10`
4. Git state: `git status --short && git log --oneline -3`
5. Services: `systemctl --user is-active cynic-kernel surrealdb llama-server surrealdb-backup.timer 2>&1`
6. Backups: `ls -lh ~/.surrealdb/backups/ 2>&1`

Summarize as a dashboard:
- KERNEL: running/down (show status + storage + dog count)
- SURREALDB: running/down (port 8000)
- LLAMA-SERVER: running/down (systemd)
- TAILSCALE: connected/down (show IPs)
- BACKUPS: last backup date + size
- GIT: branch, clean/dirty, last commit
