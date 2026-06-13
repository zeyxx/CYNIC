# Organ-X L0 Multi-Account Architecture — End-to-End Runbook

**Status**: Phase 1c (Code Changes) + Phase 1.5 (Recovery Daemon) COMPLETE

**Architecture Level**: L0 (Sequential single-active account with resilient recovery)

---

## What This Delivers

A resilient multi-account X.com capture system with automatic recovery:

1. **Primary Account** (@CynicOracle): CYNIC's production X account via email/password
2. **Secondary Account** (@jeanterre552): Your personal X account via Google OAuth (read-only mode)
3. **Automatic Recovery**: 4-layer cascade detects auth failures → retries → switches accounts → alerts kernel
4. **Data Isolation**: Each account has separate dataset.jsonl, chrome profile, verdicts directory
5. **Backward Compatibility**: Legacy symlinks allow existing code to read active account data

---

## Phase 1c: Code Changes (Already Applied)

Modified 7 files to make hermes-x account-aware:

| File | Change | Purpose |
|------|--------|---------|
| `get_x_credentials.py` | Reads account from env var + accounts.toml | Multi-account credential selection |
| `hermes_x_login.py` | Derives chrome profile from accounts.toml | Per-account browser profile |
| `launch-browser.sh` | Reads HERMES_ACCOUNT env var | Account selection at Chrome startup |
| `x_proxy.py` | Adds account_id field to tweets | Source tracking in dataset |
| `x_ingest_daemon.py` | Detects auth failures + posts heartbeat | Layer 1 failure detection |
| `hermes_x.rs` | Multi-account health aggregation | Kernel reports per-account status |
| `hermes-*.service` | Parameterized templates (@.service) | Account-specific instances |

---

## Phase 1.5: Recovery Daemon (Just Deployed)

Implements 4-layer resilience:

| Layer | Detection | Action | Result |
|-------|-----------|--------|--------|
| **1** | 2 heartbeat cycles with zero engagement_rate | Log WARN + continue | Signals auth failure |
| **2** | Failure persists 5+ min | Run `hermes_x_login.py --force` | Re-authenticate current account |
| **3** | Layer 2 fails 3× + fallback exists | `toggle-x-account.sh {fallback}` | Switch to secondary account |
| **4** | All layers fail | POST critical alert to kernel | Human intervention needed |

**New Files**:
- `hermes_x_recovery.py`: Daemon monitors heartbeat + implements cascade (351 lines)
- `toggle-x-account.sh`: Atomic account switching script (133 lines)
- `hermes-x-recovery@.service`: Systemd template for recovery daemon
- `test_recovery.py`: 10 integration tests (all pass ✓)

---

## Prerequisite Configuration

### 1. Create `~/.config/cynic/accounts.toml`

```toml
[accounts.cynic]
username = "CynicOracle"
profile = "~/.cynic/organs/hermes/x/chrome-profiles/cynic"
role = "primary"
password_env = "CYNIC_X_PASSWORD"
resume_on_failure = true

[accounts.personal]
username = "jeanterre552"
profile = "~/.cynic/organs/hermes/x/chrome-profiles/personal"
role = "read_only"
password_env = "PERSONAL_X_PASSWORD"
resume_on_failure = true  # Auto-fallback on CYNIC failure
```

### 2. Create Chrome Profile Directories

```bash
mkdir -p ~/.cynic/organs/hermes/x/{chrome-profiles,datasets}/{cynic,personal}
```

### 3. Set Environment Variables

In `~/.config/cynic/env` (systemd EnvironmentFile):

```bash
CYNIC_X_USERNAME="CynicOracle"
CYNIC_X_PASSWORD="<your-password-here>"        # For primary account  # HARDCODED_CREDS=placeholder-only
PERSONAL_X_PASSWORD="<optional-for-personal>"  # Leave empty for OAuth  # HARDCODED_CREDS=placeholder-only
HERMES_ACCOUNT="cynic"                         # Start with primary
CYNIC_REST_ADDR="http://127.0.0.1:3030"       # Kernel endpoint
CYNIC_API_KEY="<your-api-key>"                # Bearer token  # HARDCODED_CREDS=placeholder-only
```

---

## End-to-End Test Sequence

### Scenario 1: CYNIC Account Capture (Primary)

```bash
# 1. Start CYNIC account stack
systemctl --user start hermes-browser@cynic
systemctl --user start hermes-proxy@cynic
systemctl --user start hermes-browser-hub@cynic
systemctl --user start hermes-x-ingest@cynic
systemctl --user start hermes-x-recovery@cynic

# 2. Wait for services to stabilize
sleep 10

# 3. Verify services running
systemctl --user status hermes-browser@cynic
systemctl --user status hermes-x-ingest@cynic

# 4. Check dataset growth
tail -20 ~/.cynic/organs/hermes/x/datasets/cynic/dataset.jsonl

# 5. Verify symlinks updated
ls -la ~/.cynic/organs/hermes/x/dataset.jsonl  # Should point to datasets/cynic/
ls -la ~/.cynic/organs/hermes/x/ingest_cursor.txt

# 6. Check kernel observations
curl -s http://127.0.0.1:3030/observations?domain=hermes-x \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq '.observations[-5:] | .[] | {account: .context.account_id, status: .context.status}'
```

**Expected**: 
- ✓ Browser window opens to x.com/home
- ✓ Tweets captured every 30-60s
- ✓ dataset.jsonl grows: `tail` shows new tweets with account_id="cynic"
- ✓ Kernel receives /observe payloads with account_id="cynic"
- ✓ Symlinks point to datasets/cynic/

### Scenario 2: Switch to Personal Account (Fallback)

```bash
# 1. Stop CYNIC stack
./scripts/hermes-x/toggle-x-account.sh cynic  # Current active
systemctl --user is-active hermes-browser@cynic  # Should be inactive

# 2. Switch to personal
./scripts/hermes-x/toggle-x-account.sh personal

# 3. Verify stack restarted for personal
systemctl --user status hermes-browser@personal
systemctl --user status hermes-x-ingest@personal

# 4. Check symlinks updated
ls -la ~/.cynic/organs/hermes/x/dataset.jsonl  # Should point to datasets/personal/

# 5. Monitor new tweets
tail -f ~/.cynic/organs/hermes/x/datasets/personal/dataset.jsonl | grep -o '"account_id":"[^"]*"'
```

**Expected**:
- ✓ All CYNIC services stop gracefully
- ✓ Symlinks update to datasets/personal/
- ✓ Personal account stack starts
- ✓ New tweets captured with account_id="personal"

### Scenario 3: Auth Failure Detection & Recovery (Layer 1-2)

```bash
# 1. Revoke CYNIC X.com session (from another browser)
#    OR simulate auth failure by:
#    - Log out of X.com in the browser window
#    - Wait 2 heartbeat cycles (~60 seconds)

# 2. Watch recovery daemon detect failure
journalctl --user -u hermes-x-recovery@cynic.service -f

# Expected logs:
# [WARN] Auth failure detected (1/2)
# [WARN] Auth failure detected (2/2)
# [INFO] [Layer 2] Retrying login (attempt 1/3)
# [INFO] [Layer 2] Re-login successful

# 3. Verify dataset resumes growing (after re-login)
tail -f ~/.cynic/organs/hermes/x/datasets/cynic/dataset.jsonl

# 4. Check recovery.log
tail -20 ~/.cynic/organs/hermes/x/recovery.log
```

**Expected**:
- ✓ Recovery daemon detects zero engagement_rate in tweets
- ✓ Layer 2 auto-re-login triggers
- ✓ hermes_x_login.py runs, authenticates CYNIC account
- ✓ Dataset resumes growing
- ✓ recovery.log shows: `2026-05-12T18:45:00 [cynic] Auth failure detected → re-login → SUCCESS`

### Scenario 4: Auth Failure Escalation (Layer 3)

```bash
# 1. Ensure personal account is configured as fallback in accounts.toml
#    resume_on_failure = true

# 2. Revoke CYNIC session AND make re-login impossible:
#    - Wrong password in CYNIC_X_PASSWORD env var (or don't set it)
#    - Wait for Layer 2 to exhaust (3 retries, ~5 min)

# 3. Watch recovery daemon escalate:
journalctl --user -u hermes-x-recovery@cynic.service -f

# Expected logs:
# [WARN] Auth failure detected (1/2)
# [WARN] Auth failure detected (2/2)
# [INFO] [Layer 2] Retrying login (attempt 1/3)
# [ERROR] [Layer 2] Re-login failed: <error reason>
# [INFO] [Layer 2] Retrying login (attempt 2/3)
# [ERROR] [Layer 2] Re-login failed
# [INFO] [Layer 2] Retrying login (attempt 3/3)
# [ERROR] [Layer 2] Re-login failed
# [INFO] [Layer 3] Switching to fallback account: personal
# [INFO] [Layer 3] Successfully switched to personal

# 4. Verify account switched
systemctl --user is-active hermes-browser@cynic     # Should be inactive
systemctl --user is-active hermes-browser@personal  # Should be active

# 5. Verify personal account now captures
tail -f ~/.cynic/organs/hermes/x/datasets/personal/dataset.jsonl | grep '"account_id":"personal"'
```

**Expected**:
- ✓ Layer 2 attempts 3 times, all fail
- ✓ Layer 3 detects fallback (personal) configured
- ✓ toggle-x-account.sh switches services atomically
- ✓ Symlinks update to datasets/personal/
- ✓ Personal account dataset resumes growing
- ✓ recovery.log shows Layer 3 success

### Scenario 5: Critical Alert (Layer 4)

```bash
# 1. Make both accounts unreachable:
#    - Revoke both CYNIC and personal X.com sessions
#    - Make fallback account unavailable
#    - Ensure Layer 2 + Layer 3 both fail

# 2. Wait for Layer 4 alert:
journalctl --user -u hermes-x-recovery@cynic.service -f

# Expected logs:
# [CRITICAL] [Layer 4] All recovery attempts failed for cynic. Alerting kernel.
# [INFO] [Layer 4] Critical alert sent to kernel

# 3. Check kernel received the alert:
curl -s http://127.0.0.1:3030/observations?domain=hermes-x&tags=recovery-failed \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq '.observations[-1]'

# Expected response:
# {
#   "domain": "hermes-x",
#   "context": {
#     "severity": "critical",
#     "reason": "x_auth_expired",
#     "layer2_retries": 3,
#     "layer3_retries": 1
#   },
#   "tags": ["account:cynic", "recovery-failed"]
# }

# 4. Recovery.log shows full cascade attempt:
tail -30 ~/.cynic/organs/hermes/x/recovery.log
```

**Expected**:
- ✓ All 4 layers exhausted
- ✓ Critical alert POSTed to kernel
- ✓ recovery.log documents full cascade attempt
- ✓ **MANUAL**: User must fix credentials or contact T.

---

## Health Check Commands

### Check All Services Running

```bash
systemctl --user list-units 'hermes-*@cynic.service' --state=running
systemctl --user list-units 'hermes-*@personal.service' --state=running
```

### Check Recovery Daemon Status

```bash
systemctl --user status hermes-x-recovery@cynic.service
journalctl --user -u hermes-x-recovery@cynic.service -n 20
```

### Check Kernel Health

```bash
curl -s http://127.0.0.1:3030/health \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq '.organs.hermes_x'

# Expected:
# {
#   "cynic": "Alive (data: 30 min old)",
#   "personal": "Degraded (data: 8+ hours old)"
# }
```

### Check Data Flow

```bash
# CYNIC dataset growth
wc -l ~/.cynic/organs/hermes/x/datasets/cynic/dataset.jsonl
ls -lah ~/.cynic/organs/hermes/x/datasets/cynic/dataset.jsonl

# Personal dataset growth
wc -l ~/.cynic/organs/hermes/x/datasets/personal/dataset.jsonl

# Tail with account_id visible
tail -5 ~/.cynic/organs/hermes/x/dataset.jsonl | jq '.account_id'
```

---

## Architecture Diagram (Text)

```
┌─────────────────────────────────────────────────────┐
│ Systemd Unit Pool (parameterized by account)       │
├──────────────────────┬──────────────────────────────┤
│   CYNIC Account      │  Personal Account           │
│   (ACTIVE)           │  (INACTIVE)                 │
├──────────────────────┼──────────────────────────────┤
│ hermes-browser@cynic │ hermes-browser@personal     │
│ hermes-proxy@cynic   │ hermes-proxy@personal       │
│ hermes-hub@cynic     │ hermes-hub@personal         │
│ hermes-ingest@cynic  │ hermes-ingest@personal      │
│ hermes-recovery@cynic│ hermes-recovery@personal    │
└──────────────────────┴──────────────────────────────┘
      ↓ (ACTIVE ONLY)
┌──────────────────────────────────────────────────────┐
│ Chrome (CYNIC Profile)                             │
│   - X.com session cookies (cached)                 │
│   - Logged in as @CynicOracle                      │
│   - CDP :40769 → Browser Hub :40770 → Proxy :8888 │
└──────────────────────────────────────────────────────┘
      ↓ (mitmproxy addon)
┌──────────────────────────────────────────────────────┐
│ x_proxy.py (enrichment)                            │
│   - Captures tweets from Twitter API responses     │
│   - Adds account_id="cynic" to each tweet          │
│   - Writes to dataset.jsonl (account-specific)     │
└──────────────────────────────────────────────────────┘
      ↓ (file I/O)
┌──────────────────────────────────────────────────────┐
│ ~/.cynic/organs/hermes/x/datasets/cynic/           │
│   - dataset.jsonl (live captured tweets)           │
│   - ingest_cursor.txt (position in dataset)        │
│   - observations/ (metadata)                       │
└──────────────────────────────────────────────────────┘
      ↓ (tail + enrich)
┌──────────────────────────────────────────────────────┐
│ x_ingest_daemon.py (judgment interface)            │
│   1. Detects auth failures (no engagement_rate)    │
│   2. Posts heartbeat to kernel with status         │
│   3. Posts observations for tweets to /judge       │
│   4. Tracks cursor position per account            │
└──────────────────────────────────────────────────────┘
      ├─→ (heartbeat with failure_reason="x_auth_expired")
      │
      └─→ http://127.0.0.1:3030/observe
            (account_id="cynic", status="critical")
                    ↓
┌──────────────────────────────────────────────────────┐
│ CYNIC Kernel (/observations endpoint)              │
│   - Receives heartbeat observations                │
│   - Reports organ health (alive/degraded)          │
│   - Available for recovery daemon to query         │
└──────────────────────────────────────────────────────┘
            ↑
        (polls /observations)
            │
┌──────────────────────────────────────────────────────┐
│ hermes_x_recovery.py (recovery daemon)             │
│                                                    │
│ Layer 1: Detect auth failure (≥2 cycles)          │
│    ↓                                               │
│ Layer 2: Run hermes_x_login.py --force (3×)       │
│    ├─ SUCCESS → reset counters, continue          │
│    ├─ FAIL 3× → proceed to Layer 3                │
│    ↓                                               │
│ Layer 3: toggle-x-account.sh personal             │
│    ├─ SUCCESS → account switched, continue        │
│    ├─ FAIL → proceed to Layer 4                   │
│    ↓                                               │
│ Layer 4: POST critical alert to kernel            │
│          (manual intervention required)           │
└──────────────────────────────────────────────────────┘
```

---

## Troubleshooting

| Symptom | Diagnosis | Fix |
|---------|-----------|-----|
| Browser doesn't open | Chrome binary not found | Set `$CHROME_BIN` or verify `/opt/google/chrome/chrome` exists |
| Account not selected | HERMES_ACCOUNT env var not set | `export HERMES_ACCOUNT=cynic` before systemctl start |
| Services fail to start | Profile directory missing | `mkdir -p ~/.cynic/organs/hermes/x/chrome-profiles/{cynic,personal}` |
| Recovery daemon crashes | requests library missing | `pip3 install requests` |
| No tweets captured | Proxy not listening | `systemctl --user status hermes-proxy@cynic` |
| Recovery not detecting failure | engagement_rate field check wrong | Check proxy logs: `journalctl --user -u hermes-proxy@cynic` |
| Symlinks not updating | toggle script permission issue | `chmod +x scripts/hermes-x/toggle-x-account.sh` |

---

## What's Next: Phase 2 (Future)

**L2 Pooled Architecture** (not in Phase 1.5):

- Run both accounts in parallel (not sequential switching)
- Single shared proxy demultiplexes by account ID
- Both datasets grow simultaneously
- Reduces failover latency from 2+ minutes to instant
- Adds proxy complexity but eliminates service restart overhead

---

## Key Differences from Phase 1c

| Aspect | Phase 1c | Phase 1.5 |
|--------|---------|----------|
| Account awareness | ✓ (code) | ✓ (daemon) |
| Data isolation | ✓ | ✓ |
| Failure detection | ✓ (via heartbeat) | ✓ (daemon monitors) |
| Automatic recovery | ✗ | ✓ (4 layers) |
| Account switching | ✗ | ✓ (toggle-x-account.sh) |
| Kernel alerting | ✗ | ✓ (Layer 4) |
| Production ready | Partial | YES |

---

## Commands Reference

```bash
# Start full CYNIC stack
systemctl --user start hermes-{browser,proxy,browser-hub,x-ingest,x-recovery}@cynic

# Stop full CYNIC stack
systemctl --user stop hermes-{browser,proxy,browser-hub,x-ingest,x-recovery}@cynic

# Switch to personal
~/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/hermes-x/toggle-x-account.sh personal

# Switch back to CYNIC
~/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/hermes-x/toggle-x-account.sh cynic

# Watch recovery daemon in real-time
journalctl --user -u hermes-x-recovery@cynic.service -f

# View last 50 recovery log lines
tail -50 ~/.cynic/organs/hermes/x/recovery.log

# Check if CYNIC dataset growing
watch -n 1 'wc -l ~/.cynic/organs/hermes/x/datasets/cynic/dataset.jsonl'

# Extract account_id from current tweets
tail -10 ~/.cynic/organs/hermes/x/dataset.jsonl | jq '.account_id' | sort | uniq -c
```

---

**Ready to observe end-to-end?** Start with Scenario 1 above. You should see tweets flowing into the dataset within 30-60 seconds of browser launch.
