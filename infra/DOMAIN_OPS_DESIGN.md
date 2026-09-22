# CYNIC Domain Operations Design Document
## External Domain Registration, DNS Delegation, and Cloudflare Integration

- **Status**: Draft (design gate — awaiting approval)
- **Layer**: Protocol types → domain-ops hexagon → host/node CLI
- **Authority**: foundation.json §32 (provider_agnostic_domain), §37 (portable_host_contract), §40 (runtime_proof_over_prose); CRATE_MODELS.md §2, §4; AGENTS.md structure & boundaries
- **Schema**: `cynic.domain_ops.v1`

---

## 1. Context & Motivation

### 1.1 Problem

CYNIC currently manages **internal** DNS only: the `synapse.cynic` zone served by
CoreDNS, hosts files, WireGuard mesh. There is no mechanism for an agent to:

- Register an external domain through a registrar API (Porkbun)
- Delegate DNS authority to an external provider (Cloudflare)
- Migrate DNS records between registrar and provider
- Verify that the live internet sees the intended DNS/HTTP state

The Ahmad Osman article "Agentic Domain Ops" describes this exact workflow as
shell scripts (registrarctl, cf-zone-migrate, cf-pagesctl, cf-redirectctl,
cf-proxy-audit, route-bootstrap). CYNIC's mandate is supply-chain hardening via
100% Rust — **no shell wrappers**. The pattern transfers directly to CYNIC's
coherence loop:

```
runbook + desired-state  →  observe  →  reconcile  →  bounded action  →  read-back proof
```

### 1.2 What This Enables (Cahier des Charges — V1)

An agent can autonomously provision an external domain through the full
registration-to-verification chain, with explicit consent gates for paid and
irreversible operations:

1. **Domain availability** — Check Porkbun API for domain availability (read-only)
2. **Domain registration** — Register through Porkbun (paid, `--yes` required)
3. **Registrar NS read/update** — Read current registrar nameservers; update to
   Cloudflare (separate `--yes` confirmation gate for cutover)
4. **Zone migration** — Migrate DNS records from Porkbun-hosted zone to
   Cloudflare zone with shape transformation, backup-before-write, and
   idempotency keys
5. **Delegation verification** — Query authoritative nameservers for TLD
   delegation; confirm NS records match Cloudflare
6. **Edge verification** — HTTP probe via Cloudflare edge (curl --resolve
   equivalent) confirming expected status codes and redirects

**Excludes from V1** (deferred to V2):

- Cloudflare Pages deployment (cf-pagesctl)
- Cloudflare Bulk Redirects (cf-redirectctl)
- Cloudflare proxy status audit/patch (cf-proxy-audit)
- Reverse proxy route bootstrap (route-bootstrap)

These share the same structural pattern but are separate provider API surfaces;
they slot into V2 once the core port/adapter framework and proof pipeline are
established.

---

## 2. Design Gate (bottom-up)

### 2.1 Desired State & Authority

**Authority chain (top-down):**
1. `infra/registry.json` — declares provider endpoints and credential refs
   (no hardcoded hosts per user mandate)
2. `contracts/foundation.json` — mandates provider-agnostic domain logic,
   portable ports, runtime proof over prose
3. Operator consent — `--yes` for paid ops; separate `--yes` for NS cutover

**Desired state representation:**
```jsonc
{
  "schema": "cynic.domain_ops.desired_state.v1",
  "domain": "example.com",
  "registrar": { "provider": "porkbun", "credential_ref": "env:PORKBUN_API_KEY" },
  "dns_authority": { "provider": "cloudflare", "credential_ref": "env:CF_API_TOKEN" },
  "proxy_policy": { "web_records": "proxied", "mail_records": "dns_only" },
  "expected_ns": ["ns1.cloudflare.com", "ns2.cloudflare.com"],
  "edge_probes": [
    { "fqdn": "example.com", "expected_status": 200, "follow_redirects": true }
  ]
}
```

Secrets are NEVER inline. Credential refs resolve through a `SecretProvider`
port (env → principal-owned file → OS keyring). Reports show presence, never
the secret.

### 2.2 Read-Only Observation & Port

**Observation ports (pure traits, zero I/O in domain):**

| Port | Method | Output |
|---|---|---|
| `RegistrarPort` | `check_availability(&self, domain) -> DomainAvailability` | available/taken |
| `RegistrarPort` | `read_nameservers(&self, domain) -> Vec<String>` | current NS set |
| `RegistrarPort` | `read_zone_records(&self, domain) -> Vec<DnsRecordShape>` | registrar zone |
| `RegistrarPort` | `check_pricing(&self, domain, years) -> PricingInfo` | cost before mutation |
| `CloudflareDnsPort` | `zone_exists(&self, zone) -> bool` | zone presence |
| `CloudflareDnsPort` | `read_zone_records(&self, zone) -> Vec<DnsRecordShape>` | existing records |
| `CloudflareDnsPort` | `list_zones(&self) -> Vec<CloudflareZone>` | zone inventory |
| `EdgeProbePort` | `probe_ns_delegation(&self, domain) -> Vec<String>` | authoritative NS |
| `EdgeProbePort` | `probe_http(&self, fqdn, expected) -> HttpProbeResult` | edge behavior |

Read-only commands are safe for the agent to run often. Mutation methods are
on the same ports but gated by consent.

### 2.3 Pure Selection Rule

All decision logic is pure (zero I/O), lives in `domain.rs`:

1. **Record shape transform** (cf-zone-migrate logic):
   - A/AAAA/CNAME/TXT/MX → pass through
   - NS records at registrar → ignored (delegated to CF)
   - DNSSEC, CAA → DNS-only (not proxied)
   - Alias/ANAME → flattened to A/AAAA
   - MX/TXT → DNS-only per proxy policy

2. **Idempotency**: every write carries an idempotency key. The domain layer
   generates keys from `(domain, record_content_hash, operation_type)`.

3. **CNAME conflict check**: refuse ambiguous multi-record updates unless
   `replace_all` is explicitly requested.

4. **Consensus check**: compare registrar zone vs Cloudflare zone. Only
   missing/changed records are written.

### 2.4 Exact Arv, Privilege, Consent, Rollback, Stop Condition

| Operation | argv | Privilege | Consent | Rollback | Stop |
|---|---|---|---|---|---|
| Availability check | `CheckDomainAvailability{domain, registrar}` | read-only | none | N/A | N/A |
| Domain registration | `RegisterDomain{domain, years, auto_renew}` | paid API | `--yes` (paid) | cancel within transfer window (TLD-dependent) | if already registered |
| NS update | `UpdateNameservers{domain, ns_set}` | delegation change | `--yes` (separate NS gate) | capture prior NS set → restore function | if ns_set empty |
| Zone migration | `MigrateZone{domain, proxy_policy, backup}` | zone write | `--yes` (zone mutation) | Cloudflare zone backup (export records) | if zone missing |
| Delegation verify | `VerifyDelegation{domain, expected_ns}` | read-only DNS query | none | N/A | N/A |
| Edge verify | `VerifyEdge{domain, probes}` | read-only HTTP GET | none | N/A | N/A |

**Consent gates**:
- Paid operations (registration, renewal) require `--yes`
- NS cutover requires a **separate** `--yes` invocation (irreversible delegation change)
- Zone writes require `--yes` (with backup-first)

**Rollback**: Cloudflare zone is exported to backup before any write.
Porkbun zone snapshot captured before NS change. Backups stored as
`cynic-askesis` append-only proof records.

### 2.5 Post-Action Read-Back & Producing Layer

| Action | Read-back proof | Layer |
|---|---|---|
| Registration | Registrar API confirms `status=registered`, `expiry_date`, `auth_code` | cynic-domain-ops adapter |
| NS update | Registrar API returns `nameservers=[cf1, cf2]`, then TLD delegation query | cynic-domain-ops |
| Zone migration | Cloudflare zone record count matches, all records present | cynic-domain-ops |
| Delegation verify | Authoritative NS resolver returns Cloudflare nameservers | cynic-domain-ops probe |
| Edge verify | HTTP through CF edge returns expected status/redirect | cynic-domain-ops probe |

All read-back produces machine-readable JSON proof (`schema: cynic.domain_ops.proof.v1`).

### 2.6 Owning Boundary

| Answer | Owning crate |
|---|---|
| Pure types (DNS record, zone state, proof) | `cynic-protocol` |
| Pure logic (transform, idempotency, reconciliation) | `cynic-domain-ops` |
| Port traits | `cynic-domain-ops` |
| HTTP adapters (Porkbun, Cloudflare) | `cynic-domain-ops/adapters/` |
| Local probe (DNS resolver, HTTP client) | `cynic-domain-ops/adapters/` |
| CLI facade | `cynic-cli` (thin — parse → inject → render) |
| Admission / consent | `cynic-node` (bounded mutation envelope) |
| Append-only backup proofs | `cynic-askesis` |

---

## 3. Architecture & Layer Model

### 3.1 Strict DAG

```
cynic-protocol (types, ports — NO provider logic)
    └── cynic-domain-ops (pure logic + port traits + HTTP adapters)
        └── cynic-node (admission, bounded mutation envelope, consent gates)
            └── cynic-cli (thin projection: parse → inject → render)
                └── cynic-mcp (agent-facing tools, delegates to CLI/node)
```

**cynic-domain-ops depends on**: `cynic-protocol` only (for shared types).
**cynic-node depends on**: `cynic-domain-ops` (for domain logic orchestration).
**cynic-cli depends on**: `cynic-node` or `cynic-domain-ops` directly (for thin
facade pattern, like `cynic-build` direct calls today).

### 3.2 Hexagonal Layout (cynic-domain-ops)

```
crates/cynic-domain-ops/
  src/
    lib.rs              # thin facade — re-exports only
    types.rs            # local pure enums (DomainOperation, ProviderKind)
    port.rs             # driven ports (traits)
    domain/             # PURE: record transform, idempotency, reconciliation
      mod.rs
      transform.rs      # registrar → cloudflare record shape rules
      reconcile.rs      # diff desired vs observed, generate plan
      idempotency.rs    # key generation, conflict detection
    contracts/
      mod.rs
      desired_state.rs  # deserialization of cynic.domain_ops.desired_state.v1
      proof.rs          # cynic.domain_ops.proof.v1
    runner.rs           # orchestrates ports → produces proof JSON
    adapters/
      mod.rs
      porkbun.rs        # Porkbun HTTP API (reqwest)
      cloudflare.rs     # Cloudflare API (reqwest)
      probe.rs          # local DNS resolution + HTTP edge probe
      memory.rs         # in-memory fakes for tests
    contracts/
  tests/
    architecture_boundary.rs   # ENFORCES: domain/ has zero I/O
```

### 3.3 CLI Projection

Per user law ("new capabilities must nest as arms of an owning family"):

```
cynic network domain plan       # generate domain operation plan (read-only)
cynic network domain bundle     # export desired-state bundle
cynic network domain apply --yes  # execute bounded action (paid ops gated)
cynic network domain status     # verify current state (read-back)
cynic network domain verify     # authoritative delegation + edge probe
```

### 3.4 Proof & Backup Pipeline

Follows article's safety invariants:

1. **Reads are cheap** — availability, pricing, zone reads, delegation check
2. **Writes are explicit** — every mutation behind `--yes`
3. **Paid ops require confirmation** — registration, renewal
4. **NS cutover requires separate confirmation** — dedicated `--yes` gate
5. **Desired state lives in files** — JSON contracts in registry, not CLI args
6. **Cloudflare state backed up before writes** — zone export → cynic-askesis
7. **DNS delegation verified authoritatively** — query authoritative NS, not recursive
8. **Secrets stay outside workflow files** — env/injected, never inline
9. **Edge verification avoids false negatives** — resolve through CF NS + curl --resolve

---

## 4. Extraction Plan (V1)

### Phase 0: Protocol Types (cynic-protocol)

Add pure types to `cynic-protocol::domain_ops`:

```rust
// New module: src/domain_ops.rs
pub struct DomainName(String);           // validated domain name
pub struct DnsRecordShape {              // provider-agnostic record
    pub name: String,
    pub r#type: DnsRecordType,           // A, AAAA, CNAME, TXT, MX, NS, CAA, SRV
    pub content: String,                 // raw record value
    pub ttl: u32,
    pub proxied: bool,                   // Cloudflare proxy flag
}
pub enum DnsRecordType { A, AAAA, CNAME, TXT, MX, NS, CAA, SRV, DS }
pub struct ZoneRecordDiff {
    pub add: Vec<DnsRecordShape>,
    pub remove: Vec<DnsRecordShape>,
    pub update: Vec<DnsRecordShape>,
}
pub struct DomainOperationProof {
    pub schema: &'static str,            // cynic.domain_ops.proof.v1
    pub domain: String,
    pub operation: String,
    pub status: ProofStatus,
    pub idempotency_key: String,
    pub provider_read_back: serde_json::Value,
    pub timestamp_unix_s: u64,
}
```

**Acceptance**: `cargo check -p cynic-protocol --lib && cargo test -p cynic-protocol`

### Phase 1: Domain Pure Logic (cynic-domain-ops)

```
crates/cynic-domain-ops/
```

- `types.rs`: `DomainOperation` enum, `ProviderKind`, `ProxyPolicy`, `MigrationConfig`
- `domain/transform.rs`: `transform_record_for_cloudflare(record) -> DnsRecordShape`
  - NS records dropped, DS/DNSSEC → DNS-only, proxy by policy
  - Alias flattening (pure string transform, no network)
  - Tests: 100% coverage of shape rules (12 test cases minimum)
- `domain/reconcile.rs`: `reconcile_zone_diff(local, remote) -> ZoneRecordDiff`
  - Idempotency: diff is empty iff zones match
  - Tests: symmetric reconciliation, conflict detection
- `domain/idempotency.rs`: `generate_idempotency_key(op, domain, content) -> String`
  - Tests: deterministic keys, collision resistance

**Acceptance**: `architecture_boundary.rs` enforces zero I/O in `domain/` module.
Pure functions tested without any adapter.

### Phase 2: Contract deserialization (cynic-domain-ops) ✅

```rust
// contracts/desired_state.rs
pub fn parse_desired_state(json: &str) -> Result<DomainOpsDesiredState, DomainOpsError>
// contracts/proof.rs
pub fn build_proof(domain, op, status, key, read_back, ts) -> DomainOperationProof
pub fn proof_to_json(proof) -> Result<String, DomainOpsError>
pub fn parse_proof(json: &str) -> Result<DomainOperationProof, DomainOpsError>
```

Implements:
- `parse_desired_state`: deserialize JSON, validate schema domain provider probes proxy policy
- `build_proof` / `blocked_proof` / `violation_proof`: construct proof envelopes
- `proof_to_json` / `parse_proof` / `proof_chain_to_json` / `parse_proof_chain`: proof serialization
- `DomainOpsError` (thiserror): 7 variants covering schema mismatch, invalid domain, invalid provider config, proxy policy, edge probe, JSON parse, unexpected proof schema

### Phase 3: HTTP Adapters ✅ (3 micro-crates)

- `adapters/porkbun.rs`: Porkbun API v2 client via reqwest
  - Endpoints: `/domains/check`, `/domains`, `/domains/{domain}/nameservers`,
    `/dns/{domain}/records` (GET/POST), `/domains/{domain}/pricing`
  - API key from `SecretProvider`
  - Idempotency: Porkbun idempotency-key header on writes
  - Retry with backoff on 429/5xx

- `adapters/cloudflare.rs`: Cloudflare API v4 client via reqwest
  - Endpoints: `/zones`, `/zones/{id}/dns_records`, `/zones/{id}/dns_records/{id}`
  - API token from `SecretProvider`
  - Backup: `GET /zones/{id}/dns_records` → serialize as backup

- `adapters/probe.rs`: Local DNS resolution + HTTP edge probe
  - `query_authoritative_ns`: uses rust-dns-resolver or hickory to query NS
  - `http_probe`: uses reqwest with `--resolve` equivalent (force DNS resolution

- `adapters/memory.rs`: In-memory fakes for all ports (test-only)

**Acceptance**: All adapters implement ports. No adapter logic in domain module.
`architecture_boundary.rs` enforces adapters are the only I/O layer.

### Phase 4: Runner (cynic-domain-ops)

`runner.rs` orchestrates the full chain:

```rust
pub fn run_domain_ops(
    registry: &DomainOpsRegistry,
    registrar: &dyn RegistrarPort,
    cloudflare: &dyn CloudFlareDnsPort,
    probe: &dyn EdgeProbePort,
    secrets: &dyn SecretProvider,
) -> Vec<DomainOperationProof>
```

Sequence:
1. **Observe**: check availability → read registrar zone → read CF zone (if exists)
2. **Reconcile**: `reconcile_zone_diff` → compute record diff
3. **Plan**: generate idempotent operation plan
4. **Execute** (gated by `--yes`): register (paid) → update NS (separate gate) →
   backup CF zone → upsert records → 
5. **Verify**: authoritative NS delegation → edge HTTP probe
6. **Proof**: emit `cynic.domain_ops.proof.v1` for each step

The runner takes `apply: bool` and `ns_cutover_confirmed: bool` as explicit
parameters — the domain layer never decides to cut over NS without consent.

**Acceptance**: End-to-end test with memory adapters produces correct proof chain.
Architecture boundary test passes.

### Phase 5: CLI Projection (cynic-cli)

New subcommands nested under `cynic network domain`:

- `cynic network domain plan --domain <name> --registry <path>`
  - Calls `runner::plan()` → emits plan JSON
- `cynic network domain apply --domain <name> --registry <path> --yes`
  - Calls `runner::execute()` with `apply=true` → emits proof JSON
- `cynic network domain status --domain <name>`
  - Calls `runner::status()` → emits status JSON
- `cynic network domain verify --domain <name>`
  - Calls `runner::verify()` → emits verification JSON

The existing `cynic network` surface is extended; no new top-level family.

**Acceptance**: CLI compiles, `architecture_boundary.rs` in cynic-cli passes,
existing network tests still pass.

### Phase 6: Admission & MCP (cynic-node, cynic-mcp)

- cynic-node: wire domain ops through bounded mutation envelope
  - `claim` → `execute` → `read_back` → `release`
  - NS cutover is a separate claim with elevated consent scope
- cynic-mcp: add `cynic_domain_ops_plan`, `cynic_domain_ops_status`,
  `cynic_domain_ops_verify` as read-only MCP tools
  - Mutation tools (`apply`) gated by MCP consent flow

**Acceptance**: MCP tools appear in `cynic dev agent-exposure --json` output.

---

## 5. Open Decisions (resolved)

1. **New crate `cynic-domain-ops`** — APPROVED. Not absorbed by cynic-network
   (would dilute the mesh PRIMAIRE per CRATE_MODELS.md §2). Follows CRATE_MODELS
   decision framework §2(2): coherent external-provider family deserving its own PRIMAIRE.

2. **HTTP client: `reqwest`** — APPROVED. Matches existing async pattern (cynic-node
   uses tokio/libp2p). Compile-time impact contained by feature flags
   (reqwest without default TLS, or with rustls-only).

3. **DNS resolver: `hickory-resolver` (R)** — Pure Rust, no host tool dependency.
   Can query authoritative NS directly (not recursive-only like `getent`), set
   custom resolver to Cloudflare NS IPs for edge verification. `dig`-via-port
   rejected: adds host tool dependency, violates anti-scripts mandate.
   Alternative if compile time is a blocker: inject `dig`/`drill` as ArgvRunner
   port (matches network_audit/scan.rs existing pattern) — but less robust.

4. **V1 scope: registrar + DNS migration + verification only** — APPROVED.
   cf-pagesctl, cf-redirectctl, cf-proxy-audit, route-bootstrap deferred to V2.

5. **Secret injection: env → file ref → OS keyring (future V2)** — APPROVED.
   `SecretProvider` port resolves `"env:VAR"`, `"file:/path"` (V1).
   OS keyring (Linux Secret Service API / GNOME Keyring, macOS Keychain,
   Windows Credential Manager) via `keyring` crate deferred to V2.

---

## 6. What V1 Enables Concretely

### 6.1 Agent Capability

With V1, an agent can execute this exact chain for any external domain:

```
cynic network domain plan --domain example.com --registry infra/registry.json
  → emits desired-state plan JSON (read-only)

cynic network domain apply --domain example.com --registry infra/registry.json --yes
  → observe: check availability, read registrar zone
  → reconcile: transform records for Cloudflare, diff against CF zone
  → bounded action: register domain (paid), update NS (gated), migrate zone (backup-first)
  → read-back: verify TLD delegation, verify edge HTTP
  → emit: cynic.domain_ops.proof.v1 chain (JSON)
```

The agent **cannot**:
- Deploy Pages sites (V2)
- Configure bulk redirects (V2)
- Audit/patch proxy status (V2)
- Configure reverse proxy routes (V2)

### 6.2 What the Agent Can Produce (Evidence)

For each operation, V1 emits machine-readable proof:

| Proof type | Schema | Content |
|---|---|---|
| Availability | `cynic.domain_ops.availability.v1` | domain, registrar, available, premium, price |
| Registration | `cynic.domain_ops.registration.v1` | domain, expiry, eap_id, provider_read_back |
| Nameserver update | `cynic.domain_ops.ns_update.v1` | domain, old_ns, new_ns, provider_read_back |
| Zone migration | `cynic.domain_ops.zone_migration.v1` | domain, records_migrated, records_skipped, backup_ref |
| Delegation verify | `cynic.domain_ops.delegation.v1` | domain, observed_ns, expected_ns, delegated |
| Edge verify | `cynic.domain_ops.edge_probe.v1` | fqdn, expected_status, observed_status, redirect_location |

### 6.3 What Cannot Wing It (Enforced Constraints)

- No shell scripts — all provider calls through typed Rust ports
- No inline secrets in desired-state files — credential refs only
- No NS cutover without separate `--yes` confirmation
- No zone write without backup to cynic-askesis
- No verification through recursive resolvers — authoritative NS only
- No proxy status ambiguity — DNS-only vs proxied decided by pure policy
- No CNAME conflict — ambiguous multi-record writes refused unless `replace_all`

---

## 7. Crate Ecosystem (V1 + V2)

### 7.1 DAG des crates

```
cynic-protocol (types purs — DomainOpsDesiredState, DnsRecordShape, proofs)
│
├── cynic-domain-ops (logic pur + port traits + runner) [V1]
│   ├── domain/       transform, reconcile, idempotency
│   ├── port.rs       RegistrarPort, CloudflareDnsPort, EdgeProbePort, SecretProvider
│   ├── runner.rs     orchestration: observe → reconcile → act → prove
│   └── contracts/    desired-state deserialization
│
├── cynic-domain-porkbun (registrar adapter — RegistrarPort impl) [V1]
├── cynic-domain-cloudflare (CF DNS + probe adapter — CloudflareDnsPort, EdgeProbePort impl) [V1]
├── cynic-domain-askesis (backup → cynic-askesis append-only) [V1]
│
├── cynic-domain-pages (CF Pages deploy adapter) [V2]
├── cynic-domain-redirects (CF Bulk Redirects adapter) [V2]
├── cynic-domain-proxy-audit (CF proxy status audit/patch) [V2]
└── cynic-domain-route (reverse-proxy route bootstrap) [V2]
    │
    ├── cynic-cli (thin projection: `cynic network domain plan/apply/verify`) [V1]
    └── cynic-mcp (read-only agent tools: plan, status, verify) [V1]
```

### 7.2 Crate inventory

| Crate | Hex role | Phase | Dependencies | Key types |
|---|---|---|---|---|
| `cynic-protocol` | Pure types | 0 ✅ | (none) | `DomainOpsDesiredState`, `DnsRecordShape`, `DomainOperationProof` |
| `cynic-domain-ops` | Hex center | 1 ✅ | `cynic-protocol` | `reconcile_zone`, `transform_record_for_cloudflare`, `run_domain_ops` |
| `cynic-domain-porkbun` | Adapter | 3 ✅ | `cynic-domain-ops` | PorkbunApi (reqwest), implements `RegistrarPort` |
| `cynic-domain-cloudflare` | Adapter | 3 ✅ | `cynic-domain-ops`, `cynic-domain-ops/adapters/probe` | CloudflareApi, implements `CloudflareDnsPort` + `EdgeProbePort` |
| `cynic-domain-askesis` | Adapter | 3 ✅ | `cynic-askesis`, `cynic-domain-ops` | `BackupArchiver` — zone backup → append-only proof |
| `cynic-domain-pages` | Adapter (V2) | — | `cynic-domain-ops` | Cloudflare Pages deploy via Wrangler API |
| `cynic-domain-redirects` | Adapter (V2) | — | `cynic-domain-ops` | Bulk Redirect desired state + ruleset management |
| `cynic-domain-proxy-audit` | Adapter (V2) | — | `cynic-domain-ops` | CF proxy status audit + drift patch |
| `cynic-domain-route` | Adapter (V2) | — | `cynic-domain-ops` | Reverse-proxy route + DNS record creation |

### 7.3 Pourquoi des micro-crates séparés pour les providers ?

`CRATE_MODELS.md` §2(2) + anti-monolithe mandat : chaque provider API = une frontière
distincte. Un `adapters/porkbun.rs` monolithique dans `cynic-domain-ops` le pousserait
au-delà de 40k LOC et violerait le ratchet. Les providers évoluent à des rythmes
différents (Porkbun API v2 vs Cloudflare API v4). Clean-room extraction = compil
incrémental + test isolation + feature flags par provider.

Les adaptateurs partagent uniquement :
- `cynic-protocol::domain_ops::*` (types purs)
- `cynic-domain-ops::port::*` (traits)
- `serde`, `reqwest`, `serde_json` (dépendances communes)

---

## 8. Lifecycle Client Domain (poste central CYNIC)

### 8.1 Le flux complet — du client au site live verifié

```
Client request                       CYNIC central workstation
  │
  1. intake →                      cynic network domain plan --desired client.json --registry registry.json
  client_domain desired-state
  2. plan (read-only)            ← observe: availability, pricing, registrar zone
                                 ← reconcile: transform for Cloudflare, diff against CF zone
                                 ← emit: cynic.domain_ops.plan.v1
  3. consent                     → (operator review: --yes gated)
  4. apply                        → register domain (paid, --yes)
                                 → NS cutover (separate --yes gate, rollback captured)
                                 → zone backup → cynic-askesis
                                 → migrate zone (idempotency keys)
                                 → verify TLD delegation
                                 → verify edge HTTP
                                 → emit: cynic.domain_ops.proof.v1 chain
  5. site deploy (V2)           → cynic network domain pages deploy --desired client.json
  6. redirects (V2)             → cynic network domain redirects apply --desired client.json
  7. renewal watch              → cynic network domain renewals scan
                                 → (alerting via notify-send + Telegram)
  8. ongoing status              ← cynic network domain status --domain example.com
                                 ← cynic network domain verify --domain example.com
```

### 8.2 Registry externe — comment CYNIC découvre les providers

`infra/registry.json` est étendu (pas hardcodé) :

```jsonc
{
  "domain_providers": {
    "porkbun": {
      "api_endpoint": "https://api.porkbun.com/v2",
      "credential_ref": "env:PORKBUN_API_KEY",
      "adapter_crate": "cynic-domain-porkbun"
    },
    "cloudflare": {
      "api_endpoint": "https://api.cloudflare.com/client/v4",
      "credential_ref": "env:CF_API_TOKEN",
      "adapter_crate": "cynic-domain-cloudflare"
    }
  },
  "domain_projects_dir": "/etc/cynic/domain-projects/",
  "domain_backup_store": "askesis://domain-ops-backups/"
}
```

Le runner lit `--registry <path>` → résout le provider → injecte le bon adapter.
Aucun IP/host en dur (mandat utilisateur : dynamic registries > hardcode).

### 8.3 Multi-tenant desired-state

Chaque client = un fichier `domain_ops.desired_state.v1` JSON :

```json
{
  "schema": "cynic.domain_ops.desired_state.v1",
  "project_id": "client-restaurant-le-bon-bistrot",
  "domain": "lebonbistrot-menu.com",
  "registrar": { "provider": "porkbun", "credential_ref": "env:PORKBUN_API_KEY" },
  "dns_authority": { "provider": "cloudflare", "credential_ref": "env:CF_API_TOKEN" },
  "proxy_policy": { "web_records": "proxied", "mail_records": "dns_only",
                   "verification_records": "dns_only", "dnssec_records": "dns_only" },
  "dns_records": [
    { "name": "lebonbistrot-menu.com", "type": "A", "content": "1.2.3.4", "ttl": 300, "proxied": true },
    { "name": "www.lebonbistrot-menu.com", "type": "CNAME", "content": "lebonbistrot-menu.com", "ttl": 1, "proxied": true },
    { "name": "mail.lebonbistrot-menu.com", "type": "MX", "content": "mail.titouil.dev", "ttl": 300, "priority": 10, "proxied": false },
    { "name": "lebonbistrot-menu.com", "type": "TXT", "content": "v=spf1 include:_spf.cloudflare.com ~all", "ttl": 300, "proxied": false }
  ],
  "expected_nameservers": ["ns1.cloudflare.com", "ns2.cloudflare.com"],
  "edge_probes": [
    { "fqdn": "lebonbistrot-menu.com", "expected_status": 200, "follow_redirects": true },
    { "fqdn": "www.lebonbistrot-menu.com", "expected_status": 200, "follow_redirects": true }
  ],
  "pages_project": { "name": "lebonbistrot-menu", "source_dir": "sites/lebonbistrot-menu/dist" }  // V2
}
```

### 8.4 Renewal & lifecycle monitoring

```
cynic network domain renewals scan --registry infra/registry.json
  → read all desired-state files from project dir
  → for each: check_domains_expiry via RegistrarPort
  → emit: cynic.domain_ops.renewal.v1 (expiry, auto_renew, action_required)
  → alert: notify-send (local) + Telegram (if telegram_provider configured)

cynic network domain renewals apply --yes --domain <name>
  → RegistrarPort.renew_domain (paid, --yes gated)
  → emit: cynic.domain_ops.renewal.v1 proof
```

### 8.5 Proof pipeline — l'archivage CYNIC

Chaque opération → `DomainOperationProof` JSON → `cynic-askesis` append-only :

```
cynic-askesis/
  proofs/
    domain_ops/
      example.com/
        20260922T143000_register.sha3-512:...
        20260922T143100_ns_update.sha3-512:...
        20260922T143200_zone_migration.sha3-512:...
        20260922T143300_delegation.sha3-512:...
        20260922T143400_edge_probe.sha3-512:...
```

Le MCP read-back: `cynic_domain_ops_proofs(domain)` → lit depuis cynic-askesis →
affiche le proof chain sans appeler le provider (évite fausses négatives, rate limits).
