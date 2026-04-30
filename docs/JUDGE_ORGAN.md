# Judge/CCM Organ — Truth Synthesis Loop

**Unified organ within cynic-kernel:** Judge orchestrates Dogs, CCM (Crystal Coherence Machine) consolidates verdicts into learnings.

## 5-Layer Design

```
LAYER 1: OBSERVATION INTAKE
  └─ observations posted to /observe (100 in system)
  └─ domains: general, kernel-lifecycle, organ-health, etc.
  └─ K15 producer: observe-tool.sh (hooks)

LAYER 2: JUDGMENT (Judge + Dogs)
  └─ stimulus: observation + context
  └─ parallel dog evaluation (qwen-7b, qwen-35, deterministic, etc.)
  └─ verdict produced (q_score, axioms, dog_scores)
  └─ 20 verdicts stored in SurrealDB

LAYER 3: VERDICT OBSERVATION (NEW — K15 FORWARD LOOP)
  └─ verdict posted back to /observe as observation
  └─ tool="verdict", domain=verdict.domain, status=ok
  └─ K15 producer: pipeline/side_effects() → post_verdict_observation()
  └─ K15 consumer: CCM intake

LAYER 4: CCM INTAKE + CRYSTALLIZATION
  └─ observation → semantic slug → FNV hash → crystal ID
  └─ epistemically gated (domain, quorum, disagreement)
  └─ 5 crystals currently (2 canonical, 2 crystallized, 1 forming)
  └─ BUT: currently processing metadata observations only (100 obs → 5 crystals)
  └─ BLOCKED: verdicts haven't flowed to observations yet (Layer 3 was missing)

LAYER 5: CRYSTAL INJECTION
  └─ judge.py reads crystals (NOT WIRED YET)
  └─ injects crystal context into Dog prompts
  └─ closes compound loop: verdicts → crystals → better verdicts
```

## K15 Breakdown (Producer/Consumer)

| Layer | Producer | Output | Consumer | Action |
|-------|----------|--------|----------|--------|
| 1 | observe-tool.sh | observation | Judge | evaluate |
| 2 | Judge | verdict | Layer 3 | post to /observe |
| 3 | post_verdict_observation() | observation | CCM intake | crystallize |
| 4 | CCM engine | crystal | Layer 5 (Layer 2) | inject context |

**Status:**
- Layer 1-2: LIVE (100 observations → 20 verdicts)
- Layer 3: JUST WIRED (verdict → observation posting)
- Layer 4: LIVE BUT STARVED (processing metadata, not verdicts)
- Layer 5: NOT WIRED (judge never reads crystals)

## Current State (2026-04-30)

**Execution:**
- Kernel healthy: false (503 on /health)
- Nightshift paused: true (GPU reserved for Hermes hackathon 2026-04-26→05-11)
- Observations total: 100 (0 from verdicts yet)
- Verdicts total: 20 (never posted back to /observe)
- Crystals total: 5 (forming on metadata, not verdicts)

**Data flows:**
```
observe-tool.sh → /observe → SurrealDB observations (100)
                 ↓
              Judge (evaluate)
                 ↓
              Verdict storage (20) — DEAD END (until now)
                 ↓
          [NEW] post_verdict_observation() → /observe → CCM intake
                 ↓
              Crystal formation (from verdict observations)
                 ↓
          [MISSING] judge reads crystals & injects context
```

## Falsification Tests (Path B)

### Test 1: Verdict Observation Flow
**Hypothesis:** When judge produces a verdict, post_verdict_observation fires and increments metrics.

**Falsification:** Run `/judge` request → check `/metrics` for `cynic_observation_post_success_total` increment.

```bash
curl -X POST "${CYNIC_REST_ADDR}/judge" \
  -H "Content-Type: application/json" \
  -d '{"content":"test token","domain":"token"}' \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" && \
curl -s "${CYNIC_REST_ADDR}/metrics" | grep observation_post_success
```

**Expected:** Counter > 0

### Test 2: Crystal Formation from Verdicts
**Hypothesis:** After verdict observations flow, crystal count grows and contains verdict content.

**Measurement before:** `curl /crystals | jq 'length'` → baseline

**Measurement after:** Send 50 verdicts via `/judge` → wait 2min → `curl /crystals | jq 'length'`

**Expected:** Crystal count increases, formed crystals reference verdict domains.

### Test 3: Judge Reads Crystals (Layer 5)
**Hypothesis:** When crystals are injected into judge prompt, q_score improves on repeated stimuli.

**Setup:**
- Call `/judge` with token stimulus → q_score_v1
- Call same stimulus again → check if q_score improves due to crystal context

**Measure:** Pearson correlation between crystal strength and verdict q_score improvement.

**Expected:** r > 0.6 (crystals improve verdict quality)

## Implementation Checklist

- [x] Layer 3 K15 wiring: post_verdict_observation() function
- [x] Metrics: inc_observation_post_success/failed
- [x] Compile + test Layer 1-3 flow

**Next (Path B):**
- [ ] Unpause nightshift (if needed) or verify verdicts flow via REST
- [ ] Test Layer 3 falsification (verdict → observation posting)
- [ ] Wire Layer 5: judge reads crystals + injects context
- [ ] Measure compound loop: verdicts → crystals → better verdicts

## Design Decisions

**Why unified organ in kernel:**
- Judge + CCM are kernel responsibilities (domain logic, not orbit)
- MANIFEST documents kernel's introspective loop
- Organ X pattern (5 layers + falsification) applies, but structure is internal

**Why post verdicts as observations:**
- K15: every producer must have an acting consumer
- Verdicts are insights from observations → natural to post back
- CCM intake already processes observations → no new machinery
- Completes the feedback loop

**Why domain gate (skip "general"):**
- Metadata observations (unclassified requests) poison crystals
- Only domain-specific verdicts should crystallize
- Same gate used by crystal_observer for consistency

**Why forward loop is critical:**
- Nightshift (backward loop) is paused through hackathon
- Forward loop completes the cycle without nightshift
- Allows verdicts to feed crystals immediately
- Enables compound learning (verdicts → crystals → better verdicts)
