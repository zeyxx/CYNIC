# organ-domain

Public domain operations for the CYNIC organism.

Manages the lifecycle from registrar (Porkbun) to public edge (Cloudflare DNS, Pages).

## Boundaries

- **Owns**: public DNS zones, Pages deployments, bulk redirects, proxy policy for `talaria.build` and its subdomains
- **Does not own**: internal Synapse DNS (→ `organ-wireguard`), access mesh (→ `organ-headscale`)

## Commands (read the MANIFEST.yaml for the full surface)

```bash
# Non-mutating — safe to run any time
python3 organs/domain/src/entrypoints/cli.py status
python3 organs/domain/src/entrypoints/cli.py registrar portfolio
python3 organs/domain/src/entrypoints/cli.py registrar expiry-audit
python3 organs/domain/src/entrypoints/cli.py registrar check talaria.build
python3 organs/domain/src/entrypoints/cli.py zone backup
python3 organs/domain/src/entrypoints/cli.py pages verify

# Mutating — approval gate required before execution
python3 organs/domain/src/entrypoints/cli.py zone migrate --dry-run
python3 organs/domain/src/entrypoints/cli.py zone migrate
python3 organs/domain/src/entrypoints/cli.py zone set-ns --apex talaria.build --zone-id <id> --dry-run
python3 organs/domain/src/entrypoints/cli.py zone set-ns --apex talaria.build --zone-id <id>
python3 organs/domain/src/entrypoints/cli.py pages deploy --app odysseum --dry-run
python3 organs/domain/src/entrypoints/cli.py pages deploy --app odysseum
```

## Credentials

Source `~/.cynic-env` before running any command:

```bash
source ~/.cynic-env
```

Required env vars:
- `PORKBUN_API_KEY`
- `PORKBUN_SECRET_API_KEY`
- `CF_API_TOKEN`
- `CF_ACCOUNT_ID`

## Desired state

- `desired/domains.json` — apex, registrar, apps, proxy policy
- `desired/redirects.json` — bulk redirects

## Maturity

`canonical_partial` until Phase 1 is verified. The organ has bounded
core/adapters/entrypoints and machine-readable desired state, but
`desired/domains.json` still declares zone migration as pending.
