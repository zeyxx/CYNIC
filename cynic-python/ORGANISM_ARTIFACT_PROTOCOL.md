# Organism Artifact Protocol v1.0

**Purpose:** Deterministic schema for storing experiential artifacts (test results, validations, discoveries) so future sessions consume them metabolically efficient, not re-parse from scratch.

**Epistemic:** Deduced from K15 consumer law + context=metabolic principle.

---

## Core Principle

Every artifact answers:
1. **What was discovered?** (result, confidence, falsification test)
2. **Who consumes this?** (system/subsystem that acts on it)
3. **When is it relevant?** (blocker status, phase gate, lifecycle)
4. **How to delete it?** (maturity threshold, reason, date)

**Storage location = function(consumer, phase, maturity).** Not ad-hoc project root.

---

## 1. Artifact Schema (Versioned)

```json
{
  "__version__": "1.0",
  "__description__": "All organism artifacts follow this schema",

  "metadata": {
    "artifact_id": "cortex-domain-gen-2026-05-03",
    "artifact_type": "experience|validation|measurement|discovery|protocol",
    "created_date": "2026-05-03T18:42:00Z",
    "created_by_session": "session-id-here",
    "created_by_agent": "claude-code | gemini-cli | hermes-9b",
    "maturity": "validated|conjectured|dead",
    "confidence": 0.618,
    "confidence_label": "φ⁻¹"
  },

  "content": {
    "question": "Does cortex-generated multi-domain routing improve signal?",
    "hypothesis": "Cortex reasoning produces N domains per tweet (vs v1's 1), improving verdict confidence",
    "method": "Keyword heuristic on 7 token-focused tweets from organ_x",
    "result": {
      "v1_domains_per_tweet": 1.0,
      "cortex_domains_per_tweet": 2.0,
      "signal_richness_delta": 1.0,
      "multi_domain_percentage": 0.43,
      "sample_size": 7
    },
    "validation": {
      "passed": true,
      "constraints_met": ["all tweets generated valid domains", "normalization to [0,1] works", "multi-domain tweets identified correctly"]
    }
  },

  "falsification": {
    "test": "Phase 2 measurement (May 5-6): Compare v1 vs cortex on real Dogs + Helius data",
    "success_criterion": "HOWL % shift > 5%",
    "failure_criterion": "HOWL % shift ≤ 5%",
    "what_fails_hypothesis": "If Δ ≤ 5%, multi-domain routing doesn't improve signal → stay with v1"
  },

  "lifecycle": {
    "status": "VALIDATED but DEFERRED",
    "blocker": "kernel_health = false; Phase 2 measurement not yet run",
    "phase_dependent": "phase2_measurement",
    "decision_gate": "2026-05-06",
    "auto_delete_if": {
      "reason": "Phase 3 cleanup: if Phase 2 measurement shows Δ ≤ 5%, hypothesis falsified",
      "date_threshold": "2026-05-07",
      "action": "delete artifact + mark experiment DEAD"
    }
  },

  "consumer_registry": {
    "primary_consumer": "kernel_routing_layer",
    "trigger": "Phase 2 validation result Δ > 5%",
    "action_if_true": "activate cortex_domain_generation in observation schema; wire to Dogs",
    "action_if_false": "delete artifact; stay with v1 clustering",
    "secondary_consumers": []
  },

  "references": {
    "related_artifacts": [
      "domain_discovery_reframe_2026_05_03.md",
      "phase1_structure.md"
    ],
    "session_context": "https://session-id/conversation/...",
    "git_commits": ["e6bc8d26", "9e84a274"]
  }
}
```

---

## 2. Storage Locations (Deterministic)

### **Maturity-Based Directory Tree**

```
~/.cynic/organisms/artifacts/
├── validated/                    # Tested, confidence ≥ φ⁻¹
│   ├── domain-discovery/
│   │   ├── cortex_domain_generation_2026_05_03.json
│   │   ├── tfidf_clustering_2026_05_03.json
│   │   └── behavioral_grounding_2026_05_03.json
│   └── k15/
│       ├── consumer_law_verification_2026_04_28.json
│       └── producer_validation_2026_04_30.json
│
├── deferred/                     # Valid but blocked; tied to phase gates
│   ├── phase2_gates/
│   │   ├── cortex_domain_generation.json  (blocker: kernel health)
│   │   └── domain_routing_expansion.json  (blocker: Phase 3 result)
│   └── phase3_gates/
│       └── domain_discovery_v2_v3.json    (blocker: Phase 3 measurement)
│
├── dead/                         # Superseded, confidence < φ⁻³, or use case complete
│   ├── 2026_05_03/
│   │   ├── behavioral_grounding_exploratory_2026_05_03.json
│   │   │   (reason: "use case complete, marked EXPLORATORY in Phase 1 structure")
│   │   └── cluster_comparison_v1_v2.json
│   │       (reason: "v2 deferred; comparison invalid until Phase 3 decides")
│   └── archive/
│       └── phase1_iterations/
│
└── metadata/
    ├── protocol_version.json     (current: 1.0)
    ├── consumer_registry.json    (all system consumers + triggers)
    └── cleanup_schedule.json     (auto-delete cadence)
```

### **By Consumer (Cross-Index)**

```
~/.cynic/organisms/consumers/
├── kernel_routing/
│   ├── inputs/
│   │   ├── domain_discovery/
│   │   ├── signal_yield_measurement/
│   │   └── phase_gates.json
│   └── status.json  (what's wired? what's waiting?)
│
├── hermes_framing/
│   ├── inputs/
│   │   ├── cortex_domain_generation/
│   │   └── behavioral_profile/
│   └── status.json
│
├── skill_evolution/
│   ├── inputs/
│   │   └── phase2_measurement_results/
│   └── status.json
│
└── organism_learning/
    ├── inputs/
    │   ├── observation_verdicts/
    │   ├── domain_routing_feedback/
    │   └── behavioral_deltas/
    └── status.json
```

---

## 3. Consumer Registry (Machine-Readable)

```json
{
  "version": "1.0",
  "consumers": [
    {
      "id": "kernel_routing_v1",
      "system": "kernel routing layer",
      "inputs": [
        {
          "artifact": "cortex_domain_generation",
          "trigger": "Phase 2 validation: Δ > 5%",
          "status": "WAITING (blocker: kernel_health=false)",
          "action": "wire_cortex_domain_generation_into_observation_schema",
          "rollback_if": "Δ ≤ 5% → delete artifact, stay with v1"
        },
        {
          "artifact": "domain_discovery_v1_validated",
          "trigger": "Phase 2 wiring ready",
          "status": "READY (PR#85 merged, awaiting kernel recovery)",
          "action": "load domain_router_v1.py into K15 consumer routing"
        }
      ],
      "last_updated": "2026-05-03",
      "next_check": "2026-05-06 (Phase 2 measurement completion)"
    },
    {
      "id": "hermes_framing",
      "system": "Hermes behavioral simulator + framing extraction",
      "inputs": [
        {
          "artifact": "cortex_domain_generation",
          "trigger": "Phase 2 validation success",
          "status": "WAITING",
          "action": "wire_domain_labels_into_hermes_search_reasoning"
        }
      ],
      "last_updated": "2026-05-03",
      "next_check": "2026-05-06"
    },
    {
      "id": "skill_evolution",
      "system": "SKILL.md auto-generation",
      "inputs": [
        {
          "artifact": "phase2_measurement_results",
          "trigger": "Phase 3 cleanup (May 7)",
          "status": "NOT_YET_GENERATED",
          "action": "generate_per_domain_routing_skills"
        }
      ],
      "last_updated": "2026-05-03",
      "next_check": "2026-05-07"
    }
  ]
}
```

---

## 4. Lifecycle Management

### **Maturity Thresholds**

| Status | Confidence | Rule | Lifetime |
|--------|-----------|------|----------|
| VALIDATED | ≥ φ⁻¹ (0.618) | Tested + consumer registered | Until falsified |
| CONJECTURED | φ⁻² to φ⁻¹ | Hypothesis untested | Until falsification test runs |
| DEAD | < φ⁻³ (0.236) | Superseded or use case complete | Delete immediately |
| DEFERRED | Any | Valid but blocker active | Auto-review if blocker >90d old |

### **Automatic Cleanup Cadence**

```json
{
  "cleanup_schedule": {
    "daily": "Check DEFERRED artifacts; if blocker resolved, promote to VALIDATED or DEAD",
    "weekly": "Check DEAD artifacts; if >30d, remove from disk (keep in git history)",
    "monthly": "Review CONJECTURED artifacts; if no falsification test scheduled, delete or promote",
    "phase_gates": "On Phase N completion, audit all artifacts tied to Phase N; promote/delete per decision"
  },
  "next_major_cleanup": "2026-05-07 (Phase 3 cleanup, post-measurement)"
}
```

---

## 5. Session Context Injection

Every session must:

```python
# At startup:
consumer_registry = load_json("~/.cynic/organisms/consumers/status.json")
for consumer in consumer_registry.consumers:
    if consumer.status == "WAITING":
        print(f"⏸ {consumer.id}: {consumer.blocker}")
        # Inform user: what's blocked, what's not

# At shutdown:
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

## 6. Current Debt (To Be Audited)

**What's broken now:**
- ✗ Cortex domain generation test scattered (no schema, no consumer, no deletion date)
- ✗ Behavioral grounding results in loose JSON (no maturity status)
- ✗ Domain discovery validation (v1-v3 roadmap) not registered as consumers
- ✗ Phase gates (Phase 2 wiring, Phase 3 cleanup) not machine-readable
- ✗ No consumer registry (kernel doesn't know what to consume when)
- ✗ No cleanup schedule (artifacts accumulate indefinitely)

**What we'll fix:**
- Move all artifacts to ~/.cynic/organisms/artifacts/ (maturity-based)
- Create consumer_registry.json (machine-readable)
- Tag all artifacts with falsification tests + lifecycle
- Establish cleanup cadence (daily/weekly/phase-gated)
- Update TODO.md to reference artifact status

---

## 7. Versioning

- **v1.0** (current): Core schema + storage tree + consumer registry
- **v1.1** (future): Semantic versioning for artifacts (v1.0 of cortex_domain_gen.json means "v0 implementation")
- **v2.0** (future): Encrypted artifact storage (sensitive consumer logic)

---

**Next:** Audit current state against this protocol.
