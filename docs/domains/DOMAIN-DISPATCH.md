# Domain Dispatch Guide (May 3 Launch)

**How CYNIC routes /judge requests to the right Dogs via domain hints**

---

## Quick Start

### Request Format

```bash
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Token X just burned liquidity",
    "domain": "token",
    "crystals": true
  }'
```

### Domain Values (Canonical)

| Domain | Aliases | Dogs | Use Case |
|--------|---------|------|----------|
| `token` | `d1`, `solana` | deterministic-dog, qwen-7b, qwen-9b-core, qwen-9b-gpu | Solana token rug detection, launch patterns, authenticity |
| `security` | `d4`, `scam`, `scams` | qwen-7b, qwen-9b-core, qwen-9b-gpu | Exploit patterns, social engineering, honeypot detection |
| `macro` | `d5`, `politics`, `geopolitical` | qwen-7b, qwen-9b-core, qwen-9b-gpu | Regulatory impact, geopolitical risk, macro cycle alignment |
| `chess` | `chess-personality`, `game` | deterministic-dog | Personality card validation, archetype consistency |
| `wallet` | `wallet-judgment`, `sybil`, `anti-sybil` | deterministic-dog | Anti-Sybil judgment, game authenticity, archetype consistency |
| `inference` | `d2`, `llm` | (kernel-embedded, no external Dogs) | Model selection, VRAM math, latency estimation |
| `sovereignty` | `d3`, `sovereign` | deterministic-dog (STUB) | Infrastructure independence, epistemic authority |
| `philosophy` | `d6`, `epistemology` | deterministic-dog (STUB) | Calibration, confidence bounds, falsification signals |

---

## How Routing Works

### The Golden Rule: 1 Dog = 1 SOT

**Every Dog encapsulates exactly one Source of Truth (SOT). LLM Dogs are the last resort.** 
Before routing to an LLM Dog, the pipeline MUST exhaust deterministic Dogs:
1. **Database Dogs:** Local Cache, SQL validations (Absolute SOT).
2. **On-Chain Dogs:** Transaction history, liquidity locks, verified contract code (SOT).
3. **Deterministic Heuristic Dogs:** Profile matching, hardcoded anti-sybil checks.

If and only if the deterministic Dogs are inconclusive or the domain requires nuance, the pipeline routes to LLM Dogs. (e.g. `wallet-judgment` is fully handled by the `deterministic-dog`; `token-judgment` uses `rug-prefilter` dog before waking up `qwen-7b`).

### Priority Chain

1. **Explicit dog list takes priority**
   ```json
   {"content": "...", "dogs": ["deterministic-dog", "qwen-7b-hf"]}
   // Uses specified dogs, ignores domain
   ```

2. **Domain routing** (if no explicit dogs)
   ```json
   {"content": "token X...", "domain": "token"}
   // Routes to: deterministic-dog, qwen-7b, qwen-9b-core, qwen-9b-gpu
   ```

3. **"sovereign" preset** (backwards compat)
   ```json
   {"content": "...", "dogs": ["sovereign"]}
   // Expands to: all dogs marked is_sovereign() = true
   ```

4. **All dogs** (fallback if no domain, no dogs)
   ```json
   {"content": "...", "domain": "unknown"}
   // Uses: all available Dogs
   ```

---

## Domain Context Injection

When you specify a domain **without** explicit context, CYNIC injects domain-specific guidance into Dog prompts:

### Example: Token Domain

**Request:**
```json
{
  "content": "Token $BEDROCK just burned liquidity and locked deployer wallet",
  "domain": "token"
}
```

**Injected context** (auto-prepended to Dog prompts):
```
[DOMAIN: token-judgment]
Evaluate for: launch legitimacy, liquidity authenticity, rugpull indicators, team credibility.
```

### Example: Security Domain

**Request:**
```json
{
  "content": "New launchpad claims zero-day exploit in SVM",
  "domain": "security"
}
```

**Injected context:**
```
[DOMAIN: security-judgment]
Evaluate for: exploit patterns, social engineering risk, honeypot indicators, scam sophistication.
```

---

## May 3 Launch: Experiment (No Domain Required)

For the May 3-10 experiment, domain is **optional**. If you don't specify a domain:

1. CYNIC defaults to using all Dogs
2. No domain-specific context is injected
3. Verdicts are domain-agnostic (may be noisier)

**Falsification test:** Specify domain on Day 3+ to measure if Dogs improve when routed by domain.

```bash
# May 3-4: No domain (baseline)
curl http://localhost:3030/judge \
  -d '{"content": "Token X analysis", "crystals": true}'

# May 5+: With domain (if you want to test routing)
curl http://localhost:3030/judge \
  -d '{"content": "Token X analysis", "domain": "token", "crystals": true}'
```

---

## API Response Format

All /judge responses include the domain used:

```json
{
  "verdict": {
    "q_score": 0.18,
    "verdict_type": "BARK",
    "domain_hint": "token",
    "dogs_used": [
      "deterministic-dog",
      "qwen-7b-hf"
    ],
    ...
  }
}
```

- **domain_hint**: Echo of the domain you specified (or "unknown" if none)
- **dogs_used**: Actual Dogs that voted (subset of routed set)

---

## Gaps & Stubs

| Domain | Status | Note |
|--------|--------|------|
| Token | ✓ LIVE | Full Dogs, live routing |
| Security | ✓ READY | Dogs assigned, needs routing test |
| Macro | ✓ READY | Dogs assigned, needs routing test |
| Chess | ✓ LIVE | Full Dogs, live routing (B&C integration) |
| Wallet | ✓ LIVE | Full Dogs, live routing (B&C integration) |
| Inference | ✓ LIVE | Kernel-embedded, no external Dogs |
| Sovereignty (D3) | ⚠️ STUB | deterministic-dog placeholder, no axiom source yet |
| Philosophy (D6) | ⚠️ STUB | deterministic-dog placeholder, axiom calibration TBD |

---

## Testing Domain Routing (Before May 3)

### Sanity Test 0: Verify Routing is Live in /health

Before running /judge tests, verify routing is exposed in the kernel:

```bash
# Check that domain_routing section appears in health response
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/health | jq '.domain_routing | keys'

# Expected output:
# [
#   "chess",
#   "inference",
#   "macro",
#   "philosophy",
#   "security",
#   "sovereignty",
#   "token",
#   "wallet"
# ]

# Verify token domain is routed to 4 Dogs:
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/health | jq '.domain_routing.token.dogs'

# Expected:
# [
#   "deterministic-dog",
#   "qwen-7b-hf",
#   "qwen-9b-core",
#   "qwen-9b-gpu"
# ]
```

### Sanity Test 1: Token Domain (/judge)

```bash
# Should route to 4 Dogs: deterministic-dog, qwen-7b-hf, qwen-9b-core, qwen-9b-gpu
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Token X launched with zero liquidity lock and deployer has history of rugs",
    "domain": "token"
  }' | jq '{domain_hint: .verdict.domain_hint, dogs_used: .verdict.dogs_used | sort, q_score: .verdict.q_score}'

# Expected output:
# {
#   "domain_hint": "token",
#   "dogs_used": [
#     "deterministic-dog",
#     "qwen-7b-hf",
#     "qwen-9b-core",
#     "qwen-9b-gpu"
#   ],
#   "q_score": <number between 0 and 1>
# }
```

### Sanity Test 2: Chess Domain (/judge)

```bash
# Should route to 1 Dog: deterministic-dog
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "{\"wallet\": \"abc123\", \"games_played\": 7, \"archetype\": \"sicilian_specialist\"}",
    "domain": "chess"
  }' | jq '{domain_hint: .verdict.domain_hint, dogs_used: .verdict.dogs_used, q_score: .verdict.q_score}'

# Expected output (deterministic-dog only):
# {
#   "domain_hint": "chess",
#   "dogs_used": ["deterministic-dog"],
#   "q_score": <number between 0 and 1>
# }
```

### Sanity Test 3: Security Domain (/judge)

```bash
# Should route to 3 Dogs: qwen-7b-hf, qwen-9b-core, qwen-9b-gpu
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Honeypot smart contract that looks legitimate but front-runs all transactions",
    "domain": "security"
  }' | jq '{domain_hint: .verdict.domain_hint, dogs_used: .verdict.dogs_used | sort, q_score: .verdict.q_score}'

# Expected output (no deterministic-dog for security):
# {
#   "domain_hint": "security",
#   "dogs_used": [
#     "qwen-7b-hf",
#     "qwen-9b-core",
#     "qwen-9b-gpu"
#   ],
#   "q_score": <number between 0 and 1>
# }
```

### Sanity Test 4: Wallet Domain (/judge)

```bash
# Should route to 1 Dog: deterministic-dog
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "{\"wallet\": \"def456\", \"games_won\": 142, \"archetype_consistency\": 0.89}",
    "domain": "wallet"
  }' | jq '{domain_hint: .verdict.domain_hint, dogs_used: .verdict.dogs_used, q_score: .verdict.q_score}'

# Expected output (deterministic-dog only):
# {
#   "domain_hint": "wallet",
#   "dogs_used": ["deterministic-dog"],
#   "q_score": <number between 0 and 1>
# }
```

### Sanity Test 5: Domain-Agnostic Fallback (Unknown Domain)

```bash
# Explicitly pass "unknown" domain → should use default fallback
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Some arbitrary analysis",
    "domain": "unknown"
  }' | jq '{domain_hint: .verdict.domain_hint, dogs_used: .verdict.dogs_used | sort}'

# Expected output (fallback: deterministic + one LLM):
# {
#   "domain_hint": "unknown",
#   "dogs_used": [
#     "deterministic-dog",
#     "qwen-7b-hf"
#   ]
# }
```

### Sanity Test 6: Alias Resolution (domain="d1" → Token)

```bash
# Alias "d1" should resolve to Token domain
curl -X POST http://localhost:3030/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Token $ABC just locked liquidity for 1 year",
    "domain": "d1"
  }' | jq '{domain_hint: .verdict.domain_hint, dogs_used_count: (.verdict.dogs_used | length)}'

# Expected output (domain_hint=token since d1 parses to Token):
# {
#   "domain_hint": "token",
#   "dogs_used_count": 4
# }
```

---

## For May 3 Experiment

**Personal T. + @CynicOracle Coherence Test:**

1. Personal T. posts observation (e.g., "Token X shows 3 bundler patterns")
2. @CynicOracle posts verdict:
   ```bash
   curl -X POST http://localhost:3030/judge \
     -d '{
       "content": "Token X launched 2h ago, bundler MEV active, deployer known for rugs",
       "domain": "token",
       "crystals": true
     }'
   ```
3. Log verdict in OBSERVATION_LOG.md (domain + q_score)
4. Day 10: Check if verdict held in Week 2 data

---

**Ready?** Routing is live. Check /health to verify Dogs per domain are exposed.

---

Last updated: 2026-04-30 17:30  
Status: Ready for May 3 launch  
Commit: (see cynic-kernel/src/api/rest/judge.rs)
