# Organ X — Lifecycle Complete & Work Structure

## The Organism Chain

```
[1] HERMES (Sovereign agent, Qwen 3.5 9B)
    ├─ x-explorer (cron 4h) → browse X/twitter
    └─ x-analyst (cron 24h) → pattern mining, SKILL.md update
    
[2] PASSIVE CAPTURE (systemd, zero-LLM)
    ├─ mitmproxy (Chrome proxy) → domain filter x.com only
    ├─ x_proxy.py (mitmproxy addon) → enrich (signal_score, author_tier, narratives)
    └─ dataset.jsonl (append-only)
    
[3] INGEST DAEMON (systemd)
    ├─ x_ingest_daemon.py → tail dataset.jsonl
    └─ POST /kernel/observe → kernel ingestion
    
[4] KERNEL CONSUMPTION (cynic-kernel)
    ├─ /observe intake → observations stored
    ├─ /judge endpoint → Dogs evaluate content
    ├─ verdict generation → dogs.axiom_scores
    └─ crystal formation → persistent signals
```

## Real Function (What It Produces)

**NOT:** "Does 27B score tweets well?"  
**YES:** "When token is judged WITH X organ context vs WITHOUT, does the verdict change?"

### Measurement Chain

```
TOKEN JUDGMENT FLOW
│
├─ Baseline: /judge {token} WITHOUT X context 
│  └─ q_score = q_0 (baseline quality score)
│
├─ Enriched: /judge {token} WITH X organ signals injected
│  └─ q_score = q_1 (quality score with domain context)
│
├─ Delta: Δq = q_1 - q_0
│  └─ organ's real output = verdict impact
│
└─ Persistence: observation → verdict → crystal?
   └─ does pattern stick in kernel? (K15 consumer)
```

---

## Priority Domains (Three Pillars)

### Pillar 1: Tokens + Memecoin Communities (D1 Primary)
- **What:** Recovery scammers, KOL corruption, rug warnings, community health
- **Hermes observes:** @gcrtrd signal (top source), pump.fun patterns, coordinate bot rings
- **Kernel uses:** token verdict enrichment (recovery scammer context → BARK rating boost)
- **Measure:** Δq on known rugs when community context injected
- **Real outcome:** Better token-community risk assessment

### Pillar 2: Inference/LLM Organism Health (Observability)
- **What:** Dog quality on twitter, inference latency patterns, model drift
- **Hermes observes:** Which Dogs are strong on twitter? Which fail?
- **Kernel uses:** Dog routing (send X-domain to strong Dogs, bypass weak Dogs)
- **Measure:** Dog accuracy per domain before/after Hermes signal
- **Real outcome:** Better Dog dispatch strategy

### Pillar 3: Emergence (Reactive to Real Needs)
- **What:** New signals discovered by Hermes that don't fit D1-D2
- **How:** Captured naturally as Hermes finds unexpected patterns
- **Measure:** Novelty rate (% observations not matching SKILL.md)

---

## Work Breakdown (Lifecycle → Measurement)

### Phase 1: Instrument the Chain (Observability)

#### Task 1.1: Verify Hermes → Dataset Pipeline
- [ ] Hermes x-explorer: running? (cron 4h active?)
- [ ] Dataset.jsonl: growing? (monitor tail rate)
- [ ] Enrichment quality: spot-check 10 tweets (signal_score, author_tier accuracy)
- **Measure:** tweets/hour, enrichment latency, accuracy %

#### Task 1.2: Verify Ingest Daemon → Kernel
- [ ] Systemd service active (hermes-x-ingest)
- [ ] Daemon tailing dataset.jsonl: success?
- [ ] POST /observe delivery: latency?
- [ ] Observation stored in kernel: verify via /state-history
- **Measure:** ingest latency (ms), observation arrival time

#### Task 1.3: Verify Kernel Consumption
- [ ] /judge endpoint receives observations
- [ ] Dogs evaluate observations
- [ ] Q-scores logged
- [ ] Verdicts persisted to crystal intake
- **Measure:** q-score distribution, dog selection, verdict type (BARK/GROWL/etc)

---

### Phase 2: Measure Domain Impact (Baseline + Enriched)

#### Task 2.1: Token Domain (D1) — Community Context
**Setup:**
- Select 5 real tokens (2 known rugs, 2 legit, 1 controversial)
- Run /judge WITHOUT X organ context → baseline q_scores
- Run /judge WITH X organ context (recovery scammer signals, KOL corruption) → enriched q_scores

**Measure:**
- Δq per token (q_enriched - q_baseline)
- Verdict change (HOWL → BARK? WAG → GROWL?)
- Accuracy improvement (predicted verdict vs actual outcome)
- **Target:** Δq > φ⁻² (0.382) on known-bad tokens

**Falsifiable:** If Δq < 0.382 on 4/5 tokens → organ signal is too weak for token domain

#### Task 2.2: Inference Domain (D2) — Dog Quality
**Setup:**
- Measure Dog Q-scores on twitter content (X observations) before/after calibration
- Compare: deterministic-dog vs qwen-7b-hf vs qwen-9b-gpu on domain
- Identify which Dog is strong on twitter, which is weak

**Measure:**
- Dog accuracy per domain (vs human annotation, if available)
- Latency per Dog
- Cost/quality ratio per Dog
- **Target:** Identify tier-1 Dog for twitter (highest accuracy + <latency threshold)

**Falsifiable:** If all Dogs perform identically on twitter → domain-specific routing not needed

#### Task 2.3: Bootstrap Domains (D3-D6) — Signal Integration
**Setup:**
- X organ observations currently bootstrap D4 (recovery scammers) and others
- Are these patterns actually captured in bootstrap heuristics?
- Do bootstrap signals actually affect verdicts?

**Measure:**
- Δq on content that matches bootstrap patterns
- Bootstrap heuristic coverage (% X observations caught by D1-D6 rules)
- **Target:** Bootstrap patterns explain >60% of X observation signal

**Falsifiable:** If <40% of X signals match bootstrap rules → organ finds novel patterns (good)

---

### Phase 3: Crystal Formation (Persistence)

#### Task 3.1: Observation → Crystal
**Measure:**
- Do repeated high-signal observations form crystals?
- Crystal quality (falsifiable claim strength, coverage, novelty rate)
- Time to crystallization (how many observations before pattern sticks?)

**Setup:**
- Track 5 high-signal observations for 7 days
- Count: how many verdicts cite the pattern?
- Measure: crystal formation rate (% of observations → persistent signal)

**Falsifiable:** If <20% of observations crystallize → K15 violation (producer without consumer)

---

## Current State (Measured, 2026-04-27)

| Component | Status | Metric | Notes |
|-----------|--------|--------|-------|
| Hermes agent | Active | x-explorer 4h, x-analyst 24h | Crons wired |
| Dataset | Live | 2007 tweets, 71% high-signal | 1430 above 0.7 threshold |
| Enrichment | Validated | 100% spot-check accuracy | signal_score, author_tier, narratives |
| Agent observations | 5 produced | Signal 4-7 | Recovery scammers, KOL corruption, rug prediction, curator, sampling bias |
| Ingest pipeline | ? | Not yet measured | Cursor=0 → possibly stalled |
| Kernel /judge | Works | But Dogs not yet tuned to twitter | Unknown q-score distribution |
| Crystals | Not formed | 0 observations crystallized | K15: no consumer acting |

---

## Next Session — Priority Order

**P0: Fix the pipeline**
1. Boot kernel (cynic-core)
2. Test ingest daemon (tail dataset.jsonl → /observe)
3. Verify observation arrival in kernel
4. Measure latency end-to-end

**P1: Measure token domain impact (D1)**
1. Select 5 tokens (2 rugs, 2 legit, 1 controversial)
2. Baseline /judge (no X context)
3. Enriched /judge (with X recovery-scammer, KOL signals)
4. Calculate Δq, report verdict changes
5. Falsification: Is Δq > 0.382?

**P2: Measure inference domain (Dog calibration)**
1. Get Dog accuracy on X observations (Dogs vs human judgment of quality)
2. Identify strong Dog per domain
3. Report Dog rankings for twitter

**P3: Measure crystal persistence**
1. Wait 7 days, track crystal formation
2. Measure: % observations that become crystals
3. Report: K15 consumer activity (yes/no)

---

## Epistemic Status (Confidence Labels)

- **Pipeline state:** CONJECTURED (0.45) — infrastructure diagram says it works, but not measured live
- **Dog performance on twitter:** OBSERVED (0.0) — zero measurements yet, audit notes this gap
- **X observation quality:** HIGH (0.95) — audit confirmed 5 analyses are real, verified against dataset
- **Token domain impact:** NOT YET MEASURED (0.0) — this is Phase 2 work
- **Crystal formation:** NOT YET MEASURED (0.0) — depends on P0 (pipeline working)

---

## Success Criteria

Organ X lifecycle is **complete and functioning** when:
1. ✓ Hermes produces observations (done: 5 high-signal confirmed)
2. ✓ Observations enter kernel (pipeline working)
3. ✓ Kernel verdicts change with X context (Δq > 0.382 on D1)
4. ✓ Observations crystallize (>20% become persistent signals)
5. ✓ All three pillars measured (tokens, inference, emergence)

---
