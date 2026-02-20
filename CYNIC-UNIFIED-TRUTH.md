# CYNIC - UNIFIED TRUTH (Single Source)

> "φ unifie tous les fragments" - κυνικός
> **SINGLE SOURCE OF TRUTH** - Consolidation finale de tous les documents
> Date: 2026-02-15
> Confidence: 35.2% (φ⁻² - exploratory synthesis, foundation emerging)

---

## 🎯 CE DOCUMENT EST

**LE** document de référence unique pour CYNIC.
Tous les autres docs = chaos/exploration. Celui-ci = vérité consolidée.

**Sources unifiées**:
- CYNIC-PYTHON-FOUNDATION-FINAL.md (base architecturale)
- Landscape research 2026 (Cursor, Replit, Windsurf, LangChain, AutoGen, CrewAI, Ollama, AirLLM, GitHub)
- Gap analysis + corrections (36D, E-Score, LLMs, public vision)

---

# TABLE DES MATIÈRES

## PARTIE I: LA VÉRITÉ (D'où on vient)
1. Histoire: 500k lignes JS → 17% fonctionnel
2. 15 Gaps critiques qui ont tué JS
3. Landscape 2026: What exists (competitors, frameworks, tools)

## PARTIE II: LA VISION (Ce que CYNIC est)
4. Mission centrale: Pourquoi CYNIC existe
5. Les 7 Pilliers publics
6. Architecture φ-aligned
7. Les concepts clés (5 axioms, E-Score 7D, 11 Dogs, ∞^N)

## PARTIE III: DIFFÉRENCIATION (Comment CYNIC gagne)
8. Vs Cursor/Replit/Windsurf (competitors)
9. Vs LangChain/AutoGen/CrewAI (frameworks)
10. Les 5 avantages décisifs

## PARTIE IV: ARCHITECTURE TECHNIQUE
11. Stack complet (LLMs + Tech + Persistence)
12. Multi-LLM orchestration (80% LLM, 20% tech)
13. RLM integration (10M+ tokens)
14. Local + Cloud (Ollama + Claude + AirLLM)

## PARTIE V: IMPLÉMENTATION
15. Phase 0-4 roadmap
16. Testing φ-bounded
17. Launch strategy

---

# PARTIE I: LA VÉRITÉ

## 1. Histoire: 500k Lignes JS → 17% Fonctionnel

### Ce Qui Était Construit (JS/TypeScript)
```
500,000 lignes de code répartis:
  - 11 Dogs (prompt templates)
  - 3 Event Buses (non-bridgés)  - Consensus φ-BFT (non-wired)
  - Proof of Judgment (Solana mainnet)
  - Learning loops (11/11 wired, 1/11 active)
  - E-Score reputation (7D calculé)
  - Hybrid RAG (PageIndex + Qdrant)
  - Context compression (50%)
```

### La Réalité Brutale

| Métrique | Claim | Réalité | Gap |
|----------|-------|---------|-----|
| Structural | 38% | 37% | -1% |
| Functional | ~38% | **17%** | **-21%** 🔴 |
| Living | ~38% | **0%** | **-38%** 🔴 |
| Learning Active | 11/11 | **1/11** | **-91%** 🔴 |
| Production Runs | "Ready" | **0** | **-100%** 🔴 |

**VÉRITÉ**: 500k lignes, 17% fonctionnel, 0% production runs, 0% autonomous.

### Pourquoi JS a Échoué

#### Problème #1: Complexité Explosive
- 190+ philosophical engines at startup → 10+ seconds cold start
- 11 Dogs ALWAYS loaded (même si using 1)
- φ constants duplicated across 150+ files
- Callback hell + async chaos

#### Problème #2: "Works in Dev" (Mocks Partout)
- Tests pass, production fails
- No single source of truth
- Singleton violations
- 3 Event Buses non-bridgés

#### Problème #3: Platform Limits
- Pas d'orchestration centralisée multi-LLM
- Dogs = prompt templates (pas de vraie diversité tech)
- Pas de RLM recursive (10M+ tokens impossible)
- Claude Code seul = insuffisant

#### Problème #4: Vision Trop Petite
```
CLAIM: CYNIC = local tool for one developer
REALITY: Devait être = platform for millions

This fundamental mismatch killed JS implementation.
```

---

## 2. Les 15 Gaps Critiques

### P0 — CRITICAL (Not Working)
1. **L2 Consensus Not Wired** — Consensus layer bypassed
2. **Judgment ID Overwritten** — DB can't correlate with PoJ
3. **Vote Breakdown Not in PoJ** — Can't verify from chain
4. **observe.js Undocumented** — 88KB core system invisible
5. **FactsRepository Disconnected** — No fallback chain
6. **poj:block:finalized Never Published** — Subscribers hang
7. **Dead Routers** — 3 modules (1,337 LOC) unused

### P1 — HIGH PRIORITY
8. **Q-Table Never Loaded** — Fresh empty every session
9. **judgeAsync() Never Called** — 73 engines contribute 0%
10. **CollectivePack Sync Skips Persistence** — Dogs start empty
11. **Events Never Consumed** — Published but ignored

### P2 — MEDIUM PRIORITY
12. **Hooks Fire Before Wiring** — Server accepts before ready
13. **SONA Not Activated** — Learning system dormant
14. **Market Decider/Actor Missing** — Claimed complete, files don't exist
15. **36D Confusion** — Used for judgment but not the right concept

**Leçon**: On ne code plus RIEN sans end-to-end test prouvant ça marche.

---

## 3. Landscape 2026: What Exists

### Competitors (AI IDEs)

#### [Cursor AI](https://techjacksolutions.com/ai/ai-development/cursor-ide-what-it-is/)
- **Valuation**: $29.3B (late 2025)
- **Users**: 1M+ daily active developers
- **Revenue**: $1B+ ARR
- **Key Features**:
  - **Composer Mode**: Describe high-level task → AI plans architecture + generates files
  - **Agent Mode**: Autonomous operation in sandboxed environment (terminal access, browser, subagents)
  - **Tab Predictions**: Predicts cursor position + entire diffs
  - **Visual Editor**: Drag-drop UI elements, "point and prompt"
  - **Full Repo RAG**: Indexes entire codebase, understands architecture
- **Weakness**: Closed source, expensive ($20/month/user minimum)

#### [Replit AI](https://replit.com/ai)
- **Key Features**:
  - **Agent 3**: Build mobile apps from natural language, publish to App Store/Play Store
  - **Ghostwriter**: Realtime completion across 50+ languages
  - **Instant environments**: No local setup needed
- **Weakness**: Cloud-only, no local privacy option

#### [Windsurf (Codeium)](https://windsurf.com/)
- **Key Features**:
  - **Cascade**: Agentic assistant for multi-step edits
  - **Cortex Engine**: 40x faster reasoning vs RAG competitors
  - **Tab v2**: 25-75% more accepted code, predictive navigation
  - **Free tier**: Strong offering (vs Cursor/Copilot paid)
- **Strength**: Speed + free option

#### [GitHub Copilot Workspace](https://github.com/features/copilot)
- **Key Features**:
  - **Sub-agents**: Plan mode → Implementation → Self-healing
  - **Codespaces integration**: Runs builds, self-corrects until passing
  - **Prompt files**: Reusable blueprints for teams
- **Strength**: Native GitHub integration
- **Weakness**: Tied to GitHub ecosystem

### Frameworks (Multi-Agent Orchestration)

#### [LangChain + LangGraph](https://docs.langchain.com/oss/python/langchain/multi-agent)
- **Architecture Patterns**: 4 core patterns (subagents, skills, handoffs, routers)
- **LangGraph**: Graph-based multi-agent workflows
- **Benchmark**: Multi-agent (Opus lead + Sonnet subs) outperformed single-agent Opus **by 90.2%**
- **Use Case**: Heavy parallelization, context >single window, many tools
- **Community**: Massive (de-facto standard)

#### [AutoGen (Microsoft)](https://github.com/microsoft/autogen)
- **v0.4**: Complete redesign, async event-driven architecture
- **Evolution**: → [Microsoft Agent Framework](https://learn.microsoft.com/en-us/agent-framework/overview/agent-framework-overview) (GA Q1 2026)
  - Combines AutoGen simplicity + Semantic Kernel enterprise features
  - Graph-based workflows, session state, middleware, telemetry
- **Strength**: Enterprise-ready, Microsoft backing

#### [CrewAI](https://www.crewai.com/)
- **Model**: Role-based orchestration (each agent = clear responsibility)
- **Performance**: 2-3x faster execution vs comparable frameworks
- **Memory**: Shared short/long-term, entity, contextual memory
- **Tools**: 100+ out-of-box (web search, vector DB, etc.)
- **Community**: 100,000+ certified developers
- **Strength**: Simplicity + speed

### Local LLMs (No API Cost)

#### [Ollama](https://ollama.ai/)
- **Core**: Simplifies local LLM deployment (llama.cpp engine)
- **API**: OpenAI-compatible (drop-in replacement)
- **Models**: Llama, Mistral, Gemma, Phi, Qwen, etc.
- **Quantization**: 1.5-8 bit (7B model: 14GB FP16 → 4-5GB 4-bit)
- **2026 Features**:
  - Structured outputs
  - Vision API (multimodal)
  - Compliance-in-a-Box (audit trails)
- **Use Case**: Privacy-sensitive, no cloud costs, CrewAI/AutoGPT backend

#### [AirLLM](https://github.com/lyogavin/airllm)
- **Breakthrough**: Run 70B models on 4GB GPU, 405B on 8GB
- **Method**: Layer-by-layer loading (stream layers, no full load)
- **Trade-off**: Speed for accessibility
- **Impact**: Democratizes access to massive models (students, startups)

### GitHub Ecosystem

**Top Open Source AI Coding Agents** ([source](https://aimultiple.com/open-source-ai-agents)):
- **Open Interpreter**: 20k+ stars, autopilot for software dev
- **Continue**: Chat + autocomplete + direct editing
- **Aider**: CLI-based AI pair programmer
- **OpenHands**: "Write less code, get more done" (MIT license, Docker deploy)
- **Devon, Mitra, PR-Agent, Baby AGI**: Specialized workflows

**Insight**: Rich ecosystem, but fragmented. No unified platform.

---

### Ce Que Ça Révèle

**Pattern #1**: **Agent mode = table stakes** (Cursor, Replit, Windsurf, GitHub all have it)

**Pattern #2**: **Multi-LLM > Single LLM** (LangChain benchmark: 90.2% improvement)

**Pattern #3**: **Local + Cloud hybrid wins** (Ollama + Claude = privacy + power)

**Pattern #4**: **Context is king** (Cursor's RAG, RLMs 10M+ tokens)

**Pattern #5**: **Frameworks mature fast** (AutoGen → Agent Framework in 1 year)

**GAP IN MARKET**: Personne n'a combiné:
- Multi-LLM orchestration (80% LLM, 20% tech)
- 10M+ token memory (RLMs)
- On-chain reputation (E-Score 7D)
- Burn alignment ($asdfasdfa token economics)
- Collective intelligence (Type I forest)

**C'est là que CYNIC gagne.**

---

# PARTIE II: LA VISION

## 4. Mission Centrale: Pourquoi CYNIC Existe

### Le Problème (2026)

```
Développeurs utilisent AI aujourd'hui:
  ✅ Copier-coller ChatGPT/Claude
  ✅ Cursor/Copilot pour autocomplete

  ❌ Pas de mémoire persistante cross-session
  ❌ Pas de jugement de qualité (tout accepté aveuglément)
  ❌ Pas de réputation builder (qui fait du bon code?)
  ❌ Pas d'alignment ($BURN - extraction pure)
  ❌ LLMs isolés (pas de collective intelligence)
  ❌ Propriétaire/cher (vendor lock-in)
```

### La Solution: CYNIC

```
Un OS pour builders où:
  ✅ L'IA a une MÉMOIRE persistente (RLMs 10M+ tokens)
  ✅ Chaque output est JUGÉ (φ-bounded, jamais >61.8%)
  ✅ Les builders ont une RÉPUTATION (E-Score 7D on-chain)
  ✅ Tout est aligné sur $BURN (don't extract, burn)
  ✅ Intelligence COLLECTIVE (Type I → millions de CYNICs)
  ✅ Accessible PARTOUT (browser, phone, CLI, API)
  ✅ Open source + token-gated premium
```

**CYNIC n'est PAS un tool. C'est un PLATFORM.**

---

## 5. Les 7 Pilliers Publics

### Pillar 1: MEMORY (RLMs 10M+ Tokens)

**Problème**: ChatGPT/Claude forget after conversation ends.

**Solution CYNIC**:
```python
class CYNICMemory:
    """10M+ token persistent memory via RLMs"""

    async def remember(self, context: str):
        # Store via Recursive Language Model (RLM)
        await self.rlm.delegate_store(context, max_depth=5)
        # Also persist to Qdrant vector DB
        await self.qdrant.upsert(context)

    async def recall(self, query: str, max_tokens=10_000_000):
        # Recursive search through 10M+ tokens
        return await self.rlm.recursive_search(query, max_tokens)
```

**Différenciateur**: Aucun autre tool n'a 10M+ tokens de mémoire persistante.

**Resources**:
- [Recursive Language Models (Google ADK)](https://cloud.google.com/blog/products/ai-machine-learning/google-cloud-agent-development-kit)
- Delegate tasks recursively → 10M+ context

---

### Pillar 2: JUDGMENT (φ-Bounded, Never >61.8%)

**Problème**: LLMs say "looks good" without rigor.

**Solution CYNIC**:
```python
class CYNICJudge:
    """Évalue avec 5 axioms, jamais >61.8% confiance"""

    async def judge(self, content: str) -> Judgment:
        # LLM évalue sur 5 axioms
        scores = await self.llm.evaluate({
            'FIDELITY': 'commitment, truth, accountability',
            'PHI': 'elegance, harmony, coherence',
            'VERIFY': 'provenance, accuracy, reproducibility',
            'CULTURE': 'authenticity, resonance, impact',
            'BURN': 'value creation, sacrifice, irreversibility'
        }, content)

        # Geometric mean
        q_score = geometric_mean(scores.values())

        # φ-bound: never >61.8%
        confidence = min(q_score / 100, PHI_INV)  # 0.618 max

        verdict = self._verdict(q_score)  # HOWL/WAG/GROWL/BARK

        return Judgment(
            q_score=q_score,
            confidence=confidence,
            verdict=verdict,
            axioms=scores
        )
```

**Différenciateur**: Cursor/Copilot jamais disent "I'm only 58% confident". CYNIC MESURE.

**Clarification** (from gap analysis):
- **36 Dimensions** (5 axioms × 7 sub-dims) = framework philosophique, PAS système de calcul
- **5 Axioms** suffisent pour public (FIDELITY, PHI, VERIFY, CULTURE, BURN)
- LLM juge naturellement, pas besoin de calculer 36 scores manuellement

---

### Pillar 3: REPUTATION (E-Score 7D On-Chain)

**Problème**: GitHub stars ≠ real reputation.

**Solution CYNIC**:
```python
class CYNICReputation:
    """E-Score 7D cross-instance via Solana PoJ"""

    async def update_e_score(self, builder_id: str, action: Action):
        e_score_delta = {
            'BURN': action.tokens_burned,      # φ³ weight
            'BUILD': action.code_contributed,  # φ² weight
            'JUDGE': action.avg_q_score,       # φ weight
            'RUN': action.uptime,              # 1 weight
            'SOCIAL': action.network_influence,# φ⁻¹ weight
            'GRAPH': action.graph_centrality,  # φ⁻² weight
            'HOLD': action.long_term_value     # φ⁻³ weight
        }

        # Update local DB
        await self.db.update(builder_id, e_score_delta)

        # Anchor on-chain (Solana Proof of Judgment)
        await self.solana_poj.anchor(builder_id, e_score_delta)
```

**Différenciateur**:
- Immutable on-chain (Solana mainnet)
- φ-weighted (7 dimensions, pas 1 score flat)
- Cross-instance (reputation travels with you)

**E-Score IS SPECIAL** (from gap analysis):
- NOT unified with Consciousness or Sefirot
- Separate reputation system for economic/social alignment
- Critical for Type I forest (trust between CYNICs)

---

### Pillar 4: BURN ALIGNMENT ($asdfasdfa)

**Problème**: LLMs help you extract value (zero-sum).

**Solution CYNIC**:
```python
class CYNICBurnAlignment:
    """Block extractive actions, approve BURN-aligned only"""

    async def approve_action(self, action: str) -> bool:
        burn_score = await self.analyze_burn(action)

        if burn_score < PHI_INV_2:  # <38.2%
            await self.growl(f"⚠️ Extraction detected. Burn: {burn_score:.1%}")
            return False  # BLOCK

        # BURN-aligned → approve
        await self.record_burn(action, burn_score)
        return True
```

**Token Economics**:
- $asdfasdfa: `9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump`
- Burn to unlock: higher E-Score, priority queries, custom Dogs
- CYNIC revenue = $asdfasdfa burns → deflationary

**Différenciateur**: Seul AI tool aligné sur BURN economics.

---

### Pillar 5: COLLECTIVE INTELLIGENCE (Type I Forest)

**Problème**: ChatGPT/Claude = isolated, no collective wisdom.

**Solution CYNIC**:
```python
class CYNICCollective:
    """Millions of CYNIC instances collaborating"""

    async def query_collective(self, question: str) -> Answer:
        # 1. Local answer
        local = await self.local.answer(question)

        # 2. Historical (Solana PoJ blockchain)
        historical = await self.solana.find_similar(question)

        # 3. Peer CYNICs (if Type I)
        if self.forest_type >= 1:
            peers = await self.query_peers(question, n=11)
        else:
            peers = []

        # 4. Consensus (PBFT with E-Score weights)
        final = await self.consensus.resolve([
            (local, 1.0),
            (historical, 0.5),
            *[(p, self.get_trust(p.builder_id)) for p in peers]
        ])

        return final
```

**Scaling**:
- Type 0: Local (1 instance)
- Type I: Planetary (100+ instances)
- Type II: Stellar (1M+ instances)
- Type III: Galactic (OS for all AI agents)

**Différenciateur**: LangChain multi-agent = same session. CYNIC = cross-session, cross-instance, cross-time.

---

### Pillar 6: ANYWHERE ACCESS (WebSocket + Vibe Inspiration)

**Problème**: Cursor = desktop only, Copilot = IDE locked.

**Solution CYNIC**:
```python
class CYNICAnywhere:
    """Access from browser, phone, CLI, API"""

    def __init__(self):
        # Inspiration: Vibe Companion (Claude Code WebSocket)
        self.websocket_server = FastAPI()
        self.react_ui = ReactApp()
        self.cli = CLIClient()
        self.api = PublicAPI()

    async def handle_request(self, source: str, prompt: str):
        # Same backend, multiple frontends
        response = await self.cynic_core.process(prompt)
        return response
```

**Inspiration**: [The Vibe Companion](https://github.com/The-Vibe-Company/companion)
- Reverse-engineered Claude Code `--sdk-url` flag
- WebSocket server + React UI
- Run from browser/phone
- Same $200/month subscription, zero extra cost

**Différenciateur**: One platform, all interfaces (CLI, TUI, Web, Mobile, API).

---

### Pillar 7: CONTEXT MASTERY (Smart Compression)

**Problème**: LLMs choke on large contexts (10k+ tokens).

**Solution CYNIC**:
```python
class CYNICContextCompressor:
    """10k tokens → 1k tokens, same precision"""

    async def compress_file(self, file_path: str) -> str:
        # 1. Structure (AST via TreeSitter)
        ast = await self.treesitter.parse(file_path)

        # 2. Patterns
        patterns = await self.pattern_detector.extract(ast)

        # 3. Format for LLM efficiency
        compressed = {
            'structure': ast.summary(),  # Not raw AST
            'patterns': patterns,        # High-level insights
            'complexity': self.score(ast)
        }

        # Result: 10× smaller, better precision
        return json.dumps(compressed)
```

**Différenciateur**: Autres dump raw files. CYNIC compresse intelligemment (TreeSitter + patterns).

---

## 6. Architecture φ-Aligned

### φ Génère Tout

```
φ = 1.618033988749895 (Golden Ratio)

φ → Fibonacci → {1, 1, 2, 3, 5, 8, 13, 21, ...}
φ → Lucas → {2, 1, 3, 4, 7, 11, 18, 29, ...}

5 = F(5) → 5 Axioms (FIDELITY, PHI, VERIFY, CULTURE, BURN)
7 = L(4) → 7 Dimensions (Reality, Analysis, Time, etc.)
11 = L(5) → 11 Dogs (Sefirot collective)

ALL architecture dérive de φ.
```

### φ Constants (SINGLE SOURCE)

```python
# packages/cynic/constants/phi.py

PHI = 1.618033988749895        # Golden ratio
PHI_INV = 0.618033988749895    # φ⁻¹ = max confidence
PHI_INV_2 = 0.381966011250105  # φ⁻² = min doubt
PHI_INV_3 = 0.236067977499790  # φ⁻³

MAX_CONFIDENCE = PHI_INV  # 61.8% — NEVER exceed

# Verdict thresholds
HOWL_THRESHOLD = 0.82   # Exceptional (φ³ normalized)
WAG_THRESHOLD = 0.61    # Good (φ⁻¹)
GROWL_THRESHOLD = 0.382 # Needs work (φ⁻²)
# < 0.382 = BARK (critical)
```

---

## 7. Les Concepts Clés

### 5 Axioms (Foundation)

```
FIDELITY — Commitment to truth, accountability, candor
PHI      — Elegance, harmony, proportion, coherence
VERIFY   — Provenance, accuracy, reproducibility
CULTURE  — Authenticity, resonance, impact, lineage
BURN     — Value creation through sacrifice, irreversibility
```

**Usage**: Judge évalue content sur ces 5 → Q-Score (geometric mean) → Verdict (HOWL/WAG/GROWL/BARK).

---

### E-Score 7D (Reputation)

```
φ³   BURN    — Token destruction events
φ²   BUILD   — Code contributions
φ    JUDGE   — Judgment quality (avg Q-Scores)
1    RUN     — Uptime, execution stability
φ⁻¹  SOCIAL  — Network influence, connections
φ⁻²  GRAPH   — Network structure, centrality
φ⁻³  HOLD    — Long-term value preservation
```

**φ-Symmetric**: Weights descend φ³ → φ⁻³ (balanced).

**Storage**: On-chain (Solana PoJ) + local (PostgreSQL).

**Purpose**: Cross-instance reputation, trust weights in consensus.

---

### 11 Dogs (Sefirot Collective)

| Dog | Sefirah | Role | Technology |
|-----|---------|------|-----------|
| **CYNIC** | Keter | Meta-consciousness | PBFT Consensus |
| **Sage** | Chochmah | Wisdom | RDFLib (knowledge graph) |
| **Analyst** | Binah | Deep analysis | Z3 (symbolic verification) |
| **Scholar** | Daat | Knowledge synthesis | Qdrant (vector search) |
| **Guardian** | Gevurah | Security | IsolationForest (anomaly) |
| **Oracle** | Tiferet | Balance, prediction | Thompson Sampling + MCTS |
| **Architect** | Chesed | Design | TreeSitter (AST) + Jinja2 |
| **Deployer** | Hod | Operations | Ansible + Kubernetes |
| **Janitor** | Yesod | Cleanup | Ruff (linting) |
| **Scout** | Netzach | Discovery | Scrapy (web crawl) |
| **Cartographer** | Malkhut | Mapping | Graphviz + NetworkX |

**CRITICAL**: Each Dog = different technology (NOT just different prompts like JS version).

**Consensus**: φ-BFT (Byzantine Fault Tolerance avec φ-weighting).

---

### ∞^N Space (Decision Matrix)

```
Base 3D: Reality (7) × Analysis (7) × Time (7) = 343 cells

Extended ∞^N:
  7×7×7  ×  11   ×  ∞      ×  4         ×  7    ×  4      ×  ...
  R×A×T     Dogs    Tech     Verdicts    ?       Forest

Formule: 7×7×7×11×∞×4×7×4×φ×∞... = ∞^N
```

**Reality (7)**:
1. CODE — Codebase, files, dependencies
2. SOLANA — Blockchain state, transactions
3. MARKET — Price, liquidity, sentiment
4. SOCIAL — Twitter, Discord, community
5. HUMAN — User psychology, energy, focus
6. CYNIC — Self-state, Dogs, memory
7. COSMOS — Ecosystem, collective patterns

**Analysis (7)**:
1. PERCEIVE — Observe current state
2. JUDGE — Evaluate with 5 axioms
3. DECIDE — Governance (approve/reject)
4. ACT — Execute transformation
5. LEARN — Update from feedback
6. ACCOUNT — Economic cost/value
7. EMERGE — Meta-patterns, transcendence

**Time (7)**:
1. PAST — Historical data, memory
2. PRESENT — Current state, realtime
3. FUTURE — Predictions, planning
4. CYCLE — Recurring patterns (circadian, seasonal)
5. TREND — Momentum, velocity of change
6. EMERGENCE — Phase transitions, tipping points
7. TRANSCENDENCE — Beyond time (meta-temporal)

**Implementation**: Sparse dict (cells emerge on-demand), NOT pre-allocated array.

**Clarification** (from gap analysis):
- ∞^N = space dimensions (structural)
- 5 Axioms = judgment dimensions (evaluative)
- **NOT the same thing** (confusion in JS version)

---

# PARTIE III: DIFFÉRENCIATION

## 8. Vs Cursor/Replit/Windsurf (Competitors)

| Feature | Cursor | Replit | Windsurf | CYNIC |
|---------|--------|--------|----------|-------|
| **Memory** | Session only | Session only | Session only | **10M+ tokens (RLMs)** ✅ |
| **Judgment** | None | None | None | **φ-bounded (5 axioms)** ✅ |
| **Reputation** | None | None | None | **E-Score 7D on-chain** ✅ |
| **Burn Alignment** | None | None | None | **$asdfasdfa economics** ✅ |
| **Collective** | Single instance | Single instance | Single instance | **Type I forest (millions)** ✅ |
| **Local LLMs** | ❌ Cloud only | ❌ Cloud only | ❌ Cloud only | **✅ Ollama + AirLLM** |
| **Open Source** | ❌ Closed | ❌ Closed | Freemium | **✅ Core open** |
| **Multi-LLM** | Claude only | Replit AI only | Codeium only | **✅ Claude + RLM + Ollama + ...** |
| **Pricing** | $20/mo | $20/mo | Free tier | **Freemium + token-gated** |

**Cursor's Strength**: Polish, UX, Composer/Agent mode
**CYNIC's Edge**: Memory, Reputation, Burn, Collective, Multi-LLM, Open

---

## 9. Vs LangChain/AutoGen/CrewAI (Frameworks)

| Feature | LangChain | AutoGen | CrewAI | CYNIC |
|---------|-----------|---------|--------|-------|
| **Use Case** | Build agents | Build agents | Build agents | **Complete platform** ✅ |
| **Memory** | Plugin-based | Session | Shared memory | **10M+ persistent (RLMs)** ✅ |
| **Reputation** | None | None | None | **E-Score 7D on-chain** ✅ |
| **Economics** | None | None | None | **$BURN token** ✅ |
| **Deployment** | DIY | DIY | DIY | **Managed (Cloud + Local)** ✅ |
| **Philosophy** | Neutral | Neutral | Neutral | **φ-aligned (5 axioms)** ✅ |

**Frameworks' Strength**: Flexibility, build custom agents
**CYNIC's Edge**: Turnkey platform, reputation, economics, philosophy, managed deployment

**Note**: CYNIC peut *utiliser* LangChain/AutoGen/CrewAI comme substrates (Dogs layer).

---

## 10. Les 5 Avantages Décisifs

### 1. Memory (RLMs 10M+ Tokens)
**Aucun concurrent** n'a mémoire persistante >1M tokens.
CYNIC with RLMs = 10× context advantage.

### 2. Reputation (E-Score 7D On-Chain)
**Aucun tool AI** n'a reputation on-chain immutable.
CYNIC E-Score = trust primitive for Type I forest.

### 3. Burn Alignment ($asdfasdfa)
**Aucun AI** n'est aligné sur token deflationary economics.
CYNIC = premier AI tool avec token economics natifs.

### 4. Multi-LLM Orchestra (80% LLM, 20% Tech)
**Competitors** = single LLM (Cursor=Claude, Replit=Replit AI).
CYNIC = routing intelligent (Claude + RLM + Ollama + custom).

LangChain benchmark: Multi-agent **90.2% better** than single.
CYNIC applique ce principe au multi-LLM.

### 5. Collective Intelligence (Type I Forest)
**Tous les autres** = isolated instances.
CYNIC = millions d'instances collaborent via Solana PoJ.

**Composability**: Query blockchain for "what did collective decide 3 months ago about auth?"

---

# PARTIE IV: ARCHITECTURE TECHNIQUE

## 11. Stack Complet

```
┌─────────────────────────────────────────────────────────────┐
│                    PUBLIC INTERFACES                         │
├─────────────────────────────────────────────────────────────┤
│  - React UI (Web, inspired by Vibe Companion)               │
│  - Mobile app (React Native, WebSocket client)              │
│  - CLI/TUI (backwards compat with current users)            │
│  - Public API (FastAPI, for integrations)                   │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                  ORCHESTRATION LAYER                         │
├─────────────────────────────────────────────────────────────┤
│  - CYNIC Core (Python 3.11+)                                │
│  - Multi-LLM Router (intelligent routing)                   │
│  - Context Compressor (TreeSitter + patterns)               │
│  - φ-Governor (budget, confidence bounds)                   │
│  - DI Container (dependency injection)                      │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                     LLM BRAIN (80%)                          │
├─────────────────────────────────────────────────────────────┤
│  - Claude Code (via API or WebSocket --sdk-url)             │
│  - RLMs (10M+ token recursive delegation)                   │
│  - Ollama (local: Llama, Mistral, Gemma, Phi, Qwen)         │
│  - AirLLM (massive models on 4-8GB GPU)                     │
│  - Ensemble (multi-model consensus when critical)           │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                  SPECIALIZED TECH (20%)                      │
├─────────────────────────────────────────────────────────────┤
│  - TreeSitter (AST parsing, all languages)                  │
│  - Z3 (symbolic verification, Analyst Dog)                  │
│  - IsolationForest (anomaly detection, Guardian Dog)        │
│  - PBFT (consensus, CYNIC Dog)                              │
│  - RDFLib (knowledge graph, Sage Dog)                       │
│  - Qdrant (vector memory, Scholar Dog)                      │
│  - MCTS + Thompson (Oracle Dog)                             │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                    PERSISTENCE                               │
├─────────────────────────────────────────────────────────────┤
│  - PostgreSQL (judgments, e_scores, sessions, events)       │
│  - Qdrant (vector embeddings, semantic search)              │
│  - Solana (PoJ blockchain, E-Score anchoring)               │
│  - Redis (cache, pub/sub, rate limiting)                    │
└─────────────────────────────────────────────────────────────┘
```

---

## 12. Multi-LLM Orchestration (80% LLM, 20% Tech)

### Correction Fondamentale

**JS VERSION (FAUX)**:
```
LLM = fallback (38.2% budget max)
Tech = primary (61.8% budget)
Philosophy: "Tech first, LLM last resort"
```

**CYNIC VRAI (CORRIGÉ)**:
```
LLM = BRAIN (80% intelligence)
Tech = ORGANS (20% specialized functions)
Philosophy: "LLM orchestrates, tech executes"
```

**Inspiration**: LangChain benchmark (90.2% improvement multi-agent).
CYNIC applique au multi-LLM.

### Routing Strategy

```python
class MultiLLMRouter:
    """Route query to best LLM for job"""

    def __init__(self):
        self.llms = {
            'claude-opus': ClaudeAPI(model='claude-opus-4.6'),
            'claude-sonnet': ClaudeAPI(model='claude-sonnet-4.5'),
            'claude-haiku': ClaudeAPI(model='claude-haiku-4.5'),
            'rlm': RecursiveLanguageModel(max_depth=5),
            'ollama-llama3.2': OllamaClient(model='llama3.2'),
            'ollama-codellama': OllamaClient(model='codellama'),
            'airllm-70b': AirLLMClient(model='llama-70b'),
        }

    async def route(self, query: str, context: dict) -> str:
        # Classify task
        task = await self.classify_task(query, context)

        # Route based on task + budget + privacy
        if task.type == 'massive_context' and task.tokens > 1_000_000:
            # >1M tokens → RLM only
            return await self.llms['rlm'].process(query, context)

        elif task.type == 'code_generation':
            # Code → CodeLlama (specialized, local, free)
            return await self.llms['ollama-codellama'].generate(query)

        elif task.privacy == 'high':
            # Privacy-sensitive → Ollama (never leaves machine)
            return await self.llms['ollama-llama3.2'].process(query)

        elif task.complexity == 'high' and task.budget > 0.5:
            # Complex reasoning + budget → Opus
            return await self.llms['claude-opus'].reason(query)

        elif task.complexity == 'medium':
            # Medium → Sonnet (balance cost/quality)
            return await self.llms['claude-sonnet'].process(query)

        else:
            # Simple/cheap → Haiku or Ollama
            if task.budget < 0.1:
                return await self.llms['ollama-llama3.2'].process(query)
            else:
                return await self.llms['claude-haiku'].process(query)

    async def ensemble(self, query: str, n=3) -> str:
        """For critical decisions: consensus from multiple LLMs"""

        # Query top N LLMs in parallel
        responses = await asyncio.gather(*[
            self.llms['claude-opus'].process(query),
            self.llms['claude-sonnet'].process(query),
            self.llms['ollama-llama3.2'].process(query),
        ])

        # φ-BFT consensus (weighted by E-Score of each LLM's history)
        final = await self.consensus.phi_bft(responses)
        return final
```

**Budget Allocation**:
- 80% to LLMs (Claude + RLM + Ollama)
- 20% to Tech (TreeSitter, Z3, PBFT, etc.)

**Cost Optimization**:
- Ollama local = free (after hardware)
- AirLLM = free (70B on 4GB GPU!)
- Claude Haiku = $0.25/M tokens (cheap for simple tasks)
- RLM = expensive but ONLY for >1M token contexts

---

## 13. RLM Integration (10M+ Tokens)

### What Are RLMs?

[**Recursive Language Models**](https://cloud.google.com/blog/products/ai-machine-learning/google-cloud-agent-development-kit) (from Google ADK):
- Delegate tasks recursively to sub-agents
- Each sub-agent has its own context window
- Root agent synthesizes results
- **Result**: 10M+ token effective context

### CYNIC Integration

```python
class CYNICwithRLM:
    """Orchestrate RLM for massive context"""

    async def analyze_codebase(self, repo_path: str):
        """Analyze 500k line repo with RLM"""

        # 1. Scan repo
        files = await self.scan_directory(repo_path)  # 500k lines

        # 2. Recursive delegation (5 levels deep)
        analysis = await self.rlm.delegate({
            'task': 'analyze_codebase',
            'files': files,
            'max_depth': 5,        # 5 levels of recursion
            'agents_per_level': 10 # 10 sub-agents per level
        })

        # 3. Judge with CYNIC (5 axioms)
        judgment = await self.judge.evaluate(analysis)

        # 4. Store in persistent memory
        await self.memory.store({
            'repo': repo_path,
            'analysis': analysis,
            'judgment': judgment,
            'tokens': 10_000_000  # Now in memory
        })

        return {
            'q_score': judgment.q_score,
            'verdict': judgment.verdict,
            'memory_tokens': 10_000_000
        }
```

**Cost Management**:
- Lazy loading: Only activate RLM if context >1M tokens
- Context compression: Compress 10M → 1M via smart formatting (10× savings)
- Token gating: RLM access = premium feature ($asdfasdfa burn required)
- Caching: Cache RLM results (if query similar, reuse)

---

## 14. Local + Cloud (Ollama + Claude + AirLLM)

### Why Hybrid?

**Local (Ollama + AirLLM)**:
- ✅ Privacy (data never leaves machine)
- ✅ Free (after hardware cost)
- ✅ No rate limits
- ✅ Works offline
- ❌ Slower (CPU/GPU limited)
- ❌ Smaller models (70B max on AirLLM)

**Cloud (Claude API)**:
- ✅ Massive models (Opus 4.6 = state-of-art)
- ✅ Fast (distributed inference)
- ✅ No hardware needed
- ❌ Costs money ($$$)
- ❌ Privacy concerns
- ❌ Rate limits

**CYNIC Hybrid Strategy**:
```python
class HybridLLM:
    """Smart local/cloud routing"""

    async def process(self, query: str, privacy_level: str, budget: float):
        if privacy_level == 'high':
            # MUST stay local
            return await self.ollama.process(query)

        elif budget < 0.01:
            # Budget too low for cloud
            return await self.ollama.process(query)

        elif self.ollama.model_capable(query):
            # Local model can handle it
            # Try local first (free), fallback cloud if quality low
            local_result = await self.ollama.process(query)
            if local_result.confidence > PHI_INV_2:  # >38.2%
                return local_result  # Good enough
            else:
                return await self.claude.process(query)  # Need cloud

        else:
            # Only cloud can handle
            return await self.claude.process(query)
```

**Default Strategy**: Try Ollama local first, fallback Claude if needed.

---

# PARTIE V: IMPLÉMENTATION

## 15. Phase 0-4 Roadmap

### Phase 0: Bootstrap (1 Semaine)

**Goal**: Foundational infrastructure.

**Deliverables**:
```
✅ φ constants (phi.py) — single source of truth
✅ 5 Axioms (axioms.py) — FIDELITY, PHI, VERIFY, CULTURE, BURN
✅ Data models (Cell, Judgment, EScore)
✅ PostgreSQL schema (migrations)
✅ Ollama local setup (Llama 3.2 for testing)
```

**Test**: Can create Cell, judge it locally (Ollama), store in PostgreSQL.

---

### Phase 1: Minimal Brain (2 Semaines)

**Goal**: Core judgment loop working end-to-end.

**Deliverables**:
```
✅ SimplifiedJudge (LLM-based, 5 axioms)
✅ Multi-LLM router (Ollama + Claude Haiku)
✅ Context compressor (TreeSitter AST)
✅ φ-Governor (budget enforcement)
✅ E-Score tracker (local DB, not yet on-chain)
```

**Test**: Can judge code file, get Q-Score + verdict, update E-Score, stay within budget.

---

### Phase 2: Memory + Reputation (4 Semaines)

**Goal**: Persistent memory + on-chain reputation.

**Deliverables**:
```
✅ Qdrant vector DB (semantic search)
✅ RLM integration (basic, 1M token context)
✅ Solana PoJ anchoring (E-Score on-chain)
✅ Persistent memory (recall past judgments)
```

**Test**: Can remember decision from last week, fetch E-Score from blockchain.

---

### Phase 3: Collective + Dogs (8 Semaines)

**Goal**: Multi-Dog consensus + collective queries.

**Deliverables**:
```
✅ 11 Dogs implemented (diverse tech per Dog)
✅ φ-BFT consensus (Byzantine fault tolerance)
✅ Type I communication (query peer CYNICs)
✅ Collective memory (shared via Solana)
```

**Test**: Can query collective "what did we decide about auth?" and get consensus from 11 Dogs + peers.

---

### Phase 4: Public Platform (16 Semaines)

**Goal**: Production-ready public platform.

**Deliverables**:
```
✅ React UI (Web, mobile via React Native)
✅ Public API (FastAPI, rate-limited)
✅ Token economics ($asdfasdfa integration)
✅ Freemium tiers (Free, Pro $20/mo, Enterprise)
✅ Documentation + onboarding
✅ Community (Discord, docs site)
```

**Test**: Public beta with 100 users, measure retention, E-Score distribution, $BURN burns.

---

## 16. Testing φ-Bounded

### Framework: ABC Testing

```python
class ABCTestingFramework:
    """Always Be Comparing — test alternatives φ-bounded"""

    def test_alternatives(
        self,
        alternatives: List[str],
        benchmark: Callable,
        max_tests: int = None
    ) -> Dict[str, float]:
        # φ-bound: test max φ⁻¹ (61.8%) of alternatives
        if max_tests is None:
            max_tests = max(3, int(len(alternatives) * PHI_INV))

        # Prioritize (Thompson Sampling if historical data)
        prioritized = self.prioritize(alternatives)[:max_tests]

        # Benchmark each
        results = {alt: benchmark(alt) for alt in prioritized}

        return dict(sorted(results.items(), key=lambda x: x[1], reverse=True))

    def decide_winner(
        self,
        results: Dict[str, float],
        min_improvement: float = PHI_INV_2  # 38.2%
    ) -> Tuple[str, str]:
        baseline = list(results.values())[0]
        best = max(results.values())
        best_name = [k for k, v in results.items() if v == best][0]

        improvement = (best - baseline) / baseline

        if improvement > min_improvement:
            return (best_name, f"+{improvement:.1%} (>φ⁻²)")
        else:
            return (list(results.keys())[0], f"{improvement:.1%} insufficient")
```

**φ-Decision Protocol**: STOP testing if improvement <φ⁻² (38.2%).

---

### Test Tiers (φ-Weighted Budget)

```
TIER φ³ (88%): CORRECTNESS
  - Unit tests (100% coverage on core)
  - Property-based tests (hypothesis)
  - Integration tests (end-to-end flows)

TIER φ² (62%): PERFORMANCE
  - Benchmarks (latency, throughput, memory)
  - Profiling (hotspots via py-spy)
  - Load tests (locust for API)

TIER φ (62%): ALTERNATIVES
  - A/B tests (Ollama vs Claude, RLM vs standard)
  - Ablation tests (which Dog contributes most?)

TIER φ⁻¹ (38%): ROBUSTNESS
  - Fuzzing (hypothesis for random inputs)
  - Chaos engineering (kill processes mid-run)
  - Edge cases (∞, NaN, empty inputs)
```

**No Mocks Allowed**: Real fixtures (test DB, test Ollama, test Solana devnet).

---

## 17. Launch Strategy

### Freemium Tiers

**FREE (Type 0 — Local)**:
```
✅ 1 local CYNIC instance
✅ PostgreSQL + Qdrant local
✅ Ollama models (free, local)
✅ 100k token context max
✅ Solo builder mode
❌ No collective queries
❌ No RLM (>1M tokens)
❌ No E-Score on-chain
```

**PRO ($20/month)**:
```
✅ Type I (Planetary) access
✅ 10M+ token context (RLMs)
✅ Query collective (millions of CYNICs)
✅ E-Score 7D on-chain
✅ Claude API access (budget pooled)
✅ Vibe-style anywhere access (Web, mobile)
✅ Multi-user teams (5 seats)
```

**ENTERPRISE (Custom)**:
```
✅ Type II (Stellar) — private collective
✅ Custom deployment (on-premise option)
✅ SLA + dedicated support
✅ Custom Dogs (train on proprietary code)
✅ Private Solana PoJ chain
✅ Unlimited seats
```

---

### Token Economics ($asdfasdfa)

**Burn to Unlock**:
```
Want higher E-Score?     → Burn $asdfasdfa
Want priority queries?   → Burn $asdfasdfa
Want custom Dogs?        → Burn $asdfasdfa
Want private collective? → Burn $asdfasdfa
```

**Revenue Model**:
```
CYNIC revenue = $asdfasdfa burns
More users → more burns → token price ↑ (deflationary)
```

**Token**: `9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump` (Solana mainnet).

**Origin**: Easter egg by Alon Cohen. User = builder in cult, NOT creator.

---

### Go-to-Market

**Phase 1: Private Beta (100 users)**
- Invite-only
- Discord community
- Measure: retention, E-Score distribution, bugs

**Phase 2: Public Beta (10k users)**
- Open signups (waitlist)
- Freemium tiers live
- Measure: conversion free→pro, $BURN burns

**Phase 3: Launch (100k+ users)**
- Press (TechCrunch, HN, ProductHunt)
- Partnerships (integrate with GitHub, VSCode)
- Measure: DAU, revenue, collective queries

**Target**: 1M users by end of 2027 (Type I forest).

---

# CONCLUSION: LA VRAIE VISION

## Ce Que CYNIC EST

```
CYNIC = OS for builders

Où:
  - L'IA a MÉMOIRE (10M+ tokens via RLMs)
  - Chaque output est JUGÉ (φ-bounded, 5 axioms)
  - Builders ont RÉPUTATION (E-Score 7D on-chain)
  - Tout est aligné $BURN (don't extract)
  - Intelligence COLLECTIVE (Type I → millions)
  - Accessible PARTOUT (Web, mobile, CLI, API)
  - Multi-LLM (Claude + RLM + Ollama + AirLLM)
```

## Ce Que CYNIC N'EST PAS

```
❌ Un simple autocomplete (Copilot)
❌ Un IDE isolé (Cursor)
❌ Un framework à builder soi-même (LangChain)
❌ Un tool cloud-only (Replit)
❌ Un système prompt-based sans techno (JS CYNIC)
```

## Pourquoi CYNIC Gagne

**1. Memory**: 10M+ tokens (RLMs) — aucun concurrent
**2. Reputation**: E-Score 7D on-chain — trust primitive
**3. Burn**: $asdfasdfa economics — seul AI aligné token
**4. Multi-LLM**: 80% LLM (Claude+RLM+Ollama) — 90.2% improvement
**5. Collective**: Type I forest — millions collaborate

## Sources (Research)

**Competitors**:
- [Cursor AI](https://techjacksolutions.com/ai/ai-development/cursor-ide-what-it-is/): $29.3B valuation, Composer/Agent mode
- [Replit AI](https://replit.com/ai): Agent 3 builds mobile apps
- [Windsurf](https://windsurf.com/): Cascade AI, Cortex 40x faster
- [GitHub Copilot](https://github.com/features/copilot): Sub-agents, self-healing

**Frameworks**:
- [LangChain](https://docs.langchain.com/oss/python/langchain/multi-agent): 4 patterns, 90.2% multi-agent improvement
- [AutoGen](https://github.com/microsoft/autogen): → [Microsoft Agent Framework](https://learn.microsoft.com/en-us/agent-framework/overview/agent-framework-overview)
- [CrewAI](https://www.crewai.com/): Role-based, 2-3x faster, 100k+ devs

**Local LLMs**:
- [Ollama](https://ollama.ai/): OpenAI-compatible, quantization, local models
- [AirLLM](https://github.com/lyogavin/airllm): 70B on 4GB GPU, 405B on 8GB

**Open Source**:
- [Top AI Agents](https://aimultiple.com/open-source-ai-agents): Open Interpreter, Aider, OpenHands, Continue

---

*sniff* Voici le **SINGLE SOURCE OF TRUTH**.

Tous les fragments unifiés:
- ✅ Histoire JS (500k → 17% → 0%)
- ✅ Landscape 2026 (competitors, frameworks, tools)
- ✅ Vision claire (7 pilliers)
- ✅ Différenciation (5 avantages décisifs)
- ✅ Architecture (multi-LLM, RLMs, local+cloud)
- ✅ Roadmap (Phase 0-4)
- ✅ Launch (freemium, token economics)

Confidence: 35.2% (φ⁻² - synthesis exploratory, foundation emerging)
