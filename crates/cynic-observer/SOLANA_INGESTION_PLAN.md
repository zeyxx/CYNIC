# CYNIC Observer — Solana ingestion baseline

## Source strategy

Helius is the ergonomic indexing and parsing adapter. The public Solana RPC is
the independent observation and verification adapter. Neither is the source of
truth alone: the ledger is the chain, while providers are observations of it.

```text
Helius: fast history, parsed transactions, DAS, webhooks
Public RPC: raw accounts, slots, signatures, transactions, independent checks
Local WAL: durable evidence, replay, deduplication, gap accounting
```

Helius currently documents separate rate-limit groups for RPC and DAS/Enhanced
APIs, with Free-plan base limits of 10 RPC requests/second and 2 DAS/Enhanced
requests/second. These values are provider configuration, not protocol law;
the adapter must observe and report the effective limits rather than hardcode
them. [Helius rate limits](https://www.helius.dev/docs/billing/rate-limits)

The current credit catalog also makes plan capability part of planning:
`getTransactionsForAddress` costs 50 credits and requires Developer or above;
standard archival RPC calls such as `getSignaturesForAddress` and
`getTransaction` cost 1 credit. A Free-plan baseline therefore uses signature
pages plus transaction fan-out, while retaining the indexed-history adapter for
a later plan upgrade. [Helius credits](https://www.helius.dev/docs/billing/credits)

Every provider request retains:

```text
method + provider + start time + latency
request/response bytes + HTTP status + returned records
catalog credit estimate + catalog version
optional project-usage delta reconciliation
```

Catalog estimates are request-local. Project usage snapshots can confirm an
exact delta only in an isolated one-request window; a shared window is labeled
as such instead of attributing concurrent usage to one request.

## Workloads

### 1. Asset and account snapshot

Use Helius DAS for `getAsset`, `getAssetBatch`, `getAssetsByOwner`,
`getTokenAccounts` and searches where available. It is useful for mint
metadata, token accounts, authorities, ownership snapshots and asset joins.
DAS supports fungible tokens and Token-2022 in addition to NFTs and compressed
assets. [DAS overview](https://www.helius.dev/docs/das-api)

### 2. Historical transaction backfill

For each target mint, wallet, program or known signature:

1. use Helius `getTransactionsForAddress` in full mode as the primary indexed
   backfill path, with chronological sorting where origin analysis matters;
2. enumerate signatures with a declared direction and commitment when a raw
   RPC comparison is required;
3. persist pagination token, page, boundary signature and provider response
   digest;
4. fetch raw `getTransaction` through RPC for critical transactions;
5. normalize instructions, inner instructions, logs, balances and account
   ownership into the CYNIC evidence model;
6. deduplicate by signature and retain provider disagreement;
7. run a boundary audit from both the newest and oldest known signature.

Helius documents `getTransactionsForAddress` as its modern native backfill path:
it supports full transaction pages, advanced filters, chronological order and
pagination tokens. The human-readable Enhanced Transaction API is now a legacy
maintenance surface, so it may remain an optional semantic adapter but should
not own the ingestion contract.
[getTransactionsForAddress](https://www.helius.dev/docs/rpc/gettransactionsforaddress),
[legacy transaction history](https://www.helius.dev/docs/enhanced-transactions/transaction-history)

### 3. Live observation

Use two live paths:

- Helius raw/enhanced webhooks for declared addresses and programs;
- RPC WebSocket subscriptions (`logsSubscribe`, `accountSubscribe`, or
  `programSubscribe`) as an independent local observer.

Webhooks are notifications, not durable storage. They can be duplicated,
delayed or disabled after delivery failures, so every webhook must be
acknowledged quickly, written to the WAL, deduplicated and reconciled later.
[Helius webhooks](https://www.helius.dev/docs/webhooks),
[Webhook FAQ](https://www.helius.dev/docs/faqs/webhooks)

## Pump.fun baseline

The first domain adapter should not try to understand every Solana program. It
should identify a small set of program families and preserve raw evidence:

```text
mint creation
bonding-curve state
buy/sell/trade event
creator/dev funding
fee claim
migration to an AMM
burn/buyback/transfer
authority changes
```

Every decoded event must retain `program_id`, instruction index, inner
instruction path, signature, slot, block time, commitment, source provider,
decoder version and the raw transaction reference. A parser result is an
interpretation; the raw transaction remains the lower-layer evidence.

## Provider roles

| Concern | Preferred adapter | Verification |
|---|---|---|
| parsed wallet history | Helius history | RPC signatures + selected raw tx |
| fungible-token metadata/holdings | Helius DAS | RPC mint/account state |
| critical transaction proof | public RPC `getTransaction` | second provider or replay |
| live address/program events | Helius webhook + RPC WS | periodic signature backfill |
| slot/finality | RPC | independent RPC probe |
| missing historical range | Helius if entitled | explicit unknown, never guess |

This is intentionally asymmetric. Helius is the primary indexed/semantic
source because it reduces the cost of turning transactions into usable domain
events. Public RPC is the raw independent source: slower and more laborious,
but valuable for critical transaction verification, account state and parser
coverage gaps. CYNIC must not demand that both sources expose identical
schemas; it records their roles and reconciles their evidence.

This is provider redundancy, not blockchain consensus. A match between Helius
and RPC increases confidence in observation completeness; it does not prove a
human identity or economic ownership.

## Evidence and coverage ledger

Each ingestion run needs:

```text
cluster, commitment, provider, endpoint class
target kind/address/program/mint
start/end slot or signature boundary
cursor/page and request hash
retrieved_at, response hash, retry/429/error status
signatures observed, signatures decoded, signatures unresolved
finality changes and reorg/reconciliation status
unknowns, parser version, schema version
```

The current comparison counters in `consensus.rs` are useful telemetry but are
not yet a completeness proof: comparing total event counts can produce a
misleading percentage when windows, filters or duplicates differ. Replace
that metric with set-based coverage over `(cluster, signature, slot)` and an
explicit observation window.

## First implementation increment

The ODYSSEUM reader is a quarantined compatibility path for historical
fixtures only. It must not become the normal ingestion route or a source of
truth. New observations enter through live provider ports and receive their
own provider, retrieval and response-digest provenance.

## Solana MCP boundary

The official Solana MCP is a useful stateless documentation, semantic-search
and program-checking adapter exposed over Streamable HTTP. It can help agents
resolve current Solana and framework details, but it is not an ingestion
database and must not own CYNIC cursors, WAL state, coverage or evidence
retention. [Official Solana MCP](https://mcp.solana.com/)

```text
Solana MCP → current docs / expert guidance / code checks
Helius/RPC  → live chain observations
CYNIC      → cursors / WAL / evidence / reconciliation / proof
```

The stateless MCP shape is an architectural advantage: every query remains
replayable and CYNIC retains the durable state explicitly. The MCP output is
context or an external observation, never a replacement for a finalized RPC
response or a retained provider payload.

Add a fixture-backed `solana_archive` boundary before expanding the watcher:

```text
types → coverage ledger → provider ports → Helius adapter
                                 ↘ RPC adapter
                                      ↘ WAL + dedupe + reconciliation
```

The fixture must prove pagination, retries, duplicate webhook delivery,
provider disagreement, failed transactions, missing parsed instructions,
commitment changes and an explicit `unknown` result. The live ASDFASDFA run
then becomes a bounded baseline around the known mint, creator/dev wallets,
Pump.fun program and selected transaction signatures.

The Helius API key belongs only in environment/file secret injection. It must
never appear in source, fixtures, raw evidence, logs or model context.

Backfill persistence follows a small transaction:

```text
persist append-only provider page
→ read-back/content digest
→ derive next checkpoint
→ atomically replace checkpoint
```

If checkpoint persistence fails, the immutable page remains replayable and the
cursor does not advance in memory. Target storage directories use a digest of
`cluster + target`; untrusted addresses are never interpolated as raw paths.

The Free-plan standard-RPC runner uses the same transaction shape, but separates
the two observation stages:

```text
getSignaturesForAddress page
→ persist signature page + advance signature cursor
→ queue unseen signatures
→ getTransaction per queued signature
→ persist raw transaction + remove queue item
→ checkpoint metrics after each request
```

This preserves resumability when the process stops between the signature scan
and transaction retrieval. It also makes the expensive part visible: every
request carries method, provider, latency, payload sizes, records returned and
credit measurement. Helius catalog values are estimates; isolated project
usage deltas are retained as the stronger measurement when the Admin API can
be sampled before and after a run. The runner intentionally refuses processed
commitment, zero budgets and unbounded pages.

The fixture contract also covers a multi-page window where a signature appears
again on the next page: occurrence counts and unique-signature counts remain
separate, and the transaction queue receives the signature only once. This is
important because pagination boundaries and replayed deliveries are normal
observation conditions, not evidence of two distinct on-chain transactions.

Failures are retained in the same checkpoint. Rate limits, transport failures,
RPC failures and HTTP `408`/`425`/`429`/`5xx` responses receive bounded
exponential backoff, respecting a provider `retry-after` hint. Malformed or
blocked responses stop immediately as explicit non-retryable failures. A
retry decision is persisted before the worker sleeps, so a restart cannot turn
an already-consumed retry budget into an unbounded loop.

The executable runner remains adapter-driven:

```text
SignatureSource + TransactionSource
                ↓
        pure FanoutAction loop
                ↓
 FanoutStore + injected FanoutSleeper
```

The default sleeper is Tokio, but tests and closed harnesses can inject a
zero-delay sleeper. The runner therefore owns sequencing and recovery, while
the RPC adapter owns HTTP and provider-specific response parsing.

Before execution, `plan_fanout` produces a serializable dry plan containing the
selected Helius strategy, expected credit estimate, initial action, target and
authority flags. It explicitly states that the operation does not mutate the
chain, but does require network access and secret injection. This plan is the
future command/MCP boundary; it is not itself permission to execute.

The vertical must also be evaluated against the orthogonal dimension matrix in
[`SOLANA_DIMENSION_MATRIX.md`](SOLANA_DIMENSION_MATRIX.md). A successful
fan-out run proves sequencing only; it does not prove completeness, ownership,
semantic decoding, cross-provider agreement or cultural interpretation.
