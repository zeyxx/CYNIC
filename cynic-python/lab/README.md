# CYNIC Data Science Lab — Designed for Flexibility

**Core principle:** Lab produces *insights*, not infrastructure. When kernel changes, lab adapts input format, not analysis logic.

## Architecture: Avoid Blocking Future Changes

### 1. Data Contracts (Semantic, Not Structural)

Lab inputs/outputs use **semantic schemas**, not tight coupling:

```json
// INPUT: Standardized Dataset
// Version: 1.0 (can evolve to 2.0 without breaking analyses)
{
  "dataset_id": "x-organ:2026-04-28",
  "version": "1.0",
  "schema_version": "tweet-canonical-1.0",
  "tweets": [
    {
      "id": "...",
      "text": "...",
      "signal": {"version": "2", "score": 4},  // Versioned!
      "author": {"tier": "whale", "verified": true},
      "metadata": {"source": "proxy-passive", "domain": "twitter"}
      // Can add fields without breaking existing analyses
    }
  ]
}

// OUTPUT: Analysis Briefing
// Contract: Always has these fields, but can extend
{
  "dataset_id": "x-organ:2026-04-28",
  "timestamp": "2026-04-28T19:00:00Z",
  "analyses": {
    "distribution": {...},
    "disagreement": {...},
    "false_negatives": {...},
    "cascades": {...},
    "novelty": {...}
  },
  "briefing": "...",
  "uncertainties": [...],
  "hypothesis": "...",
  "_meta": {
    "lab_version": "0.1.0",
    "analyses_run": [...],
    "data_lineage": "can trace back to original capture"
  }
}
```

**Benefit:** Kernel redesign doesn't break lab. Lab outputs feed adapters, not directly to agents.

---

### 2. Modular Analyses (Independent, Composable)

Each analysis is a **pure function**: dataset → insights.

```
lab/
├── analyses/
│   ├── distribution.py      # signal_score distribution per domain
│   ├── disagreement.py      # where Dogs disagree
│   ├── false_negatives.py   # high-q tweets with low signal
│   ├── cascades.py          # temporal, network patterns
│   ├── novelty.py           # patterns not in SKILL.md
│   └── axiom_audit.py       # which axioms tested/violated
│
├── config/
│   ├── domains.yaml         # D1-D6 definitions (not hardcoded)
│   ├── axioms.yaml          # 6 axioms + how to measure
│   ├── Dogs.yaml            # Dog versions, endpoints (kernel-agnostic)
│   └── signal_versions.yaml # signal_score v1, v2, v3 definitions
│
├── transforms/
│   ├── standardize.py       # Raw dataset → canonical format
│   ├── enrich.py            # Add verdict data if available
│   └── version.py           # Handle schema migrations
│
└── orchestrate.py           # Run analyses in order, collect results
```

**Each analysis can:**
- Run independently
- Be skipped if data not ready
- Be added/removed without touching others
- Output in standardized format

---

### 3. Config-Driven (Not Hardcoded)

Domains, axioms, Dogs: **all in YAML**, not Python code.

```yaml
# domains.yaml
D1:
  name: "Solana/Tokens"
  target: 500
  priority: "HIGH"
  keywords: ["token", "rug", "pump", "$"]
  description: "Recovery scammers, KOL corruption, rug warnings"

D4:
  name: "Security/Scams"
  target: 200
  priority: "HIGH"
  keywords: ["scam", "honeypot", "fraud", "dev sold"]

# axioms.yaml
FIDELITY:
  description: "Data truthfulness"
  measures: ["signal_score_validation", "false_negative_rate", "consistency_across_sources"]
  target: ">0.95"

# Dogs.yaml (kernel-agnostic!)
deterministic-dog:
  version: "1.0"
  endpoint: null  # Optional, not required by lab
  latency: 0
  
qwen35-9b-gpu:
  version: "1.0"
  endpoint: "http://<TAILSCALE_GPU>:8080"  # Can change without lab code
  latency: 7000
```

**Benefit:** Change Dogs/domains/axioms → update YAML, lab re-runs. No code change.

---

### 4. Versioning (Replay-Safe)

Every analysis tracks provenance:

```json
{
  "dataset_version": "x-organ:2026-04-28:canonical-1.0",
  "signal_version": "2",
  "dog_versions": {"deterministic-dog": "1.0", "qwen35-9b-gpu": "1.0"},
  "lab_version": "0.1.0",
  "axiom_versions": {"FIDELITY": "1.0", "PHI": "1.0"},
  "version_id": "lab-0.1.0-signal-2-tweets-2311"
}
```

**Can ask:**
- "Re-run analysis with signal_v3 instead of v2" (just change config)
- "Did Dogs v2 perform better than v1?" (replay on canonical data)
- "What changed between lab 0.1 and 0.2?" (diff reproducibility tokens)

---

### 5. Extensibility (Add Organs Without Refactoring)

Lab designed for multi-organ:

```
# standardize.py handles ANY organ format
def standardize(dataset, organ_adapter):
    """
    organ_adapter: transforms organ-specific format → canonical
    Lab doesn't care what organ produces data, just that it conforms to contract
    """
    return canonical_dataset

# Each organ brings adapter:
OrganAdapter("x-twitter"):
  def read(path): return x_dataset format
  def transform(raw): return canonical
  
OrganAdapter("blockchain"):
  def read(path): return chain_dataset format
  def transform(raw): return canonical
```

**Later:** Add chain-organ, discourse-organ, etc. Lab automatically works.

---

### 6. Auditable (Every Finding Has Provenance)

Every insight traces back to source:

```python
# Analysis output includes:
{
  "false_negatives": [
    {
      "tweet_id": "123456",
      "signal_score": 1,
      "dog_verdict": "Bark (q=0.8)",
      "explanation": "Text contains 'rug' (keyword) + high engagement (2k likes)",
      "source_signal": "signal_v2",
      "source_dog": "qwen35-9b-gpu:1.0",
      "novelty": "This pattern not in SKILL.md"
    }
  ]
}
```

**Hermes can trace:** "Why should I explore D4?" → Lab says "False negative pattern X" → Can verify on dataset.

---

## Implementation Phases (Non-Blocking)

### Phase Lab-1 (Tonight, 4h): Skeleton
- [ ] Config system (YAML-driven domains/axioms/Dogs)
- [ ] Standardize (clean 2,287 tweets)
- [ ] 3 quick analyses (distribution, disagreement, false_negatives)
- [ ] Write briefing for Phase B

### Phase Lab-2 (This week): Flesh Out
- [ ] Cascades analysis (temporal, network)
- [ ] Novelty detection (vs SKILL.md)
- [ ] Axiom audit (which tests FIDELITY, etc)
- [ ] Reproducibility test (same data = same results)

### Phase Lab-3 (Next week): Integration
- [ ] Adapter pattern (prepare for multi-organ)
- [ ] Verdict enrichment (if kernel provides verdicts)
- [ ] Feedback loop (lab briefing → Hermes decision → dataset → lab)

### Phase Lab-4+ (Future): Evolution
- [ ] Add second organ (chain, discourse, API)
- [ ] Cross-organ analysis (wallets mentioned in X, rug timing)
- [ ] Kernel adapters (if/when kernel changes)
- [ ] Distributed lab (run on cluster)

---

## Design Principles

1. **Lab is a module, not the system.** Hermes, Claude Code, future agents import it.
2. **Config drives behavior, not code.** Change YAML → change results.
3. **Analyses are pure functions.** Input data → output insights (no side effects).
4. **Versioning enables replay.** "What if signal_v3?" = one config change.
5. **Adapters hide organ differences.** Lab doesn't know/care if data is X, chain, discourse.
6. **Provenance everywhere.** Every finding traces to source data + version + config.
7. **Uncertainty is first-class.** Lab reports confidence + unknowns, not false certainty.
8. **Extensible, not rigid.** Adding analyses, axioms, organs = add new files, not refactor.

---

## API Contract (What Hermes/Claude Consume)

```python
# Lab produces this interface:
briefing = lab.analyze(dataset_path, config_overrides={})

briefing.analyses        # Dict of all analyses
briefing.recommend()     # Domain recommendation (semi-manual)
briefing.hypothesis()    # What to test next
briefing.gaps()          # What data is missing
briefing.uncertainties() # Confidence levels
briefing.to_json()       # Serialize for agents
```

Agents consume via JSON, not Python API. This survives refactoring.

---

## First Lab Run (Tonight)

```bash
cd cynic-python/lab

# 1. Standardize
python standardize.py --input ~/.cynic/organs/hermes/x/dataset.jsonl \
                       --output datasets/x_canonical_20260428.jsonl

# 2. Analyze
python orchestrate.py --dataset datasets/x_canonical_20260428.jsonl \
                      --config config/ \
                      --output datasets/analysis_20260428.json

# 3. Briefing
python briefing.py --analysis datasets/analysis_20260428.json \
                   --output brief_20260428.json

# 4. Deploy to Hermes
cp brief_20260428.json ~/.cynic/organs/hermes/x/lab_briefing_latest.json
```

Hermes reads: `lab_briefing_latest.json` → decides → logs decision.

Done. No kernel change. No blocking. Ready to grow.
