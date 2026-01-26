# @cynic/mcp - CYNIC MCP Server

> "φ distrusts φ" - κυνικός

Model Context Protocol server for Claude Code integration.

**Last Updated**: 2026-01-15

## Installation

```bash
# From CYNIC-new root
npm install

# Or link globally
npm link packages/mcp
```

## Usage

### As standalone server

```bash
# Run directly
node packages/mcp/bin/mcp.js

# Or via npm
npm run start -w @cynic/mcp
```

### With Claude Code

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "cynic": {
      "command": "node",
      "args": ["packages/mcp/bin/mcp.js"],
      "cwd": "/path/to/CYNIC-new"
    }
  }
}
```

## Tools

### brain_cynic_judge

Judge an item using CYNIC's 25-dimension evaluation across 4 axioms.

```javascript
// Input
{
  item: { type: 'code', content: 'function foo() {}', verified: true },
  context: { source: 'review', kScore: 75 }
}

// Output
{
  requestId: 'jdg_abc123',
  score: 62.5,           // Q-Score (0-100)
  verdict: 'WAG',        // HOWL/WAG/GROWL/BARK
  confidence: 0.58,      // Max 61.8% (φ⁻¹)
  axiomScores: { PHI: 65, VERIFY: 60, CULTURE: 62, BURN: 63 },
  weaknesses: ['VERIFY dimension below threshold'],
  finalScore: 68.5,      // Only if kScore provided
  phi: { maxConfidence: 0.618, minDoubt: 0.382 }
}
```

### brain_cynic_digest

Digest text content and extract patterns.

```javascript
// Input
{ content: 'Long text with code blocks...', source: 'meeting-notes' }

// Output
{
  digestId: 'dig_xyz789',
  stats: { words: 500, sentences: 25, estimatedReadTime: 3 },
  patterns: [
    { type: 'code', count: 3 },
    { type: 'decisions', count: 2 }
  ]
}
```

### brain_health

Get system health status.

```javascript
// Input
{ verbose: true }

// Output
{
  status: 'healthy',
  identity: { name: 'CYNIC', greek: 'κυνικός' },
  phi: { maxConfidence: 0.618, minDoubt: 0.382 },
  judge: { totalJudgments: 42, avgScore: 58.3 },
  tools: ['brain_cynic_judge', ...]
}
```

### brain_search

Search knowledge base for past judgments.

```javascript
// Input
{ query: 'authentication', type: 'judgment', limit: 5 }

// Output
{
  results: [...],
  total: 3
}
```

### brain_patterns

List detected patterns and anomalies.

```javascript
// Input
{ category: 'anomaly', limit: 10 }

// Output
{
  patterns: [
    { category: 'anomaly', residual: 0.45, itemType: 'code' },
    { category: 'verdict', distribution: { HOWL: 5, WAG: 30, ... } }
  ]
}
```

### brain_cynic_feedback

Provide feedback on past judgments for learning.

```javascript
// Input
{
  judgmentId: 'jdg_abc123',
  outcome: 'incorrect',
  reason: 'Missed security issue',
  actualScore: 35
}

// Output
{
  feedbackId: 'fb_def456',
  learningDelta: -27.5,
  message: '*growl* Learning...'
}
```

## Philosophy

- **Max confidence**: 61.8% (φ⁻¹) - Never trust fully
- **Min doubt**: 38.2% (φ⁻²) - Always question
- **Verdicts**: HOWL (≥80), WAG (≥50), GROWL (≥38.2), BARK (<38.2)

## Development

```bash
# Run tests
npm test -w @cynic/mcp

# Test manually
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | node packages/mcp/bin/mcp.js
```

---

*🐕 κυνικός | Loyal to truth, not to comfort | φ⁻¹ = 61.8% max*
