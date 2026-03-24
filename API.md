<!-- AI-CONTEXT
role: api-reference
audience: frontend developers, agents, AI coding assistants
purpose: complete REST API contract for the CYNIC kernel
source_of_truth: cynic-kernel/src/api/rest/
base_url: http://<TAILSCALE_UBUNTU>:3030
cors: localhost:5173, localhost:5000, localhost:3000 (override with CYNIC_CORS_ORIGINS env)
transport: HTTP/JSON
auth: Bearer token on all endpoints except /health, /metrics, /events
rate_limit: 30 req/min global, 10 req/min on /judge. /health exempt.
-->

# CYNIC Kernel — REST API Reference

## Base URL

```
http://<TAILSCALE_UBUNTU>:3030   # Tailscale (production)
http://localhost:3030             # local development
```

## Authentication

All endpoints except `/health`, `/metrics`, and `/events` require:
```
Authorization: Bearer $CYNIC_API_KEY
```

Rate limit: 30 req/min global, 10 req/min on `/judge`. `/health` exempt. 429 on exceed.

---

## Public Endpoints

### GET /health

System health. **HTTP 200 = sovereign, HTTP 503 = degraded/critical.** Monitoring should check status code, not parse JSON.

**Without auth (public):**

```json
{ "status": "sovereign", "version": "v0.7.2", "phi_max": 0.618033988749895 }
```

**With valid Bearer token (extended):**

Additional fields: `axioms`, `dogs` (array of `{id, kind, circuit, failures}`), `storage`, `storage_namespace`, `storage_database`, `storage_metrics`, `embedding`, `verdict_cache_size`, `background_tasks`, `total_requests`, `total_tokens`, `estimated_cost_usd`, `uptime_seconds`, `alerts`.

| Status | Condition | HTTP |
|--------|-----------|------|
| `sovereign` | 2+ Dogs healthy | 200 |
| `degraded` | 1 Dog healthy | 503 |
| `critical` | 0 Dogs healthy | 503 |

### GET /metrics

Prometheus-format metrics. No auth.

### GET /events

SSE event stream. No auth. Events: `verdict`, `crystal`, `dog_failed`, `session`, `backfill`, `anomaly`. Keepalive every 15s.

---

## Judge

### POST /judge

Submit content for epistemic evaluation by independent AI validators (Dogs).

**Request:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | string | yes | Content to evaluate |
| `context` | string | no | Additional context |
| `domain` | string | no | Domain hint: `"chess"`, `"trading"`, `"code"`, `"general"` |
| `dogs` | string[] | no | Filter to specific Dog IDs |
| `crystals` | bool | no | Default `true`. Set `false` to disable crystal injection (A/B testing) |

**Response (200):**

| Field | Type | Description |
|-------|------|-------------|
| `verdict_id` | string (UUID) | Unique evaluation ID |
| `verdict` | `"Howl"` \| `"Wag"` \| `"Growl"` \| `"Bark"` | Overall verdict |
| `q_score` | QScore | Per-axiom scores + geometric mean total |
| `reasoning` | Reasoning | Per-axiom explanations |
| `dogs_used` | string | Contributing validators (`+`-separated) |
| `phi_max` | number | 0.618033988749895 |
| `dog_scores` | DogScore[] | Per-validator breakdown |
| `anomaly_detected` | bool | True when Dogs significantly disagree |
| `max_disagreement` | number | Largest score gap between Dogs on any axiom |
| `anomaly_axiom` | string \| null | Axiom with largest disagreement |
| `integrity_hash` | string | BLAKE3 hash of this verdict (omitted if unavailable) |
| `prev_hash` | string | Hash chain to previous verdict (omitted if first) |
| `cache_hit` | number | Cosine similarity score (only present on cache hits) |

**QScore:** `total`, `fidelity`, `phi`, `verify`, `culture`, `burn`, `sovereignty` — all `number` (0.0–0.618).

**Reasoning:** `fidelity`, `phi`, `verify`, `culture`, `burn`, `sovereignty` — all `string`.

**DogScore:** `dog_id` (string), `latency_ms` (number), `prompt_tokens` (number), `completion_tokens` (number), plus all 6 axiom scores (number) and `reasoning` (Reasoning).

**Verdict thresholds (q_score.total):**

| Verdict | Threshold | Meaning |
|---------|-----------|---------|
| Howl | > 0.528 | Highest epistemic quality |
| Wag | > 0.382 | Good quality |
| Growl | > 0.236 | Questionable |
| Bark | ≤ 0.236 | Rejected |

### GET /verdicts

List recent verdicts. Query param: `limit` (default 20, max 100). Returns array of verdict objects.

### GET /verdict/{id}

Get verdict by UUID. Returns single verdict object. 404 if not found.

---

## Crystals

### GET /crystals

List crystallized patterns. Query params: `limit` (default 50, max 200), `domain`, `state`.

Returns array of:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | FNV-1a hash ID |
| `content` | string | The crystallized insight |
| `domain` | string | e.g. `"chess"`, `"workflow"` |
| `confidence` | number | Running mean (0.0–1.0) |
| `observations` | number | Concordant evaluations count |
| `state` | string | `"forming"` \| `"crystallized"` \| `"canonical"` \| `"decaying"` \| `"dissolved"` |
| `created_at` | string | RFC3339 timestamp |
| `updated_at` | string | RFC3339 timestamp |

**State transitions:** Forming (< 21 obs) → Crystallized (≥ 21 obs, confidence ≥ 0.618) → Canonical (≥ 233 obs, confidence ≥ 0.618). Decaying if confidence drops below 0.382.

### GET /crystal/{id}

Single crystal by ID. 404 if not found.

### POST /crystal

Create a new crystal (starts at Forming, confidence 0.0).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | string (1-2000 chars) | yes | The pattern/insight |
| `domain` | string | no | Default `"general"` |

**Response (201):** `{ "id": "...", "domain": "...", "state": "Forming" }`

### POST /crystal/{id}/observe

Observe a score for an existing crystal (or create via UPSERT).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | string | yes | Crystal content |
| `domain` | string | no | Default `"general"` |
| `score` | number (0.0–1.0) | no | Default 0.5. Normalized by φ⁻¹ before storage |

**Response (200):** `{ "status": "observed" }`

### DELETE /crystal/{id}

Delete crystal. Idempotent. **Response (204):** No content.

---

## Analytics

### POST /observe

Record a tool-use observation (for CCM analytics, not crystal creation).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tool` | string (1-64 chars) | yes | Tool name |
| `target` | string | no | File path or target |
| `domain` | string | no | Inferred from target extension |
| `status` | string | no | Default `"success"` |
| `context` | string | no | Truncated to 200 chars |
| `project` | string | no | Default `"CYNIC"` |
| `agent_id` | string | no | Default `"unknown"` |
| `session_id` | string | no | Session identifier |

**Response (202):** `{ "status": "observed" }`

### GET /observations

List raw observations. Query params: `limit` (default 100, max 100), `domain`, `agent_id`.

### GET /sessions

List session summaries. Query param: `limit` (default 50, max 200).

Returns array of: `{ session_id, agent_id, summary, observations_count, created_at }`.

### GET /usage

Token consumption and cost tracking.

```json
{
  "total_tokens": 12345,
  "total_requests": 67,
  "estimated_cost_usd": 0.042,
  "uptime_seconds": 3600,
  "per_dog": [
    { "dog_id": "gemini-flash", "prompt_tokens": 5000, "completion_tokens": 2000,
      "total_tokens": 7000, "requests": 30, "failures": 1, "avg_latency_ms": 2100 }
  ]
}
```

### GET /audit

Query the MCP/REST audit trail. Query params: `limit` (default 50, max 100), `tool`, `agent_id`.

---

## Coordination

Multi-agent file locking and session management.

### POST /coord/register

Register an agent session. Sessions expire after 5 minutes of inactivity.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent_id` | string (1-64) | yes | Unique agent identifier |
| `intent` | string | yes | What this agent intends to do |
| `agent_type` | string | no | `"claude"`, `"gemini"`, `"hermes"`, `"human"` |

**Response (200):** `{ "status": "registered", "agent_id": "..." }`

### POST /coord/claim

Claim exclusive access to a target (file, resource).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent_id` | string (1-64) | yes | Claiming agent |
| `target` | string (1-256) | yes | Resource to claim |
| `claim_type` | string | no | Default `"file"` |

**Response (200):** `{ "status": "claimed", "agent_id": "...", "target": "..." }`
**Response (409):** `{ "error": "CONFLICT: '...' already claimed by: ..." }`

### POST /coord/claim-batch

Claim multiple targets atomically.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent_id` | string (1-64) | yes | Claiming agent |
| `targets` | string[] (1-20 items) | yes | Resources to claim |
| `claim_type` | string | no | Default `"file"` |

**Response (200):** `{ "agent_id": "...", "claimed": [...], "conflicts": [{"target": "...", "held_by": [...]}] }`

### POST /coord/release

Release claims. If `target` omitted, releases all claims for the agent.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent_id` | string (1-64) | yes | Releasing agent |
| `target` | string | no | Specific target (omit for release-all) |

**Response (200):** `{ "status": "released", "agent_id": "...", "detail": "..." }`

---

## System Info

### GET /dogs

Returns `string[]` — list of active Dog IDs (e.g. `["deterministic-dog", "gemini-flash", "sovereign-gpu", "llama-8b-hf", "qwen3-4b-ubuntu"]`).

### GET /agents

Active agent sessions and claims. Returns `{ active_agents, active_claims, agents: [...], claims: [...] }`.

---

## Domain Model

### The 6 Axioms

| Axiom | Key | Measures |
|-------|-----|----------|
| FIDELITY | `fidelity` | Truth loyalty — grounded in reality? |
| PHI | `phi` | Structural harmony — proportional? |
| VERIFY/FALSIFY | `verify` | Falsifiable — can this be proven wrong? |
| CULTURE | `culture` | Pattern continuity — honors traditions? |
| BURN | `burn` | Efficiency — no waste? |
| SOVEREIGNTY | `sovereignty` | Agency — preserves freedom? |

### Dogs (Independent Validators)

| Dog ID | Type | Where |
|--------|------|-------|
| `deterministic-dog` | Heuristic (instant) | In-kernel |
| `gemini-flash` | Gemini 2.5 Flash | Google API |
| `llama-8b-hf` | Mistral 7B | HuggingFace Inference |
| `sovereign-gpu` | Gemma 3 12B | Sovereign GPU node (RTX 4060 Ti) |
| `qwen3-4b-ubuntu` | Qwen 3.5 4B | Ubuntu CPU |

### Phi-Bounding

No score exceeds φ⁻¹ = 0.618033988749895. This is a structural epistemic limit encoding the principle that no claim deserves absolute confidence.

---

## Error Responses

All errors return: `{ "error": "description" }`

| HTTP | Meaning |
|------|---------|
| 400 | Bad request (validation failed) |
| 401 | Missing or invalid Bearer token |
| 404 | Resource not found |
| 409 | Conflict (coord claim) |
| 429 | Rate limit exceeded |
| 500 | Internal error (storage unavailable, etc.) |
| 503 | Degraded/critical health |

---

## TypeScript Interfaces

```typescript
interface JudgeRequest {
  content: string;
  context?: string;
  domain?: string;
  dogs?: string[];
  crystals?: boolean;
}

interface QScore {
  total: number;
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
}

interface Reasoning {
  fidelity: string;
  phi: string;
  verify: string;
  culture: string;
  burn: string;
  sovereignty: string;
}

interface DogScore {
  dog_id: string;
  latency_ms: number;
  prompt_tokens: number;
  completion_tokens: number;
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
  reasoning: Reasoning;
}

type VerdictKind = 'Howl' | 'Wag' | 'Growl' | 'Bark';

interface Verdict {
  verdict_id: string;
  verdict: VerdictKind;
  q_score: QScore;
  reasoning: Reasoning;
  dogs_used: string;
  phi_max: number;
  dog_scores: DogScore[];
  anomaly_detected: boolean;
  max_disagreement: number;
  anomaly_axiom: string | null;
  integrity_hash?: string;
  prev_hash?: string;
  cache_hit?: number;
}

interface HealthResponse {
  status: 'sovereign' | 'degraded' | 'critical';
  version: string;
  phi_max: number;
  // Auth-only fields:
  axioms?: string[];
  dogs?: { id: string; kind: string; circuit: string; failures: number }[];
  storage?: string;
  embedding?: string;
  verdict_cache_size?: number;
  total_requests?: number;
  total_tokens?: number;
  estimated_cost_usd?: number;
  uptime_seconds?: number;
  alerts?: { level: string; message: string; timestamp: string }[];
}

interface Crystal {
  id: string;
  content: string;
  domain: string;
  confidence: number;
  observations: number;
  state: 'forming' | 'crystallized' | 'canonical' | 'decaying' | 'dissolved';
  created_at: string;
  updated_at: string;
}
```
