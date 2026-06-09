# CYNIC Python Research Protocol

**Effective**: 2026-05-03  
**Replaces**: Ad-hoc script placement (19 untracked files, no structure)

## Philosophy

Python tier-2 owns: analysis, validation, experimentation, data processing. It is NOT thin scripts. All work must be **traceable, versioned, and lifecycle-managed**.

## Directory Structure

```
cynic-python/
├── PROTOCOL.md (this file)
├── domains/
│   ├── domain-discovery/         # Domain detection & clustering
│   │   ├── v1/
│   │   │   ├── emergent_clustering_tfidf_v1.py
│   │   │   ├── compare_clustering_v1.py
│   │   │   ├── results_v1/
│   │   │   │   ├── emergent_clusters_tfidf_v1.json
│   │   │   │   └── metrics_v1.json
│   │   │   ├── NOTES_v1.md (session notes, decisions)
│   │   │   └── ARTIFACT_LIST_v1.md
│   │   ├── v2/
│   │   │   └── (future improvements: stopword filtering, silhouette validation)
│   │   └── ROADMAP.md (v1→v2→v3 planned work)
│   │
│   ├── k15-routing/             # K15 domain router (Phase 1)
│   │   ├── v1/
│   │   │   ├── domain_router_v1.rs
│   │   │   ├── domain_router_v1_test.py
│   │   │   └── NOTES_v1.md
│   │   └── ROADMAP.md
│   │
│   ├── behavioral-analysis/     # User behavior, engagement patterns
│   │   ├── v1/
│   │   │   ├── behavioral_miner_v1.py
│   │   │   ├── profile_extractor_v1.py
│   │   │   ├── results_v1/
│   │   │   │   ├── behavioral_profile_v1.json
│   │   │   │   └── author_narrative_clusters_v1.json
│   │   │   └── NOTES_v1.md
│   │   └── ROADMAP.md
│   │
│   ├── validation/              # Cross-domain validation & measurement
│   │   ├── v1/
│   │   │   ├── measure_agreement_v1.py
│   │   │   ├── silhouette_analysis_v1.py
│   │   │   └── NOTES_v1.md
│   │   └── ROADMAP.md
│   │
│   └── archive/                 # Deprecated/exploratory (moved after lifecycle ends)
│       ├── emergent_clustering_numpy_v0.py (replaced by TF-IDF)
│       ├── emergent_domain_discovery_lite.py (exploratory)
│       └── ARCHIVE_MANIFEST.md
│
├── shared/                      # Utilities shared across domains
│   ├── data_loaders.py
│   ├── metrics.py
│   └── __init__.py
│
└── notebooks/                   # Jupyter exploration (optional, not tracked)
```

## Lifecycle Management

### Entry Stages

1. **Exploratory** (1 session)
   - Purpose: Probe, hypothesis testing, "does this approach work?"
   - Artifacts: Jupyter notebooks, throw-away scripts
   - Storage: Not committed (`.gitignore`)
   - Example: `test_embedding_clustering.ipynb`

2. **Validated** (proven approach, multi-session)
   - Purpose: Domain discovered, results reproducible
   - Artifacts: Version-tagged scripts, reproducible results (JSON, metrics)
   - Storage: Committed, `domains/{domain}/v{N}/`
   - Example: `domain-discovery/v1/emergent_clustering_tfidf_v1.py`

3. **Archived** (replaced, superseded)
   - Purpose: Historical reference, don't run
   - Artifacts: Moved to `archive/`, README explaining why replaced
   - Storage: Committed for history, not imported by active code
   - Example: `archive/emergent_clustering_numpy_v0.py` (replaced by TF-IDF)

### Transition Rules

**Exploratory → Validated**:
- Falsification tests pass (F0-F3, etc.)
- Results reproducible across sessions
- Code reviewed for P1-P6 (type hints, tests, deps, logging, entry points)
- Creates `domains/{domain}/v{N}/NOTES_v{N}.md` (session log, decisions, blockers)

**Validated → Archived**:
- Superseded by improved version (v1 → v2)
- No longer called by active code
- Moved to `archive/`, not deleted
- Creates `ARCHIVE_MANIFEST.md` entry (why replaced, reference v{N+1})

## Naming Conventions

### Scripts

```
{script_type}_{domain}_{version}.py

script_type: one of
  - emergent_*: discovery/unsupervised learning
  - measure_*: validation, metrics, A/B testing
  - extract_*: data loading, ETL, transformation
  - train_*: ML training (not production, learning use only)
  - router_*: routing logic, decision logic

domain: one of
  - clustering, routing, behavioral, validation, etc.

version: v{N} where N = integer

Examples:
  emergent_clustering_tfidf_v1.py
  measure_silhouette_v1.py
  extract_behavioral_profile_v1.py
  router_domain_v1.py
```

### Results

```
{result_type}_{domain}_v{N}.{ext}

result_type: one of
  - clusters (clustering results)
  - metrics (validation metrics)
  - profile (extracted data)
  - predictions (model output)

Extensions:
  .json: structured data (results, profiles, metrics)
  .csv: time series, bulk export
  .txt: logs, text output

Examples:
  emergent_clusters_tfidf_v1.json
  metrics_silhouette_v1.json
  profile_behavioral_v1.json
```

### Notes & Roadmaps

```
NOTES_v{N}.md: Per-version session log
  - Date, session ID (first 8 chars of commit hash)
  - What was tested, decisions made
  - Blockers, next steps
  - Link to PR/commit

ROADMAP.md: Domain-level planning
  - v1 completed (link to NOTES_v1.md)
  - v2 planned improvements
  - Known debt, future work
```

## Entry Point Protocol (P6)

Every script must announce itself:

```python
#!/usr/bin/env python3
"""
Domain: {domain}
Version: v{N}
Purpose: One-line description

Entry point: python3 {script_name}.py
Args: [optional command-line args]
Output: {result_type}_*.json

Dependencies: 
  - pyproject.toml entries
  - External services (kernel, etc.)

Blocking conditions:
  - Kernel unreachable? Fail with exit code 1
  - Missing data? Fail loudly, don't skip

Example run:
  python3 emergent_clustering_tfidf_v1.py
  → Produces emergent_clusters_tfidf_v1.json
"""

__version__ = "1.0.0"

import logging
logging.info(f"Starting domain-discovery/emergent_clustering_tfidf_v1.0.0")
```

## Current State → Refactored

### Before (19 untracked files)

```
cynic-python/
├── emergent_clustering_numpy.py (binary keywords, validated)
├── emergent_clustering_tfidf.py (TF-IDF, validated)
├── author_narrative_clusters.py (exploratory)
├── behavioral_miner.py (exploratory)
├── trace_author_interactions.py (exploratory)
├── ... (13 more, no structure)
```

### After (organized by domain, versioned)

```
cynic-python/
├── domains/
│   ├── domain-discovery/v1/
│   │   ├── emergent_clustering_tfidf_v1.py (TF-IDF, validated)
│   │   ├── compare_clustering_v1.py (validation metrics)
│   │   ├── results_v1/
│   │   │   ├── emergent_clusters_tfidf_v1.json
│   │   │   └── metrics_v1.json
│   │   ├── NOTES_v1.md (session log, decisions)
│   │   └── ROADMAP.md
│   ├── behavioral-analysis/v1/
│   │   ├── behavioral_miner_v1.py
│   │   ├── profile_extractor_v1.py
│   │   └── NOTES_v1.md
│   └── archive/
│       ├── emergent_clustering_numpy_v0.py (superseded by TF-IDF)
│       └── ARCHIVE_MANIFEST.md
```

## Quality Gates (CI)

Before merging domain work:

```bash
# Syntax & types
mypy --strict domains/{domain}/v{N}/*.py

# Test coverage
pytest domains/{domain}/v{N}/test_*.py --cov=80

# Entry point check
python3 domains/{domain}/v{N}/{script}_v{N}.py --help  # Must work

# Lint
pylint domains/{domain}/v{N}/*.py

# Documentation
- NOTES_v{N}.md exists
- ROADMAP.md updated
- All imports documented in P10 (docstring contract)
```

## Example: Domain-Discovery v1 Refactor

**Current state** (untracked):
```
emergent_clustering_tfidf.py
emergent_clusters_tfidf.json
compare_tfidf_vs_binary.py
run_bridge_detection.py
```

**After refactor**:
```
domains/domain-discovery/v1/
├── emergent_clustering_tfidf_v1.py
├── compare_clustering_v1.py
├── bridge_detector_v1.py
├── results_v1/
│   ├── emergent_clusters_tfidf_v1.json
│   ├── metrics_v1.json
│   └── bridges_v1.json
├── NOTES_v1.md (session: d3a300ad, date: 2026-05-03, F0-F3 pass)
├── ROADMAP.md (v2: stopword filtering, silhouette > 0.5)
└── test_clustering_v1.py
```

## Next Session Continuation

When resuming v1 work:
1. Check `NOTES_v1.md` for blockers, session state
2. Import from `domains/domain-discovery/v1/`
3. If improving: increment version → `v2/` (don't modify v1)
4. Update `ROADMAP.md` with progress

If breaking change or new direction:
1. Create `v2/` directory
2. Copy + modify (don't delete v1)
3. Update `ARCHIVE_MANIFEST.md` explaining delta

## Enforcement

- **No orphans**: Every script in `domains/{domain}/v{N}/`, never root
- **No dead code**: Validated code only (exploratory in notebooks, not committed)
- **Versioning mandatory**: v{N} in all filenames, never generic names
- **Lifecycle tracked**: `NOTES_v{N}.md` per version, `ARCHIVE_MANIFEST.md` for retired code
- **Entry points published**: `__version__`, `--help`, logging on startup

This prevents:
- Chaos (19 files scattered)
- Orphans (scripts running without context)
- Debt accumulation (dead code persists indefinitely)
- Lost decisions (why was v0 replaced?)
