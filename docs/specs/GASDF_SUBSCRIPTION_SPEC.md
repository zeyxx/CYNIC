# GASdf Subscription Spec — v0 (foundation)

> Status: foundation draft. Non-authoritative narrative; authority remains
> `contracts/foundation.json`. Zero I/O pure-contract first; service lands later.
> Companion: `infra/projects/gasdf/payment-tokens.json`, `infra/projects/gasdf/plans.json`.

## 1. Purpose

GASdf is the payment abstraction vertical of CYNIC: it lets a buyer pay a
subscription **in any accepted token** (SOL, USDC, well-known memecoins) while
the receiving application settles in `$ASDFASDFA`, and it sponsors on-chain gas
so the buyer never needs SOL for fees.

It exists to remove the psychological friction of an unfamiliar ticker at
checkout for cult products such as TrenchScanner and CultScreener, without
forcing those products to rewrite their subscription logic.

GASdf is **B2B/B2C agnostic and platform agnostic**: it ships as a thin SDK +
HTTP surface that any application can consume; it never owns application state.

## 2. Actors

| Actor | Role |
|-------|------|
| Buyer | End user with a wallet holding any accepted input token |
| Application (B2B client) | TrenchScanner, CultScreener, … owns its subscription tiers and benefits |
| GASdf (service) | Quotes, sponsors gas, converts, settles, tracks session state |
| Fee payer (on-chain) | GASdf-controlled signer that pays SOL fees |
| ASDF treasury (application side) | Receives $ASDF after conversion and credits the subscription |
| DEX (Jupiter/Raydium) | Route for input-token → ASDF conversion (off-chain MVP: server-side swap) |

## 3. Core concepts

- **Plan**: an application-declared subscription tier priced in `$ASDF` base units.
  GASdf never fixes prices; it consumes a plan registry the application publishes.
- **Session**: one checkout attempt by one buyer for one plan. State machine:
  `Open → Quoted → Pending → Settled | Expired | Rejected | Refunded`.
- **PaymentToken**: an entry in the dynamic token registry (mint, symbol, decimals,
  kind). Adding a token = registry edit, never a code change.
- **Quote**: a time-boxed conversion estimate from input token to ASDF, including
  slippage and fees.
- **Settlement**: the act of crediting the application with the ASDF value and
  marking the session `Settled`. MVP: off-chain credit via webhook + optional
  on-chain memo; full on-chain settlement is a follow-up.
- **Gas sponsorship**: GASdf co-signs / acts as fee payer so the buyer signs only
  the value transfer.

## 4. State machine (pure predicate)

```
Open --quote--> Quoted --pay--> Pending --confirm--> Settled
     \            \               \               \
      \            \               \               -> Refunded (on refund)
       \            -> Expired (ttl)   -> Rejected  -> Expired (ttl)
        -> Rejected
```

Pure predicates only decide transitions. No clock reads inside the core; timestamps
are inputs. (FCIS.)

## 5. Flow (checkout)

1. Application creates a `Session` via SDK/API with `{app_id, plan_id, buyer_pubkey}`.
2. GASdf looks up plan in registry; if missing or disabled → `Reject`.
3. Buyer picks input token from application-allowed set intersect registry.
4. GASdf builds a `Quote` (input amount, slippage bps, fees, expiry). Gas sponsorship
   flag attached.
5. Buyer signs the value transfer (or GASdf submits a gasless tx).
6. GASdf observes confirmation, converts, settles with application, marks `Settled`.
7. Application credits subscription; emits webhook `subscription.activated`.

Idempotency: session id is the idempotency key; duplicate submits collapse (pattern
from H4 build-latency science).

## 6. Registry contracts (dynamic, data-driven)

### 6.1 Payment tokens — `infra/projects/gasdf/payment-tokens.json`

Schema `cynic.gasdf.payment_tokens.v1`:

| Field | Type | Notes |
|-------|------|-------|
| `schema` | string | |
| `tokens` | array | |
| `tokens[].mint` | string | base58 mint pubkey (or `SOL` sentinel for native) |
| `tokens[].symbol` | string | display |
| `tokens[].decimals` | u8 | |
| `tokens[].kind` | enum | `native`, `stable`, `memecoin` |
| `tokens[].enabled` | bool | soft-disable without delete |
| `tokens[].min_amount` | integer (base units) | dust guard |
| `tokens[].slippage_bps_default` | u16 | per-token default slippage |

### 6.2 Plans — `infra/projects/gasdf/plans.json`

Schema `cynic.gasdf.plans.v1`: app-declared tiers priced in ASDF base units, with
duration and enabled flag. GASdf only validates membership; pricing authority is
the application.

## 7. Pure predicates (to live in cynic-protocol)

- `decide_token_allowed(token, registry) -> TokenDecision`
- `decide_quote(session, plan, token, now) -> QuoteDecision`
- `decide_session_transition(session, event, now) -> SessionTransition`
- `decide_settlement(session, confirmation, now) -> SettlementDecision`

Each takes all inputs; no I/O, no clock, no hidden state. Adapters (service, SDK)
supply inputs and persist outputs.

## 8. Invariants

1. No hardcoded token list in code. Registry is SSOT; code only consumes it.
2. Amounts are integers in base units; never f64 at settlement boundaries. (mirrors
   build-latency "integer amounts" gap.)
3. Gas sponsorship never transfers value it is not entitled to; fee payer key is
   classified `restricted`.
4. A session can be settled at most once (idempotent on session id).
5. Expired sessions cannot be resurrected.
6. Slippage is bounded by a registry max; quote rejected if exceeded.
7. Conversion shortfall (received ASDF < plan price) → partial/failed, never
   silent over-credit.

## 9. Cross-cutting (CYNIC)

- Crates: pure types/predicates in `cynic-protocol` (`gasdf` module). Adapters
  (HTTP, DB, DEX, signer) live in a future `cynic-gasdf` crate or `cynic-node`
  behind ports. FCIS.
- Observability: every sponsored fee is evidence (E-Score multiplies) per
  convergence map.
- Settlement currency ASDF on Solana; ASDF mint and program addresses come from
  `infra/registry.json`, never literals.

## 10. MVP scope (off-chain first)

Included:
- Types + pure predicates + registry validation (this step).
- HTTP service with quote/session/settle endpoints (follow-up).
- SDK (TS) wrapping the HTTP API (follow-up, parallel).
- Gas sponsorship via fee-payer signer (follow-up).

Excluded until funded/on-chain readiness:
- On-chain subscription program.
- Direct DEX CPI; MVP does server-side swap via HTTP API.
- Cross-chain bridges for non-Solana assets.

## 11. Pending decisions

1. Conversion shortfall policy (refund vs. partial credit) — needs operator.
2. Sponsored-gas budget cap and who funds the fee payer — needs liquidity/funding.
3. Webhook signing scheme between GASdf and application — needs contract w/ GCRtrd.
4. Whether plan registry is pushed by app or pulled by GASdf (pull preferred).
5. Commitment level (`confirmed` vs `finalized`) for settle — decide with budget.

## 12. Verification

- Pure predicates unit-tested in `cynic-protocol` (no I/O).
- Registry JSON validates against documented schema (manual now; schema validator TBD).
- No code reads token list from literals (grep gate in audit).