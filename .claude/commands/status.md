---
name: status
description: CYNIC system diagnostic — not a green facade, a real picture.
disable-model-invocation: true
allowed-tools: Bash(curl *) Bash(source *) Bash(systemctl *) Bash(free *) Bash(df *) Bash(ps *)
---

CYNIC system diagnostic — not a green facade, a real picture.

First: `source ~/.cynic-env 2>/dev/null`

Run ALL of these (parallel where independent) and report EVERYTHING:

1. **Kernel health (authenticated):**
   `curl -s "http://${CYNIC_REST_ADDR}/health" -H "Authorization: Bearer ${CYNIC_API_KEY}"`
   Extract: status, version, dog count + per-dog circuit state, storage status + metrics (queries, errors, slow, avg_latency_ms), embedding status, verdict_cache_size, uptime_seconds, total_requests, estimated_cost_usd

2. **Binary freshness (worktree vs running kernel):**
   ```
   GIT_NOW=$(git -C "$(git rev-parse --show-toplevel)" describe --tags --always --dirty 2>/dev/null)
   BIN_VER=$(curl -s "http://${CYNIC_REST_ADDR}/health" -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq -r .version)
   if [ "$GIT_NOW" = "$BIN_VER" ]; then
     echo "PASS: kernel matches worktree ($BIN_VER)"
   else
     echo "STALE: running=$BIN_VER worktree=$GIT_NOW"
   fi
   ```
   `Cargo.toml version` and `/health version` are two different formats (semver vs `git describe --dirty`) — comparing them is always false. The only honest check is `git describe` of the worktree vs what the kernel was actually built from. The Bearer header is required when the kernel runs fail-closed (no `CYNIC_ALLOW_OPEN_API=1`); without it, `/health` may return minimal JSON or the middleware may reject the call.

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

11. **MCP supervision (avoid silent duplication + Ctrl-Z trap):**
    ```
    # Running MCP processes — expect at most one cynic-kernel --mcp per active claude session
    pgrep -af 'cynic-kernel --mcp' | nl
    N=$(pgrep -cf 'cynic-kernel --mcp' 2>/dev/null || echo 0)
    [ "$N" -gt 1 ] && echo "WARN: $N cynic-kernel MCP instances — check if parent claudes are still alive"

    # Runtime identity: every MCP must run the same binary as the systemd kernel
    for pid in $(pgrep -f 'cynic-kernel --mcp'); do
      echo "  PID $pid exe=$(readlink /proc/$pid/exe 2>/dev/null)"
    done

    # Zombies under stopped claude parents (Ctrl-Z on claude leaves all MCP children unreaped)
    ps -eo pid,ppid,stat,cmd | awk '$3 ~ /^Z/ && /cynic-kernel|tailscale-mcp|npm exec/ {print "  ZOMBIE: " $0}'
    ```
    - WARN if `N > 1`
    - WARN if any two MCPs show different `exe` paths (config drift between repo `.mcp.json` and user-level `~/.claude.json`)
    - FAIL if any zombie line appears — parent claude is stopped (`Tl`/`T`), children cannot be reaped until it resumes or dies

Format as a dashboard with clear PASS/WARN/FAIL indicators:
```
=== CYNIC v{version} — {timestamp} ===
KERNEL:    {status}  {dog_count} Dogs  uptime {uptime}
BINARY:    {PASS|STALE}  running={bin_ver}  worktree={git_now}
STORAGE:   {status}  {queries}q {errors}err {avg_ms}ms {slow} slow
EMBEDDING: {status}  cache: {cache_size}
SURREAL:   {status}
LLAMA:     {ports}
TAILSCALE: {status}  {peers}
GIT:       {branch} {version} {dirty} files dirty
BACKUPS:   {count} files, latest {date}
MCP:       {N} running  exe_consistent={yes|no}
COST:      ${cost} estimated ({total_requests} requests)

--- ERRORS (10 min) ---
{actual error lines or "(none)"}

--- CCM ---
{last cycle result or "(no cycle in window)"}

--- SURREALDB ---
{recent warnings or "(clean)"}

--- ZOMBIES (under stopped claude parents) ---
{zombie lines from §11 or "none"}
```

Be honest. If something is degraded, say WARN. If something is broken, say FAIL. Do not hide problems behind "nominal."
