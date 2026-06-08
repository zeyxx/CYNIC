# Talaria Observatory — CYNIC Canonical Architecture

Talaria Observatory is owned by CYNIC. It is the sovereign backend layer for Talaria evidence, judgment, reputation, publication policy, and MetaDAO/futarchy context.

B&C is not merely a PoH producer. B&C is its own chess organism and ecosystem. For Talaria, B&C exposes selected chess/PoH signals into CYNIC; it does not become the Talaria registry.

## System boundaries

| System | Role | Boundary |
|---|---|---|
| CYNIC | Sovereign back engine, registry, judgment, normalization, reputation, public filtering | Owns Talaria event semantics and canonical observatory state |
| B&C | Autonomous chess organism: games, UX, chess community, chess identity, ratings, tournaments, PoH evidence | Owns chess product/ecosystem; emits selected events to CYNIC |
| Privy | App onboarding and account continuity | Does not replace wallet proof |
| Solana wallet | Public proof subject | Must be verified by signature before wallet judgment |
| Telegram Talaria | Community surface and enforcement point | Applies decisions, does not own truth |
| X/Talaria monitor | Public comms and interaction signal producer | Emits observations and comms events |
| GitHub/Anvil/deploy tools | Team proof-of-work producers | Emit sourced work events |
| MetaDAO | Futarchy market/proposal layer | Consumes evidence context, does not create factual truth |
| Human reviewers | Bootstrap authority and exception handling | Review decisions are events, not hidden state |

Invariant: evidence, judgment, publication, governance, and market interpretation are separate layers.

## B&C as organism, not submodule

B&C has two identities:

1. `B&C Chess Ecosystem`
   - product UX
   - chess games
   - ratings and progression
   - lobbies and QR joins
   - tournaments and community loops
   - chess-native identity and reputation
   - future chess economy

2. `B&C Talaria Integration`
   - emits game completion evidence
   - emits sealed evidence hashes
   - emits wallet binding state
   - emits PoH profile readiness
   - receives CYNIC/Talaria status for UX display

SoC rule: CYNIC must not absorb B&C's chess ecosystem. CYNIC only consumes the parts needed for Talaria proof, judgment, and observatory publication.

## Why CYNIC owns the Talaria observatory

CYNIC already has the backend primitives:

- `/observe` for heterogeneous event ingest.
- `/judge` for CYNIC verdicts.
- storage and crystals for persistence and feedback loops.
- Telegram/X organs for Talaria public signal capture.
- wallet judgment domain for Solana-oriented scoring.
- coordination and handoff conventions for operational proof-of-work.

B&C should own chess. CYNIC should own Talaria state.

## Scopes

```ts
type TalariaScope =
  | "talaria.pow.team"
  | "talaria.poh.user"
  | "talaria.chess.signal"
  | "talaria.reputation.public"
  | "talaria.governance.review"
  | "talaria.futarchy.market"
  | "talaria.alignment.capital"
  | "talaria.incident.trace"
  | "talaria.comms.signal";
```

### `talaria.pow.team`

Real execution evidence from T, S, and operating agents.

Examples:

- commit pushed
- PR opened, reviewed, reconciled, merged
- preview URL created
- build/e2e/smoke status
- deploy completed
- incident opened/remediated
- runbook/checkpoint created
- architecture decision recorded

### `talaria.poh.user`

User proof-of-humanity pipeline for Talaria membership and review.

Bootstrap policy:

- Privy login starts account continuity.
- 1 B&C chess game creates initial local proof.
- 3 games makes the user eligible for CYNIC/human review.
- 5 games is recommended for stronger confidence, not a first-launch hard wall.
- Solana wallet binding must be verified by signature before wallet judgment or permit/mint.
- Human review remains final authority for full bootstrap access.

### `talaria.chess.signal`

Chess-native signals from B&C that may inform Talaria reputation without collapsing into PoH.

Examples:

- participation in B&C tournaments
- consistent fair-play history
- game completion reliability
- social chess activity
- puzzle/challenge progression
- verified non-abusive socket/lobby behavior

This scope lets B&C remain a chess ecosystem while CYNIC consumes aggregate signals for Talaria.

### `talaria.reputation.public`

Public reputation state derived from PoH, PoW, chess signals, review, and governance history.

Initial output should be status bands, not false-precision raw scores.

```ts
type ReputationBand = "new" | "candidate" | "limited_trust" | "trusted" | "flagged";
```

### `talaria.governance.review`

Human and moderator decisions.

```ts
type ReviewStatus =
  | "candidate"
  | "ready_for_review"
  | "trusted_limited"
  | "verified"
  | "rejected"
  | "expired";
```

Bootstrap defaults:

- no full automatic acceptance during calibration
- automatic candidate/limited role is acceptable
- full role requires human review or later calibrated high-confidence policy
- inactive candidates expire after 72h

### `talaria.futarchy.market`

MetaDAO proposal and market context.

The observatory supplies factual and narrative inputs that proposals and markets can reference. It does not decide market outcomes.

Examples:

- proposal opened
- market opened/closed
- expected KPI declared
- decision executed
- post-decision evidence checkpoint
- proposal linked to PoW, incident, chess adoption, or alignment event

### `talaria.alignment.capital`

Economic and incentive-alignment evidence.

Future-facing scope. Keep conservative until legal and operational constraints are clear.

Trackable later:

- resources committed
- contributor compensation/grants
- treasury-impacting decisions
- value returned to community/token holders
- recurring operational costs
- proposal execution quality

### `talaria.incident.trace`

Incident and remediation ledger.

Incidents are trust-positive when captured honestly and remediated visibly.

### `talaria.comms.signal`

Public communications and X/Telegram interaction signals.

Existing CYNIC pieces:

- `cynic-python/sensors/talaria_tracker.py`
- `cynic-python/sensors/talaria_strategy_consumer.py`
- `cynic-python/agents/talaria_bot.py`
- `scripts/hermes-x/core/talaria_alerter.py`

These should feed observatory events instead of remaining only generic observations.

## Canonical event model

```ts
type TalariaVisibility = "public" | "internal" | "redacted";
type TalariaActorKind = "human" | "agent" | "system" | "market" | "wallet" | "app_user";
type TalariaConfidence = "observed" | "deduced" | "inferred" | "conjecture";

type TalariaEvent = {
  id: string;
  scope: TalariaScope;
  actor: string;
  actorKind: TalariaActorKind;
  kind: string;
  title: string;
  summary: string;
  source: string;
  sourceUrl?: string;
  evidenceHash?: string;
  repo?: string;
  commitSha?: string;
  proposalId?: string;
  walletAddress?: string;
  subjectId?: string;
  confidence: TalariaConfidence;
  confidenceBand?: "low" | "medium" | "high";
  visibility: TalariaVisibility;
  createdAt: string;
  value?: Record<string, unknown>;
  dependsOn?: string[];
};
```

## Mapping to current CYNIC `/observe`

| TalariaEvent field | Current `/observe` field |
|---|---|
| `scope` | `domain` |
| `kind` | `tool` or `tags[]` |
| `source/sourceUrl/evidenceHash/value` | `value` once persistence is complete; otherwise JSON in `context` |
| `confidence` | `confidence` |
| `dependsOn` | `depends_on` |
| `visibility` | `tags[]` until native field exists |
| `summary` | `context` |
| `actor` | `agent_id` or `target` depending source |

Implementation gap: `ObserveRequest` accepts ledger fields, but current Surreal `store_observation` does not persist all of them. Before relying on `value/confidence/depends_on` for Talaria, update storage persistence or store a complete JSON envelope in `context` temporarily.

## B&C integration contract

B&C may emit two families of CYNIC-facing events.

PoH events:

```ts
type BlitzChillPohEventKind =
  | "bnc.poh.game_completed"
  | "bnc.poh.evidence_sealed"
  | "bnc.poh.wallet_binding_pending"
  | "bnc.poh.wallet_binding_verified"
  | "bnc.poh.profile_ready"
  | "bnc.poh.cynic_judgment_requested";
```

Chess ecosystem events:

```ts
type BlitzChillChessEventKind =
  | "bnc.chess.game_completed"
  | "bnc.chess.tournament_joined"
  | "bnc.chess.tournament_completed"
  | "bnc.chess.rating_updated"
  | "bnc.chess.fair_play_signal"
  | "bnc.chess.community_signal";
```

B&C must not own:

- Talaria governance final status
- MetaDAO proposal status
- team proof-of-work events
- global public reputation final state
- global incident ledger unrelated to B&C

CYNIC converts selected B&C events into `talaria.poh.user` and `talaria.chess.signal` states.

## MetaDAO/futarchy relation

Futarchy requires a reality layer. Markets estimate consequences; they do not establish facts.

```txt
proposal thesis
    ↓
expected KPI / observable outcome
    ↓
market decision
    ↓
execution work events
    ↓
post-decision evidence
    ↓
reputation/alignment update
```

Design rule: every MetaDAO-facing proposal should be linkable to evidence events before and after execution.

Example event chain:

```txt
talaria.futarchy.market: proposal opened
talaria.pow.team: implementation PR opened
talaria.chess.signal: B&C tournament adoption checkpoint
talaria.pow.team: preview smoke passed
talaria.governance.review: human review approved
talaria.pow.team: deploy completed
talaria.alignment.capital: expected outcome checkpoint recorded
talaria.incident.trace: regression found and remediated
```

## Public safety filter

Never publish:

- Privy email
- OAuth identifiers
- IP address
- raw access tokens
- private Telegram identifiers unless explicitly intended
- anti-abuse internals that help attackers evade detection
- secrets, real infrastructure keys, private Tailscale addresses
- unredacted financial/legal claims not approved for publication

Prefer publishing:

- shortened wallet addresses
- status bands
- source links already public
- evidence hashes
- timestamps
- human-readable summaries
- proposal IDs and public market links
- aggregate chess ecosystem signals

## MVP implementation sequence in CYNIC

1. Canonical document and scope model.
2. Add typed Talaria event helpers in CYNIC or a Python shared module. Implemented in `cynic-python/sensors/talaria_events.py`.
3. Update `talaria_poh_bridge.py` to emit raw `talaria.poh.user` observations rather than direct human verdict assumptions from B&C `/api/verify`. Implemented in `cynic-python/sensors/talaria_poh_bridge.py`; it also maps chess ecosystem events to `talaria.chess.signal`.
4. Update storage so `/observe` persists `value`, `confidence`, `consumer`, `action`, `depends_on`, `maturity`.
5. Add public-safe projection function for observatory UI/API.
6. Add MetaDAO proposal event ingestion once proposal source is chosen.
7. Calibrate scoring/reputation after first 50-100 reviewed users/events.

## Non-goals for first integration

- no automatic full governance rights from PoH alone
- no public raw anti-abuse feed
- no financial promises in observatory copy
- no direct coupling between B&C and MetaDAO contracts
- no Privy-only final proof
- no absorption of the B&C chess ecosystem into CYNIC

## Current design truths

| Truth | Design impact |
|---|---|
| CYNIC is the sovereign backend for Talaria state | Canonical observatory docs/types/API belong here |
| B&C is an autonomous chess organism | Integrate via events, do not reduce it to PoH |
| B&C can emit both PoH and chess ecosystem signals | Add `talaria.chess.signal` beside `talaria.poh.user` |
| MetaDAO needs factual context but does not create facts | Observatory feeds proposals/markets |
| Bootstrap trust requires humans | Human review is an explicit event scope |
| Public evidence can manipulate markets | Redaction and visibility are first-class |
| Wallet proof is required for sovereign PoH | Privy alone remains account continuity |
| Current `/observe` schema is close but persistence is incomplete | Fix storage before relying on typed event fields |

## UI and repository topology

Current CYNIC repo already acts as the mother workspace for CYNIC/Talaria-facing interfaces:

```txt
CYNIC/
├── cynic-kernel/              # sovereign backend
├── cynic-python/              # organs, sensors, agents
├── packages/cynic-ui/         # CYNIC Observatory UI: timeline/topology/oracle
├── packages/shared-ui/        # shared CSS/design primitives, currently not a package
├── packages/talaria-landing/  # public Talaria landing
├── packages/talaria-demo/     # judgment/futarchy demo UI
└── docs/architecture/         # canonical architecture
```

`packages/cynic-ui` exists and is the current CYNIC Observatory front. It should not be treated as a dead reference. It is the natural host for internal/canonical observatory views, including Talaria scopes.

`packages/shared-ui` exists only as shared assets/styles at the moment; it has no `package.json`, so it is not yet a real npm workspace package despite matching the `packages/*` glob.

Decision for now:

- keep CYNIC/Talaria fronts inside CYNIC while backend/event semantics are evolving;
- restore and keep `dev:cortex` for `packages/cynic-ui`;
- treat `talaria-demo` as the external judgment/futarchy demo surface;
- treat `talaria-landing` as the public marketing surface;
- evolve Talaria Observatory first as a scope/view of `cynic-ui`, unless public deployment needs a distinct app;
- do not create an external mother repo until deployment, ownership, or contributor boundaries demand it.

Possible future topology:

```txt
talaria-apps/                 # future repo, only if needed
├── apps/landing/
├── apps/observatory/
├── apps/futarchy-demo/
└── packages/talaria-ui/

CYNIC/                        # remains backend + organs + canonical state
├── cynic-kernel/
├── cynic-python/
├── packages/cynic-ui/         # internal/canonical observatory may remain here
└── docs/architecture/

blitz-and-chill/              # remains autonomous chess organism
```

The split should be driven by operational need, not aesthetic repo cleanliness.

## Dissonances and emergences

| Item | Type | Current state | Decision |
|---|---|---|---|
| `cynic-ui` | corrected dissonance | package exists and root script should point to it | keep `dev:cortex` |
| `shared-ui` | emergence | folder exists with shared styles but no package manifest | either package it later or keep as asset folder |
| Talaria UI inside CYNIC | emergence | `talaria-demo` and `talaria-landing` exist under CYNIC | acceptable during bootstrap |
| Talaria Observatory UI | emergence | likely view/scope inside `cynic-ui` first | avoid premature new app |
| External mother repo | emergence | conceptually useful later | defer until boundaries harden |
| B&C role | corrected dissonance | was framed too narrowly as PoH producer | B&C is autonomous chess organism emitting selected signals |
| Talaria observatory ownership | resolved | risk of spreading state across B&C/fronts | CYNIC owns canonical registry/state |
| MetaDAO relation | resolved | markets could be mistaken for truth source | MetaDAO consumes observatory evidence |
| `/observe` ledger fields | resolved in progress | API accepted fields that storage dropped | Surreal storage now persists ledger fields |
```

## Naming trajectory

`packages/cynic-ui` is the current technical home of the Talaria Observatory UI. The name is historical and provisional.

Expected future rename:

```txt
packages/cynic-ui -> packages/talaria-observatory
```

Reason:

- the observatory presents the Talaria entity as a whole;
- it includes B&C as a chess organism inside the Talaria ecosystem;
- it includes CYNIC outputs, but CYNIC is the backend engine, not the public/entity-facing brand;
- it includes MetaDAO/futarchy, team PoW, user PoH, chess signals, governance, incidents, and alignment/capital scopes.

Do not rename immediately. Defer until deployment references, package scripts, Vercel project links, and imports are stable enough to update in one controlled pass.
