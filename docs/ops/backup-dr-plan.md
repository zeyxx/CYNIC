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

### Daily snapshots through organ-keep

Backup/restore is owned by organ-keep. Operators should inspect the storage primary and generate activation evidence through the organ entrypoint, not through standalone shell procedures.

```bash
python3 organs/keep/src/entrypoints/cli.py storage-primary status --json
python3 organs/keep/src/entrypoints/cli.py storage-primary bundle --output-dir ~/.cynic/backups/surrealdb
```

**Schedule:** Daily at 02:00 UTC (cron via systemd timer)

Timer ownership belongs in `infra/systemd/` and registry lifecycle activation evidence. Do not introduce ad-hoc backup timers outside the organ/deployment contract.

### RTO / RPO Targets

| Metric | Target | Notes |
|--------|--------|-------|
| **RTO** | 1 hour | Restore from backup, verify crystal integrity |
| **RPO** | 24 hours | Daily snapshots; maximum data loss = 1 day |
| **Retention** | 7 days | Weekly depth + monthly archival (future) |

## Disaster Recovery Procedure

### Step 1: Detect failure (5 min)

```bash
python3 organs/keep/src/entrypoints/cli.py storage-primary status --json
```

### Step 2: Assess corruption (10 min)

```bash
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://<HEADSCALE_CORE>:3030/crystals?limit=1
```

### Step 3: Restore from backup (30 min)

Restore must go through the registry lifecycle and storage-primary evidence path. Use `make lifecycle-blockers` to inspect readiness, then dispatch the appropriate lifecycle activation request through organ-forge/mempool. Direct filesystem restore commands are emergency operator actions, not documented organism entrypoints.

### Step 4: Verify data integrity (15 min)

```bash
# Sample crystal queries
curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://<HEADSCALE_CORE>:3030/crystals?limit=100 | jq '.[] | select(.state=="canonical")' | wc -l

# Expected: >0 canonical crystals
# If 0 → backup may be from early genesis (empty)
```

### Step 5: Resume operations (5 min)

- Verify kernel health: `curl http://<HEADSCALE_CORE>:3030/health`
- Resume inference: Dogs should be active
- K15 observation: POST recovery event to `/observe` for audit trail

## Long-term Strategy (Phase 2+)

### Monthly archival to external storage
- Store oldest backup offsite (USB drive, encrypted S3, etc.)
- Rotation: keep 3 monthly + 7 daily

### Replication to backup node (cynic-host-gpu)
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

- [ ] Keep organ-keep storage-primary status green
- [ ] Keep registry/systemd timer projection aligned through lifecycle activation
- [ ] Deploy: `sudo systemctl enable --now cynic-backup-surrealdb.timer`
- [ ] Document registry-backed restore lifecycle
- [ ] Keep manual restore actions outside source-tree entrypoints
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
