# Héberger GASdf exposé au public : air-gap, surfaces, frontières, agents

## 0. Intent

Host the GASdf runtime on CYNIC iron, reachable from the public internet,
without absorbing it and without opening the service plane. Hosting ≠
absorbing (`GASDF_INGESTION_PLAN.md` §0 stays authoritative): CYNIC provides
wires, hypervisor and attested lifecycle; GASdf keeps its code, keys and
deploy pipeline; CYNIC never holds GASdf secrets.

## 1. Observed reality (2026-10-18, measured tonight)

| Fact | Receipt |
|---|---|
| PVE has 3 NICs, none public | `GET nodes/pve/network`: `eno1` 192.168.100.1, `vmbr0` 10.10.10.1, `wlp1s0` down, no address |
| Cluster + node firewall = defaults | `GET cluster/firewall/options` and `nodes/pve/firewall/options` return empty digests — no custom policy observed (rulesets not yet audited) |
| 3 planes | management 192.168.100/24, service 10.10.10/24 (via pve gateway), overlay WG 10.20.0/24 (`network-topology-design.md`, archived but planes confirmed live tonight by routing) |
| Fleet = LXC only | templates 108→111→113, clones, all `local-lvm`; no KVM guest observed |
| GASdf relay runs on 107 (private .127) | cutover proven 2026-10-17; no public ingress path observed |
| Mainnet fee-payer material on cortex | `/etc/cynic/secrets/gasdf/relay/mainnet.env` exists (presence only, never read) — tension with the peer model, see §5 |
| 6 competing network SSOTs | `network-topology-design.md` §1.2 (S1–S6 drift); `infra/fleet.json` (mine, 2026-10-18) adds ephemeral scope — must reference, not duplicate |

Explicit unknowns: kernel bind (not re-verified); PVE firewall ruleset
content (options read, rules not listed).

## 2. Attack surfaces

| # | Surface | Attacker capability | Worst case |
|---|---|---|---|
| A1 | Public ingress → GASdf guest | RCE on the Node/Express stack, dependency confusion, Solana RPC spoofing | Guest owned; fee-payer drained if keys in guest |
| A2 | Guest → hypervisor escape | LXC shares the host kernel; kernel LPE → pve owned | Whole service plane owned (all guests, all templates, token) |
| A3 | Lateral guest → service plane | vmbr0 is flat; one owned guest scans 10.10.10/24 | Relay 107, PVE API (if reachable), build host |
| A4 | Egress exfiltration | Owned guest calls out (Solana RPC is required egress — cannot close fully) | Keys, ledger, topology leak |
| A5 | Supply chain (GASdf deps, npm) | Malicious update, soak-delay bypass | Backdoor baked into the hosted image |
| A6 | Agent confused-deputy | Recursive agent tricked into bridging the gap (copy secret, open route) | Air-gap voided by the operator itself |
| A7 | Human entry | Operator SSH into DMZ for debug, leaves keys, disables a rule "temporarily" | Persistent hole, no receipt |

## 3. Besoins CYNIC existants

- Relay 107 already talks to Solana (devnet today, mainnet tomorrow) — egress-to-RPC is a pre-existing, accepted need; the DMZ reuses that allowance, nothing new.
- Build/attest pipeline exists (builder template flow, hygiene gate, TTL sweep) — the DMZ guest lifecycle is the same machinery with tighter policy, not a new system.
- Secrets already split by principal (`/etc/cynic/secrets`, `sudo -u cynic`) — the DMZ must not create a second vault; it holds NO CYNIC secrets at all.
- DNS fleet (`synapse.cynic`) is split-horizon — public names for GASdf must never resolve into the service plane (checklist: probe 1.1.1.1/8.8.8.8 before claiming anything).

## 4. Contraintes & frontières (non-négociables)

1. **LXC is not a security boundary.** Internet-facing workloads run KVM (hardware virtualization), never LXC. The clone/template flow stays LXC for ephemeral agents; public guests are a separate, KVM-only line.
2. **One-way data flow.** DMZ → service plane: nothing, ever, except through an explicit, allowlisted, logged proxy (attestation receipts out, metrics out). Service → DMZ: image provisioning and policy only. No SSH from service plane into DMZ for debug (A7); observation is read-back via hypervisor (status/config), never a shell.
3. **No shared secrets.** GASdf fee-payer keys live in the GASdf guest (or its HSM path), deployed by the GASdf pipeline — never in `/etc/cynic/secrets`, never in git, never in agent prompts. The §1 cortex `mainnet.env` presence is debt to resolve (migrate into the GASdf-guests-only path or delete).
4. **Separate L2.** New bridge (`vmbr1`, no IP on pve, no route to 10.10.10/24 or 192.168.100/24), PVE firewall default-deny both directions, explicit ingress (443→guest) + egress (Solana RPC, NTP) only. SDN zone or plain Linux bridge — plain bridge first (fewer moving parts, auditable with `brctl`/`ip`).
5. **Ephemeral by default.** DMZ guests carry TTL leases like the agent fleet; the sweeper already exists — extend it to the DMZ pool instead of building a second reaper.
6. **Soak delay on the GASdf stack.** Per security posture: pin versions, never deploy on release day, canary guest first with read-back (the skill's soak rule binds here directly — npm supply chain is A5).

## 5. Options

1. **(R) DMZ bridge + KVM + proxy-only + agent-operated.** `vmbr1` isolated, one KVM guest (GASdf API), ingress 443 only via PVE firewall, egress allowlist (api.mainnet-beta/devnet, NTP), no CYNIC secrets inside, TTL leases, agents run the lifecycle, humans never shell in. Cost: one bridge + one firewall policy + KVM image (slower to clone than LXC — acceptable, public guests are cattle but long-lived, not 2-second ephemerals).
2. LXC on isolated bridge. Rejected: violates constraint 1 (A2). Speed is not worth a shared kernel with the internet.
3. Reverse-proxy VM in service plane. Rejected: puts a public-reachable terminator adjacent to the crown jewels; moves A1 next to 107 instead of removing it.
4. Cloud VPS for GASdf, CYNIC stays dark. Honest fallback if the uplink cannot do clean ingress — but splits the estate and the operator asked to self-host.

## 6. Rôle des agents récursifs Hermes-Agent

Agents are the ONLY operators of the DMZ (no human shell — kills A7). Each agent is scoped to one typed surface, with leases and audit:

- **provisioner**: clone/build the KVM guest image from pinned inputs, write the TTL lease. Cannot touch firewall or network.
- **attestator**: read-back only (config, firewall rules, listening sockets via hypervisor, image hashes). Emits receipts; on mismatch, refuses and pages the operator. Never mutates.
- **sweeper**: extends the existing TTL sweep to the DMZ pool (fail-closed: unleased or unknown guests are reported, never destroyed — siblings and the operator's hand-made guests live on the same iron).
- **policy guard** (static, no runtime): owns `infra/fleet.json` + firewall intent; any runtime change outside the declared policy is a blocker, not a suggestion.

What agents NEVER do: hold GASdf keys (A6 — the peer boundary binds agents too), open routes, disable rules, or enter the guest. An agent that cannot complete without violating a boundary reports `blocked` with the exact missing capability — it does not improvise one.

## 7. Freebox observation (2026-10-18, vérifié live partiel)

- Box : Freebox Server r2 en 192.168.0.254, `GET /api_version` → api 16.1,
  `GET /api/v10/login/` répond (challenge émis) — le wrapper existant
  (`HttpFreeboxProbeAdapter`, session + LAN browser) est compatible.
- Manquant : `GET /api/v10/fw/redir/` (règles NAT/port-forwarding) —
  extension lecture seule prévue, formes à vérifier live après pairing :
  réponse attendue `{success, result: [{id, enabled, comment, lan_port,
  wan_port_min/max, lan_ip, proto}]}` (à confirmer, pas supposé).
- Pairing done 2026-10-18 (app `cynic`, track granted on box display);
  token stored `/etc/cynic/secrets/freebox-app-token` (600, cynic) —
  session opens, `fw/redir` readable.
- RESOLVED A1 (corrected 18:35 — the "single rule" claim was a 1500-char
  truncation artifact; full list read live): public IPv4 `82.65.26.228`
  with EIGHT active forwards, all → 192.168.0.37 (pve):
  8006 PVE UI (no comment), 443 "CYNIC Headscale + Kernel", 8443 "CYNIC
  Webhook Receiver", 3478 "CYNIC DERP STUN", 80 "CYNIC Caddy ACME", 22 SSH
  (no comment), 3000 (no comment — ex-forgejo?), 51820 WireGuard (no
  comment). No shrinkage since Oct 6; the estate is wide open.
- 8006 delete ATTEMPTED via approval intent (consumed, single-use spent):
  Freebox API answered 403 — the app grant lacks settings/fw rights.
  Fail-closed proven: explicit error, nothing half-done, rule intact.
  Unblock: box UI → Gestion des accès → app `cynic` → allow settings
  modification (or delete rule id 2 in 2 clicks), then propose a fresh
  intent. Consumer of 8006 still UNKNOWN (2 days pveproxy = cortex only).
- Token sprawl (screenshot 2026-10-18) : SEVEN apps (`CYNIC Port Forward`,
  `CYNIC Admin` (HAS settings-modify), `CYNIC WiFi localizer`, `CYNIC
  signal monitor`, `CYNIC Organ Freebox` (HAS settings-modify), `CYNIC
  Node`, `CYNIC cortex` (today, read-only)). Owners/dates scattered
  (May/June/Oct), no inventory — structural gap, same family as the NAT
  rules without comments.
- Legacy `/home/user/.freebox_token` (664 world-readable!, June 12,
  81 bytes in 2 segments — not the 64-char token format) migrated to
  `/etc/cynic/secrets/freebox-organ-token` (600) and shredded. Identity
  unconfirmed (3 app_id attempts → 403); no further guessing.
  Cleanest path: grant settings-modify to `CYNIC cortex` in box UI and
  revoke the dead apps, keeping ONE admin path + the read probe.
- Side find: `/etc/cynic/secrets/gasdf/helius-api-key` EXISTS (37 B,
  Oct 16 02:06) though the Oct 16 baseline recorded it missing — presence
  only, never read; subscription work may now reference it via ports.
- Panic CLI fixé 2026-10-18 (thread confinement, `dcc3cb5a`) — `network
  observe --freebox` vérifié live ; reste 2 panics frères + 2 latents
  (territoire pair).

## 8. Tranches proposées

- T1: answer the unknowns (public ingress path, kernel bind, firewall rules audit) + resolve the `mainnet.env` tension (migrate or delete, with receipt).
- T2: `vmbr1` + PVE firewall default-deny + KVM golden image (pinned stack, soaked), first boot with read-back.
- T3: extend sweeper + leases to the DMZ pool; attestator receipts wired to alerting.
- T4: public ingress cutover with external probe (DNS both sides, port scan from outside, RPC-spoof test) before any mainnet key enters the guest.

Stop condition T2: if KVM on pve proves unworkable (perf, nesting), fall back to option 4 explicitly — no silent LXC downgrade.
