## Systemd Service Rules

### SYS1 — User-Level Service Constraints (NEW)

User-level systemd services (`~/.config/systemd/user/`) are fundamentally different from system-level services (`/etc/systemd/system/`). Violations cause silent failures or cryptic errors.

**Immutable Constraints:**

1. **NO User=/Group= directives.** User-level services always run as the current user. Attempting `User=user` or `Group=user` in a user-level service causes `exit 216/GROUP` with "Operation not permitted". The constraint is enforced at the kernel level.

2. **ExecStart paths are relative to WorkingDirectory unless absolute.** If a script is in a subdirectory (e.g., `core/`), you must specify `ExecStart=/usr/bin/python3 core/script.py` (relative to WorkingDirectory). Omitting the subdirectory path will fail with `[Errno 2] No such file or directory`.

3. **Validate before deployment:** After any user-level systemd service edit:
   ```bash
   systemctl --user daemon-reload
   systemctl --user restart <unit>
   systemctl --user status <unit>
   journalctl --user -xeu <unit>
   ```

**Observed Violation (2026-05-01):**

hermes-curation.service and hermes-search-generator.service:
- Had User=user/Group=user directives (inherited from system-level template)
- Had ExecStart paths without subdirectory: `ExecStart=/usr/bin/python3 curate_domain_signals.py` (script is at `core/curate_domain_signals.py`)
- Result: Both failed silently on every timer trigger, users unaware for weeks

**Fix Deployed (2026-05-01):**

1. Removed User=/Group= directives (user-level services can't use them)
2. Updated ExecStart: `curate_domain_signals.py` → `core/curate_domain_signals.py`
3. Both services now execute with status 0/SUCCESS

**Detection & Prevention:**

- Pre-merge: Grep for `User=` or `Group=` in any file under `.config/systemd/user/` — automatic REJECT
- Deployment: Copy from `/home/user/Bureau/CYNIC/infra/systemd/` to `~/.config/systemd/user/` only when constraint-compliant
- Make target: `make validate-systemd-user` (falsifies SYS1, SYS2, SYS3)

**Pattern Trigger:**

Recurring issue: Services run as wrong user (root or nobody), hardcoded paths break. Root cause: system-level service templates copied to user-level without constraint validation. **Rule enforces the delta.**

---

### SYS2 — Service Path Resolution (EXISTING, REINFORCED)

ExecStart paths in systemd services resolve as follows:

- If starts with `/`: absolute path, used as-is
- Otherwise: relative to WorkingDirectory

When moving a script into a subdirectory, the service ExecStart must be updated accordingly. This is not automatic.

**Anti-pattern:**
```ini
WorkingDirectory=/home/user/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 curate_domain_signals.py  # ❌ Looks for script at root level
```

**Correct pattern:**
```ini
WorkingDirectory=/home/user/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 core/curate_domain_signals.py  # ✓ Relative path from WorkingDirectory
```

---

### SYS3 — Service Lifecycle Validation (NEW)

Every systemd service modification must pass 3 validation steps before trusting it:

1. **Syntax check:** `systemctl --user daemon-reload` (or `sudo systemctl daemon-reload` for system services) — catches TOML/INI errors
2. **Execution test:** `systemctl --user restart <unit>` — catches runtime failures (missing executable, permission issues, binding conflicts)
3. **Status & logs:** 
   - `systemctl --user status <unit>` — confirms Active/inactive state
   - `journalctl --user -xeu <unit>` — shows last failure if any

**Shorthand after edit:**
```bash
systemctl --user daemon-reload && systemctl --user restart hermes-curation && journalctl --user -xeu hermes-curation | tail -20
```

**Anti-pattern:** Editing a service file, assuming it works, and only discovering failure weeks later when a timer misses a cycle.

---

### SYS4 — Remote Task Restart Validation (NEW)

When a systemd service or task scheduler job (e.g., Windows schtasks) is restarted remotely (via SSH or API), the restart must be **verified** to have actually taken effect on the remote host.

**Observed Violation (2026-05-03):**

GPU backend (`titou@<TAILSCALE_GPU>:8080`):
- schtasks CynicSovereign task configured with `-c 131072` (131K context)
- Task restarted via SSH: `ssh titou@<TAILSCALE_GPU> schtasks /run /tn CynicSovereign`
- SSH command returned success ("Opération réussie")
- BUT: Process still running with `-c 65536` (65K context)
- Root cause: Either process was not fully killed, or task was queued but not executed, or old llama-server process resumed instead of new one starting

**Result:** Context drift persisted. Kernel health loop continuously detected mismatch.

**Prevention:**

1. **After remote restart, probe the service state immediately** — don't just check for successful command execution
2. **Validate actual state matches expected state** — not just "healthy", but "healthy AND context=131072"
3. **Add verification probe to orchestrator** — Soma Layer 1 must validate GPU context via `/props` endpoint before marking ready
4. **Set timeout for recovery** — if context drift persists for >3 probes, escalate (don't keep retrying)

**Fix Deployed (2026-05-03):**

Soma Layer 1 (`layer1_manager.py`):
- Validates GPU context on every probe (checks `/props` for `n_ctx_train`)
- If actual != expected: treats as probe failure, triggers recovery retry
- Max 3 retries before escalating
- Logs context mismatches explicitly for debugging

**Implementation:**

```python
# In BackendProbe.check():
if self.expected_context:
    props = await client.get(f"{self.url}/props", timeout=...)
    actual_ctx = props.json().get("default_model_meta", {}).get("n_ctx_train", 0)
    if actual_ctx != self.expected_context:
        raise Exception(f"context drift: actual={actual_ctx}, expected={self.expected_context}")
    # If context valid: mark healthy
```

**Next Step:**

GPU endpoint `/props` requires API key auth. Either:
- A) Configure Soma Layer 1 with GPU API key (simplest)
- B) Manually verify Windows task configuration and restart llama-server.exe with correct flags
- C) Accept degraded state: update backends.toml line 35 to `context_size = 65536`

---

## Enforcement

- Linter: `make lint-systemd-user` (NEW) — checks for User=/Group= in ~/.config/systemd/user/, validates ExecStart paths exist
- Hook: Pre-push validates all systemd files under `.claude/rules/systemd.md` compliance
- Testing: SYS1, SYS2, SYS3, SYS4 falsification tests in `make test-gates`
- Soma Layer 1: Enforces SYS4 for GPU backend context validation (probes `/props` endpoint every 30s)
