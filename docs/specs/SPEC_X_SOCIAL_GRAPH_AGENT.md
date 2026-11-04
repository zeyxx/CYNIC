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
7. **Browser is Camoufox, headed.** Chosen over Chrome for lower detectability. Persistent
   profile. Operator and agent share the same browser. Note: Camoufox is Firefox-based, so it
   is driven through Playwright, not CDP. Earlier "CDP" wording in this spec is obsolete.
8. **Proxy sits lower in the network**, not as a per-browser setting. Exact placement
   (gateway, router, dedicated node) is still to choose from the operator's network map.
9. **Entropy is multi-signal.** Several systems ingest it: keystroke timing, scroll, focus,
   and network flows. Network flows are the base layer that structures the whole system;
   the other signals attach to it.

## Open decisions (unresolved — do not assume)

- **Exact proxy placement.** Lower in the network is decided. Which node is not.
- **TLS interception caveat.** Moving the proxy lower does not remove the need for a CA
  trusted by the browser: the proxy still re-signs certificates. It only removes the
  per-browser proxy setting. Placement does not by itself make capture invisible.
- **Observed blocker.** With the browser-proxy mode, `x.com/i/api/2/timeline/home.json`
  returned `403` through mitmproxy, and TLS handshakes failed when the CA was not trusted.
  Hypothesis, not measured: X sees the proxy-to-X leg, whose TLS/HTTP2 fingerprint differs
  from a real browser. A lower placement would not change that.
- **Entropy signal contracts.** Which events each signal emits, and how they join to the
  network flow timeline (shared clock, correlation key), is not defined.

## Architecture

```
operator ──► Camoufox (headed, persistent profile) ──► network proxy ──► x.com
                         │                            │
                         │ Playwright                 ▼
                         │                     raw flow store (SQLite, append-only)
                         ▼                            │ parse (pure)
                  agent session (Hermes)              ▼
                         ▲                     social graph (SurrealDB, cynic-surreal)
                         └──────── context ◄──────────┘
```

### Layers (apps/x-graph)

- `domain/` — pure. Aggregates, entities, value objects, events, domain services. No I/O.
- `ports/` — repository and capture interfaces.
- `adapters/` — mitmproxy addon, SQLite raw store, SurrealDB repository, Playwright driver.
- `application/` — use cases: ingest flow, build graph, propose draft.
- `entropy/` — entropy signals: network flows (base), keystroke timing, scroll, focus.
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
| 1 | Browser with persistent profile | Chrome headed done; Camoufox headed not yet adopted |
| 2 | Proxy + capture addon, raw store | partial (403 / CA issue open) |
| 3 | Choose proxy node; isolate the 403 cause | open |
| 4 | Derive domain types from real raw flows | not started |
| 5 | Graph schema + ingestion | not started |
| 6 | Agent context integration | not started |
| 7 | Draft + validation loop | not started |
| 8 | Execution layer (browser-driven, lock) | not started |

## References

- twitter-mcp (evangelinehelsinki) — Camoufox, cache, single-flight
- stealth-x — DOM automation, browser-internal fetch
- x-mcp-playwright (Glama) — grouped tools, DMs, proxy
