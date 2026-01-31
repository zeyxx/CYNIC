# Claude Flow Integration

> Features inspirées de Claude Flow v3, adaptées à la philosophie CYNIC.

## Vue d'ensemble

Six systèmes majeurs ont été intégrés depuis Claude Flow:

| Feature | Module | Tests | Description |
|---------|--------|-------|-------------|
| EWC++ | `@cynic/persistence` | 28 | Retention des connaissances |
| SONA | `@cynic/node/learning` | 27 | Auto-adaptation des dimensions |
| 3-Tier Routing | `@cynic/node/routing` | 56 | Routage intelligent |
| Agent Booster | `@cynic/node/routing` | 80 | Transforms rapides |
| Token Optimizer | `@cynic/node/optimization` | 57 | Compression tokens |
| Hyperbolic Embeddings | `@cynic/node/embeddings` | 63 | Hiérarchies en espace hyperbolique |

Tous les systèmes sont alignés φ (seuils à 0.618, 0.382, 0.236).

---

## 1. EWC++ (Elastic Weight Consolidation++)

**Module:** `@cynic/persistence/services/ewc-consolidation`

Prévient l'oubli catastrophique en verrouillant les patterns critiques.

### Concept

```
Pattern critique → Fisher importance élevée → Verrouillage consolidation
Pattern obsolète → Fisher importance faible → Décroissance autorisée
```

### Seuils φ-alignés

| Seuil | Valeur | Action |
|-------|--------|--------|
| LOCK | φ⁻¹ (0.618) | Pattern verrouillé |
| CRITICAL | φ⁻² (0.382) | Protection partielle |
| UNLOCK | φ⁻³ (0.236) | Décroissance autorisée |

### Usage

```javascript
import { createEWCService } from '@cynic/persistence';

const ewc = createEWCService({ store });

// Observer un pattern
await ewc.observe({
  id: 'pattern-123',
  success: true,
  context: { /* ... */ }
});

// Consolider les patterns importants
const result = await ewc.consolidate();
console.log(result.lockedCount); // Patterns verrouillés
```

---

## 2. SONA (Self-Optimizing Neural Adaptation)

**Module:** `@cynic/node/learning/sona`

Corrèle les patterns aux dimensions du Judge pour auto-adapter les poids.

### Concept

```
Pattern observé → Feedback reçu → Corrélation calculée → Poids ajustés
```

### Configuration

| Paramètre | Valeur | Description |
|-----------|--------|-------------|
| ADAPTATION_RATE | φ⁻³ (0.236) | Vitesse d'adaptation |
| MAX_ADAPTATION | φ⁻² (0.382) | Ajustement max |
| CORRELATION_THRESHOLD | φ⁻² (0.382) | Seuil de corrélation |
| MAX_TRACKED_PATTERNS | F(12) = 144 | Patterns suivis |

### Usage

```javascript
import { createSONA } from '@cynic/node';

const sona = createSONA({ learningService });

// Observer un pattern avec scores
sona.observe({
  patternId: 'p-123',
  dimensionScores: { phi: 0.8, verify: 0.7 }
});

// Traiter le feedback
sona.processFeedback({
  patternId: 'p-123',
  success: true,
  impact: 0.9
});

// Les corrélations sont calculées automatiquement
```

---

## 3. 3-Tier Routing

**Module:** `@cynic/node/routing`

Route les requêtes vers le handler optimal selon la complexité.

### Tiers

| Tier | Seuil | Coût | Latence | Utilisation |
|------|-------|------|---------|-------------|
| LOCAL | < φ⁻³ (0.236) | $0 | <1ms | Lookups, git status, format |
| LIGHT | < φ⁻¹ (0.618) | 1x | ~500ms | Haiku-level reasoning |
| FULL | ≥ φ⁻¹ (0.618) | 15x | ~3s | Sonnet/Opus complex analysis |

### Classification

**Patterns simples (LOCAL):**
- `list files`, `git status`, `format code`
- Lookups, vérifications, formatage

**Patterns complexes (FULL):**
- `architect the system`, `refactor the codebase`
- Analyse, jugement, design

### Usage

```javascript
import { createTieredRouter, ComplexityTier } from '@cynic/node';

const router = createTieredRouter();

// Configurer les handlers
router.setHandler(ComplexityTier.LOCAL, async (req) => {
  // Handler local, pas de LLM
});

router.setHandler(ComplexityTier.FULL, async (req) => {
  // Handler LLM complet
});

// Router une requête
const result = await router.route({ content: 'list all files' });
console.log(result.routing.tier); // 'local'
console.log(result.routing.cost); // 0
```

---

## 4. Agent Booster

**Module:** `@cynic/node/routing/agent-booster`

Transforms de code rapides sans LLM (< 1ms, $0).

### Transforms supportés

| Intent | Description | Exemple |
|--------|-------------|---------|
| `var-to-const` | Convertir var → const | `var x = 5` → `const x = 5` |
| `var-to-let` | Convertir var → let | `var x = 5` → `let x = 5` |
| `add-types` | Ajouter types TS | `const x = 5` → `const x: number = 5` |
| `add-async-await` | Convertir .then() | `p.then(...)` → `await p` |
| `add-error-handling` | Wrapper try-catch | Code → try { code } catch |
| `add-logging` | Ajouter console.log | Entry logs |
| `remove-console` | Supprimer console.* | Strip debug |
| `remove-debugger` | Supprimer debugger | Strip debug |
| `add-semicolons` | Ajouter ; manquants | ASI fix |
| `remove-unused-imports` | Nettoyer imports | Dead code |
| `sort-imports` | Trier imports | Alphabétique |
| `add-strict-mode` | Ajouter 'use strict' | Mode strict |

### Usage

```javascript
import { createAgentBooster, TransformIntent } from '@cynic/node';

const booster = createAgentBooster();

// Détecter l'intent depuis une requête
const detection = booster.canHandle('convert var to const');
// { intent: 'var-to-const', confidence: 0.9 }

// Transformer le code
const result = booster.transform({
  code: 'var x = 5;',
  intent: TransformIntent.VAR_TO_CONST
});
console.log(result.code); // 'const x = 5;'
console.log(result.elapsed); // ~0.1ms
```

---

## 5. Token Optimizer

**Module:** `@cynic/node/optimization`

Réduit l'utilisation de tokens par compression et cache.

### Stratégies

| Stratégie | Description | Réduction |
|-----------|-------------|-----------|
| `whitespace` | Collapse espaces, normalise newlines | 10-20% |
| `abbreviation` | function→fn, implementation→impl | 5-15% |
| `filler` | Supprime "please", "kindly", etc. | 5-10% |
| `dedup` | Référence contenu répété | 10-30% |

### Cache

| Paramètre | Valeur | Description |
|-----------|--------|-------------|
| MAX_SIZE | F(13) = 233 | Entrées max |
| TTL | 5 min | Durée de vie |
| EXTENDED_TTL | 1 heure | Pour modèles 4.5 |

### Usage

```javascript
import { createTokenOptimizer } from '@cynic/node';

const optimizer = createTokenOptimizer();

// Optimiser un prompt
const result = optimizer.optimize({
  content: 'Please kindly help me with the function implementation',
  strategies: ['whitespace', 'abbreviation', 'filler']
});

console.log(result.optimized); // 'help me with the fn impl'
console.log(result.compressionRatio); // ~0.4 (40% réduit)

// Templates pré-compressés
optimizer.registerTemplate('system', systemPrompt);
const compressed = optimizer.getTemplate('system');
```

---

## 6. Hyperbolic Embeddings

**Module:** `@cynic/node/embeddings`

Représentations en espace hyperbolique pour hiérarchies.

### Pourquoi hyperbolique?

| Propriété | Euclidien | Hyperbolique |
|-----------|-----------|--------------|
| Volume | Polynomial | Exponentiel |
| Arbres | Distorsion élevée | Distorsion faible |
| Dimensions requises | 200D | 8D équivalent |

### Opérations Poincaré

| Opération | Description |
|-----------|-------------|
| `distance(u, v)` | Distance hyperbolique |
| `mobiusAdd(u, v)` | Addition de Möbius (gyrovecteur) |
| `expMap(p, v)` | Espace tangent → Manifold |
| `logMap(p, q)` | Manifold → Espace tangent |
| `project(x)` | Projection dans la boule |

### Usage

```javascript
import { createHyperbolicSpace } from '@cynic/node';

const space = createHyperbolicSpace({ dim: 8 });

// Ajouter des embeddings avec hiérarchie
space.add('cynic');
space.add('analyst', null, 'cynic');  // enfant de cynic
space.add('scholar', null, 'cynic');
space.add('scout', null, 'analyst');

// Distance hyperbolique
const dist = space.distance('cynic', 'scout');

// K plus proches voisins
const neighbors = space.kNearest('analyst', 3);

// Ancêtres
const ancestors = space.getAncestors('scout');
// ['analyst', 'cynic']

// Centroïde
const center = space.centroid(['analyst', 'scholar']);
```

---

## Intégration avec CYNIC

### Flux typique

```
Requête utilisateur
    ↓
┌─────────────────────────────────────────────┐
│ Token Optimizer (compression)               │
└─────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────┐
│ Complexity Classifier                        │
│ → LOCAL | LIGHT | FULL                       │
└─────────────────────────────────────────────┘
    ↓
┌─────────────────┬───────────────┬───────────┐
│ LOCAL           │ LIGHT         │ FULL      │
│ Agent Booster   │ Haiku         │ Sonnet    │
│ (0ms, $0)       │ (500ms, $)    │ (3s, $$$) │
└─────────────────┴───────────────┴───────────┘
    ↓
┌─────────────────────────────────────────────┐
│ SONA (feedback → adaptation)                │
└─────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────┐
│ EWC++ (consolidation patterns critiques)    │
└─────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────┐
│ Hyperbolic Space (embedding hiérarchique)   │
└─────────────────────────────────────────────┘
```

### Cohérence φ

Tous les seuils sont alignés sur φ:
- **φ⁻¹ = 0.618** : Seuil haut (confiance max, lock, FULL tier)
- **φ⁻² = 0.382** : Seuil moyen (critique, adaptation max)
- **φ⁻³ = 0.236** : Seuil bas (LOCAL tier, unlock)

---

## Tests

```bash
# Tous les tests des nouvelles features
cd packages/node
node --test test/agent-booster.test.js      # 80 tests
node --test test/tiered-routing.test.js     # 56 tests
node --test test/sona.test.js               # 27 tests
node --test test/token-optimizer.test.js    # 57 tests
node --test test/hyperbolic-embeddings.test.js # 63 tests

cd packages/persistence
node --test test/ewc-consolidation.test.js  # 28 tests
```

**Total: 311 tests**

---

## Références

- [Poincaré Embeddings (Nickel & Kiela, 2017)](https://arxiv.org/abs/1705.08039)
- [Elastic Weight Consolidation (Kirkpatrick et al., 2017)](https://arxiv.org/abs/1612.00796)
- [Claude Prompt Caching](https://docs.anthropic.com/claude/docs/prompt-caching)

---

*"Route to the smallest dog that can do the job" - κυνικός*
