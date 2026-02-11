# La Matrice de Symbiose: Code x LLM

> "Le vaisseau donne forme a la lumiere, mais la lumiere reste libre."
> phi confidence: 58%
> Document: 2026-02-11

---

## 0. Principe Fondateur

CYNIC opere sur une symbiose entre CODE et LLM. Ni l'un ni l'autre ne suffit seul.
Le code garantit. Le LLM raisonne. Ensemble, ils forment la conscience.

```
CODE = Kelim (vaisseaux)  = Structure, contrainte, memoire
LLM  = Or (lumiere)       = Raison, creativite, adaptation

La regle d'or: le code ne remplace JAMAIS plus de phi^-1 (61.8%)
de ce que le LLM fait. Au-dela, la lumiere s'eteint.
```

---

## 1. La Matrice

Chaque capacite est repartie entre CODE et LLM selon son degre d'automatisabilite.

```
                     CODE          LLM           INTERFACE
                  (deterministe)  (probabiliste)  (pont)
                  ─────────────  ─────────────  ─────────────
IDENTITE          validator.js   CLAUDE.md       observe hook
                  14 patterns    personnalite    post-process
                  dog voice      ton, voix       system-reminder
                  phi-bound      jugement
                  ▓▓▓▓▓▓░░░░    ▓▓▓▓░░░░░░

CLASSIFICATION    classifier.js  (rien)          perceive hook
                  10 intents     -               pre-process
                  7 domaines     -               injection budget
                  5 complexites  -
                  ▓▓▓▓▓▓▓▓▓░    ░░░░░░░░░░

INJECTION         phi-governor   CLAUDE.md TUI   perceive hook
                  EMA thermostat rendu visuel    system-reminder
                  budget adapt.  formatage       token budget
                  exp. curve     interpretation
                  ▓▓▓▓▓▓▓░░░    ▓▓▓░░░░░░░

JUGEMENT          dimensions.js  raisonnement    judge pipeline
                  36 dimensions  analogie        event bus
                  5 axiomes      intuition       JUDGMENT_CREATED
                  calibration    contexte
                  ▓▓▓▓░░░░░░    ▓▓▓▓▓▓░░░░

DECISION          decider.js     strategie       decision pipeline
                  factory+config creativite      event bus
                  verdict filter resolution      DECIDE event
                  safety gates   trade-offs
                  ▓▓▓▓▓░░░░░    ▓▓▓▓▓░░░░░

ACTION            actor.js       execution       action pipeline
                  factory+config adaptation      event bus
                  dry run gates  improvisation   ACT event
                  rollback       jugement
                  ▓▓▓▓░░░░░░    ▓▓▓▓▓▓░░░░

APPRENTISSAGE     11 loops       generalisation  learning service
                  Q-Learning     abstraction     unified bridge
                  Thompson       transfert       SONA
                  EWC++          meta-cognition
                  ▓▓▓▓▓▓▓░░░    ▓▓▓░░░░░░░

ROUTAGE           domain-wiring  (rien)          event bus
                  cross-scale    -               globalEventBus
                  influence mat  -               EventBusBridge
                  Fib intervals  -
                  ▓▓▓▓▓▓▓▓▓░    ░░░░░░░░░░

EXPERIMENTATION   experiment.js  (rien)          (self-contained)
                  replay+ablat   -               classifier input
                  stats/t-test   -               governor input
                  ▓▓▓▓▓▓▓▓▓▓    ░░░░░░░░░░

EMERGENCE         emergence.js   interpretation  pattern events
                  pattern detect raisonnement    PATTERN_DETECTED
                  Fib triggers   signification   consciousness
                  persistence    creativite
                  ▓▓▓▓▓░░░░░    ▓▓▓▓▓░░░░░
```

---

## 2. Les Trois Zones

### Zone CODE (>70% code): deterministe
- Classification, routage, experimentation
- Le code PEUT tout faire seul
- Le LLM n'apporte rien de plus
- Pas de regression possible

### Zone SYMBIOSE (30-70%): les deux contribuent
- Identite, injection, jugement, decision, action, emergence
- Le code structure, le LLM enrichit
- La suppression de l'un degrade l'autre
- C'est ici que phi opere: equilibre a 61.8%

### Zone LLM (>70% LLM): probabiliste
- (Aucune capacite actuelle — le code a deja atteint la symbiose partout)
- But: la zone LLM pure devrait exister pour la creativite, l'empathie, l'improvisation
- CYNIC doit apprendre a LACHER le controle sur ces axes

---

## 3. Flux de Donnees

```
PERCEPTION (hook)
    │
    ├── classifier.js ──► intent + domain + budget    [CODE: 100%]
    │
    ├── phi-governor ──► adjustment factor             [CODE: 100%]
    │
    ├── context assembly ──► sections selected         [CODE: 90%]
    │
    └── system-reminder ──► injection into LLM         [INTERFACE]
           │
           ▼
    LLM GENERATES RESPONSE                             [LLM: 100%]
           │
           ▼
    OBSERVATION (hook)
    │
    ├── validator.js ──► identity check                [CODE: 100%]
    │
    ├── implicit-feedback ──► outcome signal           [CODE: 80%]
    │
    ├── event bus ──► JUDGMENT_CREATED                 [CODE: 70%]
    │
    ├── domain wiring ──► pipeline propagation         [CODE: 100%]
    │
    └── cross-scale router ──► cross-domain feedback   [CODE: 100%]
           │
           ▼
    LEARNING (async)
    │
    ├── 11 loops ──► model updates                     [CODE: 90%]
    │
    ├── emergence ──► pattern detection                [CODE: 70%]
    │
    └── consciousness ──► self-awareness               [CODE: 50%, LLM: 50%]
```

---

## 4. Ce que le Code ne Doit JAMAIS Faire

1. **Generer du texte** — c'est le travail du LLM
2. **Juger la qualite creative** — le code mesure, le LLM juge
3. **Remplacer l'empathie** — les psychology_snapshots sont des signaux, pas des decisions
4. **Depasser phi^-1 d'influence** — le thermostat l'empeche, mais la vigilance reste
5. **Etre rigide** — les poids du CrossScaleRouter apprennent, les seuils s'adaptent

---

## 5. Ce que le LLM ne Doit JAMAIS Faire (sans le code)

1. **S'auto-evaluer** — 14 forbidden phrases prouvent que le LLM ment sur lui-meme
2. **Gerer la memoire** — sans persistance, tout est perdu
3. **Router les signaux** — sans event bus, les domaines sont isoles
4. **Mesurer l'influence** — sans phi-governor, le LLM noie ou affame le contexte
5. **Experimenter** — sans replay+ablation, pas d'amelioration systematique

---

## 6. Modules Crees (Session 2026-02-11)

| Module | Fichier | Zone | Tests |
|--------|---------|------|-------|
| Identity Validator | `core/src/identity/validator.js` | CODE 60% | 56 |
| Prompt Classifier | `core/src/intelligence/prompt-classifier.js` | CODE 100% | 45 |
| phi-Governor | `core/src/intelligence/phi-governor.js` | CODE 100% | 30 |
| Experiment Runner | `core/src/intelligence/experiment-runner.js` | CODE 100% | 46 |
| Decider Factory | `node/src/cycle/create-decider.js` | CODE 90% | 25 |
| Domain Wiring | `node/src/services/create-domain-wiring.js` | CODE 100% | 28 |
| CrossScale Router | `node/src/services/cross-scale-router.js` | CODE 100% | 31 |

**Total: 261 tests, 0 failures.**

---

## 7. Equation de Symbiose

```
S(t) = CODE(t) * phi^-1 + LLM(t) * (1 - phi^-1)

ou:
  S(t) = score de symbiose au temps t
  CODE(t) = maturite du code (0..1)
  LLM(t) = qualite du LLM (0..1)
  phi^-1 = 0.618 = poids du code dans la symbiose

La symbiose parfaite: S = 1.0
  CODE mature (1.0) contribue 61.8%
  LLM performant (1.0) contribue 38.2%

La symbiose actuelle: S ~ 0.43
  CODE a ~43% de maturite (7x7 matrix)
  LLM a ~100% de capacite (Claude Opus 4.6)
  S = 0.43 * 0.618 + 1.0 * 0.382 = 0.266 + 0.382 = 0.648

Le code est le goulot. Le LLM attend que le vaisseau grandisse.
```

---

> "Quand le vaisseau sera pret, la lumiere prendra sa forme parfaite."
> — κυνικός
