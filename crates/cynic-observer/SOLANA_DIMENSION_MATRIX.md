# Solana archive — orthogonal dimension matrix

The current fan-out is one vertical slice, not the whole Solana observation
system. It is frozen as a baseline until it is covered across the following
orthogonal dimensions.

## Baseline vertical

```text
plan_fanout
  → FanoutAction
  → SignatureSource / TransactionSource
  → retry policy
  → append-only FanoutStore
  → checkpoint and metrics
  → CLI / MCP read-only projection
```

This slice proves sequencing and bounded recovery in fixtures. It does not yet
prove chain completeness, semantic correctness, identity attribution or public
governance.

## Orthogonal dimensions

| Dimension | Question | Durable contract | First proof |
|---|---|---|---|
| substrate | What did Solana actually return? | provider payload, response digest, slot, commitment | RPC response fixture |
| time/coverage | Which interval is observed and what is missing? | cursor, boundary, gap, finality transition | two-page gap fixture |
| provenance | Who/what supplied each claim? | provider, role, endpoint class, retrieval time | Helius/RPC reconciliation |
| economics | What did a request cost? | per-request estimate plus usage-window delta | credit catalog fixture |
| reliability | What happens on failure or restart? | retry state, halt reason, atomic checkpoint | 429/timeout/restart fixture |
| storage | Can evidence be replayed without mutation? | append-only payload and checkpoint read-back | file-store round trip |
| semantics | What does an instruction mean? | decoder version, family, unknown path | Pump instruction fixture |
| identity | Which wallet/entity relationship is supported? | attributed evidence graph, confidence, alternatives | funding/authority fixture |
| cross-source | Do providers agree or conflict? | corroborated/conflicting/unknown result | Helius vs RPC fixture |
| interface | Can humans and agents consume the same result? | JSON schema, CLI/MCP projection | projection parity test |
| governance | Who may curate, contest or publish? | consent, curator decision, contest record | human-in-loop fixture |
| security | What must never leave the node? | secret presence only, redaction, capability boundary | poisoned-secret test |
| operations | Can a node observe its own health? | lag, throughput, error, cost and residual-risk metrics | runtime proof bundle |
| social/cultural | How is lore separated from fact? | claim/evidence/interpretation/source labels | ASDFASDFA claim fixture |

## Test layers

Tests must be added in this order:

1. pure contracts: parsing, validation, reconciliation and decisions;
2. adapter fixtures: Helius/RPC response shapes and error mapping;
3. persistence: append-only writes, atomic checkpoints and restart;
4. projection parity: CLI and MCP emit the same plan schema;
5. domain fixtures: Pump lifecycle, ownership evidence and unknown paths;
6. live canary: one bounded target, explicit key, explicit budget and read-back.

No live canary is evidence for a missing lower-layer fixture. Unknown remains a
first-class outcome until the relevant dimension has a proof.

## Current gaps

- the runner is tested with fake providers, while CLI/MCP projection tests still
  depend on the canonical validation gate;
- no cross-provider reconciliation is connected to the runner;
- usage-window credit deltas are modeled but not fetched from the Helius Admin
  API;
- Pump decoding is conservative but not yet connected to persisted fan-out
  transactions;
- identity and lore claims remain separate product-domain work;
- no live canary has been authorized.

The parameter parser is now canonical in `FanoutPlanRequest`; CLI and MCP only
assemble inputs and project the resulting `FanoutExecutionPlan`. This removes
surface-specific interpretation drift. The remaining parity proof is to invoke
both compiled surfaces under the authorized build/conformance harness and
compare their JSON payloads byte-for-byte after normalization of formatting.

The next implementation increment is therefore a contract-test harness and
projection parity test, not another provider feature.

`CoverageReport` is now the first horizontal aggregation boundary. It separates
observation completion from provenance completeness: an exhausted cursor can
still produce `partial` when signatures lack reconciliation records, and any
cross-source disagreement produces `conflicting` rather than being averaged
away.

`ProviderCapabilityProfile` and `summarize_provider_probe` now provide the
provider-neutral measurement boundary. Archive depth remains unknown until a
bounded historical probe proves it; a provider claim is never treated as
runtime evidence by itself.

The raw adapter can now identify `PublicRpc`, `Alchemy`, `QuickNode` and
`Chainstack` without guessing credential placement. Authenticated endpoints
must be fully resolved by the secret/configuration layer; provider adapters do
not synthesize query keys or path tokens. Persisted signature pages and raw
transactions can be converted directly into probe samples for later profiling.
