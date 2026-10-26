# Engineering Patterns — Observed, Not Prescribed

Patterns observed in the CYNIC codebase. Not rules. Not gates. Observations with maturity scores.

Each pattern has: name, status (applied/partial/missing), where it lives, verify command.

## Applied (with proof)

### Registry-based resolution
- Status: partial (builder_pool exists, 32 files still hardcode)
- Where: crates/cynic-build/src/builder_pool.rs (resolve_builder_pool, select_by_pressure)
- Verify: `grep -rn "10.10.10.119" --include="*.rs" crates/ | grep -v test | wc -l` (should decrease over time)

### Fail-closed selection
- Status: applied
- Where: crates/cynic-build/src/builder_client.rs (returns error if no healthy builder)
- Verify: `grep -n "no healthy build_runner" crates/cynic-build/src/builder_client.rs`

### Ratchet (shrink-only baseline)
- Status: applied (ratchet tests exist, shrink-only)
- Where: crates/cynic-host/tests/crate_taxonomy_ownership.rs, cynic-knowledge/tests/corpus_coverage_ratchet.rs
- Verify: `grep -rn "may only shrink\|may only decrease" crates/ --include="*.rs"`

### Multi-role node support
- Status: just added (2026-10-26)
- Where: crates/cynic-build/src/builder_pool.rs (role + roles array), infra/registry.json (compute-01)
- Verify: `grep -n "roles" infra/registry.json | head`

### Consultation before action
- Status: partial (cynic know search exists, not called by default)
- Where: crates/cynic-cli/src/knowledge_surface.rs (KnowCommand::Search)
- Verify: `cynic know search builder_pool`

## Missing (observed, not yet applied)

### Epistemic motor wired end-to-end
- Status: missing (components exist, not cabled)
- Where: cynic-knowledge (epistemic facts, corpus), cynic-traces (hermes adapter), memory (Hermes 2200 chars)
- Verify: `cynic know search epistemic motor` (returns docs, not code integration)

### Automatic consultation before action
- Status: missing (agents don't call cynic know search by default)
- Where: session compass references it, but not enforced
- Verify: grep agent sessions for "cynic know search" usage

### sccache shared cache
- Status: missing (cold compile tolerated)
- Where: compute-01 pool note says "no sccache on host"
- Verify: `ssh compute-01 which sccache` (expected: not found)

### Multi-role schema in Node struct
- Status: missing (Node has role: String, not roles: Vec<String>)
- Where: crates/cynic-node/src/domain/fleet_truth/model.rs
- Verify: `grep -n "pub role\|pub roles" crates/cynic-node/src/domain/fleet_truth/model.rs`

### Vocabulary SSOT complete
- Status: partial (only biological metaphors covered)
- Where: crates/cynic-protocol/src/vocabulary.rs
- Verify: `grep -n "askasis\|proof.log\|ProofLog" crates/cynic-protocol/src/vocabulary.rs` (expected: not found)