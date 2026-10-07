# Credential Bootstrap & Recovery — OS identities have no lifecycle

Status: `proposed` — awaiting operator validation before any code moves.
Schema: `cynic.credential_bootstrap_recovery.v1`
Evidence date: 2026-10-07. Trigger: builder `cynic` password unknown to
everyone including the operator; builder root equally unknown. A fleet node
whose OS credentials exist outside every vault and procedure is one lost
key away from unrecoverable.

## 1. What already exists (read, not rebuilt)

| Piece | Source | Covers |
|---|---|---|
| Breakglass admission | `crates/cynic-protocol/src/breakglass.rs` | Signed, time-bounded, single-consumption incident authority for RUNTIME actions (`RecoveryBreakglass` class) |
| Machine enrollment + renewal | `crates/cynic-node/src/usecases/node_enrollment_lifecycle_uc.rs` | Node identity issuance and grant renewal against node-control |
| Runtime seed | `infra/runtime-basket.json` §runtime_seed | One-time HumanAdmin install, rotation only via separately signed ceremony |
| Secret classes | `cynic-protocol::secret_reference` | HumanAdmin (bootstrap only), MachineEnrollment, RuntimeMachineIdentity, RecoveryBreakglass, HeadscaleApiKey |
| Sealed stores | `cynic-secrets` (env → sealed file → OS keyring) | Application tokens, mid-migration (headscale, freebox, forgejo in keyring) |
| Privilege grants | `infra/privilege-grants.json` | Bounded sudoers, propose-then-install, read-back via `sudo -l` |

## 2. The hole (declared, not solved)

None of the above governs **OS-level credentials**: UNIX passwords, ssh
authorized-keys issuance, sudoers membership at provisioning time. The
lifecycle has four obligatory phases; all four are unowned today:

1. **Issuance**: at provisioning, per-machine credentials are generated
   (never chosen, never reused across nodes), scoped to least privilege.
2. **Escrow**: the generated material is sealed into the vault in the same
   ceremony — root-held copy for recovery, operator-held copy for daily
   use where needed. No cleartext, no chat logs, no memory of the value.
3. **Recovery**: when all live access is lost (the current builder state),
   a documented breakglass path re-establishes access: physical/console
   root OR hypervisor-mediated reset (PVE shell into LXC / console for
   bare metal), then immediate rotation + re-escrow. Recovery without
   rotation is just re-losing the credential slower.
4. **Rotation**: periodic or on-incident rotation of OS credentials, with
   the same gate discipline as releases (read-back: login with the new
   credential through a non-interactive probe, rollback = previous
   credential retained until the new one proves).

## 3. Non-goals (explicit)

- No password manager SaaS, no cloud KMS: escrow is the sovereign vault
  plus OS stores already in the posture, nothing exogenous.
- No shared passwords across nodes, ever. A credential names one
  (user, node) pair or it does not exist.
- No agent ever prints, logs, or transmits credential material. Presence
  and rotation proofs only — the same law as application secrets.

## 4. Work packages (in order)

1. **Immediate triage (builder)**: regain root via hypervisor path (PVE
   console/shell — the operator's gesture), reset `cynic` + root,
   rotate, escrow. Closes the live incident; procedure notes become the
   first draft of §2.3.
2. **Secret class + registry**: add the OS-credential class to the secret
   taxonomy and a `credential_escrow` section to the posture (ids, never
   values; rotation dates; recovery path per node).
3. **Provisioning ceremony**: new nodes are born with generated credentials
   + sealed escrow + grant registration (`privilege-grants.json`) in one
   admission — no hand-set passwords, ever again.
4. **Rotation loop**: periodic rotation with read-back proof, modeled on
   `runtime-update.json` (stage → prove → cut over → keep previous until
   proven).
5. **Breakglass binding**: tie OS recovery to `BreakglassAdmission`
   (signed, time-bounded, consumed) so emergency access is authority,
   not improvisation.

## 5. Decisions (operator — recorded 2026-10-07)

1. **Escrow holders** — DECIDED: two holders, split knowledge, dual
   control. Holder A (daily): operator keyring, per-node credentials for
   legitimate administration. Holder B (recovery): sealed OFFLINE copy
   (encrypted media, physically secured), root material only, breakglass
   use exclusively. Rationale from current incidents (scattered tokens,
   unknown passwords, no root): a single holder is a SPOF, shared
   knowledge is a leak; production-grade norms (least privilege, dual
   control, audit trail) require that no one person can both USE daily
   access and SILENTLY reconstitute root. Every escrow use — either
   holder — produces a journal entry; breakglass uses additionally
   require a signed `BreakglassAdmission` (consumed receipt).
2. **Recovery path per node** — DECIDED: builder is a container/VM on
   Proxmox (not bare metal), so recovery = PVE host shell (`pct`/`qm
   enter or equivalent) + reset + rotate + re-escrow. Precondition to
   verify: operator PVE-root access. Same path covers all LXC
   satellites; documented per node in tranche 2 (credential_escrow
   registry).
3. **Rotation cadence** — DECIDED: 90 days; on-incident always,
   immediately, with re-escrow in the same ceremony.
