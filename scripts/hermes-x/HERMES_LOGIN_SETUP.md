# Hermes X.com Authentication Setup

## Overview

The dual enrichment approach (CDP + index) requires Chrome to be authenticated on x.com. This document covers:
1. Credential storage (GPG encrypted, memory-only at runtime)
2. Headless login automation (Playwright)
3. Integration with hermes-behavior service lifecycle

## Architecture

```
Session Start
  ↓
systemd starts hermes-x-login.service (oneshot)
  ├─ Calls hermes_x_login.py
  ├─ Prompts for X password (env var or interactive)
  ├─ Uses Playwright to navigate x.com
  ├─ Fills login form (username + password)
  ├─ Waits for auth redirect
  └─ Saves cookies to persistent Chrome profile
  ↓
After login succeeds, systemd starts hermes-behavior.service
  ├─ Launches behavior_logger.py
  ├─ Chrome auto-loads persisted cookies
  ├─ behavior_logger enriches clicks via CDP (authenticated)
  └─ Falls back to index lookup for unavailable clicks
```

## Credentials

Stored in **memory only** during login. Never persisted to plaintext files.

### Option 1: Environment Variable (Recommended for CI/scripts)

```bash
export CYNIC_X_USERNAME="@CynicOracle"  # Optional, defaults to @CynicOracle
export CYNIC_X_PASSWORD="<actual_password>"
systemctl --user start hermes-x-login.service
```

### Option 2: Interactive Prompt (Default)

```bash
systemctl --user start hermes-x-login.service
# Script will prompt: "X.com password: "
# Type password (hidden)
```

### Option 3: GPG Encrypted Storage (Optional, for repeated logins)

Store encrypted password for reuse:

```bash
python3 core/get_x_credentials.py store
# Prompts: "X.com password to store (encrypted): "
# Encrypts and saves to ~/.cynic/organs/hermes/x/.x_password.gpg
```

Then decrypt at login:

```bash
export CYNIC_X_PASSWORD=$(gpg --decrypt ~/.cynic/organs/hermes/x/.x_password.gpg)
systemctl --user start hermes-x-login.service
```

## First-Time Setup

### 1. Verify GPG key exists

```bash
gpg --list-secret-keys | grep CYNIC
# Should show: CYNIC <cynic@agentmail.to>
```

If not found, key was created during this setup.

### 2. Start login service

**With environment variable (no prompt):**
```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/hermes-x
CYNIC_X_PASSWORD="<your_x_password>" systemctl --user start hermes-x-login.service
```

**With interactive prompt:**
```bash
systemctl --user start hermes-x-login.service
# Prompts for password
```

### 3. Check login status

```bash
journalctl --user -u hermes-x-login.service -n 30
# Look for: "✓ Successfully authenticated"
```

### 4. Verify cookies were saved

```bash
ls -la ~/.cynic/organs/hermes/x/chrome-profile/Default/
# Should show: Cookies (SQLite database with X session)
```

### 5. Start behavior_logger

```bash
systemctl --user start hermes-behavior.service
```

Behavior logger will now:
- Launch Chrome with authenticated cookies
- Begin capturing clicks
- Enrich with tweet_ids via CDP (primary) or index (fallback)

### 6. Monitor enrichment

```bash
python3 data_architecture/live_enrichment_monitor.py
# Watch enrichment rate climb from 0% toward 60%+
```

## Troubleshooting

### "X.com password: " prompt doesn't appear

**Cause:** Running in non-interactive context (cron, background service)  
**Fix:** Provide `CYNIC_X_PASSWORD` environment variable

```bash
CYNIC_X_PASSWORD="..." systemctl --user start hermes-x-login.service
```

### Login times out at password field

**Cause:** X.com UI changes, login flow differs  
**Fix:** Update `hermes_x_login.py` selectors for current X.com DOM structure

### Chrome profile shows no cookies

**Cause:** Playwright closed browser before cookies flushed  
**Fix:** Check logs for errors, re-run login manually

```bash
systemctl --user restart hermes-x-login.service
journalctl --user -u hermes-x-login.service -n 50
```

### Enrichment still 0% after login

**Cause 1:** CDP WebSocket not responding  
**Fix:** Check CDP endpoint:
```bash
curl http://127.0.0.1:40769/json | jq 'length'
# Should return > 0
```

**Cause 2:** Clicks on login page still (browser navigated to login)  
**Fix:** Check recent clicks:
```bash
tail -100 ~/.cynic/organs/hermes/behavior/behavior_log.jsonl | jq -r 'select(.type=="click") | .url' | sort | uniq -c | sort -rn
```

**Cause 3:** Index not built  
**Fix:** Rebuild index:
```bash
python3 data_architecture/phase_1b_mitmproxy_tweet_map.py
```

## Ongoing Use

### Automatic at system startup

Add to user's systemd default target:

```bash
systemctl --user enable hermes-x-login.service hermes-behavior.service
systemctl --user start hermes-x-login.service
# Prompts for password (one-time per boot)
```

### On-demand re-login (if session expires)

```bash
systemctl --user restart hermes-x-login.service
# Clears old cookies, re-authenticates
```

### Changing X account credentials

```bash
# Store new password encrypted
python3 core/get_x_credentials.py store

# Or update GPG-encrypted file directly
gpg --decrypt ~/.cynic/organs/hermes/x/.x_password.gpg | \
  gpg --trust-model always --encrypt --armor > ~/.cynic/organs/hermes/x/.x_password.gpg.new
mv ~/.cynic/organs/hermes/x/.x_password.gpg.new ~/.cynic/organs/hermes/x/.x_password.gpg

# Then re-login
systemctl --user restart hermes-x-login.service
```

## Security Notes

- **Password in memory only:** Never written to disk except during optional GPG encryption
- **GPG encrypted storage:** Uses CYNIC's GPG key (symmetric or public key auth possible)
- **Persistent cookies:** Stored in Chrome profile with standard OS file permissions
- **Environment variables:** Readable via `ps` — avoid on shared systems (use GPG or manual prompt)

## Exit Codes

```
hermes_x_login.py exit code meanings:
0 - Success (authenticated, cookies saved)
1 - Login failed (wrong credentials, page changed)
2 - Browser/Playwright error (install or crash)
```

Check with:
```bash
systemctl --user start hermes-x-login.service
echo $?
```
