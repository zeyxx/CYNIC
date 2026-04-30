# CYNIC Backup & Disaster Recovery Plan

**K15 Principle:** Every producer (storage) needs a consumer (backup). Observed 2026-04-30: SurrealDB has zero backup — critical data loss vector.

## Current State

| Component | Size | Location | State | RTO | RPO |
|-----------|------|----------|-------|-----|-----|
| SurrealDB | 5.3 GB | /home/user/.surrealdb/data | Single copy, no backup | TBD | Total loss |
| PostgreSQL | ~2 GB | /var/lib/postgresql/ | Single copy, no backup | TBD | Total loss |
| Git history | Variable | /home/user/Bureau/CYNIC/.git | Local + GitHub | N/A | N/A |

## Risk Assessment

**CRITICAL:** SurrealDB contains:
- Crystal verdicts (training data for ML)
- Observations (K15 sensor readings)
- Session compliance scores
- Agent task state

**Compromise scenarios:**
1. **Disk failure** → Total loss of crystal pipeline data → training loop broken
2. **Corruption** → Cascading data poisoning (bad crystals → bad Dogs)
3. **Ransomware** → Encrypted database, no recovery path

## Backup Strategy (Phase 1)

### Daily snapshots to local disk

```bash
#!/bin/bash
# backup-surrealdb-daily.sh
BACKUP_DIR=~/.cynic/backups/surrealdb
mkdir -p "$BACKUP_DIR"

# Stop surreal, backup, restart
systemctl stop surreal

# Tar + compress the data directory
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
tar --xattrs -czf "$BACKUP_DIR/surrealdb-$TIMESTAMP.tar.gz" \
  /home/user/.surrealdb/data

# Verify tarball integrity
tar -tzf "$BACKUP_DIR/surrealdb-$TIMESTAMP.tar.gz" > /dev/null && \
  echo "✓ Backup $TIMESTAMP verified" || \
  { echo "✗ Backup failed"; exit 1; }

systemctl start surreal

# Keep only last 7 days
find "$BACKUP_DIR" -name "surrealdb-*.tar.gz" -mtime +7 -delete
```

**Schedule:** Daily at 02:00 UTC (cron via systemd timer)

```ini
# /etc/systemd/system/cynic-backup-surrealdb.timer
[Unit]
Description=Daily SurrealDB backup
Requires=cynic-backup-surrealdb.service

[Timer]
OnCalendar=daily
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

### RTO / RPO Targets

| Metric | Target | Notes |
|--------|--------|-------|
| **RTO** | 1 hour | Restore from backup, verify crystal integrity |
| **RPO** | 24 hours | Daily snapshots; maximum data loss = 1 day |
| **Retention** | 7 days | Weekly depth + monthly archival (future) |

## Disaster Recovery Procedure

### Step 1: Detect failure (5 min)

```bash
# Liveness check
curl -sf http://<TAILSCALE_CORE>:3030/ready || {
  echo "ALERT: SurrealDB unhealthy"
  # POST alert to /observe with K15 tag
}
```

### Step 2: Assess corruption (10 min)

```bash
# Try querying a known crystal
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://<TAILSCALE_CORE>:3030/crystals?limit=1

# If response = error → corruption likely
```

### Step 3: Restore from backup (30 min)

```bash
#!/bin/bash
# restore-surrealdb.sh BACKUP_FILE

BACKUP_FILE="$1"
systemctl stop surreal

# Backup current (corrupted) data
mv /home/user/.surrealdb/data \
   /home/user/.surrealdb/data.corrupted-$(date +%s)

# Extract backup
mkdir -p /home/user/.surrealdb/data
tar -xzf "$BACKUP_FILE" -C /home/user/.surrealdb/data --strip-components=1

# Verify structure
[ -d "/home/user/.surrealdb/data/sstables" ] || {
  echo "✗ Restore failed: missing sstables"
  exit 1
}

systemctl start surreal

# Health check
sleep 3
curl -sf http://<TAILSCALE_CORE>:3030/ready || {
  echo "✗ Restore failed: not ready"
  exit 1
}

echo "✓ Restored from $BACKUP_FILE"
```

### Step 4: Verify data integrity (15 min)

```bash
# Sample crystal queries
curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://<TAILSCALE_CORE>:3030/crystals?limit=100 | jq '.[] | select(.state=="canonical")' | wc -l

# Expected: >0 canonical crystals
# If 0 → backup may be from early genesis (empty)
```

### Step 5: Resume operations (5 min)

- Verify kernel health: `curl http://<TAILSCALE_CORE>:3030/health`
- Resume inference: Dogs should be active
- K15 observation: POST recovery event to `/observe` for audit trail

## Long-term Strategy (Phase 2+)

### Monthly archival to external storage
- Store oldest backup offsite (USB drive, encrypted S3, etc.)
- Rotation: keep 3 monthly + 7 daily

### Replication to backup node (cynic-gpu)
- Real-time replication via SurrealDB triggers
- Reduces RTO from 1h to 5min
- Requires network stability between nodes

### PostgreSQL backup
- Similar daily snapshots to ~/.cynic/backups/postgresql
- Current KAIROS data (non-critical but useful)
- RPO = 24 hours sufficient for timeseries

## Testing Schedule

| Frequency | Test | Owner |
|-----------|------|-------|
| Weekly | Verify backup file exists and is readable | T. (manual check) |
| Monthly | Restore to test VM, verify data integrity | T. (full restore test) |
| Quarterly | RTO/RPO audit (can we actually restore in 1h?) | T. + automated logs |

## Implementation TODO

- [ ] Create backup-surrealdb.sh script
- [ ] Create systemd timer for daily backups
- [ ] Deploy: `sudo systemctl enable --now cynic-backup-surrealdb.timer`
- [ ] Create restore-surrealdb.sh recovery procedure
- [ ] Document manual restore steps in runbook
- [ ] Test restore on non-production VM
- [ ] Archive first 3 backups offsite (USB, S3)
- [ ] Set calendar reminder: monthly review of backup sizes

## Notes

**K15 consumer:** This backup plan IS the consumer for stored data. The feedback loop:
- Kernel writes verdicts → SurrealDB stores
- Backup reads storage → verifies persistenc
- Restore proves durability

Without this, storage is "fire and forget" (violation).

**Encrypt backups:** If storing offsite, use:
```bash
gpg --symmetric --cipher-algo AES256 surrealdb-TIMESTAMP.tar.gz
```

Password: store in vault (encrypted, separate from this document)

---

**Audit trail:** This document should be reviewed quarterly as part of K15 compliance audits.

*Created: 2026-04-30 | Last reviewed: 2026-04-30*
