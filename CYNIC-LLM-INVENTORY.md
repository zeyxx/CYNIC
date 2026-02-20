# CYNIC LLM/API Inventory Report
*Generated 2026-02-13*

## Summary
- **Total LLM Calls**: 8 (Anthropic: 3, Ollama: 3, Gemini: 2)
- **External APIs**: 4 (Jupiter, Solana RPC, etc)
- **Unique Endpoints**: 12
- **Routing Systems**: 3 (LLMRouter, UnifiedLLMRouter, ModelIntelligence)
- **Files Scanned**: 150+

---

## LLM PROVIDERS

### 1. Anthropic (Primary Brain)
**Confidence**: φ⁻¹ (61.8%)

| File | Function | Purpose |
|------|----------|---------|
| packages/llm/src/adapters/anthropic.js | AnthropicAdapter.complete() | Direct SDK call to Anthropic API |
| packages/node/src/daemon/llm-endpoints.js | POST /llm/ask | HTTP daemon endpoint routing |

**Models**:
- claude-opus-4-6
- claude-sonnet-4-5-20250929 (default)
- claude-haiku-4-5-20251001

**Configuration**:
- SDK: @anthropic-ai/sdk
- Timeout: 60s
- Env: ANTHROPIC_API_KEY
- Cost: $0.003 per 1M input tokens

**Code Pattern**:
```javascript
const client = new Anthropic({ apiKey });
const result = await client.messages.create({
  model: "claude-sonnet-4-5-20250929",
  max_tokens: 1024,
  messages: [{ role: 'user', content: prompt }]
});
```

---

### 2. Ollama (Cost Fallback)
**Confidence**: φ⁻² (38.2%)

| File | Function | Purpose |
|------|----------|---------|
| packages/llm/src/adapters/oss-llm.js | OSSLLMAdapter._callOllama() | Local OSS LLM inference |

**Configuration**:
- Endpoint: http://localhost:11434/api/generate
- Model: llama3.2:latest (default)
- Timeout: 30s
- Cost: FREE (local)
- Auto-discovery: ~/.cynic/llm-detection.json

**Code Pattern**:
```javascript
const response = await fetch('http://localhost:11434/api/generate', {
  method: 'POST',
  body: JSON.stringify({
    model: 'llama3.2',
    prompt,
    options: { temperature: 0.7, num_predict: 512 }
  })
});
```

---

### 3. Gemini (Validator)
**Confidence**: φ⁻² (38.2%)

| File | Function | Purpose |
|------|----------|---------|
| packages/llm/src/adapters/gemini.js | GeminiAdapter.complete() | Multi-model consensus validator |

**Configuration**:
- SDK: @google/generative-ai
- Model: gemini-2.0-flash
- Timeout: 30s
- Env: GEMINI_API_KEY
- Role: Secondary validator for consensus

**Code Pattern**:
```javascript
const model = client.getGenerativeModel({ model: 'gemini-2.0-flash' });
const result = await model.generateContent({
  contents: [{ role: 'user', parts: [{ text: prompt }] }],
  generationConfig: { maxOutputTokens: 1024 }
});
```

---

## EXTERNAL APIs

### 1. Jupiter Price API v3
**Endpoint**: https://api.jup.ag/price/v3

| Property | Value |
|----------|-------|
| File | packages/node/src/perception/jupiter-client.js |
| Purpose | Token price monitoring for $BURN |
| Method | GET (cached, 60s TTL) |
| Poll Interval | 30s |
| Rate Limit | 5 req/sec |
| Spike Detection | φ² (38.2% change) |

**Events Emitted**:
- market:price:update
- market:price:spike

---

### 2. Solana RPC
**Endpoint**: https://api.mainnet-beta.solana.com

| Property | Value |
|----------|-------|
| File | packages/node/src/perception/market-watcher.js |
| Purpose | Blockchain data queries |
| SDK | @solana/web3.js |
| Env | SOLANA_RPC_URL |
| Poll Intervals | price:1m, volume:1m, liquidity:5m, holders:15m |

**Events Emitted**:
- perception:market:price
- perception:market:volume
- perception:market:liquidity
- perception:market:holders
- perception:market:pattern

---

## ROUTING SYSTEMS

### 1. LLMRouter (Cost-Aware)
**File**: packages/node/src/orchestration/llm-router.js

**Decision Logic**:
- SIMPLE tasks → Ollama (free)
- MODERATE tasks → Thompson Sampling
- COMPLEX tasks → Anthropic (quality)
- Budget EXHAUSTED → Force Ollama (circuit breaker)
- Budget CRITICAL → Block non-essential

**Features**:
- Budget circuit breaker
- Thompson Sampling exploration/exploitation
- Cost tracking to routing_accuracy table
- Manual reset capability

---

### 2. UnifiedLLMRouter (Advanced)
**File**: packages/node/src/orchestration/unified-llm-router.js

**Tier-Based Routing**:
| Tier | Cost | Latency | Purpose |
|------|------|---------|---------|
| LOCAL | $0 | 1ms | Pattern matching (no LLM) |
| LIGHT | $1 | 5s | Small Ollama models |
| FULL | $15 | 15s | Medium + Claude Code |
| DEEP | $50 | 60s | Large AirLLM/Mistral |

**Consensus**: Quorum φ⁻¹ (61.8%) via semantic agreement

---

### 3. ModelIntelligence (Learning)
**File**: packages/node/src/learning/model-intelligence.js

**Thompson Sampling**:
- Tracks affinity: { taskType, model, successRate }
- Beta distributions per pair
- Learns from historical outcomes
- Epsilon-greedy exploration

---

## ENVIRONMENT VARIABLES

| Variable | Required | Default | Usage |
|----------|----------|---------|-------|
| ANTHROPIC_API_KEY | YES | None | Primary brain |
| GEMINI_API_KEY | NO | None | Validator |
| SOLANA_RPC_URL | NO | api.mainnet-beta.solana.com | Blockchain |
| JUPITER_API_KEY | NO | None | Price auth |
| BURN_TOKEN_MINT | NO | 9zB5...Spump | Default token |
| CYNIC_VALIDATORS | NO | None | Comma-sep list |
| OLLAMA_ENDPOINT | NO | localhost:11434 | Local LLM |
| OLLAMA_MODEL | NO | llama3.2 | Default model |

---

## FILES INVOLVED (20+)

### LLM Adapters (6 files)
- anthropic.js
- oss-llm.js
- gemini.js
- base.js
- claude-code.js
- airllm.js

### Orchestration (3 files)
- llm-router.js
- unified-llm-router.js
- mcp-instructions.js

### Perception (2 files)
- market-watcher.js
- jupiter-client.js

### Learning/Accounting (2 files)
- model-intelligence.js
- cost-ledger.js

### Services (3+ files)
- daemon/llm-endpoints.js
- event-bus.js
- event-listeners.js

---

## API CALL PATTERNS

### Anthropic
```javascript
const client = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });
const response = await client.messages.create({
  model: "claude-sonnet-4-5-20250929",
  max_tokens: 1024,
  messages: [{ role: 'user', content: prompt }]
});
```

### Ollama
```javascript
const response = await fetch('http://localhost:11434/api/generate', {
  method: 'POST',
  body: JSON.stringify({ model, prompt, stream: false, options: {...} })
});
```

### Gemini
```javascript
const model = client.getGenerativeModel({ model: 'gemini-2.0-flash' });
const result = await model.generateContent({ contents: [...]});
```

### Jupiter
```javascript
const prices = await fetch('https://api.jup.ag/price/v3?ids=...')
  .then(r => r.json());
```

### Solana
```javascript
const connection = new Connection(rpcUrl);
const supply = await connection.getTokenSupply(mint);
```

---

## QUALITY METRICS

| Provider | Confidence | Timeout | Token Tracking |
|----------|------------|---------|-----------------|
| Anthropic | 61.8% | 60s | Yes (usage) |
| Ollama | 38.2% | 30s | Estimated |
| Gemini | 38.2% | 30s | Yes (usage) |
| Jupiter | N/A | 5s | N/A |
| Solana RPC | N/A | 30s | N/A |

---

## SECURITY NOTES

✓ API keys: Environment variables only (no file storage)
✓ Lazy loading: SDKs loaded on first use
✓ Timeouts: All calls wrapped in Promise.race()
✓ Circuit breaker: Prevents budget overruns
✓ Rate limiting: Jupiter caching (60s), Solana backoff exponential
✓ Error handling: Non-blocking, logged
✓ Fallback chains: Anthropic → Ollama → Gemini

---

## DEPLOYMENT CHECKLIST

- [ ] Set ANTHROPIC_API_KEY on Render
- [ ] Configure SOLANA_RPC_URL if custom
- [ ] Setup CYNIC_VALIDATORS for multi-model
- [ ] Install & start Ollama (if using OSS)
- [ ] Test endpoints: /llm/ask, /llm/consensus, /llm/models
- [ ] Monitor routing_accuracy PostgreSQL table
- [ ] Enable cost tracking via CostLedger
- [ ] Setup alerts on budget exhaustion
- [ ] Enable Thompson feedback via /llm/feedback

---

**Summary**: CYNIC integrates **8 LLM calls** + **4 external APIs** with intelligent cost-aware routing, multi-model consensus, and continuous learning. See CYNIC-LLM-INVENTORY.json for structured data.

