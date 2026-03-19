CYNIC system diagnostic — not a green facade, a real picture.

First: `source ~/.cynic-env 2>/dev/null`

Run ALL of these (parallel where independent) and report EVERYTHING:

1. **Kernel health (authenticated):**
   `curl -s "http://${CYNIC_REST_ADDR}/health" -H "Authorization: Bearer ${CYNIC_API_KEY}"`
   Extract: status, version, dog count + per-dog circuit state, storage status + metrics (queries, errors, slow, avg_latency_ms), embedding status, verdict_cache_size, uptime_seconds, total_requests, estimated_cost_usd

2. **Version alignment:**
   `grep '^version' cynic-kernel/Cargo.toml` vs the version from /health
   Flag if they diverge (means binary is stale).

3. **SurrealDB:** `surreal is-ready --endpoint http://localhost:8000 2>&1`

4. **Tailscale:** `tailscale status 2>&1 | grep -E "ubuntu|stanislaz|forge"`

5. **Processes:**
   `ss -tlnp | grep -E '808[0-1]|3030|8000'` — all CYNIC ports
   `ps aux | grep -E "apport|defunct" | grep -v grep | wc -l` — zombie check

6. **Git:** `git log --oneline -1` + `git status --short | wc -l` dirty files

7. **Errors (last 10 min):**
   `journalctl --user -u cynic-kernel --no-pager --since "10 min ago" 2>/dev/null | grep -i "error\|401\|slow\|warn\|FATAL"`
   Show the ACTUAL error lines, not just a count. Max 10 lines.

8. **CCM health (last cycle):**
   `journalctl --user -u cynic-kernel --no-pager --since "10 min ago" 2>/dev/null | grep "CCM"`

9. **SurrealDB health:**
   `journalctl --user -u surrealdb --no-pager --since "30 min ago" 2>/dev/null | grep -i "error\|warn\|drop\|transaction" | tail -5`

10. **Backups:** `ls -lhrt ~/.surrealdb/backups/ | tail -3` + total count

Format as a dashboard with clear PASS/WARN/FAIL indicators:
```
=== CYNIC v{version} — {timestamp} ===
KERNEL:    {status}  {dog_count} Dogs  uptime {uptime}
STORAGE:   {status}  {queries}q {errors}err {avg_ms}ms {slow} slow
EMBEDDING: {status}  cache: {cache_size}
SURREAL:   {status}
LLAMA:     {ports}
TAILSCALE: {status}  {peers}
GIT:       {branch} {version} {dirty} files dirty
BACKUPS:   {count} files, latest {date}
COST:      ${cost} estimated ({total_requests} requests)

--- ERRORS (10 min) ---
{actual error lines or "(none)"}

--- CCM ---
{last cycle result or "(no cycle in window)"}

--- SURREALDB ---
{recent warnings or "(clean)"}

--- ZOMBIES ---
{count or "none"}
```

Be honest. If something is degraded, say WARN. If something is broken, say FAIL. Do not hide problems behind "nominal."
