# CallShield — Anti-Spam/Scam Phone App Design

> **"Make something people want"** — people want to stop being called by telemarketers and scammers.

**Date:** 2026-05-19
**Status:** Design validated, pending implementation plan
**Approach:** C — Registry + Selective Escalation (Hybrid)

---

## Problem Statement

Phone spam and scam calls are a universal pain point. In France:
- Bloctel (government do-not-call list) has 47% of users reporting zero perceived effect (60M de Consommateurs survey) [observed]
- CLI spoofing means callers can fake their number, defeating static block lists
- STIR/SHAKEN (caller authentication) rejected by Ofcom (UK, Feb 2024), no EU mandate [observed]
- Truecaller (dominant player, 500M+ users) uploads users' contact books — active GDPR investigation by Swedish DPA (2025) [observed]
- No federated call reputation system exists in production anywhere [inferred — absence of evidence]

## Solution

A mobile app that:
1. **Identifies** incoming calls in real-time using a community-fed reputation registry
2. **Blocks** confirmed spam (Android) or labels it (iOS)
3. **Challenges** ambiguous calls via a selective voice proxy ("say your name")
4. Feeds a **federated registry** — no single entity controls the data
5. Uses **phones as infrastructure** — server is a super-peer, not the authority
6. **Anchors** registry integrity on Solana (periodic Merkle root, not storage)

## Core Principles

- **Privacy by design:** never collect the user's phone number, never access their contact book
- **Data-centric:** the call-event is the atom, everything else is derived
- **Human-in-the-loop:** users label, validate, and contest — at three distinct levels
- **Presumption of innocence:** when in doubt, let the call through
- **Sovereignty:** every infrastructure dependency has a planned exit path
- **Phone-first:** the server bootstraps the network; the phones ARE the network

---

## 1. Data Model

### 1.1 CallEvent (the atom)

Every interaction with the system starts as a CallEvent.

```
CallEvent {
  id:               uuid
  timestamp:        datetime (UTC)
  caller_number:    e164_string         // +33612345678
  callee_id:        anonymous_hash      // hash of user, never the number itself
  duration_sec:     u32?                // null if not answered
  outcome:          enum {
                      answered,
                      missed,
                      rejected_by_user,
                      rejected_by_app,
                      proxy_challenged
                    }
  user_label:       enum? {             // filled post-call
                      legitimate,
                      nuisance,
                      scam,
                      unknown
                    }
  label_confidence: f32                 // weighted by reporter reputation
  country_code:     iso_3166            // FR, BE, CH...
  metadata: {
    call_forwarded:    bool             // went through voice proxy?
    challenge_result:  enum? { passed, failed, timeout }
    spoof_likelihood:  f32?             // if STIR/SHAKEN available someday
  }
}
```

### 1.2 NumberReputation (derived view, recomputed)

```
NumberReputation {
  number:              e164_string
  country_code:        iso_3166
  first_seen:          datetime
  last_seen:           datetime
  total_events:        u64
  label_distribution:  {
    legitimate: u32,
    nuisance:   u32,
    scam:       u32,
    unknown:    u32
  }
  weighted_score:      f32              // 0.0 (safe) -> 1.0 (scam certain)
  confidence:          f32              // rises with independent reports, capped
  challenge_pass_rate: f32?             // if proxy active: % passing challenge
  sources:             vec<source_id>   // which federation nodes contributed
  last_contested:      datetime?
  contested_by:        u32              // number of contestations
  merkle_anchor:       solana_tx_sig?   // last on-chain anchor containing this number
}
```

### 1.3 ReporterReputation (anti-bot, anti-abuse)

```
ReporterReputation {
  reporter_id:       anonymous_hash
  joined:            datetime
  total_reports:     u64
  agreement_rate:    f32                // % of labels aligned with consensus
  false_positive_rate: f32
  trust_tier:        enum { new, established, trusted, moderator }
  anti_bot_signals: {
    device_attestation:    bool         // Play Integrity (Android), DeviceCheck (iOS)
    account_age_days:      u32
    report_velocity:       f32          // reports/hour — spike = suspect
    challenge_interactions: u32         // interacted with proxy? (confirmed human)
  }
}
```

### 1.4 Key Design Decisions

- `callee_id` is a hash — the user's phone number is never stored in the registry
- `weighted_score` is computed by weighting each `user_label` by the reporter's `trust_tier`
- `confidence` is bounded (never 1.0) — more independent reports = higher confidence, but never certainty
- `ReporterReputation` is local to device. Server holds a shadow copy keyed by `reporter_id` (anonymous hash). On reinstall, the device re-attests via Play Integrity/DeviceCheck — if the same physical device, the server restores the shadow copy. If new device, reporter starts as NEW. The server never knows who the reporter is — only that the hash matches a previously-attested device.
- `weighted_score` is suppressed from API responses when `confidence < 0.1` — the gateway returns `{"score": null, "status": "insufficient_data"}` instead of a misleading low-confidence score
- The user's contact book is **never uploaded, never accessed, never read** — the GDPR differentiator vs Truecaller

---

## 2. Architecture

### 2.1 System Overview — Kernel-Backed, Phone-Autonomous

CallShield is NOT a separate system. The CYNIC kernel is the backend. The phone is an autonomous node with its own embedded Dog. No separate Gateway, no separate Score Engine, no separate Registry DB.

```
┌─────────────────────────────────────────────────────────────┐
│  PHONE (autonomous CWO node)                                 │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Layer 1 — Embedded Dog (TFLite, < 2ms, offline)       │  │
│  │  Decision tree / small NN trained on:                   │  │
│  │    call frequency, report count, time-of-day,           │  │
│  │    number age, decay, challenge_pass_rate               │  │
│  │  Output: local_score + local_confidence                 │  │
│  │  Model size: < 1MB | Inference: < 2ms any phone 2019+  │  │
│  └──────────────────────────┬─────────────────────────────┘  │
│                             │                                │
│  ┌──────────────────────────┴─────────────────────────────┐  │
│  │  Local cache (SQLite)                                   │  │
│  │  Top-N spam numbers + recently seen + crystals          │  │
│  └──────────────────────────┬─────────────────────────────┘  │
│                             │                                │
│  Android: CallScreeningService    iOS: Live Caller ID 18.2+  │
│  Post-call labeling UX           Post-call labeling UX       │
│  Phone gossip (BLE, opt-in)      Phone gossip (limited)      │
└──────────────────────────────┬───────────────────────────────┘
                               │ HTTPS (when online)
                               v
┌─────────────────────────────────────────────────────────────┐
│  CYNIC KERNEL (existing, gains domain "phone_number")        │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Layer 2 — Kernel Dogs (< 100ms, online)               │  │
│  │  deterministic-dog: heuristic scoring (consensus,       │  │
│  │    federation data, cross-domain signals)               │  │
│  │  LLM Dogs: NOT used for phone_number (BURN)             │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  /judge   domain="phone_number"  ← reporter verdicts         │
│  /observe domain="phone_number"  ← CallEvents                │
│  /crystals domain="phone_number" ← distilled wisdom          │
│                                                              │
│  Pipeline: stimulus -> Dogs -> verdict -> crystal             │
│  DomainRouter: phone_number -> [deterministic-dog]            │
│  CrystalStore: "this number is CPF telemarketing"             │
│  ReporterReputation: human Dog quality tracking               │
│                                                              │
└──────────────────┬──────────────────────────────────────────┘
                   │
       ┌───────────┼────────────┐
       v           v            v
  Voice Proxy   Federation   Solana Anchor
  (SIP/Twilio)  (gossip)     (integrity)
  challenge     inter-nodes  periodic Merkle
  vocal         multi-domain root
       │
       v
    API B2B
  /api/v1/reputation/{number}
  /api/v1/stats
  /api/v1/verify-self
```

### 2.2 Three-Layer Dog Architecture

The phone scores autonomously. The kernel enriches. Neither depends on the other.

```
Layer 1 — Embedded Dog (phone, always available)
  Runtime: TFLite (Android) / Core ML (iOS)
  Model: decision tree or small NN, < 1MB
  Input: local cache + call metadata
  Output: local_score (0-1) + local_confidence (0-0.95)
  Latency: < 2ms on any phone since 2019
  Trains on: anonymized, aggregated features pushed from kernel
  Updates: model binary updated via app store or silent download (~monthly)

  Sufficient for:
    - Blocking obvious spam (score > 0.7 from cache)
    - Allowing known safe (score < 0.3 from cache)
    - Flagging ambiguous for Layer 2 enrichment

Layer 2 — Kernel Dog (server, when online)
  Runtime: CYNIC kernel deterministic-dog
  Input: Layer 1 score + community signals + federation data + crystals
  Output: enriched_score + enriched_confidence + crystals
  Latency: < 100ms
  Adds: consensus across all reporters, federation data from other nodes,
        temporal patterns (number lifecycle), cross-domain signals

  NOT used: LLM Dogs — phone number scoring is tabular, not semantic.
  BURN says: don't burn inference slots on what a decision tree handles.

Layer 3 — On-Device NLP (Phase 3+, flagship only, DEFERRED)
  Runtime: small transformer (~50-100MB) or Gemini Nano (~1.8GB)
  Input: call audio stream (on-device, never uploaded)
  Output: "this conversation sounds like a CPF scam"
  Latency: 50-200ms
  Phones: Pixel 9+, iPhone 15 Pro+, flagships with NPU
  
  Google already does this (Pixel scam detection, Nov 2024). [observed]
  DO NOT build at MVP. Evaluate after Layers 1+2 prove product-market fit.
```

### 2.3 Coupling Rules

The phone NEVER imports kernel code. It speaks HTTP. If the kernel is down, the phone operates autonomously.

```
Call arrives:
  1. Embedded Dog scores from local cache    → 0 coupling, < 2ms
  2. If cache miss → kernel /judge           → weak coupling, < 100ms
  3. If kernel down → presomption innocence  → allow, log for later sync

The phone is a judge. The kernel is an enrichment peer. Not a SPOF.
```

**API contract in degraded state:**

```
Normal:    { "score": 0.73, "confidence": 0.45, "status": "ok" }
Degraded:  { "score": null, "status": "degraded", "action": "allow" }
Unknown:   { "score": null, "status": "unknown", "action": "allow" }
Insufficient: { "score": null, "confidence": 0.05, "status": "insufficient_data", "action": "allow" }

Rule: app NEVER blocks when status != "ok". Embedded Dog handles the rest.
```

### 2.4 Phone-First Infrastructure

```
Resolution order for an incoming call:

  Tier 0 : Embedded Dog + local cache (SQLite)
            Always available, offline-first, < 2ms
            Contains: recently seen numbers + top-N spam + crystals

  Tier 1 : Phone-to-phone gossip (opt-in, Android primarily)
            Nearby phones exchange cache updates via BLE + WiFi Direct
            Android 12+: BLUETOOTH_ADVERTISE + BLUETOOTH_SCAN required
              -> prompts deferred to post-onboarding (opt-in, non-blocking)
            iOS: foreground-only, limited value. iOS relies on Tier 2.

  Tier 2 : CYNIC kernel (super-peer)
            Fallback when Tier 0 has no data
            Enriches with community consensus + federation + crystals
            < 100ms SLA

  Tier 3 : Federation backbone (kernel-to-kernel)
            Multi-node sync between institutional participants
            Operators, consumer associations, regulators
```

If the kernel goes down, phones continue with Tier 0 + Tier 1. The kernel's role is enrichment + federation + B2B API. The phone is the primary judge.

### 2.5 Cache Strategy (Phone Tier 0)

```
TTL by score category:
  Confirmed spam (score > 0.7):     TTL = 24h   (stable)
  Ambiguous (0.3-0.7):              TTL = 4h    (volatile, needs freshness)
  Known safe (score < 0.3):         TTL = 12h   (stable)
  Unknown (not in cache):           no entry    (Tier 2 on next call)

Server push (optional, premium):
  When a number crosses a threshold (e.g. 0.3 -> 0.8), kernel pushes
  invalidation to devices that cached it. App deletes stale entry.
  Protocol: FCM/APNs/ntfy.sh
  Payload: { "invalidate": ["+33612345678"] }

Cold start:
  On first install, app downloads top-N known spam numbers (N = 10K-50K).
  Embedded Dog is useful immediately, before any user reports.

Model updates:
  Embedded Dog model retrained monthly on anonymized aggregated features.
  Pushed as app update or silent asset download (< 1MB).
```

### 2.6 Critical Flows

**Flow 1 — Incoming call (< 2ms local, < 100ms enriched)**

```
Call arrives -> OS triggers CallScreeningService / LiveCallerID
  -> Embedded Dog queries local cache
  -> Cache hit + high confidence -> block/label/allow (Tier 0 only, < 2ms)
  -> Cache hit + low confidence -> enrich via kernel (Tier 2, < 100ms)
  -> Cache miss -> query kernel /judge domain="phone_number"
  -> Kernel returns enriched score OR degraded -> allow
  -> Log CallEvent locally
  -> Post-call: notification "Label this call?"
  -> If gossip active: share with nearby peers (Tier 1)
```

**Flow 2 — Report (user labels = human Dog verdict)**

```
User labels call -> App stores CallEvent locally
  -> App POST /observe to kernel (domain="phone_number", CallEvent + label)
  -> Kernel pipeline: stimulus -> deterministic-dog -> verdict
  -> If score crosses threshold -> crystal created
  -> ReporterReputation updated (agreement_rate)
  -> Federation propagation if confidence > 0.3
  -> If gossip active: push update to nearby peers
```

**Flow 3 — Voice proxy (premium tier)**

```
Unknown call + ambiguous score (0.3 < score < 0.7)
  -> Embedded Dog flags ambiguous -> redirect to Voice Proxy
  -> Proxy answers: "Your contact is screening calls. Please say your name."
  -> Timeout/silence -> reject
     + CallEvent(outcome: proxy_challenged, challenge: failed)
  -> Name spoken -> 3s recording -> push notification to user with audio
  -> Caller hears hold music (max 30s total round-trip budget)
  -> User accepts (within 30s) -> proxy forwards the live call
  -> User rejects OR timeout 30s -> proxy hangs up
     + CallEvent(label: nuisance/scam)
  -> Note: 30s is aggressive. Calibrate empirically — if > 50% of legit
     callers hang up, increase to 45s or add "please hold" message.
```

**Flow 4 — Contestation**

```
Number owner -> POST /contest (via app or web portal)
  -> OTP verification (SMS sent to the contested number)
  -> Contestation recorded -> score adjusted by -0.1
  -> If >= 3 verified contestations -> moderator review
  -> Moderator can temporary whitelist (90 days, renewable)
```

**Flow 5 — Solana anchor (automated, hourly)**

```
Every N hours:
  -> Kernel computes Merkle tree of all verdicts (all domains)
  -> Root hash (32 bytes) submitted as Solana transaction memo
  -> Cost: ~0.000025 SOL per anchor (~$33/year at $150/SOL)
  -> Anyone can verify: download verdict set + verify against on-chain root
```
```

**Flow 4 — Contestation**

```
Number owner -> POST /contest
  -> OTP verification (SMS sent to the contested number)
  -> Contestation recorded -> score adjusted by -0.1
  -> If >= N verified contestations -> score reset + moderator review
  -> Moderator can temporary whitelist (90 days, renewable)
```

**Flow 5 — Solana anchor (automated, hourly)**

```
Every N hours:
  -> Score Engine computes Merkle tree of all NumberReputations
  -> Root hash (32 bytes) submitted as Solana transaction memo
  -> Transaction signature stored in registry (merkle_anchor field)
  -> Anyone can verify: download registry snapshot + verify against on-chain root
  -> Cost: ~0.000025 SOL per anchor (~0.004$/day at hourly rate, see section 4.3)
```

### 2.7 Technical Stack

| Decision | Choice | Why |
|----------|--------|-----|
| Server backend | **CYNIC kernel (existing)** | Domain-agnostic judgment pipeline, already deployed |
| Server language | Rust (axum/tokio) | Already built, < 100ms on /judge |
| Primary DB | SurrealDB (existing kernel DB) | Already deployed, stores observations + verdicts |
| Hot cache | Redis (existing) | Already deployed in kernel |
| Local app cache | SQLite (Room/SwiftData) | Offline-first, embedded Dog input |
| Embedded Dog | TFLite (Android) / Core ML (iOS) | < 1MB model, < 2ms inference |
| Voice proxy | Twilio -> OVH -> FreeSWITCH | Sovereignty progression (see section 8) |
| Federation | libp2p gossipsub + ed25519-dalek | Kernel feature, serves all domains |
| Phone gossip | BLE + WiFi Direct (Android) | P2P cache enrichment, opt-in |
| User auth | Anonymous device tokens | No account, no email, no phone stored |
| Anti-bot | Play Integrity + DeviceCheck + velocity | 3 independent signals |
| Android app | Kotlin native | Direct CallScreeningService access |
| iOS app | Swift native | Direct CallKit / LiveCallerIDLookup access |
| On-chain anchor | Solana memo program | Shared CWO integrity layer |

---

## 3. Trust Model & Federation

### 3.1 Number Scoring

```
For a number N with K reports:

  raw_score(N) = sum_i (label_i * trust_i) / sum_i trust_i

  where:
    label_i   = { legitimate: 0.0, unknown: 0.5, nuisance: 0.75, scam: 1.0 }
    trust_i   = reporter_trust(reporter_i) * source_trust(node_i) * decay(age_i)

  confidence(N) = 1 - (1 / (1 + log(K_effective)))
    where K_effective = number of weighted independent reports
    clamped to [0, 0.95] — never certainty

  weighted_score(N) = raw_score(N)  (published only if confidence > 0.1)
```

**Temporal decay:**

```
  decay(age) = exp(-lambda * age_days)
  lambda calibrated for half-life ~90 days (to validate empirically)
```

A report from 6 months ago weighs less than yesterday's. Scammers rotate numbers; a flagged number may have been reassigned to a legitimate person.

**Decision thresholds:**

| Score | Confidence | Action |
|-------|-----------|--------|
| < 0.3 | any | Allow (presumed safe) |
| 0.3 - 0.7 | < 0.3 | Allow + label "unknown" |
| 0.3 - 0.7 | >= 0.3 | **Ambiguous -> voice proxy if premium, else warning** |
| > 0.7 | >= 0.3 | Block (Android) / Label "probable spam" (iOS) |
| > 0.7 | < 0.3 | Warning label (not enough data to block) |

**Presumption of innocence:** When in doubt, let through. Blocking a legitimate call (false positive) is worse than letting spam through (false negative).

### 3.2 Reporter Reputation

```
Trust tier progression:

  NEW (0-10 reports)
    trust_weight: 0.2
    Rate limit: 5 reports/day
    No single report can change a score alone

  ESTABLISHED (11-50 reports, agreement_rate > 0.6)
    trust_weight: 0.5
    Rate limit: 20 reports/day

  TRUSTED (51+ reports, agreement_rate > 0.75, account > 30 days)
    trust_weight: 1.0
    Rate limit: 50 reports/day
    Can vote on contestations

  MODERATOR (invited, agreement_rate > 0.85, account > 90 days)
    trust_weight: 1.5
    Reports accelerate federation propagation
    Can resolve contestations
```

**Agreement rate:** Measures how a reporter's labels converge with final consensus. A reporter who flags everything as "scam" while consensus says "legitimate" sees their agreement_rate drop and weight decrease.

**Bootstrap problem:** For numbers with < 5 total events, there is no stable consensus yet. In this case, `agreement_rate` is not updated for any reporter on that number. The number's score uses unweighted labels (all reporters treated as trust_weight 0.2 = NEW tier). Once the number reaches >= 5 events, consensus is computed retroactively and all reporters' `agreement_rate` is updated. This avoids division-by-zero and prevents early reporters from being penalized for labeling numbers that later get contradictory reports.

**Anti-gaming:**
- **Mass-flag bot** (1000 fake devices): each is NEW (trust 0.2), rate-limited to 5/day, Play Integrity/DeviceCheck filters emulators
- **Telemarketer mass-contesting:** OTP verified per number. Contestation adjusts score marginally, not reset. 500 independent "scam" vs 1 contestation = marginal decrease.
- **Sybil attack** (coordinated fake reporters): temporal correlation detection. 50 reports on same number in same minute from same carrier = suspect -> weight reduced in batch
- **Solana anchor** provides retroactive tamper detection: if scores are modified after anchoring, the Merkle proof fails

### 3.3 Federation Between Nodes

No blockchain for consensus. Signed gossip model, inspired by email DNSBL. Solana only for integrity anchoring.

```
Each node:
  Identity: Ed25519 key (public key in bootstrap registry)
  Publishes: aggregated + signed NumberReputations
  Receives: NumberReputations from peers + verifies signature

Node types:
  - Server nodes (high availability, always-on)
  - Phone nodes (intermittent, cache-based, gossip-capable)
  - Institutional nodes (operators, associations, regulators)

Federated message format:
  FederatedReputation {
    number:            e164_string
    score:             f32
    confidence:        f32
    event_count:       u64
    label_distribution: { ... }
    node_id:           ed25519_pubkey
    timestamp:         datetime
    signature:         ed25519_sig(all_fields)
    schema_version:    u8
  }
```

**Merge between nodes:**

```
For a number N seen by nodes A, B, C:

  merged_score(N) = sum_j (score_j * confidence_j * node_trust_j)
                    / sum_j (confidence_j * node_trust_j)

  merged_confidence = max(confidence_j)

  Conflict: if one node says 0.1 (safe) and another says 0.9 (scam)
    -> "conservative" = the LOWER score (toward safe/legitimate)
    -> Rationale: presumption of innocence — false positive (blocking legit)
       is worse than false negative (letting spam through)
    -> Exception: if the higher-trust node says scam AND its confidence > 0.7,
       use the higher-trust node's score (strong evidence overrides caution)
    -> ALL conflicts flagged for moderator review regardless of resolution
```

**Node trust:**

| Node type | Initial trust | Evolution |
|-----------|--------------|-----------|
| Bootstrap server (us) | 1.0 | Decreases if false positive rate rises |
| Telecom operator | 0.8 | Network metadata = strong signal |
| Consumer association | 0.7 | Verified manual reports |
| Trusted phone cluster | 0.4 | Phones with TRUSTED reporters, aggregated |
| Third-party app | 0.3 | Must prove reliability over 90 days |
| New node | 0.1 | Rises by agreement with the network |

**Propagation:**
- Local report stays local until confidence > 0.3
- Beyond threshold -> pushed to peer nodes (gossip)
- Each node can reject inconsistent messages
- Node injecting noise (false positives > 20%) downgraded by peers
- Hourly Merkle root anchored on Solana for tamper detection (see section 4.3)

### 3.4 Privacy by Design

| Data | Stored where | Who sees |
|------|-------------|----------|
| User's phone number | **Nowhere** — never collected | Nobody |
| Contact book | **Device only** — never uploaded | The user |
| CallEvents with callee_id | Server, hashed | Nobody (non-reversible hash) |
| Caller number | Federated registry | Public (that's the point) |
| User labels | Aggregated anonymously | Nobody individually |
| Voice recording (proxy) | Callee's device, 24h max | The callee only |
| Device attestation | Verified in transit, not stored | Nobody |

**GDPR compliance:**
- No user account -> no personal data per Article 4
- Caller number treated as "legitimate interest" (Article 6.1.f) — fraud protection
- Right to object = contestation (Flow 4)
- Right to erasure = temporal decay naturally erases data in ~6 months

---

## 4. CWO Integration — CallShield as a Domain

### 4.1 The Core Insight

CWO's mission is "making the cost of lying visible." CYNIC does this for tokens. CallShield does this for phone numbers. They are **domains of the same judgment pattern**:

```
stimulus  ->  Dogs  ->  verdict  ->  crystal
```

| | CYNIC (tokens) | CallShield (phone) |
|--|--|--|
| **Stimulus** | Token metadata + on-chain data | Caller number + call metadata |
| **Dogs** | LLM + heuristics (server) | Humans (reporters) + heuristics |
| **Verdict** | HOWL/WAG/GROWL/BARK | legitimate/nuisance/scam |
| **Crystal** | "this token is a rug pull because..." | "this number is CPF telemarketing" |
| **Node** | Server (cynic-core, cynic-gpu) | **Phone** (phone-first) |

CallShield is not "an app that optionally uses CYNIC." It is the **first deployment of CWO judgment on phones** — a new domain, new Dog type (human reporters), new node type (phones).

### 4.2 What is Shared vs Domain-Specific

**Shared (the proto-CWO substrate, emergent):**
- Verdict format: q_score (0-1), confidence (capped 0.95), label
- Federation transport: Ed25519 signed gossip between nodes
- Node identity: Ed25519 keypair, same format regardless of device/OS
- Crystal format: domain-tagged distilled wisdom, transportable
- Integrity anchoring: periodic Merkle root on Solana

**Domain-specific (CallShield only):**
- CallScreeningService / CallKit integration
- Voice proxy (SIP challenge flow)
- Post-call labeling UX
- Phone-as-node gossip (BLE/WiFi Direct)
- Reporter reputation (human-specific trust tiers)

### 4.3 The Protocol Emerges from Domains

The CWO Judgment Protocol (CJP) is NOT designed upfront. It **emerges** from building real domains:

```
Phase 1: Build CallShield with its own scoring/federation
         Build CYNIC with its own scoring/federation
         -> Two working systems, similar patterns, different code

Phase 2: Extract the common substrate
         -> Verdict format, federation transport, node identity, crystal format
         -> Shared Rust crate: cwo-core (lightweight, embeddable)
         -> Both CYNIC and CallShield import cwo-core

Phase 3: Third domain validates the protocol
         -> If cwo-core works for domain #3 without modification: protocol is real
         -> If cwo-core needs domain-specific hacks: protocol is premature, iterate
```

**Anti-pattern:** Designing CJP before two domains validate it. A protocol without two independent implementations is a wish, not a standard. *(confidence 0.55 — inferred from protocol design history: HTTP, ActivityPub, SMTP all emerged from implementations)*

### 4.4 Architectural Compatibility

CallShield's architecture is designed so the CWO substrate can be extracted later without rewriting:

```
ScoreEngine trait
  |-- SimpleWeightedScorer    (MVP — CallShield-specific)
  |-- CwoVerdictScorer        (Phase 2+ — delegates to shared cwo-core)

FederationTransport trait
  |-- LibP2PGossip            (MVP — CallShield-specific gossip)
  |-- CwoFederationTransport  (Phase 2+ — shared protocol)

CrystalStore trait
  |-- LocalCrystalStore       (MVP — phone SQLite)
  |-- CwoCrystalStore         (Phase 2+ — shared format, cross-domain)
```

The trait boundaries are the extraction points. When two domains exist, the shared implementation moves behind these traits into cwo-core. The CallShield and CYNIC codebases don't change — only the adapter behind the trait.

### 4.5 Phone as CWO Node

The phone is not a client. It's a **first-class CWO node** with the same citizenship as a server:

```
CWO Node capabilities (regardless of hardware):
  - Hold an Ed25519 identity
  - Receive stimuli for its domain(s)
  - Apply local Dogs (heuristic or human)
  - Emit signed verdicts
  - Store and serve crystals
  - Participate in federation gossip

What differs by hardware:
  - Server: always-on, can run LLM Dogs, high bandwidth gossip
  - Phone: intermittent, human Dogs only, BLE/WiFi gossip
  - RPi/embedded: always-on but resource-constrained, heuristic Dogs only
```

This means a phone running CallShield and a server running CYNIC are **peers in the same network**. They speak the same federation protocol. They can relay each other's signed verdicts (without understanding the domain content). A phone node that sees a token verdict passes it along — it validates the signature, not the judgment.

### 4.6 Solana Integrity Layer

Solana is the shared integrity anchor for all CWO domains. Not domain-specific — any domain's verdicts can be anchored.

```
Mechanism:
  Every N hours (configurable, default: 1 hour):
  1. Node computes Merkle tree of all verdicts since last anchor
  2. Root hash (32 bytes) submitted as Solana transaction memo
  3. Anyone can verify: download verdict set + verify against on-chain root

Cost per anchor: ~0.000025 SOL (~$33/year at hourly rate, $150/SOL)

What Solana does:
  - Proves verdicts haven't been tampered with after the fact
  - Shared across domains (token verdicts + phone verdicts in same tree)
  - Public auditability for regulators/partners

What Solana does NOT do:
  - Store verdicts (too voluminous)
  - Run scoring (too slow, ~400ms vs < 100ms requirement)
  - Replace federation (complementary: gossip = real-time, Solana = periodic proof)
  - Manage identity (Ed25519 keys are off-chain)
```

**Reporter incentives (DEFERRED, Phase 3+ at earliest):**
SPL token for reliable reporters is conceptually aligned but must NOT be built before product-market fit. Token incentives before organic community = gaming. Evaluate after 12 months of organic growth and only if community health metrics are strong. *(conjecture — confidence 0.3)*

### 4.7 What This Means for the Spec

CallShield MVP is built **standalone** — no CYNIC dependency, no cwo-core import. But every architectural boundary (ScoreEngine, FederationTransport, CrystalStore, NodeIdentity) is a **future extraction point** for the CWO protocol. The protocol doesn't exist yet. It emerges when CallShield + CYNIC have enough shared structure to factor out.

**Falsification:** If after building CallShield, the shared patterns with CYNIC are < 30% of the codebase, the CWO protocol is premature. Keep them as independent products with philosophical kinship only.

---

## 6. UX & User Flows

### 6.1 Onboarding (< 2 minutes, zero account)

```
Screen 1: "Marre du demarchage?"
  3-point explanation (identify, block, community)
  CTA: "Activer la protection"

Screen 2: OS Permissions
  Android: "Set as call screening app" -> ROLE_CALL_SCREENING
  iOS:     "Enable call identification" -> Settings > Phone > Call Blocking

Screen 3: Protection level
  Green  (Gentle):  identify only (labels, no blocking)
  Yellow (Normal):  block confirmed spam (score > 0.7 + confidence > 0.3)
  Red    (Strict):  block spam + nuisance (score > 0.5)
  Changeable anytime

Screen 4 (optional): "Enable voice proxy?"
  Free 7-day trial, then premium
  Skip possible

Screen 5 (optional): "Help the network?"
  Enable phone-to-phone sharing (gossip)
  Explanation: "Your phone helps nearby users identify spam"
  Privacy note: "Only spam scores are shared, never your data"

No account. No email. No phone number requested.
Device attestation runs in background (invisible).
```

### 6.2 Incoming Call Experience

**Case 1 — Known safe (in contacts or score < 0.3):** Normal behavior. App invisible.

**Case 2 — Confirmed spam (score > 0.7, confidence > 0.3, mode >= Normal):**
- Android: silently rejected. Notification: "Blocked: +33 6XX — Spam (87%)". Action: [This is an error]
- iOS: label on call screen: "Probable spam". User decides.

**Case 3 — Ambiguous (0.3 < score < 0.7):**
- Without proxy (free): phone rings. Label: "Reported 12 times - 60% nuisance". Post-call labeling.
- With proxy (premium): call intercepted. Proxy challenges. Push with audio. Accept/reject.

**Case 4 — Completely unknown:** Phone rings normally. Post-call: "Unknown number. What was it?" [Legitimate] [Nuisance] [Scam] [Ignore]

### 6.3 Post-Call Labeling

```
Notification (5s after hangup):

  +33 1 23 45 67 89
  Duration: 0:42
  What was it?
  [OK]  [Telemarketing]  [Scam]
  [Ignore]

Single tap -> done.
Optional expand: [Insurance] [Energy] [CPF] [Parcel] [Robot] [Other]
```

Minimal gamification: "Vigie" badge at 10 verified reports. Counter: "You helped block 1,247 spam calls this month." No leaderboard, no streaks.

### 6.4 Main Screen

```
  Protection active | Mode: Normal

  This week: 12 blocked - 3 reported

  Recent filtered calls:
    +33 9 XX  Spam (94%)   yesterday
    +33 1 XX  Nuisance     Monday
    +33 7 XX  Blocked      Monday

  Community: 1.2M reports this month
  Top scam: fake CPF advisor
  Your reputation: Trusted
  Network: 847 peers nearby (gossip active)

  [Settings]  [Contest a number]
```

### 6.5 Contestation

1. "Is my number flagged?" -> enter number
2. OTP verification (SMS to contested number)
3. Score display + breakdown
4. "Contest" -> Who? [Individual / Business / Association] + Activity
5. Score adjusted (-0.1 per verified contestation). If >= 3 verified contestations -> moderator review -> possible whitelist (90 days). N=3 chosen because: 1 contestation could be the spammer themselves, 2 could be coordinated, 3 independent OTP-verified contestations is a meaningful signal. Calibrate empirically after launch.

---

## 7. Business Model & Go-to-Market

### 7.1 Revenue Sources

**Free tier — acquisition engine**
- Cost/user: ~EUR 0.01/month
- Value: every free user is a sensor feeding the registry

**Premium — "Bouclier" (~EUR 3.99/month or EUR 29.99/year)**
- Cost/user: EUR 0.50-2/month (SIP proxy on ~10% of calls)
- Margin: 60-85%
- Includes: voice proxy, strict mode, 90-day history, detailed stats

**B2B API**
- Operators: EUR 0.001-0.005/lookup, millions/day
- Businesses: EUR 99-499/month (reputation dashboard)
- Regulators: annual contract or free institutional access

### 7.2 Self-Sufficiency Thresholds

```
Infrastructure only:
  EUR 18K/year -> 375 premium subscribers (0.375% of 100K users)

Infra + 1 dev:
  EUR 67K/year -> 1400 premium (1.4%) OR 1 B2B contract + 350 premium

Infra + 2 devs + ops:
  EUR 150K/year -> 3200 premium + 2-3 B2B contracts
  Achievable at 200-300K users
```

### 7.3 Cost Structure

| Item | Year 1 | Note |
|------|--------|------|
| Server infra (marginal on kernel) | EUR 2-6K | Kernel already deployed. Phone domain adds DB storage + bandwidth. |
| SIP trunking | EUR 2.4-12K | Proxy-only for premium subscribers (~10% of calls per premium user). Lower bound: 500 premium x 2 proxy calls/month x EUR 0.20/call. Upper bound: 5000 premium x 2 calls/month x EUR 0.10/call (OVH rates). |
| App stores | 15-20% of premium | Unavoidable |
| Development | EUR 150-250K | Main cost |
| Legal/GDPR | EUR 10-20K | DPO, DPIA |
| Solana anchoring | EUR ~33 | Negligible (shared CWO infra) |
| **Total** | **~EUR 200-300K** | |

### 7.4 Funding Strategy

```
NO VC. Freemium + B2B needs retention, not exponential growth.
VC = growth pressure = privacy compromises = moat death.

Options:
  - Bootstrapped (personal time)
  - Public grant (BPI France, French Tech, CNIL innovation)
  - Pre-seed < EUR 100K (angel, not VC)
```

### 7.5 Go-to-Market

**Phase 0 — Cold start (the existential risk)**

- DGCCRF public sanctions (seed confirmed scam numbers)
- Consumer association partnerships (federation nodes)
- Existing communities (signal-arnaques.com, forums)
- First 1000 beta testers x 5 numbers = 5000 seeded numbers

**Phase 1 — Launch France (M1-M6)**

- Anti-Truecaller positioning: "blocks spam without spying on your contacts"
- Target: consumer media (60M, UFC, Numerama, Frandroid)
- Natural virality: universal pain point, word-of-mouth

**Phase 2 — Traction (M6-M12)**

- B2B pilot contracts
- Open API
- First external federation node
- Voice proxy + premium conversion

**Phase 3 — Expansion (M12-M24)**

- Belgium, Swiss Romandie, Spain, Italy
- Each country = new federation node + local seed

### 7.6 Risks

| Risk | Prob. | Impact | Mitigation |
|------|-------|--------|------------|
| Cold start | High | Fatal | Multi-source seeding + beta |
| Apple changes CallKit | Medium | High | iOS 18.2 trend is openness |
| Truecaller goes GDPR-clean | Low | High | Federated moat |
| Registry abuse | High | Medium | Anti-gaming + presumption of innocence |
| SIP proxy too expensive | Medium | Medium | Selective escalation + sovereignty path |
| CNIL challenge | Low | High | Pre-launch DPIA + decay |
| Operator copies feature | Medium | Medium | Neutrality moat (can't federate across competitors) |

### 7.7 Moat

1. **Federated network effect** — more nodes = more reliable = more incentive to stay
2. **Privacy by design** — Truecaller can't reverse their model
3. **Neutrality** — Orange can't federate with Free. We can.
4. **Community data** — harder to replicate than an algorithm
5. **CWO protocol potential** — first phone domain of a multi-domain judgment network. As CYNIC covers tokens and CallShield covers phones, the shared substrate becomes a cross-domain moat no single-purpose competitor can replicate

---

## 8. Sovereignty & Infrastructure

### 8.1 SIP Sovereignty Path

```
S0 — Twilio (M0-M6)
  Turnkey, zero ops, focus product
  ~EUR 0.015/min FR | Full US dependency
  Exit: proxy > EUR 1K/month

S1 — OVH SIP (M6-M12)
  3-5x cheaper, French hosted
  ~EUR 0.004/min | OVH dependency
  Migration: config swap, same code

S2 — FreeSWITCH self-hosted (M12-M24)
  Zero SIP dependency, full control
  Server ~EUR 50-100/month + ~EUR 0.001-0.003/min
  Requires: 1 telecom/DevOps engineer

S3 — ARCEP operator declaration (M24+)
  Direct FR network peering
  Near-zero interconnect cost
  Justified only at > 100K premium users
```

**Architectural decoupling:**

```
VoiceProxyPort (trait)
  |-- TwilioAdapter       (S0)
  |-- OvhSipAdapter       (S1)
  |-- FreeSwitchAdapter    (S2-S3)
  |-- MockAdapter          (tests)
```

### 8.2 Self-Sufficiency Model

Every component runs at zero marginal cost when idle.

```
Component         Floor           At 100K users      Sovereign?
CYNIC kernel      Already running Already running     Yes (Rust binary)
  (phone domain)  (marginal: ~0)  (+EUR 20-50/mo DB)
Embedded Dog      EUR 0 (in-app)  EUR 0               Yes (TFLite/CoreML)
Voice proxy       EUR 0 (off)     EUR 500-2K/mo       Phase S2+
Federation        EUR 0 (P2P)     EUR 0                By design
Phone gossip      EUR 0 (phones)  EUR 0                By design
Solana anchor     EUR ~3/yr       EUR ~33/yr           Yes (public chain)
Apps              EUR 0 (FOSS)    EUR 99/yr Apple      Partial
```

Note: the kernel already runs for token-analysis. Adding domain "phone_number" is marginal cost (DB storage for observations/verdicts). No new server to deploy.

### 8.3 Dependency Audit

| Dependency | Sovereign? | Criticality | Exit |
|-----------|-----------|-------------|------|
| CYNIC kernel | Yes (own code) | Medium | Embedded Dog operates without it |
| SurrealDB | Yes (open source) | Medium | Kernel dep, already managed |
| TFLite / Core ML | Yes (open source / Apple native) | High | On-device, no server dep |
| Twilio | **No** | Medium | OVH -> FreeSWITCH |
| Play Store | **No** | High | F-Droid parallel |
| App Store | **No** | High | No iOS alt. Accepted. |
| Play Integrity | **No** | Medium | 1 of 3 anti-bot signals |
| Firebase/APNs | **No** | Low | ntfy.sh (Android) |
| libp2p | Yes | High | Standard, multi-impl |
| Solana | Yes (public chain) | Low | Shared CWO integrity. Can drop. |

### 8.4 Open Source Strategy

- CYNIC kernel: **already open source** (enables community nodes running all domains)
- Apps: **open source** (F-Droid, trust, contributions)
- Embedded Dog model: **open source** (TFLite weights published)
- Federation protocol: **open source** (shared with all CWO domains)
- Premium features: kernel-side gate (proxy activation), not client-side
- If the project dies, the protocol and embedded Dog survive. Any community can fork.

---

## 9. Delivery Plan

```
M0-M1   Kernel Domain + Embedded Dog
        - Add domain "phone_number" to CYNIC kernel:
          domains/phone_number.md (domain prompt)
          embedded_domains.rs (one-line addition)
          build_phone_stimulus() in stimulus.rs
          deterministic Dog for phone scoring (heuristics)
        - Train v0 Embedded Dog (decision tree, TFLite export, < 1MB)
          Features: call frequency, report count, number age, time-of-day
          Training data: public spam databases + DGCCRF sanctions
        - Seed data in kernel DB (DGCCRF + signal-arnaques.com)
        - Integration tests: /judge domain="phone_number" round-trip

M1-M3   Apps MVP
        - Android: CallScreeningService + post-call labeling
          Embedded Dog (TFLite) for local scoring
          SQLite cache (top-N spam + recent lookups)
          HTTP client to kernel /judge, /observe
        - iOS: Call Directory Extension (batch, pre-18.2)
          Embedded Dog (Core ML) for local scoring
          SwiftData cache
        - Closed beta 100-500 testers
        - Note: reporter scoring UNWEIGHTED in this phase (all = NEW tier,
          trust_weight 0.2). Acceptable: beta testers are vetted.

M3-M5   Live + Community
        - iOS 18.2: LiveCallerIDLookupExtension (real-time)
        - ReporterReputation system in kernel (human Dog quality tracking)
        - Anti-bot (Play Integrity + DeviceCheck)
        - Community dashboard in app
        - Solana anchoring live (hourly, all domains)
        - Embedded Dog v1 retrained on beta data
        - Open beta France

M5-M7   Proxy + Premium
        - Voice proxy (Twilio adapter, VoiceProxyPort trait in kernel)
        - Challenge flow
        - Premium tier (in-app payment)
        - Public launch France

M7-M10  Federation + Gossip
        - Federation layer in kernel (libp2p gossip, serves ALL domains)
        - Phone-to-phone gossip (Android, opt-in)
        - First external node (consumer association)
        - B2B API v1
        - CWO substrate assessment: what % of kernel is domain-agnostic?
          If > 30% -> consider cwo-core extraction
          If < 30% -> keep as-is

M10-M14 Sovereignty + Expansion
        - SIP -> OVH (S1)
        - F-Droid build
        - Belgium + Swiss Romandie
        - Layer 3 evaluation (on-device NLP, flagship only)
        - Infra self-sufficiency target

M14+    Maturity
        - FreeSWITCH (S2)
        - EU expansion
        - Full self-sufficiency
        - CWO protocol extraction (if validated across 2+ domains)
        - ARCEP declaration evaluation (S3)
        - Reporter incentive token evaluation (Phase 3+)
```

---

## 10. Market Context

**Epistemic labels:** observed (probed), deduced (from observed), inferred (pattern), conjecture (hypothesis)

### Existing Players

- **Truecaller** (Sweden): 500M+ users, 75% India, GDPR investigation 2025, uploads contacts [observed]
- **Hiya** (US): B2B to carriers (AT&T, Samsung). No France. [observed]
- **Should I Answer** (Czech): Privacy-first, community-only, small [observed]
- **Orange Telephone**: Basic blocking. Limited docs. [inferred]
- **Bloctel** (FR gov): 47% zero effect. Low company compliance. [observed]

### Regulatory

- STIR/SHAKEN rejected EU. SS7 can't carry signature payload. [observed]
- Truecaller GDPR investigation (Swedish DPA, 2025). Contact upload = Article 6 risk. [observed]
- iOS 18.2: Live Caller ID Lookup — third-party real-time lookup. [observed]

### Gaps Not Verified

- France annual spam volume (ARCEP 2024 inaccessible)
- SIP trunking exact costs (OVH 404)
- User willingness to pay in France (no survey data)

---

## 11. Falsification Criteria

| Hypothesis | What would falsify it |
|-----------|----------------------|
| Community labeling produces accurate scores | Agreement rate < 60% after 10K reports |
| Cold start solvable with public data + beta | < 1000 unique numbers seeded after 1 month |
| Premium conversion justifies proxy costs | Conversion < 0.5% at 50K users AND no B2B |
| Federation adds value over centralized | External nodes contribute < 5% of reports after 12 months |
| Privacy positioning differentiates | < 10% cite privacy as reason for choosing app |
| iOS 18.2 Live Caller ID sufficient | Apple restricts API or latency > 500ms |
| Selective proxy controls costs | > 30% of calls route through proxy |
| Phone gossip enriches cache | < 2% cache hit improvement from gossip after 6 months |
| Solana anchor adds trust | Zero users/partners cite auditability as value after 12 months |
| CWO protocol is extractable | Shared patterns between CallShield and CYNIC < 30% of codebase after both are built |
