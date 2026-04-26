<!-- AI-CONTEXT
role: api-reference
audience: frontend developers, agents, AI coding assistants
purpose: complete REST API contract for the CYNIC kernel
source_of_truth: cynic-kernel/src/api/rest/
base_url: http://<TAILSCALE_CORE>:3030
cors: localhost:5173, localhost:5000, localhost:3000 (override with CYNIC_CORS_ORIGINS env)
transport: HTTP/JSON
auth: Bearer token on all endpoints except /health, /live, /ready, /metrics, /events
rate_limit: 30 req/min global, 10 req/min on /judge. /health exempt.
-->

# CYNIC Kernel — REST API Reference

## Base URL

```
http://<TAILSCALE_CORE>:3030   # Tailscale (production)
http://localhost:3030             # local development
```

## Authentication

All endpoints except `/health`, `/live`, `/ready`, `/metrics`, and `/events` require:
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
{ "status": "sovereign", "version": "v0.7.6", "phi_max": 0.618033988749895 }
```

**With valid Bearer token (extended):**

Additional fields: `axioms`, `dogs` (array of `{id, kind, circuit, failures}`), `storage`, `storage_namespace`, `storage_database`, `storage_metrics`, `embedding`, `crystals` (object: `{total, forming, crystallized, canonical, decaying, loop_active}`), `verdict_cache_size`, `background_tasks`, `total_requests`, `total_tokens`, `estimated_cost_usd`, `uptime_seconds`, `alerts`, `chain_verified`, `environment`.

| Status | Condition | HTTP |
|--------|-----------|------|
| `sovereign` | 2+ Dogs healthy | 200 |
| `degraded` | 1 Dog healthy | 503 |
| `critical` | 0 Dogs healthy | 503 |

### GET /live

Kubernetes/systemd liveness probe. Returns bare **200 OK** (no JSON body). No auth.

### GET /ready

Readiness probe. Returns **200 OK** if kernel can serve (≥1 healthy Dog + storage reachable), **503** otherwise. No JSON body. Caches DB ping for 30s to avoid hammering storage. No auth.

### GET /metrics

Prometheus-format metrics. No auth.

### GET /events

SSE event stream. No auth. Events: `verdict`, `crystal`, `dog_failed`, `session`, `backfill`, `anomaly`. Keepalive every 15s. Limited to 32 concurrent connections — returns **503** (plain text) when full.

---

## Judge

### POST /judge

Submit content for epistemic evaluation by independent AI validators (Dogs).

**Request:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | string | yes | Content to evaluate |
| `context` | string | no | Additional context |
| `domain` | string | no | Domain hint: `"chess"`, `"trading"`, `"code"`, `"token-analysis"`, `"wallet-judgment"`, `"general"` |
| `dogs` | string[] | no | Filter to specific Dog IDs |
| `crystals` | bool | no | Default `true`. Set `false` to disable crystal injection (A/B testing) |

**Response (200):**

| Field | Type | Description |
|-------|------|-------------|
| `verdict_id` | string (UUID) | Unique evaluation ID |
| `domain` | string | Evaluation domain (`"chess"`, `"trading"`, `"general"`, etc.) |
| `verdict` | `"Howl"` \| `"Wag"` \| `"Growl"` \| `"Bark"` | Overall verdict |
| `q_score` | QScore | Per-axiom scores + geometric mean total |
| `reasoning` | Reasoning | Per-axiom explanations |
| `dogs_used` | string | Contributing validators (`+`-separated) |
| `phi_max` | number | 0.618033988749895 |
| `dog_scores` | DogScore[] | Per-validator breakdown |
| `voter_count` | number | Number of Dogs that contributed (distinguishes single-Dog from consensus) |
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

### POST /judge (domain: wallet-judgment)

**Wallet Anti-Sybil Evaluation** — Fast-path deterministic verdict (no LLM Dogs).

Used to validate chess wallet authenticity for Personality Card minting. Dogs score on game history: archetype consistency, temporal distribution, Sybil risk markers.

**Request:**

```json
{
  "content": "<wallet_address>",
  "context": "{\"wallet_address\":\"<address>\",\"games_completed\":10,\"archetype_consistency\":0.80,\"wallet_age_days\":30,\"average_game_duration\":300,\"duration_variance\":0.20,\"opening_repertoire_hash\":\"...\",\"move_sequence_hash\":\"...\",\"suspicious_cluster\":false,\"replay_risk\":false}",
  "domain": "wallet-judgment"
}
```

**Context** (`WalletProfile` JSON, required):
- `wallet_address` (string): Solana wallet address
- `games_completed` (u32): Total games played
- `archetype_consistency` (f64, 0–1): Consistency of play style
- `wallet_age_days` (u32): Days since first game
- `average_game_duration` (u32): Median game duration in seconds
- `duration_variance` (f64): Coefficient of variation in game durations
- `opening_repertoire_hash` (string): Hash of opening sequence diversity
- `move_sequence_hash` (string): Hash of move patterns
- `suspicious_cluster` (bool): Circular funding / shared IP detected
- `replay_risk` (bool): Moves copied from other wallet

**Evaluation gates:**
1. `games_completed ≥ 5` — minimum samples for statistical confidence
2. `suspicious_cluster = false AND replay_risk = false` — no critical Sybil markers

**Response:** Same `JudgeResponse` schema. `dogs_used` will be `"wallet-deterministic-dog"` (no LLM latency).

**Example verdict (Wag):**
```json
{
  "verdict_id": "550e8400-e29b-41d4-a716-446655440000",
  "domain": "wallet-judgment",
  "verdict": "Wag",
  "q_score": {
    "total": 0.52,
    "fidelity": 0.55,
    "phi": 0.48,
    "verify": 0.50,
    "culture": 0.52,
    "burn": 0.50,
    "sovereignty": 0.54
  },
  "dogs_used": "wallet-deterministic-dog",
  "voter_count": 1,
  "dog_scores": [
    {
      "dog_id": "wallet-deterministic-dog",
      "latency_ms": 0,
      "prompt_tokens": 0,
      "completion_tokens": 0,
      "fidelity": 0.55,
      "phi": 0.48,
      "verify": 0.50,
      "culture": 0.52,
      "burn": 0.50,
      "sovereignty": 0.54,
      "reasoning": {
        "fidelity": "Archetype consistency 80% indicates authentic play style",
        "phi": "Time distribution harmonious with 30-day age and low variance",
        "verify": "Move history verifiable on-chain",
        "culture": "Multiple games across dated span shows engagement",
        "burn": "Active player with commitment proportional to wallet age",
        "sovereignty": "No Sybil coordination or replay evidence"
      }
    }
  ]
}
```

**Errors:**
- `400` if context is missing or invalid JSON: `"wallet-judgment requires valid WalletProfile JSON in context field"`
- `422` if context fails validation gates (e.g., `games_completed < 5`): returns `Bark` verdict

### POST /judge/async

Spawn a background judgment job and return immediately with a polling handle.

**Request:** same JSON body as `POST /judge`.

**Response (202):**

| Field | Type | Description |
|-------|------|-------------|
| `request_id` | string (UUID) | Polling identifier for this async job |
| `status` | `"pending"` | Initial async job state |
| `dogs_total` | number | Number of Dogs expected to respond after filter/context/health gating |

### GET /judge/status/{id}

Poll the progressive state of an async judgment job created by `POST /judge/async`.

**Response (200):**

| Field | Type | Description |
|-------|------|-------------|
| `request_id` | string (UUID) | Async job identifier |
| `status` | `"pending"` \| `"evaluating"` \| `"complete"` \| `"failed"` | Current job state |
| `dogs_total` | number | Number of Dogs expected to respond |
| `dogs_arrived` | DogArrival[] | Per-Dog progressive arrivals in completion order |
| `verdict` | JudgeResponse \| null | Present only when status is `"complete"` |
| `error` | string \| null | Present only when status is `"failed"` |

**DogArrival:** `dog_id` (string), `arrived_at_ms` (number), `success` (bool), `score` (DogScore, success only), `error` (string, failure only).

### GET /verdicts

List recent verdicts. Returns the 20 most recent verdict objects (limit hardcoded).

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
  ],
  "retired": {
    "count": 2,
    "total_tokens": 1234,
    "total_requests": 56
  }
}
```

### GET /audit

Query the MCP/REST audit trail. Query params: `limit` (default 50, max 100), `tool`, `agent_id`.

### POST /agent-tasks

Dispatch a task to the agent queue. Returns task_id.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `kind` | string (1-64) | yes | Task kind (e.g. "hermes", "nightshift") |
| `domain` | string (1-64) | yes | Domain (e.g. "social-signal") |
| `content` | string (1-10000) | yes | Task content/instructions |
| `agent_id` | string | no | Dispatching agent ID |

### GET /agent-tasks

Poll pending tasks. Query params: `kind` (default "hermes"), `limit` (default 10).

### POST /agent-tasks/{id}/result

Complete a task with result or error. Body: `{ "result": "...", "error": "..." }`.

### GET /state-history

Hash-chained organism state log. Returns state blocks ordered by sequence number.

| Param | Default | Description |
|-------|---------|-------------|
| `since` | all | RFC3339 timestamp filter |
| `limit` | 100 | Max blocks to return (max 1000) |

Response includes `chain_valid` (prev_hash linkage) and `blocks_valid` (SHA-256 verification). Each block contains Dog snapshots, system status, and resource metrics.

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

### POST /coord/heartbeat

Keep an agent session alive. Resets the TTL expiry timer.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent_id` | string (1-64) | yes | Agent to keep alive |

**Response (200):** `{ "status": "heartbeat_accepted", "agent_id": "..." }`

---

## Compliance

### GET /session/{agent_id}/compliance

Workflow compliance score for a specific agent session. Analyzes tool-use observations to detect rule violations (e.g., Edit without prior Read, bash retry loops).

**Response (200):**

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session identifier |
| `agent_id` | string | Agent identifier |
| `score` | number | Composite score 0.0–0.618 (φ-bounded) |
| `warnings` | string[] | Human-readable rule violation descriptions |
| `read_before_edit` | number | Ratio of Edits preceded by a Read of the same file |
| `bash_retry_violations` | number | Count of consecutive similar Bash failures (>2 = Rule 6 violation) |
| `files_modified` | number | Count of unique files modified (Edit/Write) |
| `created_at` | string | RFC3339 timestamp |

### GET /compliance

Compliance trend across recent sessions. Query param: `limit` (default 20, max 100). Returns array of `SessionCompliance` objects.

---

## System Info

### GET /dogs

Returns `string[]` — list of active Dog IDs (e.g. `["deterministic-dog", "gemini-flash", "qwen-7b-hf", "qwen35-9b-gpu", "gemma-4b-ubuntu"]`).

### POST /dogs/register

Register a new Dog at runtime via calibration challenge. Auth-gated.

**Request body:**
```json
{
  "name": "my-new-dog",
  "base_url": "http://host:8080/v1",
  "model": "my-model",
  "api_key": "optional-key",
  "context_size": 4096,
  "timeout_secs": 60
}
```

**Calibration:** The new Dog must evaluate a known stimulus and pass `validate_scores()` (no zero floods, no degenerate variance).

**Response:** `{ "dog_id": "my-new-dog", "calibration": "passed", "roster_size": 6 }`

**Errors:** `400` (bad config), `409` (name collision), `422` (calibration failed), `504` (calibration timeout).

### POST /dogs/{id}/heartbeat

Refresh TTL for a dynamically registered Dog. Must be called periodically (default TTL: 120s).

**Response 200:** `{ "dog_id": "...", "status": "alive", "ttl_remaining_secs": 120 }`
**Response 404:** Dog not registered — re-register required.

### DELETE /dogs/{id}

Remove a dynamically registered Dog from the roster. Config-based Dogs cannot be removed via API.

**Response 200:** `{ "dog_id": "...", "status": "deregistered", "roster_size": 4 }`
**Response 404:** Dog not found or not a registered Dog.

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
| `qwen-7b-hf` | Qwen 2.5 7B | HuggingFace Inference |
| `qwen35-9b-gpu` | Qwen 3.5 9B Q4 | Sovereign GPU (RTX 4060 Ti, 55 tok/s) |
| `gemma-4b-ubuntu` | Gemma 3 4B Q4 | Ubuntu CPU (13 tok/s) |

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
| 422 | Unprocessable entity (crystal observe quorum failure) |
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
  domain: string;
  verdict: VerdictKind;
  q_score: QScore;
  reasoning: Reasoning;
  dogs_used: string;
  phi_max: number;
  dog_scores: DogScore[];
  voter_count: number;
  anomaly_detected: boolean;
  max_disagreement: number;
  anomaly_axiom: string | null;
  integrity_hash?: string;
  prev_hash?: string;
  cache_hit?: number;
}

interface SessionCompliance {
  session_id: string;
  agent_id: string;
  score: number;
  warnings: string[];
  read_before_edit: number;
  bash_retry_violations: number;
  files_modified: number;
  created_at: string;
}

interface HealthResponse {
  status: 'sovereign' | 'degraded' | 'critical';
  version: string;
  phi_max: number;
  // Auth-only fields:
  axioms?: string[];
  dogs?: { id: string; kind: string; circuit: string; failures: number }[];
  storage?: string;
  storage_namespace?: string;
  storage_database?: string;
  storage_metrics?: { verdicts: number; crystals: number; observations: number };
  embedding?: string;
  crystals?: { total: number; forming: number; crystallized: number; canonical: number; decaying: number; loop_active: boolean };
  verdict_cache_size?: number;
  background_tasks?: Record<string, { status: string; last_seen?: string }>;
  total_requests?: number;
  total_tokens?: number;
  estimated_cost_usd?: number;
  uptime_seconds?: number;
  chain_verified?: boolean;
  environment?: Record<string, unknown>;
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
