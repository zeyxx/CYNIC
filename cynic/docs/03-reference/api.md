# CYNIC API Reference

> **REST API and SDK documentation**
>
> *🐕 κυνικός | "The interface to the organism"*

---

## REST API

### Base URL

```
http://localhost:6180
```

### Authentication

```bash
# API key (optional, for production)
Authorization: Bearer YOUR_API_KEY
```

---

## Endpoints

### Health

```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "uptime_seconds": 86400,
  "maturity": 0.68,
  "learning": {
    "q_updates_today": 47,
    "sessions_total": 3847
  },
  "checks": {
    "database": "connected",
    "migrations": 47,
    "daemon": "running",
    "learning": "active"
  }
}
```

---

### Judge Code

```http
POST /llm/complete
```

**Request:**
```json
{
  "prompt": "Create a function to parse JSON",
  "options": {
    "task_type": "code_generation",
    "domain": "code",
    "max_tokens": 2000
  }
}
```

**Response:**
```json
{
  "response": "def parse_json(data):\n    import json\n    return json.loads(data)",
  "judgment": {
    "q_score": 72,
    "verdict": "WAG",
    "confidence": 0.58,
    "dimensions": {
      "COHERENCE": 82,
      "ACCURACY": 75,
      "UTILITY": 68
    }
  },
  "llm_used": "deepseek-coder",
  "latency_ms": 1834,
  "cost_usd": 0.0023
}
```

---

### Consensus (Multi-LLM)

```http
POST /llm/consensus
```

**Request:**
```json
{
  "prompt": "Is this code secure? [code here]",
  "quorum": 0.618
}
```

**Response:**
```json
{
  "votes": [
    {"llm": "claude-opus", "verdict": "HOWL", "q_score": 88},
    {"llm": "deepseek", "verdict": "WAG", "q_score": 71},
    {"llm": "llama-70b", "verdict": "GROWL", "q_score": 58}
  ],
  "consensus": "WAG",
  "agreement": 0.66,
  "recommendation": "Safe with minor improvements"
}
```

---

### Get Patterns

```http
GET /patterns
```

**Response:**
```json
{
  "patterns": [
    {
      "name": "User prefers verbose comments",
      "count": 23,
      "confidence": 0.72
    }
  ]
}
```

---

### Budget Status

```http
GET /budget
```

**Response:**
```json
{
  "daily_limit": 10.00,
  "spent_today": 2.45,
  "remaining": 7.55,
  "state": "abundant"
}
```

---

## Python SDK

### Installation

```bash
pip install cynic
```

### Basic Usage

```python
from cynic import CYNICKernel

# Initialize
kernel = CYNICKernel(
    storage='postgres://localhost:5432/cynic',
    llm='ollama://qwen2.5:14b'
)

# Judge code
verdict = kernel.judge("def foo(): pass")

# Get dimensions
print(verdict.dimensions)

# Provide feedback
kernel.feedback(verdict.id, correct=True)
```

### Advanced Usage

```python
# Custom LLM configuration
kernel = CYNICKernel(
    storage='postgres://...',
    llm={
        'primary': 'ollama://qwen2.5:14b',
        'fallback': 'claude://claude-sonnet-4.5'
    }
)

# Domain-specific judgment
verdict = kernel.judge(
    code,
    domain='solana',
    dimensions=['SECURITY', 'CORRECTNESS', 'GAS_EFFICIENCY']
)

# Batch judgment
verdicts = kernel.judge_batch([
    "code1",
    "code2",
    "code3"
])
```

---

## MCP Tools

### Available Tools (90+)

| Tool | Description |
|------|-------------|
| `brain_cynic_judge` | Judge code/content |
| `brain_search` | Search memory |
| `brain_patterns` | Get learned patterns |
| `brain_health` | Check organism health |
| `brain_learn` | Submit feedback |
| `brain_consolidate` | Force consolidation |

### Example (Claude Code)

```
User: Judge this code
CYNIC: [calls brain_cynic_judge]

Result:
Q-Score: 72 (WAG)
Confidence: 58%
Dimensions: ...
```

---

## Error Handling

| Code | Error | Solution |
|------|-------|----------|
| 503 | LLM unavailable | Fallback to local model |
| 429 | Rate limited | Wait and retry |
| 500 | Internal error | Check logs, restart daemon |
| 402 | Budget exhausted | Use local-only mode |

---

## Rate Limits

| Tier | Requests/min | Tokens/day |
|------|--------------|------------|
| Free | 10 | 10,000 |
| Pro | 100 | 100,000 |
| Enterprise | Unlimited | Unlimited |

---

*🐕 κυνικός | "Every API call teaches the organism"*