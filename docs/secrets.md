# Secrets (industrial, frictionless, agnostic)

Secrets are **not** git content and **not** kernel SSOT. They are injected
material classified by *when* they may exist.

## Secret classes (host bootstrap law)

| Class | When allowed | Examples |
|-------|----------------|----------|
| `HumanAdmin` | bootstrap only (console/once) | human password, sudo gesture, Forgejo login |
| `MachineEnrollment` | first join / enroll | install token, one-time join key |
| `RecoveryBreakglass` | incident only | emergency root, recovery codes |
| `RuntimeMachineIdentity` | **runtime steady-state** | llama API key file, machine TLS, helper identity |

Rule already in host bootstrap: **runtime must not depend on HumanAdmin**
(`runtime_human_secret_dependency` stays blocked until bridge/helper replaces
human sudo for steady work).

## Placement (agnostic — no vendor lock)

Prefer this order of authority for *runtime* secrets:

1. **Env inject** (process scope): `CYNIC_BACKEND_<NAME>_API_KEY`,
   `CYNIC_CHAT_API_KEY`, `FORGEJO_TOKEN` for one-shot tools.
2. **Principal-owned file** (mode `0640`, owner `cynic`, group `cynic`):
   - `$PRINCIPAL_HOME/.config/cynic/<name>` (e.g. `llama-api-key`)
   - optional mirror `/etc/cynic/<name>` root:cynic for installed SSOT
3. **OS secret store adapter** (optional later): systemd-creds, kernel keyring,
   age/sops, external vault — always behind a port, never hardcoded provider.
4. **Never**: git, `infra/agent-exposure.json`, docs, chat JSONL, proof JSON.

Config that is *not* secret (URLs, model names, pins) stays in
`inference.toml` / pins / registry. TOML may have a stub `api_key` only for
tests; env/file wins in loaders.

## Frictionless resolution (already partially implemented)

- Inference backends: `CYNIC_BACKEND_<NAME>_API_KEY` → loader inject.
- Chat: `CYNIC_CHAT_API_KEY` → `CYNIC_CHAT_API_KEY_FILE` → backend env →
  principal/`/etc/cynic` `llama-api-key`.
- Git Forgejo: **session token in env** for `scripts/forgejo-bootstrap`, not
  embedded permanently in `git remote` URL. Prefer `git credential` /
  `pass` / OS keyring for recurring push; token rotation is human admin.

## Operator practice

```bash
# Runtime machine identity (llama local)
install -m 640 -o cynic -g cynic /path/to/key /var/lib/cynic/.config/cynic/llama-api-key
# optional installed mirror
sudo install -m 640 -o root -g cynic /path/to/key /etc/cynic/llama-api-key

# One-shot sovereign git (HumanAdmin → temporary)
FORGEJO_TOKEN=… ./scripts/forgejo-bootstrap
# do not commit the token; revoke/rotate when done
```

Reports and MCP must show **presence** (`has_api_key=true`) never the secret.
