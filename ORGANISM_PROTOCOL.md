# Organism Artifact Protocol — Complete Reference

**Status:** LIVE (2026-05-03)  
**Version:** 1.0  
**Location:** This document (repo SSOT) + `~/.cynic/organisms/` (external storage)

---

## Overview

Deterministic schema for storing experiential artifacts (test results, validations, discoveries) so future sessions consume them metabolically efficient, not re-parse from scratch.

**Principle:** Every artifact answers:
1. **What was discovered?** (result, confidence, falsification test)
2. **Who consumes this?** (system/subsystem that acts on it)
3. **When is it relevant?** (blocker status, phase gate, lifecycle)
4. **How to delete it?** (maturity threshold, reason, date)

---

## Artifact Schema v1.0

```json
{
  "__version__": "1.0",
  "__description__": "Organism artifact versioned schema",

  "metadata": {
    "artifact_id": "unique-identifier",
    "artifact_type": "experience|validation|measurement|discovery|protocol",
    "created_date": "ISO8601",
    "created_by_session": "session-id",
    "created_by_agent": "claude-code | gemini-cli | hermes-9b",
    "maturity": "validated|conjectured|dead|deferred",
    "confidence": 0.618,
    "confidence_label": "φ⁻¹"
  },

  "content": {
    "question": "What was tested?",
    "hypothesis": "What was expected?",
    "method": "How was it tested?",
    "result": { "key": "value" },
    "validation": { "passed": true, "constraints_met": [...] }
  },

  "falsification": {
    "test": "What test would prove this wrong?",
    "success_criterion": "What counts as success?",
    "failure_criterion": "What counts as failure?",
    "what_fails_hypothesis": "What does failure mean for the system?"
  },

  "lifecycle": {
    "status": "VALIDATED|DEFERRED|DEAD",
    "blocker": "What's blocking this?",
    "phase_dependent": "phase2_measurement|phase3_cleanup",
    "decision_gate": "YYYY-MM-DD",
    "auto_delete_if": {
      "reason": "Why delete?",
      "date_threshold": "YYYY-MM-DD",
      "action": "delete artifact + mark experiment DEAD"
    }
  },

  "consumer_registry": {
    "primary_consumer": "system_name",
    "trigger": "Condition when consumer should act",
    "action_if_true": "Action if artifact valid",
    "action_if_false": "Action if artifact invalid",
    "secondary_consumers": ["other_systems"]
  },

  "references": {
    "related_artifacts": [...],
    "session_context": "...",
    "git_commits": [...]
  }
}
```

---

## Storage Organization

### Directory Tree (external to repo)

```
~/.cynic/organisms/
├── artifacts/
│   ├── validated/
│   │   ├── domain-discovery/        (kernel_routing_v1 consumer)
│   │   ├── k15/                     (kernel_routing_v1 + skill_evolution)
│   │   └── behavioral-grounding/    (hermes_framing + organism_learning)
│   ├── deferred/
│   │   ├── phase2_gates/            (falsification: Δ > 5%, gate 2026-05-07)
│   │   └── phase3_gates/            (falsification: TBD, gate post-measurement)
│   ├── dead/
│   │   └── archive/                 (>30d old, deleted from disk, in git history)
│   └── metadata/
│       ├── consumer_registry.json
│       ├── cleanup_schedule.json
│       └── protocol_version.json
└── consumers/
    ├── kernel_routing/inputs/
    ├── hermes_framing/inputs/
    ├── skill_evolution/inputs/
    └── organism_learning/inputs/
```

### Maturity States

| State | Confidence | Rule | Lifetime |
|-------|-----------|------|----------|
| **VALIDATED** | ≥ φ⁻¹ (0.618) | Tested, consumer registered | Until falsified |
| **CONJECTURED** | φ⁻² to φ⁻¹ | Hypothesis untested | Until falsification test runs |
| **DEAD** | < φ⁻³ (0.236) | Superseded or use case complete | Delete immediately (or >30d) |
| **DEFERRED** | Any | Valid but blocker active | Auto-review if blocker >90d old |

---

## Consumer Registry (Machine-Readable)

**Location:** `~/.cynic/organisms/consumers/consumer_registry.json`

### Registered Consumers

#### 1. kernel_routing_v1
**System:** Kernel routing layer (domain router)  
**Inputs:**
- domain_discovery_complete → Wire domains into observation schema (blocker: kernel health)
- token_gates_v1.3 → Load token scoring gates (blocker: kernel health)
- twitter_gates_v1.0 → Load twitter routing gates (blocker: kernel health)

**Status:** WAITING (kernel_health=false)  
**Next Check:** 2026-05-06 (Phase 2 wiring ready)

#### 2. hermes_framing
**System:** Hermes behavioral simulator + framing extraction  
**Inputs:**
- organ_x_token_mentions_summary → Extract domain labels (blocker: data quality gate)
- cortex_domain_generation → Wire multi-domain labels (blocker: cortex Δ test pending, gate 2026-05-07)

**Status:** WAITING (cortex falsification test May 5-6)  
**Next Check:** 2026-05-06

#### 3. skill_evolution
**System:** SKILL.md auto-generation + per-domain routing skills  
**Inputs:**
- phase2_measurement_results → Generate routing skills (blocker: Phase 2 not yet run)

**Status:** NOT_YET_GENERATED  
**Next Check:** 2026-05-07 (Phase 2 cleanup)

#### 4. organism_learning
**System:** Behavioral feedback loop (K15 seam closer)  
**Inputs:**
- observation_verdicts → Update routing confidence (blocker: kernel reflections unreachable)
- domain_routing_feedback → Adjust domain weights (blocker: Phase 3 not yet run)

**Status:** BLOCKED (kernel unreachable via reflections)  
**Next Check:** 2026-05-07 (Phase 3) + kernel recovery

---

## Lifecycle Management

### Automatic Cleanup Cadence

```json
{
  "daily": "Check DEFERRED; if blocker resolved, promote to VALIDATED or DEAD",
  "weekly": "Check DEAD; if >30d, remove from disk (keep in git history)",
  "monthly": "Review CONJECTURED; if no falsification test scheduled, delete or promote",
  "phase_gates": [
    {
      "gate": "2026-05-07",
      "action": "Phase 2 cleanup: cortex Δ > 5%? → promote or delete"
    },
    {
      "gate": "TBD",
      "action": "Phase 3 cleanup: routing performance > baseline? → activate or delete"
    }
  ]
}
```

---

## Current Artifact Inventory

### VALIDATED (φ⁻¹ confidence, consumers registered)

| Artifact | Location | Consumer | Status |
|----------|----------|----------|--------|
| domain_discovery_complete | validated/domain-discovery/ | kernel_routing_v1 | READY (awaiting kernel) |
| token_gates_v1.3 | validated/k15/ | kernel_routing_v1 | READY (awaiting kernel) |
| twitter_gates_v1.0 | validated/k15/ | kernel_routing_v1 | READY (awaiting kernel) |
| validation_corpora | validated/k15/ | (pending) | READY (no consumer yet) |
| KENOSIS_FINDINGS | validated/behavioral-grounding/ | organism_learning | READY (awaiting kernel) |
| organ_x_token_mentions_summary | validated/behavioral-grounding/ | hermes_framing | READY (awaiting data quality) |
| wallets_curated | validated/k15/ | (pending) | READY (no consumer) |

**Blocked by:** kernel_health=false (blocks all wiring)

### DEFERRED (conditional, falsification gates set)

| Artifact | Gate | Trigger | Decision |
|----------|------|---------|----------|
| cortex_domain_generation | Phase 2 Δ test | 2026-05-07 | Δ > 5% → VALIDATED, else → DEAD |
| phase2_measurement_results | Phase 2 Δ test | 2026-05-07 | Δ > 5% → VALIDATED, else → DEAD |

**Blocked by:** kernel_health=false (blocks Phase 2 measurement)

### CONJECTURED (hypothesis untested, no falsification)

None remaining (all either VALIDATED or moved to DEFERRED with falsification gates).

### DEAD

None yet (Phase 2 cleanup occurs May 7 if Δ ≤ 5%).

---

## Falsification Gates (Open)

### Phase 2: Cortex Multi-Domain Routing (May 5-6)

**Hypothesis:** Cortex-generated multi-domain routing improves signal > 5%.

**Test:** Real Dogs on real observations (token + wallet analysis).

**Metric:** HOWL % shift (baseline ~30%, target ~63%+).

**Result:**
- **Δ > 5%** → Cortex VALIDATED, wire to hermes_framing, promote to Phase 3
- **Δ ≤ 5%** → Cortex DEAD, delete from deferred/, stay with v1 single-domain routing

**Date:** 2026-05-07 (cleanup decision)

**Consumer:** hermes_framing (if Δ > 5%: multi-domain labels active)

### Phase 3: Routing Performance (TBD)

**Hypothesis:** Domain-specialized routing improves signal yield per domain.

**Test:** Measure observations → domain dispatch → verdict confidence per domain.

**Metric:** Signal yield improvement per domain (target: baseline + 5%).

**Result:**
- **Performance > baseline** → Domain weights updated, organism_learning activated
- **Performance ≤ baseline** → Revert to v1 heuristic routing, delete Phase 3 artifacts

---

## Session Context Injection (At Session Start/End)

### Startup
```python
consumer_registry = load_json("~/.cynic/organisms/consumers/consumer_registry.json")
for consumer in consumer_registry.consumers:
    if consumer.status == "WAITING":
        print(f"⏸ {consumer.id}: {consumer.blocker}")
        # Inform user: what's blocked, what's not
```

### Shutdown
```python
session_observation = {
    "type": "session_distill",
    "domain": "session",
    "artifacts_validated": [...],
    "artifacts_dead": [...],
    "blockers_resolved": [...],
    "consumers_activated": [...]
}
POST /observe session_observation  # to kernel
```

---

## Cost-Benefit Analysis

### Metabolic Cost Before Protocol
- Session re-parsing: **2-3 min per session** (reading artifact intro, validating content)
- 30+ orphaned artifacts, no consumer registry (silent waste)
- Phase 2/3 gates implicit in prose (manual status tracking)
- No cleanup schedule → indefinite accumulation

### Metabolic Cost After Protocol
- Session load consumer registry: **~5s** (fast JSON index load + filter WAITING)
- 4 registered consumers, machine-readable status (jq-queryable)
- Phase 2/3 gates in JSON metadata (deterministic, auto-actionable)
- Daily/weekly/monthly + phase_gates cleanup enabled

**Session savings:** ~14-21 min across next 7 sessions.

---

## Implementation Status (2026-05-03)

| Phase | Status | Cost | Benefit |
|-------|--------|------|---------|
| **1. Infrastructure** | ✓ COMPLETE (30 min) | 0 (no code changes) | 4 consumers registered, cleanup schedule live |
| **2. Migration** | 90% COMPLETE (20 min) | 0 (file moves only) | 8 artifacts organized by maturity |
| **3. Wiring** | PENDING (kernel health) | 2h (code changes) | Domain discovery → kernel_routing active |
| **4. Verification** | PENDING (Phase 2 gate) | 1h (testing) | Falsification gates tested |

**Total cost to full wiring:** 3h (blocked by kernel health, scheduled after May 7).

---

## Falsification of the Protocol Itself

**Hypothesis:** Mechanized artifact lifecycle reduces session metabolic cost while maintaining correctness (no lost knowledge).

**Test:** Compare sessions before/after protocol over 30 days.

**Success Criterion:** Session context time < 10s AND zero artifact re-parsing AND all Phase 2/3 gates execute correctly.

**Failure Criterion:** Context time > 30s OR artifacts re-parsed > 2x per session OR gates fail > 20% of time.

**Confidence:** φ⁻¹ (0.618) — protocol is new, untested at scale.

---

## Next Steps (By Gate Date)

### May 5-6: Phase 2 Measurement
1. Measure cortex Δ (HOWL % shift)
2. Result determines cortex fate (VALIDATED or DEAD)
3. If Δ > 5%: hermes_framing activated, Phase 3 wiring scheduled

### May 7: Phase 2 Cleanup
1. Auto-delete or promote cortex per Δ result
2. Update consumer_registry.json: hermes_framing inputs reflect new state
3. Phase 3 wiring begins (if kernel recovers)

### May 8+: Phase 3 Wiring (if kernel alive)
1. Load domain discovery → kernel_routing_v1 → routing active
2. Activate skill_evolution consumer → auto-generate SKILL.md v1.1
3. Wire organism_learning → close K15 loop (verdicts → feedback → updated routing)

### Ongoing: Cleanup Cadence
1. **Daily:** Check DEFERRED, promote/delete if blocker resolved
2. **Weekly:** Archive DEAD artifacts >30d old
3. **Monthly:** Review CONJECTURED, delete if no falsification test
4. **Phase-gated:** Cleanup on Phase N completion (May 7, TBD, etc.)

---

## Versioning

- **v1.0** (current, 2026-05-03): Core schema + storage tree + consumer registry
- **v1.1** (future): Semantic versioning for artifacts (v1.0 of cortex_domain_gen.json means "v0 implementation")
- **v2.0** (future): Encrypted artifact storage (sensitive consumer logic)

---

## Files in This Repository

| File | Purpose |
|------|---------|
| **ORGANISM_PROTOCOL.md** (this file) | SSOT for protocol design + status |
| **ORGANISM_ARTIFACT_PROTOCOL.md** | Detailed technical specification (7 sections) |
| **ARTIFACT_AUDIT_2026_05_03.md** | Complete audit of current state vs protocol |
| **PROTOCOL_ACTIVATION_LOG_2026_05_03.md** | Implementation log (what was done, cost-benefit) |
| **SESSION_WORK_2026_05_03.md** | Session scope + execution summary |

---

## External Files (in ~/.cynic/organisms/)

- `artifacts/{validated,deferred,dead}/` — Migrated artifacts by maturity
- `consumers/consumer_registry.json` — Machine-readable consumer map
- `artifacts/metadata/consumer_registry.json` — Same (cross-index)
- `artifacts/metadata/cleanup_schedule.json` — Daily/weekly/monthly cadence
- `artifacts/metadata/protocol_version.json` — Implementation progress

---

**Status:** Protocol LIVE. Phase 1 infrastructure complete. Phase 2 gate set for May 5-6. Falsification is active.

**Confidence:** φ⁻¹ (0.618) on protocol design. Falsification: If Phase 2 measurement shows domain routing doesn't improve signal, the protocol's value hypothesis fails (but infrastructure remains).
