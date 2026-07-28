# CYNIC Whitepaper

Version: 0.1 skeleton  
Status: architecture draft  
Runtime authority: none

This document describes the intended architecture and its demonstrated state.
It is subordinate to observed runtime state, `contracts/foundation.json`,
`infra/runtime-authority.json`, typed Rust contracts, and runtime proof.

It is not legal, financial, investment, privacy, or security advice. Token,
corporate, regulatory, and market claims require dedicated review before public
release.

## Claim Status Vocabulary

Every material claim in this paper must carry one of these statuses:

| Status | Meaning |
|---|---|
| `principle` | Durable design direction, not implementation evidence |
| `implemented` | Canonical code exists |
| `demonstrated` | Reproducible runtime proof exists |
| `partial` | Some canonical implementation or proof exists |
| `planned` | Accepted direction without implementation |
| `research` | Hypothesis requiring experiments and review |
| `blocked` | Known prerequisite prevents honest activation |
| `external` | Depends on an authority outside the CYNIC runtime |

`implemented` must not be silently promoted to `demonstrated`. A successful
command invocation without read-back is not a demonstration.

## Abstract

Status: `principle`

CYNIC is a private sovereign runtime membrane for one-person futarchy. It
connects user intent, operating systems, source matter, builds, artifacts,
agents, inference, hosts, fleets, evidence, and selected public settlement
without collapsing those planes into a single authority.

Talaria writes and proposes code for CYNIC. CYNIC bounds and proves the
production and execution of that code. CYNIC Askesis projects a durable digital
companion, while CYNIC Inference provides portable sovereign intelligence.
Explicit disclosure policies may transform selected private evidence into
public commitments on Solana and audience projections through Futard.io.

The common execution law is:

```text
desired state -> observe -> reconcile -> bounded action -> read-back proof
```

## 1. Scope and Non-Goals

Status: `principle`

CYNIC is intended to own:

- typed intent and execution contracts;
- observation and action ports;
- canonical reconciliation decisions;
- identity, capabilities, leases, fencing, and budgets;
- source-to-runtime provenance;
- durable proof and explicit unknowns;
- private-to-public disclosure policy;
- node-local sovereignty and bounded fleet coordination.

CYNIC is not intended to become:

- a universal database that replaces every primary source of truth;
- a clone of Kubernetes;
- a hidden shell automation collection;
- an omnipotent global agent;
- a model or inference-provider lock-in layer;
- an automatic publisher of user activity;
- a universal human value score;
- a blockchain whose transactions are assumed to prove off-chain facts;
- a company, token, market, application, or legal authority.

## 2. Canonical Actors

Status: `principle`

| Actor | Intended role | Must not become |
|---|---|---|
| Human operator | Sovereign root, consent and admission authority | Unobserved implementation detail |
| CYNIC | Private membrane, runtime, coordination, evidence | Universal owner of external truth |
| Talaria | Legal genesis operator and code producer | Unbounded runtime principal |
| Askesis | User-facing companion and context projection | Opaque sovereign agent |
| CYNIC Inference | Portable inference capability | Mandatory local or remote provider |
| Talaria Observatory | Evidence, judgment and publication projection | Hidden source of factual truth |
| Solana | Public commitment and settlement plane | Source of local runtime observation |
| Futard.io | Futarchic audience and interaction surface | Private runtime authority |
| MetaDAO/markets | Consequence estimation and decision context | Creator of factual outcomes |
| Odysseum | Economic application consuming primitives | Owner of CYNIC runtime law |
| B&C and peer organisms | Autonomous evidence producers | CYNIC submodules by default |

## 3. Authority Model

Status: `partial`

Truth flows upward from the lowest layer capable of observing it. Intent flows
downward through explicit authority.

| Concern | Primary authority | Public projection |
|---|---|---|
| Host state | Host observation and read-back | Redacted runtime proof |
| Source history | Declared sovereign repository | Commit or provenance evidence |
| Build result | Builder observation and artifact digest | Attestation or commitment |
| Runtime health | Target node observer | Health proof |
| Human consent | User-controlled policy and record | Consent receipt if disclosed |
| Corporate rights | Applicable law and corporate instruments | Approved corporate disclosure |
| Solana state | Solana consensus | Transaction and account proof |
| Market outcome | Declared market program and resolution rule | Resolved market state |
| Real-world outcome | Named observer and evidence policy | Attested result |

No higher projection may repair missing lower-layer evidence by assertion.

### Open work

- typed authority references across external sources;
- proof freshness and revocation;
- cryptographic workload identity;
- contestable judgment;
- multi-authority conflict resolution.

## 4. The Membrane Model

Status: `partial`

The membrane does not centralize all state. It mediates crossings between
planes:

```text
producer
  -> typed event or request
  -> identity and authority check
  -> policy and classification
  -> bounded transformation
  -> destination adapter
  -> destination read-back
  -> linked evidence
```

Every crossing should expose:

- producer and subject;
- source authority;
- desired destination;
- data classification;
- consent basis;
- transformation and redaction;
- mutation authority;
- rollback or compensation intent;
- read-back result;
- evidence references;
- residual unknowns.

## 5. Execution and Reconciliation

Status: `partial`

CYNIC adopts Kubernetes-like control-loop patterns, not Kubernetes as a hidden
dependency or universal model.

Reusable patterns:

- typed desired resources;
- read-only substrate observers;
- pure reconciliation decisions;
- idempotent node-scoped plans;
- explicit finalizers and cleanup;
- ownership and generation tracking;
- leases and fencing;
- conditions carrying unknown and blocked states;
- controller-specific responsibility.

Departures:

- local human sovereignty precedes cluster authority;
- mutation requires explicit bounded consent;
- proof is a first-class output rather than only status;
- one host is the first fleet case;
- nodes remain recursion roots;
- public settlement is optional and separately authorized.

## 6. Source, Build, Artifact, and Deployment

Status: `partial`

The first complete material lifecycle is:

```text
agent request
  -> identity and lease
  -> isolated sovereign workspace
  -> verified source preimage
  -> bounded change
  -> attributed commit
  -> push to declared source authority
  -> admitted build
  -> content-addressed artifact
  -> target-scoped deployment
  -> runtime read-back
  -> persistent proof bundle
```

Required evidence:

- source authority and revision;
- workspace owner and isolation boundary;
- lease and fencing token;
- action argv and mutation class;
- build environment and inputs;
- artifact digest;
- deployment target and principal;
- preflight and rollback intent;
- resulting observation;
- proof persistence reference.

### Current claim

The contracts and portions of source bootstrap, workspace, conformance, and
proof persistence exist, but the complete lifecycle is not yet demonstrated
across Cortex, Proxmox, and a VM. Status remains `partial`.

## 7. Multi-Agent Coordination

Status: `partial`

Agents are bounded workloads. Coordination does not imply a globally enforced
agent hierarchy.

Local delegation must preserve:

- node identity;
- parent identity;
- monotonic capability subsets;
- context, action, child, and wall-time budgets;
- cancellation;
- durable evidence;
- completion and unknown propagation.

Enforcement is required where a workload crosses a mutation, secret, identity,
storage, network, repository, build, deployment, or publication boundary.

Recursive execution remains `blocked` until durable scheduling, aggregate
budgets, sibling coordination, fencing, cancellation, and recovery are proved.

## 8. Askesis and Personal Context

Status: `planned`

Askesis is the companion projection of the private organism. Its purpose is to
help the user perceive, decide, learn, and act across connected planes.

It should derive context from attributable, content-addressed evidence rather
than treating raw conversation history as memory or authority.

Required properties:

- user-visible sources and uncertainty;
- reversible memory policy;
- private-by-default context;
- bounded delegated action;
- explicit distinction between suggestion, decision, and mutation;
- portable model and interface adapters;
- graceful operation with no remote provider.

Askesis is not demonstrated as a durable companion.

## 9. CYNIC Inference and Accessible ML

Status: `partial`

CYNIC Inference aims to expose common inference capabilities through portable
ports:

- text and structured generation;
- embeddings and retrieval;
- classification;
- ranking;
- extraction;
- speech and vision where justified;
- evaluation and model comparison;
- local and remote runtime supervision.

The operator-facing contract should favor tasks and capabilities over
provider-specific APIs. The runtime must retain:

- model and revision;
- execution provider;
- input data authority and consent;
- resource and monetary cost;
- output confidence or evaluation;
- retention policy;
- evidence sufficient to reproduce or contest the result.

Rust-native implementations are selected when they materially improve
sovereignty, safety, portability, or observation. Mature external engines remain
valid adapters.

## 10. Observatory and Event Semantics

Status: `planned`

The Talaria Observatory is the canonical projection for Talaria-facing
evidence, judgment, reputation, publication policy, incidents, and futarchy
context. It must not absorb the primary state of independent organisms.

An observatory event should include:

- event and schema version;
- scope;
- actor, subject, and actor kind;
- source authority;
- evidence references;
- confidence class;
- visibility and disclosure policy;
- dependencies;
- creation and observation timestamps;
- revision or supersession relation.

Evidence, judgment, publication, governance, and market interpretation remain
separate event families.

Legacy observatory descriptions reference removed Python organs and historical
API surfaces. They are source material, not evidence of current implementation.

## 11. Disclosure and Privacy

Status: `planned`

Private activity is not public by default. Publication is a separate bounded
transaction:

```text
private evidence
  -> classification
  -> policy evaluation
  -> consent
  -> minimization or redaction
  -> commitment/publication plan
  -> apply
  -> public read-back
  -> disclosure proof
```

Required policy dimensions:

- subject and data owner;
- allowed audience;
- purpose;
- granularity;
- expiration;
- revocation semantics;
- correlation and re-identification risk;
- permitted public adapters;
- human confirmation requirements.

Secret capture, covert interception, and publication merely because data is
technically observable are outside the sovereignty model.

## 12. Solana Commitment and Settlement

Status: `planned`

Solana is the strategic public plane for CYNIC's futarchic target. It may carry:

- evidence commitments;
- identity or authority attestations;
- proposal and market references;
- public outcome checkpoints;
- treasury and settlement actions;
- protocol registries;
- application-specific programs.

Solana adapters must not:

- block local operation when RPC access is unavailable;
- receive raw private evidence by default;
- use a general runtime identity as a wallet authority;
- treat transaction inclusion as proof of an off-chain assertion;
- hardcode one RPC, wallet, indexer, or SDK into domain policy.

Publication should be queueable, replayable, idempotent, and confirmed through
chain read-back.

## 13. Futard.io and One-Person Futarchy

Status: `external`

Futard.io is the intended audience projection for selected decisions, work,
predictions, and outcomes.

The target decision loop is:

```text
thesis
  -> resolution specification
  -> expected metric
  -> public commitment or market
  -> execution evidence
  -> observed result
  -> resolution
  -> decision-policy update
```

A valid resolution specification names:

- proposition;
- decision alternatives;
- metric and target;
- observation source;
- time window;
- invalidation conditions;
- resolver and dispute process;
- evidence publication policy.

Markets estimate consequences. They do not own implementation evidence or
real-world truth.

## 14. E-Score

Status: `research`

E-Score is a candidate framework for contextual, community-specific
interpretation of attributed evidence. Historical documents define seven
dimensions, achievements, facets, access tiers, and cultural registries.

Before activation, the model requires:

- explicit purpose limitation;
- versioned scoring policy;
- consent and data rights;
- explainability and evidence access;
- correction and appeal;
- uncertainty and decay;
- anti-Goodhart and anti-capture analysis;
- discrimination and safety review;
- separation between reputation, identity, humanity, wealth, and authority;
- empirical calibration against reviewed outcomes.

No behavioral pattern should be described as impossible for a bot to simulate.
No loss, sacrifice, holding duration, popularity, or inferred personality should
automatically produce governance authority.

E-Score is not currently a demonstrated runtime capability.

## 15. Fleet and Federation

Status: `partial`

A CYNIC node is a sovereign recursion root. A fleet coordinates nodes that share
an operational authority. Federation connects independently administered trust
or authority domains.

Fleet concerns:

- placement;
- rollout;
- node health;
- leases and fencing;
- artifact transport;
- aggregate budgets;
- rollback and recovery.

Federation adds:

- explicit trust establishment;
- foreign authority references;
- identity and proof verification;
- policy negotiation;
- revocation and key rotation;
- conflict and partial-connectivity semantics.

Scale changes data and adapters, not the core reconciliation law.

## 16. Talaria-Driven Evolution

Status: `research`

Talaria writes and proposes code for CYNIC. It is a code-producing authority,
not a direct mutation bypass.

The intended evolution transaction is:

```text
observed gap
  -> typed hypothesis
  -> bounded Talaria workload
  -> source mutation candidate
  -> build and conformance
  -> comparative evidence
  -> human or declared governance admission
  -> canary
  -> outcome observation
  -> promote, revise, or rollback
```

Required before recursive activation:

- immutable lower-layer safety contracts;
- isolated workspaces and reproducible builds;
- authority non-amplification;
- independent admission;
- rollback and recovery;
- decision/outcome history;
- protection against optimizing the proof rather than the real outcome.

## 17. Security Model

Status: `partial`

Primary threat classes:

- authority amplification;
- confused deputy behavior;
- stale observation and proof replay;
- lease loss during mutation;
- source or artifact substitution;
- secret leakage into evidence;
- malicious or compromised adapters;
- opaque model influence;
- coercive disclosure;
- Sybil and reputation capture;
- false real-world resolution;
- unavailable external infrastructure;
- self-modification that weakens admission.

The security model must distinguish prevention, detection, recovery, and
residual risk. Claims such as absolute safety, impossibility, unstoppable
operation, or guaranteed uptime are prohibited without a precise formal scope.

## 18. Legal, Corporate, and Protocol Separation

Status: `external`

The intended naming boundary is:

```text
Talaria SAS     legal genesis entity and software operator
CYNIC Framework private/runtime and developer framework
CYNIC Protocol  public protocol contracts and network rules
$CYNIC          potential protocol asset, not corporate equity
```

Corporate rights arise from applicable law and corporate instruments. Protocol
rights arise only from published program and governance rules. Token,
licensing, treasury, offering, market, offshore-entity, tax, privacy, and
regulatory claims remain external until reviewed and instantiated.

Historical dates and claims of legal immunity are not canonical commitments.

## 19. Current Capability Ledger

Status: projected

The capability ledger is not maintained in this narrative. Its canonical owner
is `cynic-protocol::FrameworkSpec`.

Machine-readable projection:

```bash
cynic dev framework --json
```

Publication-ready Markdown projection:

```bash
cynic dev framework --markdown
```

The Markdown renderer projects each ambition's identifier, classification,
runtime status, current owner, and required proof from the same typed catalog
used by runtime conformance. A publication process may embed that output, but
must not edit the generated rows or infer convergence from this whitepaper.

## 20. Proof-Driven Roadmap

Status: `principle`

Horizontal expansion follows completed verticals:

1. Close source modification to sovereign push.
2. Close source revision to reproducible artifact.
3. Close artifact to Cortex deployment and read-back.
4. Repeat unchanged semantics on Proxmox and one VM.
5. Prove node-local identity, leases, recovery, and evidence persistence.
6. Form the first multi-node fleet.
7. Add an external project through the same ports.
8. Add opt-in evidence disclosure.
9. Add offline-first Solana commitment and read-back.
10. Connect a Futard.io decision to execution and measured resolution.
11. Experiment with contestable E-Score and Talaria-driven evolution only after
    the lower evidence chain is reliable.

## 21. Open Research Questions

Status: `research`

- How is human sovereignty represented without making every action depend on an
  interactive human gesture?
- Which evidence can be disclosed through commitments without enabling harmful
  correlation?
- How are conflicting observations resolved without appointing one universal
  oracle?
- How does a personal futarchy specify outcomes that are measurable but not
  trivially gameable?
- How can Talaria optimize CYNIC without weakening the contracts that judge its
  changes?
- Which Kubernetes-like primitives remain useful outside cluster assumptions?
- Which ML capabilities justify native Rust implementations?
- How should contextual reputation decay, fork, and support appeals?
- How do independently sovereign nodes federate identity and proof without
  importing one global control plane?

## 22. Publication Gate

This whitepaper is ready for a public release only when:

- the authority and naming matrices are accepted;
- capability statuses are generated or checked against canonical contracts;
- evidence references accompany every `demonstrated` claim;
- privacy and disclosure policy has an executable contract;
- E-Score language has undergone safety and rights review;
- Solana program claims match deployed or explicitly planned programs;
- corporate, token, treasury, and regulatory sections receive specialist
  review;
- historical absolute claims and contradictory launch dates are removed;
- the document can be reproduced from a named source revision.
