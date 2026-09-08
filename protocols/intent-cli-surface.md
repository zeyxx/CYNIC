# Intent CLI surface — `cynic agent intent seal|verify` (Phase 0, prose only)

Source discipline: painfully-explicit-specs (@TheAhmadOsman on X).
STATUS: PROPOSAL — human validation required before Rust.
Consumer: field teams (bootcamp NEXUS SHIELD M2+) record transmissions;
this surface is how they seal and check records natively. Replaces
scripts/intent-seal-TEMP.py on landing (TEMP file deleted in same slice).
HOME: the Agent family absorbs it (`cynic agent intent ...`) — no new
top-level family for two leaf commands. This home is CONDITIONAL, see
§Move-out condition.

## Why

- intent.rs exists and is proven, but unreachable without a Rust toolchain.
  Field consumers have none. The TEMP python adapter fills the gap with an
  explicit expiry — a native surface is the durable fix.
- Thin-surface law: CLI projects domain policy, never owns it. All validation
  stays in intent.rs validate(); the CLI parses argv, calls, prints, exits.

## What (two subcommands, thin facade)

- `cynic agent intent seal [--in-place] <record.json>` — read record without
  digest, compute qualified sha3-512 digest, print or write back.
- `cynic agent intent verify <sealed.json>` — recompute, exit 0 + print digest on
  match, exit 1 + MISMATCH line otherwise (machine-readable: exit code is
  the contract).
- `--json` flag for both (verify prints {status, digest} object).
- No record creation wizard, no editing, no journal — separate slices.

## Where

- cynic-cli agent_surface: `intent` arm (facade ≤ 250 lines for the arm),
  wired through the existing Agent dispatch in main.rs + root_dispatch.rs.
  Registry entry under agent_surface (surface-registry fragment +
  agent-exposure fallback command), per recipe.
- Nesting law complied: nested arm, zero new top-level hyphens
  (see cynic-cli-thin-surface §Nesting over hyphens).
- Calls into cynic-protocol intent.rs only. No new types, no policy copy.
- TEMP python deleted in the landing commit (same slice, explicit paths).

## Move-out condition (maintainability gate)

This home holds WHILE the Agent family stays absorbable: the intent arm
plus its registry entries remain a thin projection (no policy, no state,
no second coordination model inside agent_surface). It EXPIRES when D2/D3
land (Unknown migration, Admission + journal): at that point Intent is a
domain with its own lifecycle and storage, and a top-level `cynic intent`
family (or a coordination family) gets its own slice + absorption proof.
The expiry is checked at each slice review — staying silent while
agent_surface bloats would be implicit debt.

## Gates

- `seal` output on the trap vector (accents, quotes, newline, slash,
  backslash) equals f752c3…8fbfc9 (regression test pins it).
- `verify` on tampered content exits 1.
- make check green; kernel-audit not degraded.
- TEMP file gone; no python intent tooling remains.

## Explicit unknowns

- U1: clap arg-shape precedent in current main.rs (inventory at build time) —
  owner slice implementor, due before code.
- U2: exit-code-only vs JSON default for verify — owner human. Proposal:
  exit code primary (unix contract), --json opt-in projection.

## Rollback

Remove module + dispatch arms + restore TEMP file. Zero consumers at landing
except field teams (notified via their pointer README).
