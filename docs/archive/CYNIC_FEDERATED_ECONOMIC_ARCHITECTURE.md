# CYNIC Federated Economic Architecture

Status: working draft.

This document reduces uncertainty around economic flows and the target
architecture: corporate first, federated next, decentralised only after proof.
It does not set percentages, token allocations or legal conclusions.

Related documents:

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md`
- `docs/foundation/CYNIC_PROTOCOL_REVENUE_WATERFALL.md`
- `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`
- `docs/foundation/CYNIC_TECHNICAL_WHITEPAPER.md`
- `docs/foundation/CYNIC_LEGAL_MICA_ROUTE_MEMO.md`

---

## 1. Core Model

```text
TALARIA SAS = corporate surface.
CYNIC Framework = technical engine.
CYNIC Protocol = rule layer.
$CYNIC = protocol fuel.
Branches = products and market interfaces.
Operators = federated service providers.
Users / clients = demand.
```

The architecture must avoid two traps:

1. corporate revenue pretending to be protocol yield;
2. protocol fees failing to support the root network that creates the value.

The solution is not one pool. The solution is a typed flow graph.

---

## 2. Economic Actors

| Actor | Role | Receives | Pays |
|-------|------|----------|------|
| User | consumes product or protocol service | service output | usage fee, subscription, gas, token fee |
| Client | signs B2B/B2G contract | service, SLA, integration | invoice to TALARIA or branch |
| Branch | product surface | product revenue, protocol routing rights | costs, operators, protocol fees, IP/governance fee |
| Operator | provides measurable service | operator payment, rewards, reputation | stake, infra cost, slashing risk |
| TALARIA SAS | corporate/legal/IP surface | corporate revenue, IP/governance fees | taxes, salaries, legal, accounting, R&D |
| CYNIC Protocol | rule and incentive layer | protocol fees, treasury inflows | rewards, grants, security, incentives |
| `$CYNIC` holder | protocol participant | access/reward only if rules and contribution allow | token risk, staking risk, usage cost |

No actor receives economic rights from vibes. Every flow needs a source, owner,
rule and proof.

---

## 3. Three Rails

### 3.1 Corporate Rail

```text
Client
  -> contract / invoice
  -> TALARIA SAS or branch company
  -> direct costs
  -> payroll / contractors / infra
  -> tax and accounting reserve
  -> legal / security / R&D
  -> retained earnings or legal distribution
```

This rail is governed by contracts, invoices, accounting and company law.

`$CYNIC` holders have no automatic claim on this rail.

### 3.2 Protocol Rail

```text
User / app / branch / operator
  -> protocol fee
  -> smart contract or protocol multisig
  -> direct protocol costs
  -> security / risk reserve
  -> operator payments
  -> measured contribution rewards
  -> TALARIA IP/governance fee if applicable
  -> protocol surplus
       -> grants / liquidity / buyback / burn / ecosystem reserve
```

This rail is governed by published protocol rules, smart contracts, multisigs
and the protocol revenue waterfall.

### 3.3 Operator Rail

```text
Operator
  -> provides compute / RPC / data / liquidity / security / uptime
  -> proves work through primary source + Kernel observation
  -> receives payment or reward
  -> gains or loses E-Score
  -> can be slashed or downgraded if rules allow
```

This rail is the bridge from federated infrastructure to future decentralised
participation.

---

## 4. Phase Model

### Phase 0: Corporate Bootstrap

TALARIA SAS and controlled branches operate most surfaces.

Allowed:

- B2B/B2G contracts;
- product revenue;
- internal operators;
- manual accounting;
- protocol design without token launch;
- `$CYNIC` discussed only as planned protocol fuel.

Not allowed:

- pretending corporate profits are tokenholder yield;
- hidden token allocations;
- hidden launch timing;
- automatic buyback promises;
- decentralisation claims without external operators.

### Phase 1: Federated Network

External operators can join under published rules.

Requirements:

- operator registry;
- service categories;
- proof requirements;
- payment formula;
- slashing conditions;
- dispute process;
- E-Score integration;
- public branch waterfall;
- multisig or contract-controlled treasury paths.

The network is federated, not decentralised. TALARIA SAS can still set standards,
protect IP, sign contracts and coordinate governance.

### Phase 2: Protocol-Mediated Federation

More routing moves from manual corporate decisions to contracts, registries and
public policies.

Requirements:

- on-chain registry for eligible operators or branches;
- signed service reports or verifiable metrics;
- public treasury dashboards;
- upgrade governance process;
- branch-level waterfall templates;
- independent security review;
- emergency pause / migration policy.

### Phase 3: Decentralisable Network

Decentralisation becomes possible only when the system can survive without
trusting TALARIA SAS for ordinary execution.

Readiness tests:

- at least three independent operator classes exist;
- no single operator controls critical service availability;
- treasury rules are executable without hidden discretion;
- slashing and disputes have due process;
- protocol governance cannot override corporate law;
- users can verify rules, wallets and flows;
- emergency powers are explicit and time-bounded;
- decentralisation does not break tax, accounting or legal duties.

Until then, "decentralised" is banned as a claim. Use "federated" or
"protocol-mediated" instead.

---

## 5. Branch Flow Template

Every branch must publish its own flow before significant usage.

```text
Branch revenue source:
  - user fee
  - subscription
  - API fee
  - protocol integration fee
  - B2B invoice
  - market fee

Branch flow:
  100% gross revenue
    -> direct product costs
    -> tax/accounting reserve
    -> operator/service costs
    -> security/risk reserve
    -> TALARIA IP/governance fee if using CYNIC IP
    -> protocol fee if using CYNIC Protocol
    -> branch surplus
         -> reinvestment
         -> grants
         -> protocol surplus routing if allowed
```

Each branch must state whether its revenue is corporate, protocol, or mixed.

---

## 6. Flow Examples

### 6.1 B2B Judgment API

```text
Enterprise client
  -> pays TALARIA SAS invoice
  -> TALARIA pays cloud/compute/RPC/accounting/tax
  -> TALARIA may pay internal or external operators
  -> TALARIA may route a documented protocol fee if CYNIC Protocol is used
  -> no automatic payment to passive $CYNIC holders
```

### 6.2 Protocol `/judge` Fee

```text
App or user
  -> pays protocol fee
  -> protocol treasury / contract
  -> direct inference and RPC costs
  -> validator/operator payments
  -> risk reserve
  -> contribution rewards if earned
  -> protocol surplus under published waterfall
```

### 6.3 GASdf

```text
User wants gasless transaction
  -> fee payer fronts SOL
  -> user or app pays accepted asset / fee
  -> branch covers SOL cost and slippage
  -> LP / fee payer / operator margin
  -> risk reserve
  -> protocol fee if CYNIC routing is used
  -> surplus only after survival costs
```

### 6.4 Perp Market On `$CYNIC`

```text
Trading venue / market
  -> market fees
  -> venue route by its rules
  -> any CYNIC-linked fee must be public and counsel-reviewed
  -> no claim that perp fees guarantee buyback, burn or holder reward
```

Perp markets are market structure, not protocol legitimacy.

---

## 7. Decision Matrix

| Question | If yes | If no |
|----------|--------|-------|
| Is there an invoice or contract? | corporate rail | check protocol/user rail |
| Is a smart contract collecting the fee? | protocol rail | check branch/corporate rail |
| Is a service provider doing measured work? | operator rail | no operator reward |
| Is CYNIC IP/brand used? | TALARIA IP/governance fee may apply | no IP fee by default |
| Is the payment for passive holding? | forbidden by default | continue |
| Is the rule published before money moves? | eligible for execution | block and document |
| Is tax/accounting unclear? | reserve first, counsel/accountant review | continue |
| Is decentralisation claimed? | require Phase 3 tests | use federated language |

---

## 8. What Remains Uncertain

These uncertainties are legitimate and must stay explicit:

1. exact percentage of branch revenue routed to TALARIA SAS;
2. exact percentage of protocol fees routed to operators;
3. exact percentage of protocol surplus eligible for buyback or burn;
4. whether `$CYNIC` launches with fixed supply, emissions or no emissions;
5. whether branch fees are paid in fiat, USDC, SOL, `$CYNIC` or mixed assets;
6. whether external operators stake `$CYNIC`, stablecoins or other collateral;
7. whether rewards are automatic, committee-reviewed or hybrid;
8. when a branch becomes independent enough to be protocol-mediated;
9. what legal review permits for public fee/buyback/burn language.

Unknown percentages are acceptable. Unknown flow owners are not.

---

## 9. Near-Term Decisions

Before the post-stage buffer ends:

1. choose first branch to model fully;
2. define corporate vs protocol rail for that branch;
3. draft wallet registry v0;
4. draft operator registry v0;
5. choose whether `$CYNIC` launch design includes staking at genesis or later;
6. decide which flows are public at genesis and which remain internal;
7. prepare counsel questions for buyback/burn/perp language;
8. remove or quarantine any public language implying automatic corporate profit
   routing to token holders.

---

## 10. Definition Of Done

The federated economic architecture is ready when:

- every flow has an owner;
- every branch knows its rail;
- every operator payment is tied to measured work;
- TALARIA revenue is not confused with tokenholder yield;
- protocol surplus is downstream of costs, tax, security and operators;
- "federated" and "decentralised" are not used interchangeably;
- decentralisation has objective readiness tests;
- counsel can review the model without guessing what money moves where.
