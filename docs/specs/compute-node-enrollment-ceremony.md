# Compute-node enrollment (re-)issuance ceremony

Operator-mediated issuance for a node whose enrollment grant expired or
was never issued. The authority holder vouches out of band; the report
always states `require_token: false`. No private key material below —
only file paths and verification steps.

## When

- `:9090/health` refuses while `cynic-node.service` is `active`.
- `node enrollment-check` reports `enrollment_proven: false` with an
  expiry in the past (or error), while the binary, unit, and config
  are otherwise converged.

## Inputs (all on the node, read-only except where noted)

- Node identity (never regenerated):
  `identity/node-signing.key` (+ `node-incarnation.key`).
- Previous request/grant/authority-pub as reference (never edited in place;
  every generation writes `-gN` files first, backups are kept as `.bak-*`).

Continuity rule: the fresh request MUST reuse the node's
`machine_public_key_hex` and `machine_identity_digest`. Verify before
issuing:

```
python3 -c "import hashlib; print(hashlib.sha256(bytes.fromhex('<machine_public_key_hex>')).hexdigest())"
```

must equal `machine_identity_digest`. If it does not, stop — wrong node.

## Authority keypair (the pitfall)

Generate with OpenSSL and parse the `-text` output. NEVER slice raw DER
with `tail -c 32`: depending on the OpenSSL version the file may or may
not append the public key, so the slice silently yields the wrong half.

```
openssl genpkey -algorithm ed25519 -out /tmp/auth-gN.pem
openssl pkey -in /tmp/auth-gN.pem -text -noout
```

Parse `priv:` (32 bytes seed) and `pub:` (32 bytes) from the text.
Write the seed hex to `identity/enrollment-authority-gN.key` (mode 600).

Cross-check (abort on mismatch): rebuild a PKCS8 DER from the seed
(`302e020100300506032b657004220420` prefix + seed), re-derive the public
key with `openssl pkey`, compare with the parsed `pub:`.

```
key_id = sha256(pub)
```

Record the expected `sha256:<key_id>` BEFORE issuing.

## Authority record and request

- Authority JSON: same `authority_id` lineage (`…-gN`), same `seed_id`
  as the node request, `token_digest: ""` (operator-mediated),
  `expires_at_unix_s` / `renewal_authority_expires_at_unix_s` = now + 1y,
  `max_grant_ttl_s` = 90d (`7776000`), `allowed_roles: ["build_runner"]`,
  `allowed_capabilities` = exactly the node's requested list.
- Request JSON: new `request_id`/`nonce`/timestamps, `expires_at_unix_s`
  = now + 90d. WARNING: the granted TTL is capped by the request expiry
  (`grant_expiry = min(now + max_grant_ttl_s, request.expires_at)`), so a
  short-lived request silently shortens the grant. This is how the first
  generation ended up with a 50-minute grant.

## Issue and verify (triple-match)

```
cynic node enrollment-issue \
  --request  identity/enrollment-request-gN.json \
  --authority identity/enrollment-authority-gN.json \
  --signing-key identity/enrollment-authority-gN.key \
  --output identity/enrollment-grant-gN.json --json
```

1. Issued `authority_key_id` MUST equal the precomputed `sha256:<key_id>`.
   Otherwise the wrong key signed — discard, do not stage.
2. Stage (backups first): grant, request, and `enrollment-authority.pub`
   (raw 32-byte pubkey hex) to their live paths from `node-control.toml`.
3. Triple-match: `grant.request_id == request.request_id` AND
   `grant.authority_key_id == sha256(live .pub)`.
4. `systemctl restart cynic-node` (root), then prove:
   `:9090/health` → `ok`, `node service prove` → `converged`.

## Custody

- The live node keeps the authority private key ONLY if future local
  re-issuance is intended; otherwise the operator vaults it off-node and
  shreds the on-disk copy. Either way: never in git, never in logs.
- The renewal loop (`run_enrollment_renewal_loop`) cannot extend a grant
  while `kernel_url` is unreachable from the node (e.g. loopback with no
  local kernel). Until remote renewal exists, the grant TTL IS the service
  lifetime: set it deliberately, and watch `node enrollment-check`.
