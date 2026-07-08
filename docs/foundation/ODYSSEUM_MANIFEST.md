# ODYSSEUM MANIFEST

Status: normative foundation.

Odysseum is not a white paper yet. It is the working manifesto for a Solana
standard that turns fragile community tokens into governed, fee-capturing,
auditable assets without handing control to a human committee.

The first job is clarity: what belongs to the public protocol, what belongs to
the TALARIA SAS business surface, and what must remain an implementation detail
until it is proven.

## 1. Thesis

Solana community tokens have a recurring failure mode: they can attract
attention and liquidity faster than they can build durable governance,
treasury discipline, operational accountability, or credible communication.

Odysseum exists to make that failure mode expensive.

The standard defines an **Odysseum-Locked** state: a token or community asset
has verifiable fee routing, deterministic treasury rules, conditional market
governance, and public evidence strong enough that holders can distinguish a
governed asset from a raw social token.

Odysseum does not promise safety. It defines the machinery required to make
capture, drift, and discretionary extraction visible.

The product intuition is simple: the Solana market already has most of the raw
bricks required to turn meme coins into small digital enterprises. It has token
launch, bonding curves, AMMs, creator-fee surfaces, community attention,
wallets, mobile distribution, futarchy experiments, relays and public chain
data. Odysseum's role is to assemble those bricks into a standard that makes a
community behave more like a PME on Solana: treasury, governance, revenue
routing, auditability and operational communication.

## 2. Separation Of Surfaces

Odysseum has two public surfaces and they must not collapse into each other.

### 2.1 Protocol Surface

The protocol surface is blockchain-native and public. It owns:

- Odysseum-Locked verification.
- Vault routing rules.
- Proposal lifecycle.
- Conditional market state.
- Fee capture and burn accounting.
- Public technical documentation.
- Program IDs, audits, verifiable build proofs, and integration guides.

This surface speaks to builders, token communities, market participants and
auditors.

It must avoid implying that TALARIA SAS has custody over community treasuries
or discretion over governance outcomes.

### 2.2 TALARIA SAS Surface

The TALARIA SAS surface is commercial and operational. It owns:

- Enterprise intake.
- Contracts and invoices.
- Partnership conversations.
- Implementation support.
- Public communications calendar.
- Domain deployment and business operations.

This surface speaks to partners, clients, founders, institutions and service
providers.

It must avoid presenting commercial support as protocol control. TALARIA SAS
can sell integration, tooling and support. It does not become the vault.

## 3. Canonical Product Shape

Odysseum is a stack, not a single app.

| Layer | Function | Current repo anchor |
|---|---|---|
| Public interface | Explain, qualify and route projects | `packages/odysseum-ui` |
| Domain surface | Publish `odysseum.talaria.build` | `organs/domain/desired/domains.json` |
| Protocol contract | Define Odysseum-Locked state | `axioms/odysseum-manifest-v1.json` |
| Manifest | Human-readable doctrine and boundaries | `docs/foundation/ODYSSEUM_MANIFEST.md` |
| Ignis | Burn and buyback execution candidate | `services/ignis/` |
| GASdf | Relay/privacy boundary candidate | `organs/gasdf/` |
| Communications | X, Telegram, Discord, Slack rails | `organs/twitter/`, `organs/telegram/`, `organs/discord/`, `organs/slack/` |

The product should expose three first interfaces:

1. A public verification page for Odysseum-Locked assets.
2. A founder/operator intake flow for candidate communities.
3. A builder integration path with program addresses, SDKs and examples.

Odysseum's current internal sources split into three classes:

- `docs/foundation/ODYSSEUM_MANIFEST.md`: stable doctrine and boundaries.
- `docs/internal/ODYSSEUM_FUTARDIO_WHITEPAPER.md`: internal whitepaper draft and
  aggressive roadmap material, not yet public authority.
- `docs/ODYSSEUM_PITCH_WAKEUP_2026-06-30.md`: event-specific oral pitch and
  narrative material, not a technical contract.

The manifesto may absorb stable concepts from the other two files. It must not
inherit unverifiable market numbers, legal claims, tokenomics or dates without a
separate evidence path.

## 4. Solana Mechanism Investigation

The Solana implementation must start from observed mechanisms rather than a
preferred architecture.

Solana programs are executable, stateless accounts. Mutable state lives in
separate accounts passed to instructions. Odysseum state should therefore be
explicitly modeled as accounts: vault config, proposal state, fee route state,
market state, evidence state, and authority state.

Anchor is the default framework candidate for first implementation because it
accelerates secure Solana program development and gives a clearer path to
testing, IDLs, client generation and verifiable builds. Native Rust remains a
future option for tight control if Anchor becomes the limiting factor.

Pump and PumpSwap mechanisms must be mapped before Odysseum chooses a contract
shape. Current official Pump public docs expose these first-class mechanisms:

- bonding curve token creation and trading.
- completion when bonding curve token reserves are exhausted.
- permissionless migration from completed bonding curve to PumpSwap.
- canonical PumpSwap pools created by the Pump `migrate` instruction.
- protocol fee recipients.
- creator vaults and creator fee collection.
- dynamic fee tiers based on market cap.
- cashback / volume accumulator accounts.
- `create_v2`, Token-2022 ownership, and changing instruction surfaces.
- new unified buy/sell instruction variants for SOL and future quote assets.

Token Extensions remain a research input, not a design assumption. Existing
mints cannot be treated as if Odysseum can freely add transfer-time extensions
after deployment. A wrapper for existing tokens, a launch-time route for new
tokens, an AMM-side route, and a voluntary vault route may be different products.

The initial market focus is Pump.fun survivors, because that is where attention,
launch velocity and post-founder community chaos are concentrated. Odysseum
should ignore generic launchpad abstraction until Pump/PumpSwap mechanics are
understood well enough to defend an insertion point.

## 5. Candidate Contract Families

The first contract split should be conservative.

### 5.1 Vault Route

Purpose: define where captured fees go and what authority can update routing.

Likely state:

- token mint or wrapped asset reference.
- vault account.
- fee receiver account.
- route percentage or basis points.
- update authority policy.
- freeze/immutability state.

### 5.2 Proposal Market

Purpose: convert governance actions into conditional market decisions.

Likely state:

- proposal id.
- action payload hash.
- pass market.
- fail market.
- resolution conditions.
- execution gate.
- expiry.

### 5.3 Ignis Burn Executor

Purpose: execute buyback and burn policy from collected fees.

Likely state:

- spend budget.
- execution interval.
- slippage bounds.
- burn target.
- proof of burn.

### 5.4 Evidence And Verification

Purpose: make Odysseum-Locked status auditable.

Likely state:

- asset identity.
- program ids.
- vault route proof.
- governance proof.
- fee collection proof.
- public metadata URI.
- verification status.

### 5.5 Survivor Intake

Purpose: qualify communities that already survived founder absence, bonding
curve volatility, migration, or governance disorder.

Likely state:

- mint and token program.
- launch source.
- bonding curve / pool status.
- community ownership signals.
- fee surfaces observed.
- operator contacts.
- requested Odysseum route.
- opt-in proof.

This is the bridge between market reality and contracts. It prevents Odysseum
from selling an abstract treasury product to communities whose actual mechanism
does not support it.

## 6. Communication Doctrine

Odysseum cannot communicate through one channel. Each audience requires a
separate surface.

| Audience | Surface | Purpose |
|---|---|---|
| Market | X / public site | Narrative, proof, launches, educational threads |
| Operators | Telegram / Signal | Time-sensitive coordination and approvals |
| Builders | GitHub / docs / Discord | SDK, examples, contracts, integration questions |
| Enterprise | TALARIA SAS | Commercial intake, contracts, invoices, support |

Tweets, market observations and competitor claims should be treated as raw
signals. They can change positioning, examples and urgency, but they do not
rewrite the protocol contract without evidence.

## 7. Solana Market Entry Doctrine

Odysseum must be built from inside the Solana market, not as an abstract
fintech idea projected onto Solana.

The operating loop is:

1. Use the ecosystem.
2. Learn the fundamentals.
3. Show up where Solana builders already coordinate.
4. Build in public.
5. Ship before asking for attention.
6. Convert feedback into contracts, interfaces and proof.
7. Repeat.

This creates immediate product constraints:

- Odysseum builders must trade, use wallets, inspect launches, follow
  Pump/PumpSwap behavior, and understand where token communities actually
  coordinate before designing flows.
- The project should plug into Superteam, Solana events, Colosseum hackathons,
  Blueshift, Solana Turbine, and official Solana docs as learning and
  distribution rails.
- Public posts should expose concrete problems, implementation decisions,
  diagrams, test results and failure modes. Noise and vague ideology are lower
  value than shipping logs.
- The first users are not generic "web3 users"; they are Solana-native token
  operators, traders, builders, community maintainers and ecosystem reviewers.

WhiteMoon's correction is part of the doctrine: a team that does not use the
ecosystem first will likely build generic ticketing, payments, or dashboard
software instead of solving Solana-native market structure problems.

The "13 Legends" concept from the internal whitepaper is retained as a
go-to-market hypothesis, not a protocol rule: select a very small cohort of
surviving communities, offer a cost-basis integration path, observe how the
standard behaves under real pressure, and use that evidence before broad public
claims. The root community and selection method must be validated separately
before becoming public doctrine.

## 8. Near-Term Build Priorities

1. Stabilize the public Odysseum UI as a real control surface, not a landing
   page.
2. Write the founder/operator intake checklist.
3. Draft the Odysseum-Locked verification schema.
4. Build a Pump/PumpSwap mechanism table from live mints before choosing the
   v0 contract insertion point.
5. Prototype one local Anchor program for vault route state.
6. Map Ignis and GASdf into the Odysseum lifecycle without merging concerns.
7. Publish a communication map separating protocol, market and TALARIA SAS
   messages.
8. Create a build-in-public cadence for Superteam, Solana events and Colosseum
   alignment.
9. Turn every credible market objection into either a product constraint, a
   rejected non-goal, or a test.
10. Convert the "meme coin as PME on Solana" thesis into a falsifiable
    readiness checklist.

## 9. Non-Goals

Odysseum v0 must not pretend to solve everything.

- It is not a custodial treasury product.
- It is not a legal wrapper for every jurisdiction.
- It is not a promise of token performance.
- It is not a centralized multisig with better branding.
- It is not a white paper until the contracts, flows and market claims are
  precise enough to defend.
- It is not allowed to present internal pitch numbers or roadmap dates as
  public protocol truth before evidence and legal review.

## 10. Open Questions

- Should Odysseum v0 prioritize new mints using Token Extensions, or wrappers
  for existing Pump/PumpSwap-style assets?
- Which fee capture path is least fragile: native transfer fee extension,
  transfer hook, wrapper program, AMM-side routing, or a hybrid?
- What is the minimal evidence required for Odysseum-Locked status?
- Which governance actions can execute automatically, and which require a
  slower human/business boundary?
- How should TALARIA SAS price support without creating confusion around
  protocol control?
- Which Solana-native communities should be studied first before the UI and
  contracts harden?
- Which public build cadence creates useful feedback without leaking weak,
  unverified claims?
- Which Pump/PumpSwap mechanism can Odysseum compose with without pretending to
  control program-owned fee recipients or admin-controlled global config?
- Can Odysseum define value through verification, routing agreements and vault
  proofs even when the underlying token mechanics are immutable?
- Which exact criteria make a meme coin eligible to be treated as a PME on
  Solana rather than a short-lived attention market?
- Which parts of the 13 Legends strategy are protocol-neutral onboarding, and
  which parts are narrative or tokenomics that must stay internal for now?

## 11. Reference Anchors

- `axioms/odysseum-manifest-v1.json`
- `packages/odysseum-ui`
- `services/ignis`
- `organs/gasdf`
- `docs/internal/STOCKS_ON_SOLANA_GROUND_TRUTH.md`
- `docs/internal/ODYSSEUM_FUTARDIO_WHITEPAPER.md`
- `docs/ODYSSEUM_PITCH_WAKEUP_2026-06-30.md`
- Official Solana docs: Programs, Anchor, Token Extensions, Transfer Fees,
  Transfer Hook, Program Deployment.
- Official Pump docs: `pump-fun/pump-public-docs`.
