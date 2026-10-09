# H1 Intent — Typed Human Intent (Phase 0 design doc, prose only)

Source discipline: painfully-explicit-specs (@TheAhmadOsman on X).
Constitutional basis: explicit_over_implicit (foundation.json hard boundary,
CONSTITUTION.md Article VIII §8) — the implicit is debt.
Status: PROPOSAL v3 — reduced scope after 4-prism review (constitutional,
minimalist, adversarial, field). Human validation required before Rust.

## Why (observed findings, live proof)

- H1 (debt-audit Tier1, avéré): intent non typé — reinterpreted per session.
- Derived-status audit 2026-09-08: Transmission = prose_only.
- 4-prism review 2026-09-08: full-scope v2 unshippable (13 gaming vectors
  uncaught, 7/18 fields unfillable in the field). Verdict: one type first.
- Digest ambiguity (live, lib.rs:48 vs lib.rs:327): TWO digest_json coexist —
  integrity::digest_json (sha256, glob-re-exported at crate root) and
  digest::digest_json (sha3-512, qualified path only). Any implementer writing
  crate::digest_json seals with the wrong algorithm. Filed as organic debt;
  v3 pins the qualified path explicitly.

## What (one type, nothing more)

IntentTransmission (schema cynic.intent_transmission.v1). Pure structural
data + structural validation, zero I/O, owner cynic-protocol:

  id: string (uuid v4, generated at creation — cross-refs target this, NOT digest)
  schema: "cynic.intent_transmission.v1"
  created_at: string (RFC 3339 UTC — temporal ordering without journal)
  emitter: string (convention "pole:<name>" | "human:<name>" — see §Conventions)
  recipient: string (convention "agent:<id>" | "node:<id>" | "role:<name>")
  deliverable_ref: string (free-form mission tag, e.g. "M2-carto")
  content: string (1–2000 chars; the "3–5 lines" cap is DROPPED — field review
    proved it unenforceable and undersized for tool output)
  provenance_class: official_source | provided_kit | observed_fact | unverified_proposal
    (observed_fact ADDED — field review: Nmap results are neither official nor
    proposal; forcing them into unverified_proposal corrupts semantics)
  provenance_source: string (non-empty; who/what produced it)
  digest: string (sha3-512 via crate::digest::digest_json — QUALIFIED path,
    never the crate-root re-export)

validate() checks: non-empty id/emitter/recipient/content/source; content
length bound; provenance_class in enum; digest == recomputed digest_json(self
minus digest field); created_at parses as RFC 3339. NOTHING ELSE. No quality
judgment, no comprehension proof, no ordering enforcement — shape only, and
the doc says so (see §Gates).

## Where (exact boundaries)

- Phase 1 creates: crates/cynic-protocol/src/intent.rs (+ inline unit tests),
  re-export in lib.rs. No other file touched.
- Structural validation only. Domain predicates (conflict, admission, routing)
  belong to the Judicial layer and do not exist yet — their absence is stated,
  not hidden.
- Precedent paths (corrected after review): agent_coordination::predicates,
  residual_unknown::evaluate_residual_unknowns — arithmetic invariants, not
  policy. The stated line stands: invariant arithmetic is structural; conflict
  and admission decisions are policy and live in kernel when built.
- Forbidden: a second type in this slice; logic in CLI; parallel control plane.

## Conventions (strings today, structs when a consumer dispatches on them)

Emitter/recipient stay strings until a dispatcher consumes the namespace.
Convention is documented and validated by prefix check (must contain ':',
prefix in {pole, human, agent, node, role}). Parsing into a struct is DEFERRED
to the slice that routes on namespace — building the enum now is speculative
abstraction with zero consumers.

## Deferred (own slices, own docs, named consumers)

- IntentUnknown → slice D2, consumer: legacy migration (31 sites) + journal.
- IntentAdmission → slice D3, consumer: CLI projection / journal. Carries the
  constitutional questions with it: judicial validation assignment (Art. I —
  judiciary validates sovereign directives) and ratification-vs-routing
  classification (mandate_ref or documented sovereign act).
- Budget ref → with D3, after capability_budget inventory; never Option<String>.
- Transmission lifecycle (DRAFT/EMITTED/RECEIVED), Unknown lifecycle,
  coexistence serde rule (evolutive policy, not constitutional text) → with
  the persistence slice that gives transitions something to guard.
- Digest glob-shadow cleanup (remove or rename the ambiguity) → own micro-slice.

## Field tooling (required before M2, or the team finishes on Discord)

Types without tooling are ceremony. Before field use: (1) JSON Schema file
for IntentTransmission shared to all 5 assistants; (2) one-command digest
tool (reads JSON without digest field, prints qualified sha3-512 digest).
Both are adapters, not protocol — they live outside cynic-protocol.

## Gates (labeled: what they catch AND what they don't)

Machine (shape): compiles; validate() rejects empty/oversize/mismatched
digest/unparsable timestamp; make check green; kernel-audit not degraded.
Explicitly NOT caught: vacuous content ("ok", "TBD"), rubber-stamp motives
(no admission type yet — nothing to stamp), digest-without-reading, role
catch-alls, non-adoption (types are opt-in until persistence lands). Stating
the uncovered surface is required by explicit_over_implicit; count-gates that
pretend to prove quality are forbidden.

## Explicit unknowns (owned, due scale)

- U2 (carried): budget unit — owner human, due D3. No budget field in v3.
- U5 (narrowed): namespace-to-identity binding — owner Phase 1 inventory
  (node_identity.rs, workload_identity.rs), due before types freeze. Outcome
  is documentation only (which prefix maps to which registry); no struct.
- U6 (new): id collision policy (uuid v4 assumed sufficient; no registry to
  check against pre-persistence) — owner persistence slice.

## Rollback intent

Prose: amend/delete this file. Phase 1: drop intent.rs + re-export, zero
consumers, no migration, no state.
