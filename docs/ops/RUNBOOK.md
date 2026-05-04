# CYNIC Runbook — Operational Recovery Procedures

**Audience:** Anyone operating CYNIC without the primary developer present.
**Prerequisite:** SSH access to the Ubuntu machine via Tailscale, `~/.cynic-env` sourced.

## Quick Reference

```bash
source ~/.cynic-env
make status          # System dashboard
make check-storage   # Integration tests against real DB
make test-restore    # Verify backups are restorable
make rollback        # Revert to previous kernel binary
make restore F=path  # Restore DB from backup (DESTRUCTIVE)
```

## Service Restart Order

Always restart in this order (dependencies flow downward):

```
1. surrealdb        → systemctl --user restart surrealdb
2. cynic-kernel     → systemctl --user restart cynic-kernel
3. llama-server     → systemctl --user restart llama-server  (sovereign inference)
4. llama-embed      → systemctl --user restart llama-embed   (embedding server, port 8081)
```

Wait 5 seconds between each restart. Verify with `make status`.

## Common Failures

### Kernel unreachable (http://$CYNIC_REST_ADDR/health times out)

1. Check service: `systemctl --user status cynic-kernel`
2. If failed: `journalctl --user -u cynic-kernel --since "5 min ago" | tail -20`
3. Common causes:
   - SurrealDB down → restart SurrealDB first (see order above)
   - Port conflict → `ss -tlnp | grep 3030`
   - OOM kill → `journalctl -k | grep -i oom`
4. Restart: `systemctl --user restart cynic-kernel`
5. Verify: `curl -s http://${CYNIC_REST_ADDR}/health`

### SurrealDB down

1. Check: `systemctl --user status surrealdb`
2. Check: `curl -s http://localhost:8000/health`
3. If failed: `systemctl --user restart surrealdb`
4. If DB corrupt: restore from backup (see Backup Recovery below)
5. Note: SurrealDB credentials are persisted on first start. The service does NOT pass `--user`/`--pass` flags — it relies on persisted root credentials.

### Sovereign backend (llama-server) not responding

1. Check local: `systemctl --user status llama-server`
2. Check remote GPU host (remote): `ssh <REMOTE_GPU_HOST> "tasklist | findstr llama-server"`
3. The health loop probes every 30s and opens circuit breakers after 3 failures
4. If remediation.toml is configured, SSH restart fires automatically after 90s
5. Manual restart local: `systemctl --user restart llama-server`
6. Manual restart remote GPU host: `ssh <REMOTE_GPU_HOST> "schtasks /run /tn llama-server"`

### Disk space low (<15%)

1. Check: `df -h /`
2. Largest consumers: `du -sh ~/.surrealdb/ ~/Bureau/CYNIC/target/ /tmp/`
3. Quick wins:
   - `cargo clean` in CYNIC repo (can reclaim 1-2 GB)
   - Old backups: `ls -lhrt ~/.surrealdb/backups/ | head -10` — delete if >14 days
   - Rust toolchain: `rustup toolchain list` — remove old versions

### API key rotation

1. Generate new key on the provider's website
2. Update `~/.cynic-env` with the new key
3. Regenerate systemd env: copy the key to `~/.config/cynic/env`
4. Restart: `systemctl --user restart cynic-kernel`
5. Verify: `curl -s http://${CYNIC_REST_ADDR}/health -H "Authorization: Bearer ${CYNIC_API_KEY}"`

## Backup Recovery

### Verify backups work (non-destructive)
```bash
make test-restore
```
This imports the latest backup into a test namespace, checks verdict+crystal counts, then drops the test DB.

### Full restore (DESTRUCTIVE — stops kernel)
```bash
# List available backups
ls -lhrt ~/.surrealdb/backups/

# Restore from a specific backup
make restore F=~/.surrealdb/backups/cynic_v2_YYYYMMDD_HHMMSS.surql
```
This stops the kernel, imports the backup, restarts the kernel.

### Backup schedule
- **Automated daily:** `surrealdb-backup.timer` fires at midnight, exports to `~/.surrealdb/backups/`, gzips, verifies
- **Pre-deploy:** `make deploy` creates a backup before deploying
- **Manual:** `make backup`
- **Pruning:** .gz files >7 days, .surql files >14 days (automatic)
- **Off-site:** NOT configured. All backups are on the same machine.

## Monitoring

### Health check timer
- `cynic-healthcheck.timer` runs every 5 minutes
- Checks: kernel reachable, SurrealDB up, disk >15% free, memory >512MB
- Results in journal: `journalctl -t cynic-healthcheck --since "1 hour ago"`

### Manual dashboard
```bash
make status
```

### View recent errors
```bash
journalctl --user -u cynic-kernel --since "30 min ago" | grep -iE "error|warn|fail"
```

## Architecture Quick Reference

| Service | Port | Interface | What |
|---------|------|-----------|------|
| cynic-kernel | 3030 | Tailscale only | REST API + MCP server |
| surrealdb | 8000 | localhost only | Persistent storage |
| llama-server | 8080 | Tailscale only | Sovereign LLM inference |
| llama-embed | 8081 | Tailscale only | Embedding server |

All services run as systemd user units. Check with:
```bash
systemctl --user list-units 'cynic*' 'surreal*' 'llama*'
```
