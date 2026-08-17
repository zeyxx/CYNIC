# CYNIC Observer — X historical ingestion baseline

## Purpose

X is an evidence source for public discourse, provenance hypotheses, narrative
formation and social relations around Solana assets. It is not, by itself, a
proof of wallet ownership, legal identity, intent or truth.

The observer must preserve the distinction between:

- an observed post;
- an extracted claim;
- an inferred relation or entity;
- a proof assembled from several independent observations.

The human curator remains the authority for admitting a claim into the
interpretation layer. The raw observation layer remains append-only and
reproducible.

## What “exhaustive” means

For X, exhaustive means complete within a declared coverage scope:

```text
provider + entitlement + query family + time window + pagination state
        + response retention + retry/error ledger + retrieval timestamp
```

Every baseline must therefore publish its residual unknowns. A missing full
archive entitlement, deleted post, unavailable media, rate-limit interval or
ambiguous query is a coverage gap—not a successful empty result.

X documents the full-archive search endpoint as capable of searching back to
March 2006, with pagination and up to 500 results per page. Access and limits
depend on the account/product entitlement and must be observed at runtime:

- full archive: `/2/tweets/search/all`;
- recent search: `/2/tweets/search/recent`;
- counts: expected-volume estimation and partition planning;
- filtered stream: live continuation for admitted rules.

## Dual ingestion loop

### Historical backfill

1. Register targets: accounts, aliases, token mints, tickers, URLs, phrases,
   conversation IDs and known post IDs.
2. Generate query families rather than one giant query:
   account-centric, asset-centric, URL-centric, phrase-centric and relation
   queries (reply, quote, mention, repost).
3. Estimate density with counts before downloading posts.
4. Partition dense windows by time and, where necessary, by query family.
5. Fetch pages with `next_token`; persist the request envelope and response
   digest before normalization.
6. Reconcile overlapping partitions by immutable post ID.
7. Re-run boundary and high-density windows as a controlled audit.

### Live continuation

Use filtered stream rules for selected accounts/assets/phrases, then run a
periodic historical reconciliation. Stream capture is not a substitute for
backfill: it proves what this node received, not what X contained.

## Durable evidence records

The X adapter should remain behind a port in `cynic-observer`; provider details
must not leak into CYNIC policy. The minimum observation envelope contains:

```text
source/provider
request_id, query_id, query_hash
window_start, window_end, pagination_cursor
retrieved_at, response_digest, status, retry_count
post_id, author_id, username_at_capture, created_at
conversation_id, edit_history_ids, referenced_post_ids
text retention policy, media metadata/hash
coverage status, residual unknowns, consent/publication policy
```

Raw provider responses are retained according to an explicit retention policy;
the normalized event and its content digest remain the durable join surface.
Edits and deletions create new observations. They must not silently rewrite
earlier evidence.

## Layering

```text
X adapter
  → raw observation/WAL
  → normalized post + relation graph
  → entity/claim extraction (CYNIC Intel)
  → human curation and contestation (Askesis)
  → cross-source proof graph (X + Solana + Telegram + web)
  → ODYSSEUM projections and market/risk hypotheses
```

The first cross-source joins should be conservative:

- X post ↔ explicit mint address;
- X post ↔ explicit transaction/signature URL;
- X account ↔ account ID, never inferred legal person;
- X timestamp ↔ Solana event window, with clock uncertainty recorded;
- claim ↔ supporting and contradicting evidence references.

## ASDFASDFA first baseline

The first run should be deliberately bounded:

- known ASDFASDFA/PUMP accounts and aliases;
- the mint address and known transaction/account URLs;
- launch-to-present window, then selected pre-launch windows;
- three query families: exact identifiers, account timelines, narrative terms;
- counts before fetch, paginated fetch, overlap reconciliation;
- a machine-readable coverage ledger and a human curation sample.

The baseline is complete only when it reports both retrieved evidence and the
uncovered surface. It should answer “what did we observe?” before answering
“who is the OG?”, “which vamp came first?” or “which token is healthier?”.

## First implementation boundary

Proposed durable boundary:

```text
cynic-observer::x_archive::port
cynic-observer::x_archive::types
cynic-observer::x_archive::coverage
cynic-observer::x_archive::partition
cynic-observer::x_archive::adapter
```

The first code increment should implement types, coverage accounting and a
fixture-backed fake adapter. A live X adapter follows only after the fixture
proves pagination, retries, overlap deduplication, boundary partitions and
explicit blocked/unknown states.

No scraping workaround, rate-limit evasion, private-message ingestion or
identity deanonymization belongs in this baseline. If historical access is not
available, the system records `blocked_historical_access` and continues with
the narrower opt-in/stream scope.

## Network observation: top-down complement

The X API is a source-specific observer. A local network observer can provide
broader context: which applications were active, when a user interacted with
an application, which domains were contacted and which source event may have
been missed by the API. It cannot be treated as a universal truth source.

Use three capture tiers with separate consent and retention policies:

| Tier | Example | Default | Evidence value |
|---|---|---:|---|
| `metadata` | device, app/process, domain, timestamp, byte/connection summary | opt-in per device | activity context, not content |
| `application_event` | browser extension or explicit mobile share/bookmark/like event | opt-in per application/action | high-confidence human curation signal |
| `content` | HTTP response/request body after explicit TLS proxy consent | off | source content, subject to redaction and policy |

The safe default is deny until the operator declares a capture profile. “All
traffic on my network” must therefore compile into an explicit scope:

```text
principal → device → application/domain → event/content class → time window
         → retention → redaction → publication policy → revocation
```

### Computer and phone topology

```text
browser/app instrumentation ─┐
OS/network metadata ──────────┼→ local capture gateway → consent filter
mobile VPN/profile ───────────┘                         → encrypted WAL
                                                          → observer ingest
```

Capture should happen locally, with the raw WAL encrypted and bounded. The
gateway emits only admitted observations to CYNIC Observer. Source adapters
then normalize X, Solana, Telegram and web events into the same evidence
protocol.

### MITM/TLS interception boundary

`mitmproxy` may be useful in a controlled personal lab, but it must remain an
adapter behind a network-observation port. TLS interception requires explicit
installation and removal of a local CA, must exclude banking, password,
health, private-message and secret-bearing domains, and must respect
certificate pinning rather than attempting to defeat it. The CA private key
must never enter the repository, evidence store or model context.

Prefer, in order:

1. browser extension or explicit share/bookmark/like capture;
2. passive local metadata;
3. application-aware events;
4. content interception only for declared domains and sessions.

A like is not automatically a truth claim. It is a human-curation event with
confidence and meaning chosen by the operator. Store the event, its policy and
the surrounding observation window; let Askesis ask whether it means “relevant”,
“approved”, “to investigate” or something else.

### Proof and privacy invariants

- record capture policy, consent version, device and adapter provenance;
- hash content and minimize raw retention where a normalized observation is
  sufficient;
- distinguish operator traffic from guest, family, service and background
  traffic;
- preserve deletion/revocation requests as state transitions;
- never infer private identity or intent from DNS, timing or packet metadata;
- expose coverage gaps and capture failures as explicit unknowns;
- do not publish local-network observations without a separate publication
  decision.

This creates the intended dual loop:

```text
top-down: broad local observation → candidate events → filter/normalize
bottom-up: deliberate likes/shares/labels → trusted curation → model refinement
                         ↘ human remains final admission authority
```
