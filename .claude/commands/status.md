Show full CYNIC system status — kernel, DB, backends, network.

Run these in parallel and report:
1. Kernel health: `curl -s http://localhost:3030/health | python3 -m json.tool`
2. SurrealDB: `surreal is-ready --endpoint http://localhost:8000 2>&1`
3. Tailscale: `tailscale status 2>&1 | head -10`
4. Git state: `git status --short && git log --oneline -3`
5. Process check: `lsof -i :3030 2>/dev/null | head -3; lsof -i :50051 2>/dev/null | head -3`

Summarize as a dashboard:
- KERNEL: running/down (port 3030)
- SURREALDB: running/down (port 8000)
- TAILSCALE: connected/down (show IPs)
- GIT: branch, clean/dirty, last commit
