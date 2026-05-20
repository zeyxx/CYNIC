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

### 2.1 System Overview

```
CLIENTS (phones = infrastructure nodes)
  Android App (Kotlin) ─────────┐
    CallScreeningService        │
    Local SQLite cache          │
    Phone gossip (opt-in)       │     HTTPS/gRPC
                                ├──────────────────┐
  iOS App (Swift) ──────────────┘                   │
    Live Caller ID (18.2+)                          v
    Local SQLite cache                        GATEWAY API
    Phone gossip (limited)                      (super-peer, not authority)
                                          /lookup   (< 100ms SLA)
                                          /report   (submit CallEvent)
                                          /contest  (challenge a flag)
                                          /sync     (federation inter-nodes)
                                          /challenge (trigger voice proxy)

                                          Auth: device attestation
                                                + anonymous token
                                          Rate limit: per hashed device_id
                                                  |
                            ┌───────────────────────┼──────────────────┐
                            v                       v                  v
                      SCORE ENGINE            REGISTRY STORE     VOICE PROXY
                      Aggregates              CallEvents         (premium only)
                      CallEvents ->           NumberReputations   SIP trunk
                      weighted score          ReporterReps
                      + confidence                                Challenge flow
                            |                       |                  |
                            v                       v                  v
                                    FEDERATION LAYER
                            Gossip protocol between nodes
                            Ed25519 signed NumberReputations
                            Merge by weighted consensus
                            Nodes = servers + phones (super-peer model)
                                          |
                              ┌───────────┴────────────┐
                              v                        v
                           API B2B               SOLANA ANCHOR
                    /api/v1/reputation/       Periodic Merkle root
                    /api/v1/stats             (~hourly, ~0.000025 SOL)
                    /api/v1/verify-self       Integrity verification
```

### 2.2 Phone-First Infrastructure (Super-Peer Model)

The server is not the authority. It's a highly-available peer in a network where phones are the primary nodes.

```
Lookup resolution order:

  Tier 0 : Local cache (SQLite on phone)
            Always available, offline-first, < 1ms
            Contains: recently seen numbers + top-N spam numbers

  Tier 1 : Phone-to-phone gossip (opt-in, Android primarily)
            Nearby phones exchange cache updates
            Protocol: BLE discovery + WiFi Direct data transfer
            Enriches local cache without server contact
            Android 12+ constraint: requires BLUETOOTH_ADVERTISE + BLUETOOTH_SCAN
              -> prompts deferred to post-onboarding (Screen 5, not Screen 2)
              -> gossip is opt-in and non-blocking; app works without it
            iOS constraint: no background BLE discovery allowed
              -> iOS gossip limited to foreground-only, minimal value
              -> iOS relies on Tier 2 (server) for cache enrichment

  Tier 2 : Server gateway (super-peer)
            Fallback when local cache misses
            Higher availability than any single phone
            < 100ms SLA

  Tier 3 : Federation backbone (server-to-server)
            Heavy sync between institutional nodes
            Operators, associations, regulators
```

**What this means:**
- If the server goes down, phones continue with their cache + local gossip
- The server's role is bootstrap (seed data) + availability (always-on peer) + B2B API
- Long-term, the server becomes optional for the core screening function
- iOS constraints limit phone gossip (no background BLE discovery), so iOS relies more on Tier 2

### 2.3 Critical Flows

**Flow 1 — Incoming call (free tier, < 100ms)**

```
Call arrives -> OS triggers CallScreeningService / LiveCallerID
  -> App queries Tier 0 (local SQLite cache)
  -> Cache hit? -> score -> block/label/allow
  -> Cache miss? -> query Tier 2 (Gateway /lookup, < 100ms SLA)
  -> score -> decision -> log CallEvent locally
  -> Post-call: notification "Label this call?"
  -> If gossip active: share new data with nearby peers (Tier 1)
```

**Flow 2 — Report (free tier)**

```
User labels call -> App stores CallEvent locally
  -> App POST /report to Gateway (CallEvent + label)
  -> Gateway verifies: device attestation OK? rate limit OK?
  -> Score Engine recomputes NumberReputation
  -> If score crosses threshold -> federation propagation
  -> ReporterReputation updated (agreement_rate)
  -> If gossip active: push update to nearby peers
```

**Flow 3 — Voice proxy (premium tier)**

```
Unknown call + ambiguous score (0.3 < score < 0.7)
  -> App detects -> redirects to Voice Proxy
  -> Proxy answers: "Your contact is screening calls. Please say your name."
  -> Timeout/silence -> reject
     + CallEvent(outcome: proxy_challenged, challenge: failed)
  -> Name spoken -> 3s recording -> push notification to user with audio
  -> Caller hears hold music/tone while waiting (max 30s total round-trip budget)
  -> User accepts (within 30s of call start) -> proxy forwards the live call
  -> User rejects OR timeout 30s -> proxy hangs up
     + CallEvent(label: nuisance/scam)
  -> If caller hangs up during wait -> CallEvent(challenge: timeout)
     Note: 30s is aggressive. Empirical data needed — if > 50% of legitimate
     callers hang up before 30s, increase to 45s or add "please hold" message.
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

### 2.4 Technical Stack

| Decision | Choice | Why |
|----------|--------|-----|
| Server language | Rust | < 100ms on /lookup, single binary, no runtime, sovereign deploy |
| HTTP framework | axum | Proven, tokio-native |
| Async runtime | tokio | Standard |
| Primary DB | Postgres (sqlx) | Reliable, good aggregation support |
| Hot cache | Redis | Top 100K queried numbers in memory |
| Local app cache | SQLite | Offline-first, no network required for lookup |
| Voice proxy | Twilio -> OVH -> FreeSWITCH | Sovereignty progression (see section 6) |
| Federation | libp2p gossipsub + ed25519-dalek | No heavy consensus, signed propagation |
| Phone gossip | BLE + WiFi Direct (Android), limited on iOS | P2P cache enrichment |
| Serialization | MessagePack (federation), JSON (external API) | Compact for P2P, readable for API |
| User auth | Anonymous device tokens | No account, no email, no phone stored |
| Anti-bot | Play Integrity + DeviceCheck + velocity | 3 independent signals |
| Android app | Kotlin native | Direct CallScreeningService access |
| iOS app | Swift native | Direct CallKit / LiveCallerIDLookup access |
| On-chain anchor | Solana (memo program) | Cheap, fast finality, existing tooling (Helius) |
| Solana SDK | anchor-lang or solana-sdk | Merkle root submission only |

### 2.5 Degraded State & API Contract

When the Gateway or Score Engine is degraded, `/lookup` must still return a usable response. The app must make a binary decision (block/allow) regardless of backend state.

```
Normal response:
  { "number": "+33612345678", "score": 0.73, "confidence": 0.45,
    "label_distribution": {...}, "status": "ok" }

Degraded responses:
  Score Engine down:
    { "score": null, "confidence": null, "status": "degraded",
      "action": "allow" }
    -> App follows presumption of innocence: let the call through

  Registry DB unreachable:
    { "score": null, "confidence": null, "status": "degraded",
      "action": "allow" }

  Number not in registry (cache miss + DB miss):
    { "score": null, "confidence": null, "status": "unknown",
      "action": "allow" }

  Insufficient data (confidence < 0.1):
    { "score": null, "confidence": 0.05, "status": "insufficient_data",
      "action": "allow" }

Rule: the app NEVER blocks when status != "ok".
The "action" field is authoritative — the app follows it, not the score.
```

### 2.6 Cache Invalidation (Phone Tier 0)

```
Strategy: TTL-based + server push for high-impact changes

TTL by score category:
  Confirmed spam (score > 0.7):     TTL = 24h  (stable, rarely changes)
  Ambiguous (0.3-0.7):              TTL = 4h   (volatile, needs freshness)
  Known safe (score < 0.3):         TTL = 12h  (stable)
  Unknown (not in cache):           no entry   (Tier 2 lookup on next call)

Server push (optional, premium):
  When a number's score crosses a threshold (e.g., 0.3 -> 0.8 = new spam),
  push invalidation to all devices that cached it.
  Protocol: lightweight push via FCM/APNs/ntfy.sh
  Payload: { "invalidate": ["+33612345678", "+33698765432"] }
  App deletes from local cache -> next call triggers fresh Tier 2 lookup.

Gossip cache sync:
  When phone gossip is active (Tier 1), peers exchange cache entries
  with their TTL. Receiving phone takes the fresher entry.
  Conflict: if peer has different score, take the one with higher confidence.

Cold start cache:
  On first install, app downloads top-N known spam numbers (N = 10K-50K).
  This ensures Tier 0 is useful immediately, before the user reports anything.
```

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
- Hourly Merkle root anchored on Solana for tamper detection

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
| Solana anchors | On-chain (Merkle roots only) | Public (hashes, no PII) |

**GDPR compliance:**
- No user account -> no personal data per Article 4
- Caller number treated as "legitimate interest" (Article 6.1.f) — fraud protection
- Right to object = contestation (Flow 4)
- Right to erasure = temporal decay naturally erases data in ~6 months
- Solana anchors contain only Merkle roots (32-byte hashes), no personal data

---

## 4. Solana Integration

### 4.1 Purpose

Solana serves as an **integrity anchor**, not a database. The registry lives off-chain. Solana proves it hasn't been tampered with.

### 4.2 Mechanism

```
Every N hours (configurable, default: 1 hour):

1. Score Engine snapshots all NumberReputations
2. Builds Merkle tree from sorted (number, score, confidence, event_count) tuples
3. Submits Merkle root (32 bytes) as a Solana transaction memo
4. Stores transaction signature in registry metadata

Verification (anyone can do this):
1. Download registry snapshot from any federation node
2. Rebuild Merkle tree locally
3. Compare root against on-chain value
4. If match: registry is intact since last anchor
5. If mismatch: tampering detected — alert network
```

### 4.3 Cost Model

```
Per anchor transaction:
  Base fee: 5000 lamports = 0.000005 SOL
  Memo program: ~0.00002 SOL (32 bytes of data)
  Total: ~0.000025 SOL per anchor

At hourly anchoring:
  24 anchors/day * 0.000025 SOL = 0.0006 SOL/day
  ~0.22 SOL/year
  At $150/SOL = ~$33/year

  Negligible. Even at 10x SOL price, still < $1/day.
```

### 4.4 What Solana Does NOT Do

- **Does not store CallEvents** (too voluminous, too expensive)
- **Does not run scoring logic** (latency: Solana ~400ms, we need < 100ms)
- **Does not replace federation gossip** (complementary: gossip = real-time, Solana = periodic proof)
- **Does not manage user identity** (anonymous device tokens, off-chain)

### 4.5 Optional Future: Reporter Incentives (Phase 3+)

```
Concept (CONJECTURE — not validated):
  - SPL token rewarding reliable reporters
  - Earned by: reports that align with consensus (agreement_rate > 0.75)
  - Spent on: premium features or governance votes
  - Risk: token incentives attract bots (gaming for rewards)
  - Mitigation: rewards proportional to trust_tier, not volume
  - Decision: evaluate empirically after 12 months of organic growth

  DO NOT build this at MVP. The community must grow organically first.
  Token incentives before product-market fit = death spiral.
```

### 4.6 Helius Integration

Existing Helius tooling applies to the Solana anchor layer:
- `getBalance`: monitor anchor wallet SOL balance
- `parseTransactions`: audit anchor history
- `getTransactionHistory`: verify anchor cadence
- Not needed for core product functionality

---

## 5. CYNIC Relationship

### 5.1 Philosophical Alignment

CYNIC's six axioms map directly to CallShield:

| Axiom | CallShield Application |
|-------|----------------------|
| FIDELITY | Reports must be faithful — anti-gaming ensures signal integrity |
| PHI | Scoring is proportional — weighted by trust, bounded by confidence |
| VERIFY | Everything is falsifiable — scores are derived, testable, contestable |
| CULTURE | Community-fed — the registry IS the culture of its reporters |
| BURN | No bloat — selective proxy, phone-first, minimal infra |
| SOVEREIGNTY | No single point of control — federated, open source, exit paths |

### 5.2 Technical Integration Path

```
Phase MVP (standalone):
  CallShield has its own Score Engine
  Simple weighted average, no CYNIC dependency
  Faster to ship, simpler to maintain

Phase 2+ (CYNIC-compatible):
  Score Engine can optionally delegate to CYNIC kernel
  A phone number becomes a "stimulus" judged by "Dogs" (reporters)
  NumberReputation maps to CYNIC verdict (HOWL/WAG/GROWL/BARK)
  Crystals = registry memory (high-confidence learned patterns)

  Mapping:
    HOWL  (> 0.528) -> score < 0.2  (high confidence legitimate)
    WAG   (> 0.382) -> score < 0.4  (probably legitimate)
    GROWL (> 0.236) -> score 0.4-0.7 (ambiguous, proxy zone)
    BARK  (<= 0.236) -> score > 0.7  (spam/scam)

  Note: inverted because CYNIC scores quality (high = good)
  while CallShield scores spam likelihood (high = bad).
  The CynicPipelineScorer adapter MUST invert: callshield_score = 1.0 - cynic_score.
  This inversion happens in the adapter, not in the Score Engine or storage layer.

Phase 3+ (shared kernel):
  cynic-kernel gains a `phone_number` domain
  Deterministic-dog handles heuristic scoring
  LLM Dogs NOT needed (BURN: numeric scoring doesn't need inference)
  Federation layer reuses CYNIC's crystal transport
```

### 5.3 Architectural Compatibility

CallShield's architecture is designed so CYNIC integration requires zero structural changes:

```
ScoreEngine trait
  |-- SimpleWeightedScorer    (MVP, standalone)
  |-- CynicPipelineScorer     (Phase 2+, delegates to CYNIC kernel)

FederationTransport trait
  |-- LibP2PGossip            (MVP, standalone)
  |-- CrystalMycelium         (Phase 3+, CYNIC crystal transport)
```

The trait boundary means CYNIC integration is an adapter swap, not a rewrite.

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
| Server infra | EUR 6-12K | Scales linearly |
| SIP trunking | EUR 2.4-12K | Proxy-only for premium subscribers (~10% of calls per premium user). Lower bound: 500 premium x 2 proxy calls/month x EUR 0.20/call. Upper bound: 5000 premium x 2 calls/month x EUR 0.10/call (OVH rates). |
| App stores | 15-20% of premium | Unavoidable |
| Development | EUR 150-250K | Main cost |
| Legal/GDPR | EUR 10-20K | DPO, DPIA |
| Solana anchoring | EUR ~33 | Negligible |
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
5. **Solana anchor** — auditable integrity no centralized competitor can match

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
Gateway API       EUR 0 (fly.io)  EUR 50-100/mo      Yes (Rust binary)
Registry DB       EUR 0 (SQLite)  EUR 30-80/mo       Yes (Postgres)
Cache             EUR 0 (embed)   EUR 20-50/mo       Yes
Voice proxy       EUR 0 (off)     EUR 500-2K/mo      Phase S2+
Federation        EUR 0 (P2P)     EUR 0               By design
Phone gossip      EUR 0 (phones)  EUR 0               By design
Solana anchor     EUR ~3/yr       EUR ~33/yr          Yes (public chain)
Apps              EUR 0 (FOSS)    EUR 99/yr Apple     Partial
```

### 8.3 Dependency Audit

| Dependency | Sovereign? | Criticality | Exit |
|-----------|-----------|-------------|------|
| Rust/cargo | Yes | Core | No lock-in |
| Postgres | Yes | High | Runs anywhere |
| Redis | Yes (source-avail) | Medium | Embedded replacement |
| Twilio | **No** | Medium | OVH -> FreeSWITCH |
| Play Store | **No** | High | F-Droid parallel |
| App Store | **No** | High | No iOS alt. Accepted. |
| Play Integrity | **No** | Medium | 1 of 3 anti-bot signals |
| Firebase/APNs | **No** | Low | ntfy.sh (Android) |
| libp2p | Yes | High | Standard, multi-impl |
| Solana | Yes (public chain) | Low | Can anchor elsewhere or drop |
| Helius | **No** | Very low | Direct RPC as fallback |

### 8.4 Open Source Strategy

- Registry protocol + federation spec: **open source**
- Server implementation: **open source** (enables community nodes)
- Apps: **open source** (F-Droid, trust, contributions)
- Premium features: server-side gate (proxy activation), not client-side
- If the project dies, the protocol survives. Any community can fork.

---

## 9. Delivery Plan

```
M0-M1   Foundation
        - Gateway API (Rust/axum): /lookup, /report, /contest
        - Registry DB (Postgres) + Score Engine
        - Seed data (DGCCRF + signal-arnaques)
        - Scoring integration tests
        - Solana anchor prototype (memo program)

M1-M3   Apps MVP
        - Android: CallScreeningService + post-call labeling
        - iOS: Call Directory Extension (batch, pre-18.2)
        - Local SQLite cache
        - Closed beta 100-500 testers
        - Note: scoring is UNWEIGHTED in this phase (all reporters = NEW tier,
          trust_weight 0.2). ReporterReputation bootstraps from M3 onward.
          This is acceptable because beta testers are vetted + small population.

M3-M5   Live + Community
        - iOS 18.2: LiveCallerIDLookupExtension
        - Reporter reputation system
        - Anti-bot (Play Integrity + DeviceCheck)
        - Community dashboard
        - Solana anchoring live (hourly)
        - Open beta France

M5-M7   Proxy + Premium
        - Voice proxy (Twilio adapter)
        - Challenge flow (/challenge endpoint goes live)
        - Premium tier (in-app payment)
        - Public launch France

M7-M10  Federation + Gossip
        - Server federation (libp2p gossip)
        - Phone-to-phone gossip (Android, opt-in)
        - First external node (association)
        - B2B API v1
        - CYNIC integration assessment (plug or skip?)

M10-M14 Sovereignty + Expansion
        - SIP -> OVH (S1)
        - F-Droid build
        - Belgium + Swiss Romandie
        - Infra self-sufficiency target

M14+    Maturity
        - FreeSWITCH (S2)
        - EU expansion
        - Full self-sufficiency (infra + devs)
        - CYNIC kernel integration (if validated)
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
| Solana anchor adds trust | Zero users/partners cite auditability as value |
| CYNIC integration adds scoring quality | CYNIC-scored numbers have same false positive rate as simple scorer |
