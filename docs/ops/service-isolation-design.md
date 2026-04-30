# Service Isolation Architecture

**Status:** STUDY (Task #12)  
**Updated:** 2026-04-30  
**Principle:** Least privilege — each service runs under its own non-root user with minimal filesystem/network access.

---

## Current State

All services run as `user` (non-root but shared):
- cynic-kernel (PID ~1407151) — User=user
- llama-server (PID ~1407152) — User=user  
- surreal (PID ~1522678) — User=user
- hermes-k15-consumer — User=root (K15 consumer needs host access)
- hermes-infrastructure-monitor — User=root (remediate calls need ts_exec)

**Risk:** If one service is compromised, attacker has access to all shared files under `user` account (kernels, model caches, databases).

---

## Proposed Architecture

### Service → User Mapping

| Service | New User | Home | Files | Network | Rationale |
|---------|----------|------|-------|---------|-----------|
| cynic-kernel | `cynic` | `/home/cynic` | /home/cynic/Bureau/CYNIC/.cynic | <TAILSCALE_CORE>:3030 | Core logic engine |
| llama-server | `llama` | `/home/llama` | /home/llama/llama.cpp | <TAILSCALE_CORE>:8080 | Inference engine, isolated |
| surreal | `surreal` | `/home/surreal` | /home/surreal/.surrealdb | 127.0.0.1:8000 | Database, localhost only |
| hermes-k15-consumer | `root` | `/root` | /root/.cynic-env | <TAILSCALE_CORE>:3030 | Consumer reads observations |
| hermes-infrastructure-monitor | `root` | `/root` | /root/.cynic-env | Tailscale mesh | Recovery needs ts_exec |

---

## Implementation Strategy

### Phase 1: User Creation

Create system users with no login shell:
- `useradd -r -s /bin/false -d /home/cynic cynic`
- `useradd -r -s /bin/false -d /home/llama llama`
- `useradd -r -s /bin/false -d /home/surreal surreal`

### Phase 2: Directory Structure

For each service, create home directory with state directories:

**cynic-kernel:**
- `/home/cynic/.cynic` — Move/copy from `/home/user/.cynic`
- Symlink: `/home/cynic/Bureau/CYNIC` → `/home/user/Bureau/CYNIC`

**llama-server:**
- `/home/llama/llama.cpp` — Symlink to `/home/user/llama.cpp` (binary location)
- Model files readable via symlink

**surreal:**
- `/home/surreal/.surrealdb` — Move from `/home/user/.surrealdb`

### Phase 3: Service Updates

Update systemd units:
- Change `User=user` → `User=cynic|llama|surreal`
- Update `ReadWritePaths=` to point to service home directories
- Verify EnvironmentFile paths (may need to copy configs to /etc/)

### Phase 4: Testing

Falsification tests to verify isolation:
1. Service startup: `systemctl restart <service>` succeeds
2. Service communication: cross-service APIs work (kernel ↔ DB)
3. Isolation: one service cannot read another's data
4. Symlinks: indirect file access works correctly

---

## Configuration Dependencies

### Potential Blockers

**EnvironmentFile paths:**
- cynic-kernel: `/home/user/.config/cynic/env`
- surreal, llama-server: Check what env files they read

Solution: Copy to `/etc/cynic/env` (root-owned, readable by all), or adjust group ownership.

**Backup locations:**
- Cron jobs may reference `/home/user/.cynic/backups/`
- Need to verify restore scripts work after move to `/home/cynic/.cynic/backups/`

**Log file ownership:**
- StandardOutput=journal handles this automatically
- File-based logs (if any) need ownership adjustment

---

## Rollback Plan

If isolation breaks a service:
1. Immediately revert `User=` back to `user` in systemd unit
2. Check logs: `journalctl -u <service> -n 50`
3. Identify permission error
4. Adjust ownership/symlinks based on error message
5. Re-test with updated configuration

---

## Benefits

✓ **Breach isolation:** Compromised service cannot access others' data  
✓ **Audit trail:** systemd journal shows which service performed actions  
✓ **Future hardening:** Per-service CapabilitySet, UMask, ulimits  
✓ **Compliance:** Aligns with CIS Benchmarks for service isolation  

---

## References

- systemd.exec(5) — User=, ReadWritePaths=, PrivateDevices=  
- useradd(8) — Create system users  
- CYNIC CLAUDE.md § Security & Least Privilege
