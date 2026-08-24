# CYNIC Technical Whitepaper

Status: working draft.

This document explains the technical architecture of CYNIC Framework and CYNIC
Protocol. It must not promise yield, token appreciation, guaranteed buyback,
legal ownership, or commercial outcomes.

Related documents:

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`
- `docs/foundation/CYNIC_E_SCORE.md`
- `docs/foundation/CYNIC_PROTOCOL_TOKEN_PAPER.md`
- `docs/foundation/CYNIC_FEDERATED_ECONOMIC_ARCHITECTURE.md`
- `docs/foundation/CYNIC_LEGAL_MICA_ROUTE_MEMO.md`
- `docs/CYNIC_ECOSYSTEM_ARCHITECTURE.md`

---

## 1. Abstract

CYNIC is a sovereign judgment framework for digital systems. It combines a
kernel, independent validators, proof-of-history observations, reputation
signals, operator roles and protocol incentives to make claims more auditable
and less dependent on social trust.

The system begins with token and wallet judgment, but the architecture is meant
to generalise to agents, infrastructure, governance, compute, market structure
and protocol coordination.

`$CYNIC`, if launched, is intended as a protocol asset for access, compute,
staking, rewards, governance signals and memetic coordination. It is not equity
in TALARIA SAS.

---

## 2. Problem

Digital markets suffer from low-trust coordination:

- token launches are noisy and often adversarial;
- wallets, protocols and communities lack reliable judgment layers;
- social proof is easy to fake;
- governance is vulnerable to plutocracy and hidden coordination;
- AI agents need verifiable boundaries, not only prompts;
- infrastructure providers need measurable contribution and accountability;
- users cannot easily distinguish signal from narrative.

CYNIC treats judgment as infrastructure.

---

## 3. System Overview

```text
External world
  -> sensors / adapters
  -> Kernel observation intake
  -> Dogs / validators
  -> verdicts and Q-Scores
  -> crystals / learned coherence patterns
  -> E-Score and operator signals
  -> protocol actions, rewards, warnings or governance signals
```

Core components:

| Component | Role |
|-----------|------|
| CYNIC Kernel | coordination bus, observations, tasks, verdict routing |
| Dogs | independent validators with bounded confidence |
| Crystals | coherence patterns learned from verdict history |
| E-Score | reputation and responsibility signal |
| Organs | persistent subsystems for sensing and acting |
| Cortex | episodic human/AI governance and DNA mutation |
| Protocol | published rules, smart contracts, treasuries and incentives |

---

## 4. Kernel

The Kernel is the nervous system of CYNIC. It coordinates:

- observations;
- agent tasks;
- verdicts;
- claims and coordination locks;
- health signals;
- crystals;
- audit trails;
- protocol-relevant state.

The Kernel should not be treated as the primary source of truth for everything.
It computes and routes signals from primary sources such as contracts,
databases, git history, legal documents, wallets and signed records.

---

## 5. Validators And Verdicts

Dogs are independent validators. They judge content, tokens, claims or events
against explicit criteria and return bounded-confidence verdicts.

Canonical verdict vocabulary:

| Verdict | Meaning |
|---------|---------|
| HOWL | strong positive signal |
| WAG | acceptable positive signal |
| GROWL | warning or uncertainty |
| BARK | rejection or high-risk signal |

Confidence must remain bounded. A validator that claims total certainty becomes
part of the problem CYNIC is built to resist.

---

## 6. Proof Of History And Observations

CYNIC records material events as observations. Observations can include:

- tool calls;
- agent actions;
- code changes;
- social signals;
- wallet facts;
- token metadata;
- market events;
- governance decisions;
- infrastructure state;
- human decisions.

The objective is not to store every fact forever. The objective is to preserve
enough provenance for later judgment, audit and accountability.

---

## 7. E-Score

The E-Score measures verified responsibility in the network. It is not a human
worth score, not legal ownership, and not pure token balance.

Inputs may include:

- useful contribution;
- uptime;
- compute served;
- liquidity risk;
- data quality;
- security response;
- governance signal quality;
- slashing history;
- operator reliability;
- token commitment where relevant.

The E-Score can influence priority, governance weight, rewards or operator
status only where rules are public.

---

## 8. Operator And Compute Model

CYNIC can coordinate operators that provide:

- inference;
- RPC;
- indexing;
- storage;
- monitoring;
- market data;
- liquidity;
- security review;
- incident response.

Operators should be rewarded for measurable work and accountable for measurable
failure. Staking and slashing rules must be public before funds are at risk.

---

## 9. `$CYNIC` Technical Role

`$CYNIC` may activate or support:

- API access;
- compute metering;
- operator staking;
- rewards for verified contribution;
- E-Score weighting;
- protocol governance signals;
- grants;
- liquidity or security incentives;
- public burn or buyback mechanisms after costs, taxes and reserves.

This section must remain subordinate to the legal route memo and token/equity
firewall. Technical possibility does not create legal permission.

Pending before launch:

- supply mechanics;
- mint and freeze authority policy;
- upgrade authority policy;
- treasury addresses;
- allocation table;
- vesting and lockups;
- launch path;
- market structure rules.

---

## 10. Security Model

CYNIC assumes adversarial conditions:

- fake social proof;
- manipulated liquidity;
- Sybil wallets;
- validator drift;
- prompt injection;
- model overconfidence;
- insider asymmetry;
- compromised operators;
- state corruption;
- legal and communication ambiguity.

Security priorities:

1. bounded confidence;
2. provenance;
3. multi-validator disagreement visibility;
4. separation of legal, protocol and runtime state;
5. public rules before incentives;
6. no hidden launch power.

---

## 11. Roadmap

The roadmap must describe technical direction without implying guaranteed
commercial outcomes.

Near term:

- consolidate legal and technical genesis papers;
- publish token/equity firewall;
- define wallet registry;
- formalise operator model;
- harden Kernel observation and task memory;
- document CYNIC Protocol boundaries.

Later:

- expand validator domains;
- decentralise operator participation;
- publish protocol contracts;
- formalise E-Score mechanics;
- integrate with wallets, DEXs, agents and infrastructure providers;
- explore privacy-preserving proof systems.

The target path is not immediate decentralisation. The economic and operator
model is defined as corporate bootstrap, federated network, protocol-mediated
federation, then decentralisable network only after objective readiness tests.
The phase model is maintained in
`docs/foundation/CYNIC_FEDERATED_ECONOMIC_ARCHITECTURE.md`.

---

## 12. Open Questions

1. Which CYNIC components are public at genesis?
2. Which components remain internal until security review?
3. What is the first production domain: token judgment, agent coordination,
   compute routing, or operator registry?
4. Which metrics are safe to expose publicly?
5. Which rewards can be automated, and which require human/multisig review?
6. Which `$CYNIC` mechanics are technical commitments, and which are only design
   candidates?
7. What must counsel review before any token launch or trading venue request?

---

## 13. Definition Of Done

The technical whitepaper is ready for public review when:

- it explains the system without relying on token price;
- it matches the legal route memo;
- it matches the token/equity firewall;
- it separates framework, protocol, company and token;
- it names open questions instead of hiding them;
- it avoids yield, equity, profit-share and guaranteed market language.
