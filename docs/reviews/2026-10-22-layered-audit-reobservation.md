# Re-observation — 2026-10-22 layered audit (read-only)

- Observer: hermes, on cortex
- Base: `origin/agent/claude/layered-audit-2026-10-22` @ `bc86456e`
- Source report: `docs/reviews/2026-10-22-layered-infrastructure-audit.md`
- Method: read-only. No mutation, no approval requested, no PR opened.
- Worktree: `~/.cynic/worktrees/agent_claude_layered-audit-2026-10-22`

## C4-1 — desktop/agent user is root

Commands:

```bash
sudo -n true; echo EXIT=$?
ls -la /etc/sudoers.d/
id user
cat /etc/sudoers.d/cynic-tools
```

Observed 2026-10-22:

- `sudo -n true` -> EXIT 0
- `/etc/sudoers.d/temp-claude` still present (29 bytes, dated 2026-04-23)
- `/etc/sudoers.d/cynic-tools` still grants NOPASSWD `/usr/bin/find`, `/usr/bin/cat` (plus rustdesk, apt)
- `id user` groups include `sudo`, `docker`, `cynic`, `cynic-agent`

Verdict: CONFIRMED, unchanged since audit.

## C1-1 — kernel authority file writable by desktop user

Commands:

```bash
stat /etc/cynic/registry.json
```

Observed 2026-10-22:

- `/etc/cynic/registry.json`: mode `0664`, owner `user:user`
- mtime 2026-10-17 18:53

Verdict: CONFIRMED, unchanged since audit.

## N-1 — kernel knowledge store corrupt, N-2 no backup, N-3 unsafe settings

Commands (read-only, secret values redacted):

```bash
grep -i url /etc/cynic/storage.toml
journalctl -u cynic-kernel --since -1h | grep -c 'checksum verification failed'
journalctl -u cynic-kernel --since 2026-10-17 | grep -c 'expire_stale failed'
journalctl -u surreal.service --since 2026-10-17 | grep -ci 'checksum\|corrupt'
journalctl -u surreal.service --since -1h | grep -ci 'checksum\|corrupt'
journalctl -u cynic-kernel --since -1h | grep -c 'introspection cycle: 0 alerts'
ls -la ~/.surrealdb/backups
du -sh ~/.surrealdb/data
uptime -s
systemctl show cynic-kernel -p NRestarts
systemctl is-active surreal.service
pgrep -a -f 'surreal start' (args truncated, credential not recorded)
```

Observed 2026-10-22:

- `storage.toml` url still `http://127.0.0.1:8000` (cortex-local store, not CT103)
- cynic-kernel last hour: 62 `checksum verification failed`
- cynic-kernel since 2026-10-17: 6646 `expire_stale failed`
- surreal.service since 2026-10-17: 294515 checksum/corrupt lines
- surreal.service last hour: 2880 checksum/corrupt lines
- cynic-kernel last hour: 12 `introspection cycle: 0 alerts` (blind to the corruption)
- `~/.surrealdb/backups/`: last files dated June, 83 bytes each (empty)
- `~/.surrealdb/data`: 213M
- `uptime -s`: 2026-10-15 19:58:49
- `cynic-kernel` NRestarts=0; `surreal.service` active
- surreal process still runs as `user` with credentials passed in argv and full capabilities allowed (exact argv value not recorded here)
- Auditor hot copy `cortex-surrealkv-data-hot-20261022_092801.tar.gz` not found on this host

Verdict: CONFIRMED and still active. N-2 and N-3 also still true.

## Conclusion

All three re-observed findings hold. No remediation step from report section 8 was applied on cortex at observation time. This file is evidence only.
