# Relay migration cortex → LXC 107 (10.10.10.127)

## Status

Proposed. Parallel-run migration: the cortex stack stays live until 107
proves itself. No downtime, instant rollback (DNS/endpoint points back).

## Inventory (measured 2026-10-18, cortex)

- `gasdf-kora-mainnet.service` → kora :14001 (binary 22M at
  `/var/lib/cynic/source/repositories/external/kora/target/release/kora`,
  config `kora-mainnet.toml`, signers `signers-mainnet.toml`).
- `gasdf-facilitator.service` → node `dist/facilitator/server.js` :4021.
- `gasdf-relay.service` → node `dist/index.js` :3000 (`KORA_URL`
  `http://localhost:14001`, signer `Bcuw...rcC`).
- Manual kora devnet :14000 (NOT migrated — stays on cortex, out of scope).
- Secrets (names only): `facilitator-key.json`, `flow2-user.json`,
  `mainnet.env`, `mainnet-payer.json`, `signer.env`, `upto-authorizer.json`.
- Source: relay-gateway lives in scratch
  (`~/.hermes/cache/scratch/relay-gateway`), NOT in git. `dist/` is the
  built artifact; `node_modules` present. Flow: copy `dist + node_modules +
  package.json`, rebuild later after source adoption (separate tranche).

## Steps

1. **Access (operator, root pve):** provision SSH on 107 (key + user),
   then hand to agent.
2. **Base (agent, 107):** `apt update && apt install -y nodejs` (match
   v22.x), user `cynic`, dirs `/var/lib/cynic/projects/kora`,
   `/etc/cynic/secrets/gasdf/relay` (same ownership/mode as cortex).
3. **Binaries (agent):** copy kora binary (22M, same arch) + tomls;
   copy `relay-gateway/{dist,node_modules,package.json}` + `flow2-relay.js`.
   No rebuild on 107 (slow vCPU); checksums compared after copy.
4. **Secrets (operator-led):** secrets move only with explicit operator
   handling — never pasted in chat, never world-readable in transit
   (scp as root/cynic preserving 0640/0750).
5. **Units (agent):** install the 3 unit files with `localhost` bindings
   unchanged (KORA_URL stays localhost — same-host topology preserved),
   `systemctl daemon-reload`, start, `config validate` first.
6. **Proof (agent):** ports listen on 107 (`:14001`, `:4021`, `:3000`),
   `validate-mainnet.sh` green, one sponsored devnet-equivalent dry call
   through 107's :3000 WITHOUT spending (quote-only), cortex stack
   untouched and still serving.
7. **Cutover (operator decision):** only when step 6 is archived — repoint
   consumers (list them first: who calls :3000/:4021 today is still OPEN,
   must be answered before cutover) or reverse-proxy. Rollback = point
   back, zero data loss (stateless gateway + on-chain signers).

## Non-goals

- No devnet :14000 move. No source adoption (separate tranche).
- No secret rotation (same keys, new host). No firewall/SDN changes
  beyond what exists (107 already on vmbr0/DHCP).

## Open before cutover

- Consumer list of :3000/:4021 (unknown — measure: logs/conns on cortex).

## Cutover record (2026-10-18)

- Live proof: sponsored 1-ASDF swap through 107's kora, sig
  `4ScTeGBWCiF628nc3Dryf1ec6AiZfscwqjWi7DBUEebuLLYEgTwE6NnQYK8VD4FrS3Lv88GzWjhjLtyHheFJFN4c`.
- Blocker found and fixed: user's WSOL ATA missing → kora
  `allow_create_account=false` rejected the send (chain state, identical
  on both hosts). Created once with the user key (approved), rent ~0.002.
- Clarified: `Bcuw...` is the kora fee-payer signer, the flow user is
  `22XuH...` (`flow2-user.json`).
- Cortex `gasdf-{relay,facilitator,kora-mainnet}` stopped; 107 serves.
  Manual devnet kora :14000 stays on cortex (out of scope).
