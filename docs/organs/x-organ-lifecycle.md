# X Organ — Lifecycle Design (emerging from observed sessions)

> Data-centric: this design emerges from real agent sessions, not theory.
> Updated: 2026-04-27. Based on 7 x-explorer sessions (6 on 9B, 1 on 27B).

## What the Organ IS

A sovereign intelligence loop on crypto Twitter:
- **Eyes**: browser (CDP) + mitmproxy (passive capture) → raw data
- **Brain**: Hermes Agent (Qwen3.6-27B, 131K context) → reasoning, decisions, evolution
- **Memory**: SKILL.md (patterns) + dataset.jsonl (raw) + observations/ (curated) + crystals (CCM)
- **Hands**: cynic_observe, cynic_judge, SKILL.md writes → actions that change the system

## Three Operating Modes (observed)

### Mode 1: Explorer (active browsing)
**When**: Browser + proxy running. Agent navigates X.
**What**: Follow threads, profile authors, search for gaps in SKILL.md knowledge.
**Output**: New data in dataset.jsonl (via proxy), curated observations, cynic_judge calls.
**Value**: Discovers what passive capture misses — thread context, author credibility, emerging patterns.
**Constraint**: Needs browser on cynic-core. Browsing discipline (human-like, rate limits).

### Mode 2: Analyst (local dataset analysis)
**When**: No browser needed. Can run anytime.
**What**: Statistical analysis of dataset.jsonl, pattern recognition, SKILL.md evolution.
**Output**: Updated SKILL.md, observation files, cron reports.
**Value**: Finds structure in accumulated data. Measures what changed between sessions.
**Constraint**: Dataset quality depends on proxy enrichment. Analysis quality depends on model.

### Mode 3: Degraded (MCP down)
**When**: Kernel unreachable.
**What**: Local analysis only, queue observations for later, update skill.
**Output**: Local files, queued observations, memory notes.
**Value**: Session not wasted — analysis continues, findings persist.
**Constraint**: No crystal accumulation, no Dog evaluation. Findings are unvalidated.

## Dataset Quality Hierarchy (to build)

| Level | What | Source | Value |
|---|---|---|---|
| **L0: Raw** | Proxy-captured tweets | mitmproxy passive | Volume, noisy |
| **L1: Enriched** | Signal score + author tier + narratives | x_proxy.py heuristics | Filtered, still heuristic |
| **L2: Curated** | Agent-selected high-signal with context | Hermes explorer sessions | Quality, reasoned |
| **L3: Judged** | Dog-evaluated with q_score + axioms | cynic_judge (5 Dogs) | Calibrated, epistemic |
| **L4: Crystallized** | 21+ observations → stable pattern | CCM (kernel) | Permanent knowledge |

Currently: L0→L1 is automated (proxy+daemon). L1→L2 barely exists (1 manual report). L2→L3 starved (1 verdict ever). L3→L4 broken (all forming, 0 crystallized).

The agent's job is to build the L1→L2 bridge methodically and feed L2→L3 with quality input.

## Auto-Evolution Pattern (observed from 27B session)

### What happened (session 140752):
1. Agent found the dataset (30 turns of bootstrapping — too many)
2. Wrote a Python analyzer → extracted 5 structured findings
3. Created observation files with signal scores and significance
4. Produced a synthesis report with recommendations
5. Identified gaps to explore next

### What should happen (target lifecycle):
1. **Boot** (3 turns): load skill → check health → check crystals
2. **Orient** (2 turns): what changed since last session? What gaps exist?
3. **Explore** (10-20 turns): browse X OR analyze dataset — based on mode
4. **Curate** (5-10 turns): select high-value signals, build L2 data, observe/judge
5. **Evolve** (3-5 turns): update SKILL.md with findings, stats deltas, new patterns
6. **Report** (1 turn): session summary → memory + cron report

Total: ~25-40 turns. Not 79.

### SKILL.md Evolution Rules:
- Version number increments on every meaningful update
- Stats section updated with current dataset counts
- New patterns MUST cite evidence (N tweets, specific accounts)
- Gaps section updated: filled gaps removed, new gaps added
- Methods section: what analysis techniques worked, what didn't

## Infrastructure Dependencies

| Component | Where | Status | Impact if down |
|---|---|---|---|
| llama-server (27B) | cynic-gpu:8080 | TurboQuant turbo3, 131K ctx | Agent can't reason → session dead |
| CYNIC kernel | cynic-core:3030 | v0.7.7-275 | No MCP → degraded mode (local only) |
| Browser (Chrome CDP) | cynic-core:40769 | Via hermes-browser.service | No browsing → analyst mode only |
| Proxy (mitmproxy) | cynic-core:8888 | Via hermes-proxy.service | No passive capture → stale dataset |
| SurrealDB | cynic-core:8000 | Active | No persistence → observations lost |
| Embedding | cynic-core:8081 | Qwen3-Embed 0.6B | No crystal KNN → degraded crystal search |

## Metrics to Track (per session)

| Metric | Good | Bad | Why |
|---|---|---|---|
| Bootstrap turns | ≤5 | >15 | Efficiency of session start |
| Observations created | ≥3 | 0 | Feeding the crystal pipeline |
| cynic_judge calls | 1-3 | 0 | Exercising the Dogs |
| SKILL.md updated | yes | no | Evolution happening |
| New patterns found | ≥1 | 0 | Intelligence growing |
| Findings per turn | >0.1 | <0.05 | Signal-to-noise of agent reasoning |
| Total session turns | 25-40 | >60 | Focused vs wandering |

## Next Design Iterations

1. **Exploration methodology** — teach the agent HOW to browse X for specific gaps (profile deep-dives, thread following, search queries that target unknown patterns)
2. **Dataset curation workflow** — agent creates L2 curated records with structured annotations (falsifiability, confidence, verification status)
3. **Cross-session state** — skill evolution tracking (diff between versions), session-to-session knowledge transfer via crystals
4. **Quality metrics** — measure if SKILL.md predictions match future data (does a "rug warning" pattern predict actual rugs?)
5. **Multi-agent coordination** — explorer finds signals, analyst validates patterns, kernel crystallizes. Each role is clear.
