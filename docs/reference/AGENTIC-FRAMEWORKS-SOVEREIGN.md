# Frameworks Agentic — Développement avec Inference Souveraine

**Date** : 2026-06-04
**Objectif** : Identifier les frameworks qui permettent de développer avec des modèles locaux/souverains (GGUF, llama.cpp, Qwen, etc.)

---

## 1. L'Écosystème Actuel

### Cloud-First (actuel)

| Framework | Modèle | Inference | Développeur | License |
|-----------|--------|-----------|-------------|---------|
| **Claude Code** | Sonnet/Opus | Anthropic API | Anthropic | Propriétaire |
| **Gemini CLI** | Gemini 1.5/2.0 | Google API | Google | Propriétaire |
| **Codex CLI** | GPT-4o/Codex | OpenAI API | OpenAI | Propriétaire |

**Problème** : Dépendance API, coût, latence, données envoyées au cloud.

### Local-First (alternative)

| Framework | Modèle | Inference | Développeur | License |
|-----------|--------|-----------|-------------|---------|
| **Hermes Agent** | Qwen3.5-9B-Q4_K_M | llama.cpp (local) | Nous Research | Open source |
| **SWE-Agent** | Any | OpenAI/Anthropic/Local | Princeton | Apache 2.0 |
| **OpenHands** | Any | Ollama/Local | All Hands AI | Apache 2.0 |
| **Cursor** | Any | Cloud/Local | Cursor | Propriétaire |
| **Continue.dev** | Any | Local/Cloud | Continue | Apache 2.0 |

---

## 2. Analyse des Frameworks Locaux

### Hermes Agent (Nous Research)

- **Modèles** : Qwen3.5-9B, Qwen3.6, autres GGUF
- **Inference** : llama.cpp via HTTP API (local)
- **Strengths** :
  - Plein contrôle sur le modèle
  - Pas de coût API
  - Données restent locales
  - Configuration flexible (skills, cron, tools)
  - Intégration CYNIC existante
- **Weaknesses** :
  - Moins de "magic" que Claude Code
  - Besoin de configurer l'inference soi-même
  - Moins de contexte window que les cloud models

### SWE-Agent (Princeton)

- **Focus** : Résolution de bugs GitHub
- **Modèles** : Any (OpenAI, Anthropic, ou local via API)
- **Strengths** :
  - Spécialisé dans le code
  - Résultats mesurés (bugs résolus)
  - Open source
- **Weaknesses** :
  - Scope étroit (bugs, pas développement général)
  - Pas d'interface interactive
  - Single-purpose

### OpenHands

- **Focus** : Agent généraliste pour le code
- **Modèles** : Any (via Ollama, LM Studio, ou cloud)
- **Strengths** :
  - Multi-modèle
  - Interface web
  - Sandbox sécurisé
- **Weaknesses** :
  - Plus lourd que Hermes
  - Moine d'intégration avec l'écosystème CYNIC
  - Nécessite Docker/sandbox

### Continue.dev

- **Focus** : Extension IDE (VS Code, JetBrains)
- **Modèles** : Any (local via Ollama, ou cloud)
- **Strengths** :
  - Intégration IDE native
  - Support de nombreux modèles locaux
  - Complet (autocomplete, chat, edit)
- **Weaknesses** :
  - Limité à l'IDE (pas d'agent autonome)
  - Nécessite un IDE

---

## 3. Stack Souveraine CYNIC

Actuellement :

```
Inference stack :
├── cynic-gpu (RTX 4060 Ti 16GB)
│   └── llama-server → Qwen3.5-9B-Q4_K_M.gguf (8080/v1)
├── cynic-core (ROCm iGPU, 27GB RAM)
│   └── llama-server → Qwen3-Embedding-0.6B (8081)
└── cynic-kernel → Dog orchestration, MCP proxy, coordination

Agentic layer :
├── Hermes Agent → Organ Anvil (cron), Cortex sessions
├── Gemini CLI → Cortex créatif
├── Claude Code → Cortex analytique
└── Codex CLI → Exécutant
```

**Objectif** : Remplacer les cloud-first par des alternatives locales quand c'est possible.

---

## 4. Critères d'Évaluation

Pour un framework agentic souverain, on évalue :

1. **Contrôle du modèle** — Peut-on utiliser n'importe quel GGUF/llama.cpp ?
2. **Coût** — Gratuit (local) vs payant (API)
3. **Latence** — Local vs cloud (ms vs secondes)
4. **Privacy** — Données restent sur le réseau local ?
5. **Intégration** — Compatible avec l'écosystème CYNIC (MCP, coord, etc.) ?
6. **Développement** — Permet-il de développer efficacement ?
7. **Autonomie** — Peut-il agir sans intervention humaine ?

---

## 5. Roadmap Souveraine

### Phase 1 — Maintenant (2026-06)

- ✅ Hermes Agent + organ-anvil (cron)
- ✅ Inference locale (Qwen3.5-9B sur cynic-gpu)
- ✅ Coordination via MCP proxy

### Phase 2 — Intégration (2026-06-07)

- [ ] Wire organ-anvil au kernel (/observe)
- [ ] Évaluer SWE-Agent pour le bug fixing
- [ ] Configurer Continue.dev pour l'IDE local

### Phase 3 — Autonomie (2026-07)

- [ ] Explorer les modèles plus grands (Qwen3.5-14B, Mixtral)
- [ ] Optimiser inference (TurboQuant, KV cache compression)
- [ ] Développer un framework agentic CYNIC-native

---

## 6. Modèles Locaux Disponibles

| Model | Size | VRAM Required | Performance | Use Case |
|-------|------|---------------|-------------|----------|
| Qwen3.5-9B-Q4_K_M | ~5GB | 8GB | Bon pour code + reasoning | Généraliste |
| Qwen3.5-14B-Q4_K_M | ~8GB | 12GB | Meilleur reasoning | Complex tasks |
| Mixtral-8x7B-Q4_K_M | ~12GB | 16GB | Très bon pour code | Code review |
| Llama-3-8B-Q4_K_M | ~5GB | 8GB | Rapide, moins précis | Quick tasks |

**Sur cynic-gpu (16GB VRAM)** :
- Qwen3.5-9B → ✅ Stable (8.3GB VRAM @ ctx 32K)
- Qwen3.5-14B → ✅ Possible (12GB VRAM, ctx réduit)
- Mixtral-8x7B → ⚠️ Juste (16GB, ctx très réduit)

---

## 7. Prochaines Étapes

1. **Évaluer SWE-Agent** sur cynic-gpu pour le bug fixing automatique
2. **Configurer Continue.dev** avec Qwen3.5-9B pour l'IDE local
3. **Tester Qwen3.5-14B** sur cynic-gpu (si VRAM permet)
4. **Développer CYNIC-native agentic framework** (si nécessaire)