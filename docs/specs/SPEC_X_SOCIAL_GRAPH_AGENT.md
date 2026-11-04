# SPEC — X Social Graph & Agent-Assisted Writing

Status: draft. Capture layer partially observed; everything downstream is unvalidated.

## Vision

Passively capture everything the operator does and sees on X, build a social graph from it,
and give an agent that context to assist writing (posts, replies, DMs).
The agent proposes. The operator validates. The agent never replaces the operator.

## Decisions (from the grill-me session)

1. **mitmproxy is the exhaustive ingestion layer.** It is a core component, not a one-off
   reconnaissance tool. It stays in production.
2. **Full entropy, not a predefined table.** Capture raw flows (headers, bodies, timing, order),
   not a pre-filtered subset. Filtering and parsing happen later, from the raw store.
3. **Capture everything now**, even data that does not help writing assistance today.
   It prepares future use. (Operator decision, Q4.)
4. **Leave no automation trace toward X.** Control goes through the real browser the operator
   uses. No direct API calls replaying captured payloads. Captured endpoints and payloads are
   input for the graph, not an execution path.
5. **Bottom-up.** Domain types are derived from real captured data, never invented first.
6. **DDD + FCIS + hexagonal**, same level as in `AGENTS.md`.

## Open decisions (unresolved — do not assume)

- **Proxy placement.** Browser-proxy mode (Chrome `--proxy-server`, a CA must be trusted) is
  observable by X in principle. Network-level transparent placement, lower in the operator's
  network, is the alternative. The choice is not recorded here yet. Detectability of each
  mode has not been measured.
- **Observed blocker.** With the browser-proxy mode, `x.com/i/api/2/timeline/home.json`
  returned `403` through mitmproxy, and TLS handshakes failed when the CA was not trusted.
  Cause not isolated (CA trust, HTTP/3/QUIC, or X-side detection).
- **Entropy scope.** Which signals count as "entropy" (network flows only, or also input timing,
  scroll, focus) is not defined.

## Architecture

```
operator ──► Chrome (headed, persistent profile) ──► proxy ──► x.com
                         │                            │
                         │ CDP (read-only first)      ▼
                         │                     raw flow store (SQLite, append-only)
                         ▼                            │ parse (pure)
                  agent session (Hermes)              ▼
                         ▲                     social graph (SurrealDB, cynic-surreal)
                         └──────── context ◄──────────┘
```

### Layers (apps/x-graph)

- `domain/` — pure. Aggregates, entities, value objects, events, domain services. No I/O.
- `ports/` — repository and capture interfaces.
- `adapters/` — mitmproxy addon, SQLite raw store, SurrealDB repository, CDP driver.
- `application/` — use cases: ingest flow, build graph, propose draft.
- `entropy/` — entropy signal definitions (scope open, see above).
- `mcp/` — thin MCP surface over application use cases.

### Bounded context and ubiquitous language

Terms to ground in real data before freezing: `User`, `Tweet`, `Interaction`, `DmThread`,
`Flow` (one captured request/response), `Actor` (operator or agent). Fields on `User` and
`Tweet` are provisional until checked against real payloads in the raw store.

## Data flow

1. **Capture** — proxy writes every relevant flow to the raw store, tagged with actor.
2. **Parse** — pure functions turn raw flows into domain objects. Unknown shapes are kept
   as raw, never dropped.
3. **Graph** — domain events update nodes (users, tweets, DM threads) and edges
   (follow, reply, like, retweet, mention).
4. **Context** — the agent reads a graph snapshot plus the current project.
5. **Propose** — the agent drafts. The operator validates (`go` / `stop`).
6. **Execute** — through the operator's browser, with human-like timing, under the lock.
7. **Log** — every agent action recorded with actor, timestamp, result.

## Principles

- Assistance, not substitution.
- Agent touches the browser only while holding the agent/human lock.
- DMs strictly serial, rate-limited, no retry on failure (duplicate risk).
- Capture is passive and must not change operator behavior.
- Secrets (account passcodes, tokens) never live in specs, code, or the raw store.
  They are requested from the operator at use time.

## Roadmap

| Phase | Deliverable | Status |
|-------|-------------|--------|
| 1 | Chrome headed + CDP + persistent profile | done |
| 2 | Proxy + capture addon, raw store | partial (403 / CA issue open) |
| 3 | Resolve proxy placement and measure detectability | open |
| 4 | Derive domain types from real raw flows | not started |
| 5 | Graph schema + ingestion | not started |
| 6 | Agent context integration | not started |
| 7 | Draft + validation loop | not started |
| 8 | Execution layer (browser-driven, lock) | not started |

## References

- twitter-mcp (evangelinehelsinki) — Camoufox, cache, single-flight
- stealth-x — DOM automation, browser-internal fetch
- x-mcp-playwright (Glama) — grouped tools, DMs, proxy
