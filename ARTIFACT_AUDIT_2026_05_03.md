# Organism Artifact Audit — 2026-05-03

**Purpose:** Classify current artifacts against ORGANISM_ARTIFACT_PROTOCOL.md. Identify debt and implementation cost.

**Protocol Compliance Status:** 0/100 (not yet implemented). All artifacts are orphaned (no consumer registry, no maturity states, no deletion criteria).

---

## Current Artifact Inventory

### By Category & Status

#### **DOMAIN DISCOVERY (K15 Routing Foundation)**

| Artifact | Location | Type | Lines | Maturity | Consumer | Blocker |
|----------|----------|------|-------|----------|----------|---------|
| DOMAIN_DISCOVERY_COMPLETE.md | cynic-python/ | Markdown | 120 | VALIDATED φ⁻¹ | kernel_routing (PENDING WIRING) | kernel health |
| emergent_clustering_results_2026_05_03.md | memory/ | Markdown | 28 | CONJECTURED | (none registered) | no consumer |
| tfidf_clustering_validation_2026_05_03.md | memory/ | Markdown | 38 | VALIDATED φ⁻¹ | (none registered) | no consumer |
| domain_verdicts_personalized.json | cynic-python/ | JSON | 10K+ | VALIDATED | (none registered) | no consumer |

**Analysis:** All domain discovery outputs in /memory (temporary) not in artifacts/ (permanent). No consumer registered. K15 gap: routing layer needs to CONSUME these to change observation behavior.

---

#### **PHASE GATES & FALSIFICATION**

| Artifact | Location | Type | Created | Maturity | Decision Gate | Auto-Delete If |
|----------|----------|------|---------|----------|---|---|
| CORTEX_DOMAIN_GENERATION_FINDINGS.md | memory/ | Markdown | 2026-05-03 | CONJECTURED | Phase 2 (May 5-6) | Δ ≤ 5% → delete, mark DEAD |
| cortex_domain_generator.py | (deleted) | Python | 2026-05-03 | CONJECTURED | Phase 2 | N/A (deleted per user feedback) |
| cortex_domain_phase2_test.py | (deleted) | Python | 2026-05-03 | CONJECTURED | Phase 2 | N/A (deleted per user feedback) |
| phase2_dry_run_results.json | cynic-python/ | JSON | 2026-04-xx | CONJECTURED | Phase 2 | Δ ≤ 5% → delete |
| phase2_measurement_results.json | cynic-python/ | JSON | 2026-04-xx | CONJECTURED | Phase 2 | Δ ≤ 5% → delete |

**Analysis:** Cortex multi-domain concept is CONJECTURED. Falsification test is Phase 2 (BLOCKED by kernel health). All phase 2 test harnesses scattered at root, no lifecycle metadata.

---

#### **BEHAVIORAL GROUNDING & VALIDATION**

| Artifact | Location | Type | Maturity | Consumer | Issue |
|----------|----------|------|----------|----------|-------|
| KENOSIS_FINDINGS.md | cynic-python/ | Markdown | CONJECTURED | (none) | Wu-Wei decision pattern found, no consumer (K15 gap) |
| validation_results.json | cynic-python/ | JSON | CONJECTURED | (none) | Token validation corpus, no consumer |
| validation_corpus_manual.json | cynic-python/ | JSON | VALIDATED | (none) | Curated token set, not consumed |
| validation_corpus_real.json | cynic-python/ | JSON | VALIDATED | (none) | Real token dataset, not consumed |

**Analysis:** All behavioral grounding lives in /cynic-python root with no registration. Validation datasets exist but no consumer is registered to act on them (R5 violation).

---

#### **CALIBRATION & GATES**

| Artifact | Location | Type | Maturity | Consumer |
|----------|----------|------|----------|----------|
| artifacts/token_gates_v1.3.json | cynic-python/artifacts/ | JSON | VALIDATED | kernel (not wired) |
| artifacts/twitter_gates_v1.0.json | cynic-python/artifacts/ | JSON | VALIDATED | kernel (not wired) |
| real_wallets_curated.json | cynic-python/ | JSON | VALIDATED | (none registered) |
| manual_wallets_curated.json | cynic-python/ | JSON | VALIDATED | (none registered) |
| active_wallets_curated.json | cynic-python/ | JSON | VALIDATED | (none registered) |

**Analysis:** Gate files exist in artifacts/ subdirectory (partial protocol adoption). Wallets are curated but no consumer is registered. K15: these should feed into a wallet-judgment consumer.

---

#### **MEASUREMENT & OBSERVATION**

| Artifact | Location | Type | Lines | Maturity | Consumer |
|----------|----------|------|-------|----------|----------|
| observations/ (directory) | cynic-python/ | Directory | 30+ files | CONJECTURED | kernel (via /observe endpoint) |
| observations/convergence-*.md | cynic-python/observations/ | Markdown | 50-100 | VALIDATED | (inference lab, no consumer) |
| observations/*.json | cynic-python/observations/ | JSON | variable | CONJECTURED | (none registered) |

**Analysis:** Observation directory exists with measurement output. Some files marked VALIDATED (convergence tests), others CONJECTURED. No consumer registry mapping these to kernel endpoints.

---

#### **INSTRUMENT CORPORA & DATASETS**

| Artifact | Location | Type | Maturity | Consumer | Issue |
|----------|----------|------|----------|----------|-------|
| phase2_token_corpus.json | cynic-python/ | JSON | VALIDATED | (none) | Token vocabulary, not consumed |
| phase2_wallet_corpus_real.json | cynic-python/ | JSON | VALIDATED | (none) | Real wallet set, not consumed |
| organ_x_token_mentions_summary.json | cynic-python/ | JSON | VALIDATED | hermes_framing (not wired) | Organ X token analysis |

**Analysis:** All corpus files at root. Some are production-ready (VALIDATED) but have no registered consumer. Hermes framing should consume token summary (K15 gap).

---

## Debt Summary

### **R5 Violations (Rule 5: No Dead Architecture)**

**Current:** 30+ artifacts with zero registered consumers
- Domain discovery docs → no kernel_routing consumer
- Behavioral grounding → no organism_learning consumer
- Validation corpora → no automated testing consumer
- Observation output → no skill_evolution consumer

**Cost:** Every session re-parses these artifacts (metabolic waste documented in protocol).

---

### **Storage Scattered (Maturity Structure Absent)**

**Current:**
```
cynic-python/
├── DOMAIN_DISCOVERY_COMPLETE.md        (VALIDATED, no location metadata)
├── KENOSIS_FINDINGS.md                 (CONJECTURED, no falsification test)
├── phase2_*.json                       (CONJECTURED, orphaned)
├── validation_*.json                   (VALIDATED, no consumer)
├── *_wallets_curated.json              (VALIDATED, scattered)
├── artifacts/
│   ├── token_gates_v1.3.json          (VALIDATED, not wired)
│   └── twitter_gates_v1.0.json        (VALIDATED, not wired)
└── observations/
    └── (30+ files, mixed maturity, no lifecycle metadata)
```

**Protocol requirement:** 
```
~/.cynic/organisms/artifacts/
├── validated/
│   ├── domain-discovery/
│   ├── k15/
│   └── behavioral-grounding/
├── deferred/
│   └── phase2_gates/
├── dead/
│   └── archive/
└── metadata/
```

**Debt:** 0 directories created. Need `mkdir -p` + file migration + metadata injection.

---

### **Consumer Registry Absent**

**Protocol requirement:** `~/.cynic/organisms/consumers/consumer_registry.json`

**Current state:** No machine-readable registry. Status queries must grep code.

**Missing consumers:**

1. **kernel_routing_v1**
   - Should consume: domain_discovery_complete, domain_verdicts_personalized
   - Action: Wire domains into observation schema
   - Status: WAITING (kernel health=false)

2. **hermes_framing**
   - Should consume: organ_x_token_mentions_summary, cortex_domain_generation (conditional)
   - Action: Extract domain labels for behavioral simulator
   - Status: WAITING (cortex Δ test pending)

3. **skill_evolution** (NEW)
   - Should consume: phase2_measurement_results, observations
   - Action: Auto-generate SKILL.md entries per domain routing performance
   - Status: NOT_YET_GENERATED (awaits Phase 3)

4. **organism_learning** (DORMANT)
   - Should consume: behavioral_profile, domain_routing_feedback, observation_verdicts
   - Action: Close K15 loop (observations → routing → verdicts → learning)
   - Status: BLOCKED (kernel unreachable via reflections, no feedback consumer)

---

## Lifecycle Violations

| Status | Count | Examples | Issue |
|--------|-------|----------|-------|
| VALIDATED but NO CONSUMER | 9 | domain_discovery, validation corpora, wallets | Can't act on validated knowledge |
| CONJECTURED, NO FALSIFICATION | 5 | KENOSIS_FINDINGS, phase2_results | Indefinite limbo (violates §4 Auto-Cleanup) |
| DEFERRED, NO PHASE GATE | 3 | CORTEX (Δ test May 5-6) | Implicitly gated but not machine-readable |
| DEAD but NOT DELETED | 0 | (none archived) | Clean here |

---

## Implementation Plan: Pay the Debt

### **Phase 1: Infrastructure (2h)**

1. Create maturity-based directory tree:
   ```bash
   mkdir -p ~/.cynic/organisms/artifacts/{validated,deferred,dead}/\
     {domain-discovery,k15,behavioral-grounding,phase2_gates,phase3_gates,archive}
   mkdir -p ~/.cynic/organisms/consumers/{kernel_routing,hermes_framing,skill_evolution,organism_learning}/inputs
   mkdir -p ~/.cynic/organisms/artifacts/metadata
   ```

2. Create consumer_registry.json (all 4 consumers listed with triggers & actions)

3. Create cleanup_schedule.json (daily/weekly/monthly cadence)

4. Write session init/stop hooks to load registry and report status

### **Phase 2: Migration (3h)**

For each artifact, create protocol-compliant wrapper:

1. **domain_discovery_complete.json** (VALIDATED, k15)
   - Copy to ~/.cynic/organisms/artifacts/validated/domain-discovery/
   - Add metadata wrapper: artifact_id, maturity, consumer_registry=kernel_routing_v1
   - Add lifecycle: status=VALIDATED, blocker=kernel_health, decision_gate=Phase 2 wiring

2. **validation_corpora** (VALIDATED, k15)
   - Copy to ~/.cynic/organisms/artifacts/validated/k15/
   - Add consumer_registry: no consumer yet (DEBT: skill_evolution needs to consume for test harnesses)

3. **phase2_measurement_results.json** (CONJECTURED, phase2_gates)
   - Move to ~/.cynic/organisms/artifacts/deferred/phase2_gates/
   - Add falsification test metadata: Δ > 5% → promote to VALIDATED, Δ ≤ 5% → mark DEAD + delete 2026-05-07

4. **gate files** (VALIDATED, k15)
   - Move to ~/.cynic/organisms/artifacts/validated/k15/
   - Wire into consumer_registry: kernel_routing, trigger=kernel recovery

5. **observations/** (MIXED, various)
   - Triage each by date/content
   - Convergence tests → validated/ (inference-validated)
   - Speculative docs → conjectured/
   - Stale (>30d) → dead/archive/

6. **KENOSIS_FINDINGS** (CONJECTURED, p1-complete)
   - Move to artifacts/validated/ (Wu-Wei pattern is OBSERVED, φ⁻¹)
   - Add consumer_registry: organism_learning (action: extend axiom?)
   - Falsification test: "Do 3 more Wu-Wei decisions surface?" (confidence thru May 20)

### **Phase 3: Wiring (2h)**

1. Update session-init.sh: Load consumer_registry, report blockers
   ```bash
   for consumer in $(jq -r '.consumers[] | select(.status == "WAITING") | .id' ~/.cynic/organisms/consumers/consumer_registry.json); do
     echo "⏸ $consumer: $(jq -r ".consumers[] | select(.id==\"$consumer\") | .blocker" ...)"
   done
   ```

2. Inject artifact paths into code (domain_router.rs, skill_manager.py):
   ```rust
   const DOMAIN_DISCOVERY: &str = include_str!("../../artifacts/validated/domain-discovery/discovery.json");
   ```

3. Add K15 consumer link: domain_discovery output → kernel routing decision

### **Phase 4: Verification (1h)**

1. Run make check (no code changes, only file migration)
2. Verify all 4 consumers registered and reachable
3. Test cleanup cadence on synthetic DEAD artifact
4. Confirm session-init reports status correctly

---

## Falsification Gates (Open Before Closing Debt)

| Gate | Trigger | Success | Failure |
|------|---------|---------|---------|
| **Phase 2 Measurement** | May 5-6 | Δ > 5% HOWL shift → VALIDATED cortex | Δ ≤ 5% → DELETE cortex, mark DEAD |
| **K15 Consumer Wiring** | Kernel recovery | Observations route to kernel verdict → VALIDATED | Routing fails → extend blocker |
| **Cleanup Cadence** | Daily cron | DEFERRED artifact promoted/deleted per blocker | Cron fails → manual cleanup |

---

## Cost-Benefit

| Aspect | Cost | Benefit |
|--------|------|---------|
| Create directories & wrappers | 2h | 50% reduction in artifact re-parsing (metabolic) |
| Consumer registry | 1h | Machine-readable status + automated remediation |
| Migration + metadata injection | 3h | VALIDATED vs DEAD/DEFERRED separation enables cleanup |
| Session init/stop hooks | 1h | Operator visibility: what's blocked, why, when |
| **Total** | **7h** | **Closes all R5 violations, enables K15 seam closure** |

---

## Current vs. Protocol-Compliant

### **TODAY (Chaotic)**
- 30+ artifacts at root + subdirs
- No maturity states → all treated equally
- No consumer registry → silent waste
- No cleanup schedule → indefinite accumulation
- Session cost: ~2-3 min re-parsing artifact intro

### **AFTER (Ordered)**
- Maturity-based tree (clear at a glance)
- Consumer registry (machine-readable status)
- Cleanup cadence (automatic lifecycle)
- Session cost: ~5s (fast index load, report blockers)

---

## Recommendation

**Execute Phase 1 + 2 immediately** (5h total):
- Unblock kernel routing from domain discovery
- Close R5 violations
- Enable Phase 2 measurement gate

**Defer Phase 3** to when kernel health recovers (true blocker for K15 consumer wiring anyway).

**Defer Phase 4** until Phase 2 gate completes (May 6).

---

**Session Gap Cost Recovered:** 7 sessions of context re-parsing ≈ 14-21 min context overhead eliminated.
