# ODYSSEUM → CYNIC Observer

## Audit scope

ODYSSEUM is treated as a working laboratory and evidence fixture. It is not
copied as runtime authority. The useful question is: which behaviours already
survived contact with Solana, and which responsibilities must be separated
before they become a CYNIC contract?

The Pump warehouse already demonstrates a substantial baseline:

- signature pagination with a resumable cursor;
- retry/backoff and request pacing;
- atomic manifest/page writes;
- compressed JSONL raw pages and compact day partitions;
- SHA-256 payload/archive hashes;
- signature-level deduplication during replay;
- raw → token trace → market/revenue ledgers;
- raw-integrity validation and repair reporting;
- local Pump, PumpSwap and fee-program IDLs and parsed documentation;
- account/PDA inspection for bonding curve, sharing config and AMM states.

These are the right empirical primitives for CYNIC. They should become typed
ports, policies and proof schemas rather than remain embedded in scripts.

## What the prototype teaches

### Durable patterns to retain

1. **The signature is the first immutable join key.** Everything derived from a
   transaction must retain signature, slot, block time, commitment, instruction
   path and decoder version.
2. **The warehouse is append-oriented.** New pages and observations are added;
   derived views can be regenerated.
3. **Manifest and archive are different objects.** The manifest describes
   coverage and files; the archive carries observations.
4. **Raw and interpreted data are separate.** A token trace or revenue ledger
   is a projection, not a replacement for the transaction.
5. **The chain must be inspected at account level.** Mint, bonding curve,
   token accounts, vaults, sharing config and AMM pool cannot be inferred only
   from provider labels.

### Prototype debt to contain

- scripts currently combine provider access, pagination, decoding, persistence
  and reporting in one execution path;
- the source label `helius` can conceal that the underlying method is generic
  JSON-RPC and that provider provenance needs to be recorded per response;
- a cursor or `max-pages` run proves progress, not exhaustion;
- count-based provider comparison is not set-based completeness proof;
- a repaired malformed raw file must remain a repair event and cannot silently
  become original evidence;
- absolute paths in manifests prevent portable replay;
- JSON-shaped events are flexible but do not enforce cluster, commitment,
  ownership, discriminator or decoder invariants;
- old and new Pump/PumpSwap instruction families coexist, so one decoder
  profile cannot safely represent every token lifecycle;
- revenue and market ledgers can accidentally turn an interpretation into a
  financial fact unless every amount points back to observed balance deltas or
  decoded event evidence.

## Hexagonal target architecture

```text
                 ┌─────────────────────────────┐
                 │ CYNIC Observer application  │
                 │ backfill / reconcile / trace │
                 └──────────────┬──────────────┘
                                │ uses ports
        ┌───────────────────────┼────────────────────────┐
        │                       │                        │
  source ports             decode ports             evidence ports
  signatures               program profile          WAL/archive
  transactions             instruction decoder      coverage ledger
  accounts                  state decoder            integrity verifier
  live events               event classifier         replay reader
        │                       │                        │
  Helius/RPC/WS          IDL + raw fallback       local files/object store
```

### Core domain

The domain owns only portable concepts:

- `ObservationEnvelope`;
- `TransactionObservation`;
- `AccountSnapshot`;
- `CoverageWindow` and `CoverageGap`;
- `ProgramProfile` and decoder version;
- `DecodedFact` with evidence references;
- `DerivedProjection` and contestation state.

It does not own Helius URLs, `reqwest`, WebSocket details, filesystem paths or
Pump SDK types.

### Ports

The first ports should be:

```text
SignatureSource
TransactionSource
AccountSource
AssetSource
LiveEventSource
ProgramDecoder
ObservationStore
CoverageStore
ClockAndRatePolicy
```

Each port returns provider provenance, raw response digest, cursor state,
commitment and explicit `unknown`/`blocked` results. A provider adapter may be
replaced without changing the domain or reconciliation policy.

### Application use cases

Keep these separate:

```text
backfill_target
reconcile_window
decode_transaction
inspect_asset_state
build_token_trace
build_market_ledger
verify_archive_integrity
explain_claim
```

`explain_claim` belongs at the CYNIC Intel boundary. Observer supplies evidence;
it does not decide who is an OG, whether a token is healthy or whether a
wallet belongs to a named person.

## Pump.fun opacity map

Pump.fun is difficult not because the chain is unknowable, but because the
truth is fragmented across incompatible surfaces:

| Surface | What it contributes | Main risk |
|---|---|---|
| finalized transaction | executed instructions, accounts, logs, balances | difficult decoding |
| program accounts | current state and authorities | state does not preserve all history |
| IDL/SDK | names, layouts, PDA seeds | version drift and incomplete coverage |
| provider parser | convenient semantic labels | provider-specific interpretation |
| public docs | intended mechanism | documentation lag |
| metadata URI/web | symbol, image, narrative | mutable/off-chain provenance |
| social posts | chronology and intent claims | deletions, edits, ambiguity |

The CYNIC response is not one “Pump parser”. It is a versioned evidence graph:

```text
program family + version
→ account/discriminator profile
→ raw instruction/event
→ decoded fact
→ balance/state transition
→ cross-source claim
```

At minimum, profiles must distinguish bonding curve, PumpSwap AMM, fee and
creator-fee-sharing surfaces, plus legacy/v2 instruction families. Quote mint,
virtual/effective reserves, migration state, creator vault and sharing config
must be explicit fields, not inferred from a symbol or provider label.

## Migration sequence

### M0 — Freeze the fixture

Treat the existing ASDFASDFA warehouse as a read-only regression fixture. Record
its manifest, archive hashes, repair report and derived outputs without
pretending that every page is an exhaustive historical corpus.

### M1 — Extract contracts

Define Rust types and tests for observations, coverage, raw integrity and
provider provenance. Preserve a compatibility reader for ODYSSEUM JSONL while
the new schema stabilizes.

### M2 — Extract adapters

Implement Helius and public-RPC adapters behind the same ports. Add a file
adapter so all backfill and reconciliation tests run offline.

### M3 — Extract program profiles

Load Pump/PumpSwap/fee IDLs as versioned assets. Decode one transaction into
raw instruction, decoded event, balance delta and unresolved fields. Unknown
decoding must be retained, not dropped.

### M4 — Rebuild projections

Regenerate token trace, market ledger and revenue ledger from CYNIC evidence
references. Every row must be traceable to one or more raw observations and
carry a derivation version.

### M5 — Human review surface

Expose “why this fact exists”, “which provider disagreed”, “what is missing” and
“which interpretation is contested”. This is the bridge to Askesis and the
future X/Telegram graph.

## First concrete CYNIC increment

Do not begin by rewriting the entire ODYSSEUM pipeline. Begin with a fixture
reader and a typed coverage report:

```text
ODYSSEUM JSONL
  → compatibility adapter
  → ObservationEnvelope
  → signature index + coverage ledger
  → integrity report
  → one regenerated token trace
```

The success criterion is not “the Rust version parses everything”. It is that
one transaction can be followed from source response to decoded fact to
derived projection, while every missing, repaired, ambiguous or provider-only
piece remains visible.

## Pump.fun as a feature, not a bug

Resolving the Pump.fun equation is the first calibration problem for CYNIC.
Its fragmentation is not merely technical debt to hide behind a cleaner API.
It forces the system to model the real conditions under which public truth is
assembled:

```text
partial sources + evolving programs + provider interpretations
        + off-chain context + human claims + adversarial ambiguity
        → explicit evidence graph
```

If CYNIC can make a Pump asset explainable without pretending that every field
is certain, the same architecture can be reused for launchpads, AMMs, creator
economies, social narratives and future financial instruments. Pump becomes a
training ground for:

- versioned protocol profiles;
- cross-provider reconciliation;
- raw-versus-derived separation;
- coverage and residual-unknown accounting;
- disagreement as a first-class signal;
- human contestation and correction;
- ownership claims bounded by evidence rather than naming conventions.

The target is not to erase Pump.fun’s complexity. It is to convert that
complexity into a legible, inspectable and reusable geometry of proof.
