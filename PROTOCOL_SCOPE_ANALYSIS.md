# Protocol Scope Analysis — Static vs. Live Data

**Question:** Should the protocol cover observations + verdicts + crystals, or just static artifacts?

**Concern:** We're proposing to wire Cortexes into the organism. If wiring is incomplete or wrong, we could:
- Lose observation data (if no consumer reads /observe endpoint)
- Break feedback loops (if verdicts don't route back to routing decisions)
- Corrupt crystals (if concurrent writes collide)

---

## Data Types in the Organism

### STATIC ARTIFACTS (What We Just Did)
```
Domain discovery, gates, findings, measurement results
├── Lifecycle: VALIDATED → DEAD (slow transitions)
├── Volume: ~10-50 per cycle (low)
├── Lifetime: Days to months
├── Storage: ~/.cynic/organisms/artifacts/ (filesystem)
├── Access: Load-once at session start
└── Consumer: K15 system that ACTS on knowledge
```

**Current Protocol Coverage:** ✓ COMPLETE (maturity, consumer, falsification, auto-delete)

### LIVE DATA (Real-Time Streams)
```
Observations (hermes captures)
├── Volume: 100-1000 per day (high)
├── Lifetime: Hours to days (expires)
├── Storage: ~/.cynic/organs/hermes/... or kernel RTC?
├── Access: Real-time reads (don't batch-load at session start)
├── Consumer: kernel routing, skill_evolution, organism_learning
└── Expiry: Auto-delete if >7 days old

Verdicts (Dog outputs: HOWL/WAG/GROWL/BARK)
├── Volume: 10-100 per day
├── Lifetime: Weeks (needed for learning feedback)
├── Storage: SurrealDB (kernel storage)
├── Access: Query by verdict_id or time range
├── Consumer: organism_learning (closes K15 loop)
└── Expiry: Never (audit trail)

Crystals (Refined verdicts, CCM memory)
├── Volume: 1-10 per day
├── Lifetime: Permanent (SSOT for organism memory)
├── Storage: SurrealDB (kernel storage)
├── Access: Semantic search (for Dog context injection)
├── Consumer: Every Dog (via /inference endpoint)
└── Expiry: Never
```

**Current Protocol Coverage:** ✗ NONE (not scoped yet)

---

## If We Extend Protocol to Live Data

### What We'd Need to Define

1. **Storage Locations** (same maturity-based tree?)
   ```
   ~/.cynic/organisms/observations/
   ├── YYYY-MM-DD/
   │   ├── hermes-x-captures/      (raw captures from hermes)
   │   ├── kernel-observations/    (parsed observations, routed to Dogs)
   │   └── verdicts/               (Dog outputs: HOWL/WAG/etc)
   └── expired/
       └── (>7 days old, deleted weekly)
   ```

2. **Consumer Registry for Live Data**
   ```json
   {
     "consumer_id": "kernel_routing_v1",
     "input": "observations/{domain}/",
     "trigger": "New observation arrives (real-time)",
     "action": "Route to domain-specialist Dog",
     "expiry": "7 days after creation"
   }
   ```

3. **Falsification Tests** (not just gates)
   ```
   Hypothesis: If observation X is routed to Dog Y, does verdict improve?
   Test: Compare routing-blind vs routing-aware verdict on same observation
   Success: routing-aware verdict > routing-blind by >5%
   Failure: No difference → routing doesn't improve signal → revert to v1
   ```

4. **Real-Time Monitoring** (not batch)
   ```
   No "load all observations at session start"
   Instead: Stream observations via MCP (Option C)
   - Hermes pushes new capture → /observe endpoint
   - kernel routes → consumers
   - consumers ACK or NACK (K15: act or delete)
   ```

---

## Cost Analysis: Extend or Defer?

### If We Extend Protocol NOW (before Phase 2 wiring)
**Cost:** 8-12 hours
- Design observation lifecycle schema (2h)
- Design live data storage + consumer registry (2h)
- Wire observations to kernel /observe endpoint (2h)
- Create falsification tests for routing (2h)
- Test with real hermes captures (2h)

**Risk:** Scope creep. We're supposed to wire static artifacts first, THEN learn what consumers need.

**Benefit:** Phase 2 gate (May 5-6) becomes fully falsifiable:
- Cortex loads domain_discovery ✓
- Kernel routes observations via domain discovery ✓
- Signal improves (testable) ✓

### If We Defer (focus on static artifacts + wiring first)
**Cost:** 1.5 hours (Option A+B: hook + manifest)

**Benefit:** Simpler scope. Prove static artifact protocol works before extending to live data.

**Risk:** Phase 2 gate may not fully fire (observations not routed yet). Signal improvement remains hypothetical until observations are wired.

---

## The Real Question: What Is Phase 2 Actually Testing?

### Current (Deferred Live Data)
```
Hypothesis: Domain discovery improves signal
Test: Static measurement (cortex Δ > 5% on pre-captured tweets)
Blocker: No new live observations needed
Gate: May 5-6
```

### If We Extend (Live Data Wired)
```
Hypothesis: Domain-aware routing improves signal on LIVE observations
Test: Real hermes captures → domain routing → Dog verdicts
Blocker: hermes must be capturing, observations must route to kernel, kernel must route to Dogs
Gate: May 5-6 (REQUIRES everything wired)
```

**The second is the real test. The first is just "does cortex code work?"**

---

## Architecture Diagram (Current State)

```
STATIC ARTIFACTS (Protocol ✓)
├── domain_discovery.json
├── token_gates_v1.3.json
└── ORGANISM_PROTOCOL.md
    ↓ (via Option A hook / Option B manifest)
    Cortex loads at session start

LIVE DATA (Protocol ✗ — Not Wired)
├── Hermes X Captures (filesystem)
├── /observe endpoint (kernel REST)
├── Verdicts (SurrealDB)
└── Crystals (SurrealDB)
    ↓ (no consumer wired yet)
    Data accumulates, unused
```

**K15 violation:** Observations are produced (/observe) but no consumer is registered. This is dead nervous system.

---

## Recommendation

### Path 1: Minimal Scope (Recommended)
**By May 4:** Wire static artifacts only (Option A+B, 1.5h)
- Domain discovery → kernel_routing_v1
- Gates → kernel_routing_v1
- Protocol metadata → session_init

**Phase 2 gate (May 5-6):** Test static knowledge improves cortex reasoning
- Falsification: Cortex Δ > 5% on synthetic test set

**Post-Phase-2:** If gate succeeds, design live data protocol
- Observations → kernel routing → verdicts
- Real-world falsification test

**Confidence:** Higher (simpler, staged)

### Path 2: Full Scope (Ambitious)
**By May 4:** Wire BOTH static artifacts + live data (6-8h)
- Option A+B for static (1.5h)
- Live data schema + consumer registry (2h)
- Real-time observation routing (2h)
- Falsification test harness (1.5h)

**Phase 2 gate (May 5-6):** Test full K15 loop on live data
- Falsification: Real hermes observations routed by domain → signal improves

**Confidence:** Lower (complex, untested, multiple failure points)

---

## Decision Tree

**Q: Do you want to test Phase 2 on STATIC knowledge or LIVE data?**

```
Static (cortex reasoning improves with domain knowledge)
├── Scope: Wiring Option A+B only
├── Effort: 1.5h
├── Risk: Low (isolated)
├── Falsification: Δ > 5% on synthetic tweets
└── Next: Live data wiring post-Phase-2

Live (end-to-end K15 loop works)
├── Scope: Static artifacts + observations + routing + verdicts
├── Effort: 6-8h
├── Risk: High (many integration points)
├── Falsification: Δ > 5% on REAL hermes captures
└── Next: Measure Phase 3 routing performance
```

---

## If We Extend: Proposed Live Data Protocol

### Observation Schema (similar to artifact)
```json
{
  "__version__": "1.0",
  "observation_id": "hermes-x-2026-05-03-14:22:15",
  "observation_type": "twitter_capture",
  "created_date": "2026-05-03T14:22:15Z",
  "source": "hermes-x-search",
  
  "content": {
    "tweet_id": "...",
    "text": "...",
    "captured_domains": ["token_analysis", "governance"]
  },
  
  "routing": {
    "assigned_domain": "token_analysis",
    "domain_confidence": 0.72,
    "assigned_dogs": ["deterministic-dog", "qwen-7b"],
    "routing_timestamp": "2026-05-03T14:22:20Z"
  },
  
  "verdicts": {
    "deterministic_dog": {score: 0.68, verdict: "WAG"},
    "qwen_7b": {score: 0.71, verdict: "HOWL"}
  },
  
  "lifecycle": {
    "status": "PROCESSED",
    "expires_at": "2026-05-10T14:22:15Z",
    "keep_until": "2026-06-03T14:22:15Z",
    "delete_at": "2026-06-10T14:22:15Z"
  }
}
```

### Consumer Registry for Live Data
```json
{
  "consumer_id": "kernel_routing_v1",
  "input": "observations/{domain}/",
  "trigger": "Observation created",
  "action": "route_to_dog",
  "success_criterion": "Observation reaches appropriate Dog within 5s"
}
```

### Falsification Test
```
Hypothesis: Observations routed by domain → Dogs produce better verdicts
Test: Random sample of 100 hermes observations
Method:
  - v1 path: random Dog assignment
  - v2 path: domain-aware Dog assignment
Compare verdicts on same observations
Success: v2 verdicts > v1 by >5% (precision or confidence)
Failure: No difference → domain routing doesn't help → revert to v1
Gate: Phase 3 (post-May-7)
```

---

## Files to Create (If Path 2)

1. `PROTOCOL_OBSERVATION_SCHEMA.md` — Live data structure + lifecycle
2. `OBSERVATION_CONSUMER_REGISTRY.json` — Who reads observations, when, how
3. `OBSERVATION_FALSIFICATION_GATE.md` — Phase 3 test (domain routing improves verdicts)
4. Update `consumer_registry.json` with `kernel_routing_v1` observation consumer
5. Wire `/observe` endpoint → observation storage → consumer dispatch

---

## Current Recommendation

**Do Path 1 (minimal scope, 1.5h):**

1. Wire static artifacts only (domain discovery + gates)
2. Test Phase 2 gate on synthetic data (Δ > 5% on cortex reasoning)
3. If gate succeeds → proceed to live data wiring (Phase 3)
4. If gate fails → diagnose static artifact wiring before extending

**Rationale:** We don't know yet if domain discovery actually helps signal. Better to validate the mechanism (static knowledge) before scaling to live data (which adds infrastructure, observability, and operational risk).

**Confidence:** φ⁻¹ (0.618) — deferred scope lets us learn from Phase 2 results.
