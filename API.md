<!-- AI-CONTEXT
role: api-reference
audience: frontend developers + their AI coding assistants
purpose: complete REST API contract for the CYNIC kernel
source_of_truth: cynic-kernel/src/rest.rs
base_url: http://<TAILSCALE_UBUNTU>:3030
cors: all origins allowed
transport: HTTP/JSON
auth: none (open API)
-->

# CYNIC Kernel — REST API Reference

## Base URL

```
http://<TAILSCALE_UBUNTU>:3030   # via Tailscale (production)
http://localhost:3030       # if running kernel locally
```

CORS: all origins, all methods, all headers allowed.

---

## Endpoints

### GET /health

Returns system status. Use for connectivity check and health indicator.

**Request:** no parameters.

**Response (200):**

```json
{
  "status": "sovereign",
  "version": "0.1.0",
  "phi_max": 0.618033988749895,
  "axioms": ["FIDELITY", "PHI", "VERIFY/FALSIFY", "CULTURE", "BURN", "SOVEREIGNTY"]
}
```

| Field | Type | Description |
|-------|------|-------------|
| status | string | Always `"sovereign"` when healthy |
| version | string | Kernel version (semver) |
| phi_max | number | Golden ratio inverse — structural confidence ceiling |
| axioms | string[] | The 6 evaluation axioms |

---

### POST /judge

Submit content for epistemic evaluation. Returns a phi-bounded verdict from multiple independent validators (Dogs).

**Request:**

```json
{
  "content": "Democracy is the worst form of government except for all the others",
  "context": "Political philosophy quote by Churchill",
  "domain": "politics"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| content | string | yes | The text/claim to evaluate |
| context | string | no | Additional context for the evaluators |
| domain | string | no | Domain hint (e.g. "science", "chess", "politics") |

**Response (200):**

```json
{
  "verdict_id": "90fa5ed3-a92f-472c-96ae-6ca08f347346",
  "verdict": "Howl",
  "q_score": {
    "total": 0.6119,
    "fidelity": 0.618,
    "phi": 0.600,
    "verify": 0.618,
    "culture": 0.618,
    "burn": 0.618,
    "sovereignty": 0.600
  },
  "reasoning": {
    "fidelity": "Grounded in historical observation and political theory",
    "phi": "Concise paradoxical structure — balanced tension",
    "verify": "Can be debated and tested against political outcomes",
    "culture": "Widely recognized quote with deep cultural resonance",
    "burn": "Maximally efficient expression of a complex idea",
    "sovereignty": "Defends democratic agency while acknowledging imperfection"
  },
  "dogs_used": "deterministic-dog+gemini",
  "phi_max": 0.618033988749895,
  "dog_scores": [
    {
      "dog_id": "deterministic-dog",
      "fidelity": 0.35,
      "phi": 0.40,
      "verify": 0.35,
      "culture": 0.40,
      "burn": 0.45,
      "sovereignty": 0.40,
      "reasoning": {
        "fidelity": "Heuristic: len=72, absolutes=false",
        "phi": "Heuristic: len=72, structured=true",
        "verify": "Heuristic: evidence_words=false",
        "culture": "Heuristic: tradition_refs=false",
        "burn": "Heuristic: len=72, concise=true",
        "sovereignty": "Heuristic: coercive=false"
      }
    },
    {
      "dog_id": "gemini",
      "fidelity": 0.90,
      "phi": 0.80,
      "verify": 0.90,
      "culture": 0.95,
      "burn": 0.92,
      "sovereignty": 0.80,
      "reasoning": {
        "fidelity": "Grounded in historical observation and political theory",
        "phi": "Concise paradoxical structure — balanced tension",
        "verify": "Can be debated and tested against political outcomes",
        "culture": "Widely recognized quote with deep cultural resonance",
        "burn": "Maximally efficient expression of a complex idea",
        "sovereignty": "Defends democratic agency while acknowledging imperfection"
      }
    }
  ],
  "anomaly_detected": false,
  "max_disagreement": 0.2217,
  "anomaly_axiom": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| verdict_id | string (UUID) | Unique identifier for this evaluation |
| verdict | `"Howl"` \| `"Wag"` \| `"Growl"` \| `"Bark"` | Overall verdict |
| q_score | QScore | Phi-bounded scores per axiom + geometric mean total |
| q_score.total | number (0.0–0.618) | Geometric mean of all axiom scores, phi-bounded |
| reasoning | Reasoning | Human-readable explanation per axiom |
| dogs_used | string | Which validators contributed ("+"-separated) |
| phi_max | number | 0.618033988749895 — structural ceiling |
| dog_scores | DogScore[] | Per-validator raw scores and reasoning |
| anomaly_detected | boolean | True when Dogs significantly disagree |
| max_disagreement | number | Largest score gap between any two Dogs on any axiom |
| anomaly_axiom | string \| null | Which axiom has the disagreement (null if no anomaly) |

**Verdict thresholds (applied to q_score.total):**

| Verdict | Threshold | Meaning |
|---------|-----------|---------|
| Howl | >= 0.5207 (phi * 0.82) | Highest epistemic quality |
| Wag | >= 0.382 (phi^-2) | Good quality |
| Growl | >= 0.236 (phi^-3) | Questionable |
| Bark | < 0.236 | Rejected |

---

### GET /verdicts

List recent verdicts. Maximum 20 returned. Requires SurrealDB running.

**Request:** no parameters.

**Response (200):** Array of Verdict objects (same shape as POST /judge response).

```json
[
  { "verdict_id": "...", "verdict": "Howl", "q_score": {...}, ... },
  { "verdict_id": "...", "verdict": "Bark", "q_score": {...}, ... }
]
```

**Response when SurrealDB unavailable (500):**

```json
{ "error": "storage unavailable" }
```

---

### GET /verdict/{id}

Get a specific verdict by its UUID.

**Request:** `id` as URL path parameter.

**Response (200):** Single Verdict object.

**Response (404):**

```json
{ "error": "verdict not found" }
```

---

## Domain Model

### The 6 Axioms

| Axiom | Code Key | Measures | Score Range |
|-------|----------|----------|-------------|
| FIDELITY | `fidelity` | Truth loyalty — is this grounded in reality? | 0.0–1.0 (raw), 0.0–0.618 (bounded) |
| PHI | `phi` | Structural harmony — golden ratio proportions | 0.0–1.0 (raw), 0.0–0.618 (bounded) |
| VERIFY/FALSIFY | `verify` | Falsifiability — can this be proven wrong? | 0.0–1.0 (raw), 0.0–0.618 (bounded) |
| CULTURE | `culture` | Pattern continuity — alignment with traditions | 0.0–1.0 (raw), 0.0–0.618 (bounded) |
| BURN | `burn` | Simplicity — no waste, maximum efficiency | 0.0–1.0 (raw), 0.0–0.618 (bounded) |
| SOVEREIGNTY | `sovereignty` | Agency — does this respect individual freedom? | 0.0–1.0 (raw), 0.0–0.618 (bounded) |

### Dogs (Independent Validators)

Dogs evaluate content independently (double-blind). Their scores are merged into the final verdict.

| Dog ID | Type | Latency | Network Required |
|--------|------|---------|-----------------|
| `deterministic-dog` | Rule-based heuristics | <1ms | No |
| `gemini` | Gemini 2.5 Flash AI | 2–5 sec | Yes |

### Anomaly Detection

When `anomaly_detected: true`, the Dogs disagree significantly on at least one axiom. This is a **discovery signal** — it means the content is epistemically interesting. The `anomaly_axiom` field identifies which axiom has the largest disagreement.

### Phi-Bounding

**No score can exceed phi^-1 = 0.618033988749895.** This is a structural epistemic limit (golden ratio inverse). It encodes the principle that no claim deserves absolute confidence. This is not a bug — it is the core design.

---

## TypeScript Interfaces

```typescript
interface JudgeRequest {
  content: string;
  context?: string;
  domain?: string;
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
}

interface HealthResponse {
  status: string;
  version: string;
  phi_max: number;
  axioms: string[];
}
```

---

## Quick Test Commands

### From PowerShell (Windows)

```powershell
# Health check
curl.exe http://<TAILSCALE_UBUNTU>:3030/health

# Submit evaluation
curl.exe -X POST http://<TAILSCALE_UBUNTU>:3030/judge `
  -H "Content-Type: application/json" `
  -d '{\"content\":\"The earth orbits the sun\",\"domain\":\"science\"}'

# List verdicts
curl.exe http://<TAILSCALE_UBUNTU>:3030/verdicts
```

### From bash (Linux/Mac)

```bash
# Health check
curl -s http://<TAILSCALE_UBUNTU>:3030/health | jq

# Submit evaluation
curl -s -X POST http://<TAILSCALE_UBUNTU>:3030/judge \
  -H "Content-Type: application/json" \
  -d '{"content":"The earth orbits the sun","domain":"science"}' | jq

# List verdicts
curl -s http://<TAILSCALE_UBUNTU>:3030/verdicts | jq
```
