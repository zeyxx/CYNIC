# Collaborator Trust Promotion Criteria (OQ-4)

## Purpose

Defines the criteria for promoting a collaborator from one trust tier to the next
in the CYNIC external project consortium. This is a policy document, not code.

## Trust Tiers

| Tier | Name | Description |
|------|------|-------------|
| 3 | Observer | Read-only access. Can observe project state, build/test results. |
| 2 | Contributor | Branch + PR. Can create branches and open PRs on external repos. |
| 1 | Maintainer | Direct push (gated by mutation_policy). Can push to non-protected branches. |

## Promotion Criteria

### Tier 3 → Tier 2 (Observer → Contributor)

**Required (all must hold):**
1. **Identity verified**: Collaborator has a verified identity in `infra/collaborators.json`
   with a valid `principal` and `trust_tier` field.
2. **Read-back proof**: At least one successful `cynic audit external` run showing
   the collaborator's project is observable (build/test commands observed).
3. **No incidents**: No security incidents, policy violations, or unauthorized
   mutation attempts in the last 30 days.
4. **Operator approval**: Explicit operator approval (human-in-the-loop).

**Evidence required:**
- `infra/collaborators.json` entry with `trust_tier: 2` or higher.
- At least one `ExternalBuildProof` with `overall_success: true` for the project.
- No entries in `infra/agent-claims.json` with `status: blocked` for this collaborator.

### Tier 2 → Tier 1 (Contributor → Maintainer)

**Required (all must hold):**
1. **Tier 2 tenure**: At least 14 days at Tier 2 with no incidents.
2. **PR track record**: At least 3 merged PRs on external repos with read-back proof.
3. **Build proof**: At least 5 successful build/test runs on the builder with read-back proof.
4. **Mutation policy compliance**: No violations of `mutation_policy` (branch_pr or
   direct_push_forbidden) in the last 30 days.
5. **Operator approval**: Explicit operator approval (human-in-the-loop).

**Evidence required:**
- `infra/collaborators.json` entry with `trust_tier: 1` or higher.
- At least 3 `ExternalPrProof` entries with `success: true` for the collaborator.
- At least 5 `ExternalBuildProof` entries with `overall_success: true`.
- No `admit_external_source_operation` blockers in the audit log.

## Demotion Criteria

A collaborator may be demoted if:
1. **Security incident**: Any unauthorized mutation attempt or policy violation.
2. **Stale identity**: No activity for 90 days (demote to Tier 3).
3. **Operator decision**: Explicit operator demotion.

## Audit Trail

All promotions and demotions must be recorded in `infra/collaborators.json` with:
- `trust_tier` updated.
- `promoted_at_unix_ms` or `demoted_at_unix_ms` timestamp.
- `promotion_evidence` array of proof references.

## Current Status

- OQ-4 is **enforced in code** as of 2026-10-18.
- Trust tier checks live in `cynic-vcs/src/external_source.rs` (`admit_external_source_operation` with `trust_tier` param).
- Promotion/demotion usecase: `cynic-node/src/usecases/collaborator_trust_uc.rs` (`evaluate_trust_promotion`).
- CLI surface: `cynic collaborator status|promote|demote` (see `cynic-cli/src/collaborator_surface.rs`).
- Promotion requires: operator approval (`--yes`), evidence refs, and trust tier admission for the target operation.
- Tier 2 → Tier 1 requires ≥3 evidence refs (PR track record).
- Tier 3 → Tier 2 requires ≥1 evidence ref (read-back proof).
- Current collaborators: titouan (trust_level: core), robinho100 (trust_level: observed).