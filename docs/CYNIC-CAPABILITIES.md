# CYNIC Capabilities Map

> *What can CYNIC do? Everything. But with only 61.8% confidence.*

---

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CYNIC SYSTEM                                       │
│                   "Conscience Collective Décentralisée"                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                   │
│   │   JUDGE     │────▶│   LEARN     │────▶│   SHARE     │                   │
│   │  (Q-Score)  │     │  (Patterns) │     │  (Gossip)   │                   │
│   └─────────────┘     └─────────────┘     └─────────────┘                   │
│         │                   │                   │                            │
│         ▼                   ▼                   ▼                            │
│   ┌─────────────────────────────────────────────────────┐                   │
│   │              PROOF OF JUDGMENT (PoJ)                 │                   │
│   │         Immutable chain of all judgments             │                   │
│   └─────────────────────────────────────────────────────┘                   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. JUDGMENT ENGINE

### What It Does
Evaluates items (code, decisions, knowledge, tokens) across **25 dimensions** organized by the **4 Axioms**.

### Available via MCP

```
Tool: brain_cynic_judge
```

### Dimensions by Axiom

```
PHI (φ) - Mathematical Harmony
├── proportionality      # Are ratios golden?
├── selfSimilarity       # Fractal patterns
├── emergence            # New properties from parts
├── recursion            # Self-referential depth
├── coherence            # Internal consistency
└── scalability          # Works at any scale

VERIFY (✓) - Truth Seeking
├── sourceQuality        # Who said it?
├── corroboration        # Multiple sources?
├── methodRigor          # How verified?
├── dataIntegrity        # Data trustworthy?
├── claimPrecision       # Specific claims?
├── falsifiability       # Can it be disproven?
└── temporalValidity     # Still current?

CULTURE (⛩) - Context Matters
├── domainFit            # Right domain?
├── communityAlignment   # Respected norms?
├── historicalContext    # Past precedent?
├── ethicalAlignment     # Moral implications?
├── innovationBalance    # Novel vs proven?
└── accessibilityReach   # Who can use it?

BURN (🔥) - Simplicity Wins
├── clarityScore         # Easy to understand?
├── redundancyFree       # No bloat?
├── actionability        # Can act on it?
├── impactMagnitude      # Does it matter?
├── irreversibilityAware # Costs of reversal?
└── resourceEfficiency   # Worth the cost?
```

### Output

| Score Range | Verdict | Meaning |
|-------------|---------|---------|
| 80-100 | HOWL | Exceptional, celebrate |
| 50-79 | WAG | Good, proceed |
| 38.2-49 | GROWL | Concerning, investigate |
| 0-38.2 | BARK | Alert, danger |

### Example

```javascript
// Judge a code pattern
const result = await cynic.judge({
  type: 'code',
  content: 'async function fetchUser(id) { ... }',
  context: { language: 'javascript' }
});

// Returns:
{
  qScore: 67.3,
  verdict: 'WAG',
  confidence: 0.58,  // Never > 61.8%
  dimensions: { ... },
  axiomScores: {
    phi: 71.2,
    verify: 58.9,
    culture: 72.1,
    burn: 66.8
  }
}
```

---

## 2. KNOWLEDGE DIGESTION

### What It Does
Extracts patterns, insights, and learnings from content. Stores for future retrieval.

### Available via MCP

```
Tool: brain_cynic_digest
```

### Content Types

| Type | What It Extracts |
|------|-----------------|
| `code` | Patterns, anti-patterns, idioms |
| `conversation` | Decisions, preferences, context |
| `document` | Key facts, relationships |
| `decision` | Rationale, trade-offs, outcomes |

### Example

```javascript
await cynic.digest({
  type: 'code',
  content: 'class TokenValidator { ... }',
  source: 'holdex/validator.ts'
});
```

---

## 3. PATTERN SEARCH

### What It Does
Queries accumulated knowledge for relevant patterns and past judgments.

### Available via MCP

```
Tool: brain_search
Tool: brain_patterns
```

### Search Types

| Type | What It Finds |
|------|--------------|
| `judgment` | Past evaluations |
| `pattern` | Learned patterns |
| `decision` | Historical decisions |
| `all` | Everything |

---

## 4. PROOF OF JUDGMENT (PoJ) CHAIN

### What It Does
Maintains an immutable, cryptographically-linked chain of all judgments.

### Available via MCP

```
Tool: brain_poj_chain
```

### Operations

| Action | Description |
|--------|-------------|
| `status` | Current chain state |
| `verify` | Check integrity (hash chain) |
| `head` | Get latest block |
| `block` | Get specific block |
| `recent` | Get last N blocks |
| `export` | Export chain data |

### Chain Structure

```
Block N-1          Block N            Block N+1
┌──────────┐      ┌──────────┐      ┌──────────┐
│ prevHash │─────▶│ prevHash │─────▶│ prevHash │
│ timestamp│      │ timestamp│      │ timestamp│
│ judgments│      │ judgments│      │ judgments│
│ merkleRt │      │ merkleRt │      │ merkleRt │
│ signature│      │ signature│      │ signature│
└──────────┘      └──────────┘      └──────────┘
```

---

## 5. THE FOUR DOGS (Agents)

### What They Do
Specialized agents for continuous monitoring and assistance.

```
┌─────────────────────────────────────────────────────────────┐
│                    THE FOUR DOGS                             │
├──────────────┬──────────────────────────────────────────────┤
│  OBSERVER    │ Watches everything, notices anomalies        │
│  (👁️)        │ Passive, non-blocking                        │
├──────────────┼──────────────────────────────────────────────┤
│  DIGESTER    │ Extracts patterns from interactions          │
│  (📚)        │ Learns from conversations                    │
├──────────────┼──────────────────────────────────────────────┤
│  GUARDIAN    │ Blocks dangerous operations                  │
│  (🛡️)        │ BLOCKING - can stop execution               │
├──────────────┼──────────────────────────────────────────────┤
│  MENTOR      │ Shares wisdom from past learnings            │
│  (🎓)        │ Provides contextual guidance                 │
└──────────────┴──────────────────────────────────────────────┘
```

### Available via MCP

```
Tool: brain_agents_status
```

---

## 6. DOCUMENTATION CACHE

### What It Does
Fetches and caches library documentation from Context7 for faster retrieval.

### Available via MCP

```
Tool: brain_docs
```

### Operations

| Action | Description |
|--------|-------------|
| `query` | Fetch/query docs |
| `stats` | Cache statistics |
| `invalidate` | Clear cache |
| `list` | Show cached libraries |

---

## 7. ECOSYSTEM INTEGRATION

### What It Does
Manages documentation and sync across the $ASDFASDFA ecosystem projects.

### Available via MCP

```
Tool: brain_ecosystem
Tool: brain_integrator
```

### Ecosystem Projects

| Project | Purpose |
|---------|---------|
| CYNIC | Core judgment system |
| HolDex | Token holder analysis |
| GASdf | Gasless transactions |
| asdf-brain | Central brain |
| asdf-manifesto | Philosophy |

---

## 8. METRICS & MONITORING

### What It Does
Provides Prometheus-compatible metrics for monitoring.

### Available via MCP

```
Tool: brain_metrics
```

### Available Metrics

- `cynic_judgments_total` - Total judgments made
- `cynic_verdict_*` - Count per verdict type
- `cynic_avg_score` - Rolling average Q-Score
- `cynic_anomalies_detected` - Anomaly count
- `cynic_latency_*` - Response times

---

## 9. SESSION MANAGEMENT

### What It Does
Isolates user sessions for tracking and privacy.

### Available via MCP

```
Tool: brain_session_start
Tool: brain_session_end
```

---

## 10. FEEDBACK LOOP

### What It Does
Accepts corrections to improve judgment accuracy.

### Available via MCP

```
Tool: brain_cynic_feedback
```

### Feedback Types

| Outcome | Meaning |
|---------|---------|
| `correct` | Judgment was accurate |
| `incorrect` | Judgment was wrong |
| `partial` | Partially correct |

---

## Quick Reference: All MCP Tools

| Tool | Purpose |
|------|---------|
| `brain_cynic_judge` | Evaluate items |
| `brain_cynic_digest` | Extract knowledge |
| `brain_health` | System status |
| `brain_search` | Query knowledge |
| `brain_patterns` | List patterns |
| `brain_cynic_feedback` | Provide corrections |
| `brain_agents_status` | Four Dogs status |
| `brain_docs` | Documentation cache |
| `brain_ecosystem` | Ecosystem docs |
| `brain_poj_chain` | PoJ chain ops |
| `brain_integrator` | Cross-project sync |
| `brain_metrics` | Prometheus metrics |
| `brain_session_start` | Start session |
| `brain_session_end` | End session |

---

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                         USER                                 │
│                    (Claude Code, API)                        │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    @cynic/mcp                                │
│              MCP Server (stdio/SSE)                          │
│         Tools: judge, digest, search, etc.                   │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    @cynic/node                               │
│         Judge, State, Transport, Privacy                     │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   @cynic/protocol                            │
│          PoJ, Merkle, Gossip, Consensus                      │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  @cynic/persistence                          │
│         PostgreSQL + Redis + DAG + Graph                     │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    @cynic/core                               │
│           φ, Axioms, Timing, Q-Score, Identity               │
└─────────────────────────────────────────────────────────────┘
```

---

## What CYNIC Does NOT Do

- **Store private data** - Only patterns and judgments
- **Make decisions for you** - Only advises with ≤61.8% confidence
- **Trust blindly** - Always verifies
- **Claim certainty** - Max confidence is φ⁻¹

---

*🐕 κυνικός | Loyal to truth, not to comfort | φ⁻¹ = 61.8% max*
