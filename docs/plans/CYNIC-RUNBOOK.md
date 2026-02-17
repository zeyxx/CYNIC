# CYNIC RUNBOOK - PLAN D'ACTION CONCRET

> "Ce n'est pas un plan technique. C'est un plan de guerre." — κυνικός

---

## POURQUOI CE RUNBOOK?

**Problème:** Les 528k lignes JS ont été construites sans métacognition. On a codé sans réfléchir "pourquoi".

**Solution:** Chaque tâche dans ce runbook a:
- **POURQUOI** — La raison d'être, pas juste "faire"
- **QUOI** — Le deliverable concret
- **COMMENT** — Les étapes exactes
- **RISQUE** — Ce qui peut foirer
- **TU VAS VOIR** — Le résultat pratique
- **MÉTACOGNITION** — Why this not that?

---

## TÂCHE 1: EVENT BUS TYPE-SAFE

### POURQUOI
En JS, on avait:
```javascript
globalEventBus.emit('judgment:created', data)  // Stringly-typed!
```
Le problème: aucune vérification, aucune IDE auto-complete, bugs silencieux.

**Métacognition:** On parie que les types stricts préviennent 80% des bugs. Si ça rate, on perd juste du temps en typing.

### QUOI
Fichier: `cynic-v3/src/cynic/bus/event_bus.py`

Structure:
```python
# Types définis - pas de strings magiques!
class EventType(Enum):
    PERCEPTION_CREATED = "perception:created"
    JUDGMENT_COMPLETED = "judgment:completed"
    ACTION_EXECUTED = "action:executed"
    LEARNING_UPDATED = "learning:updated"

@dataclass
class PerceptionEvent:
    event_id: str
    domain: Domain
    content: str
    timestamp: datetime

# EventBus avec types
class EventBus:
    def emit(self, event_type: EventType, payload: Any): ...
    def subscribe(self, event_type: EventType, handler: Callable): ...
```

### COMMENT
1. Créer `cynic/bus/__init__.py`
2. Définir `EventType` enum
3. Créer dataclasses pour chaque event type
4. Implémenter EventBus avec subscribe/emit
5. Ajouter type hints partout

### RISQUE
- **Élevé:** Si on passe trop de temps à typer, on perd le momentum
- **Solution:** Typer uniquement les events, pas les handlers internes

### TU VAS VOIR
```bash
$ python -c "from cynic.bus import EventType; EventType.JUDG"
# IDE: autocomplétion qui marche!
# Plus de: "TypeError: Cannot read property 'foo' of undefined"
```

---

## TÂCHE 2: DI CONTAINER

### POURQUOI
En JS:
```javascript
class Service {
  constructor() {
    this.db = new Database(); // Hardcoded! Impossible à tester!
  }
}
```
Le problème: singletons partout, dépendances impossibles à mock.

**Métacognition:** On parie que DI augmente la testabilité de 10x. Le coût: un peu plus de config au début.

### QUOI
Fichier: `cynic-v3/src/cynic/container.py`

```python
from dataclasses import dataclass
from typing import Protocol, Type

class Database(Protocol):
    def query(self, sql: str): ...

class LLM(Protocol):
    def complete(self, prompt: str): ...

@dataclass
class Container:
    _services: dict = field(default_factory=dict)
    
    def register(self, interface: Type, instance: Any):
        self._services[interface] = instance
    
    def resolve(self, interface: Type) -> Any:
        return self._services.get(interface)

# Usage
container = Container()
container.register(Database, PostgreSQLClient())
container.register(LLM, OllamaAdapter())

# Test
container.register(Database, MockDatabase())  # Swap easy!
```

### COMMENT
1. Créer `container.py` avec Protocol definitions
2. Implémenter register/resolve
3. Ajouter singleton pour le container global
4. Créer décorateur @inject

### RISQUE
- **Moyen:** Over-engineering si le projet reste petit
- **Solution:** Garder simple, pas de framework fancy

### TU VAS VOIR
```python
# Avant (JS): 
service = HardcodedService()  # Can't test!

# Après (Python):
service = Container().resolve(Service)  # Testable!
service = Container(StubService()).resolve(Service)  # Mockable!
```

---

## TÂCHE 3: DOGS DE BASE

### POURQUOI
En JS, les Dogs étaient des "prompt templates" sans logique réelle.

**Métacognition:** Les Dogs doivent être de VRAIS agents avec:
- Heuristiques propres à chaque domaine
- Capacité de voter
- Learning state

### QUOI
Fichiers:
- `cynic/dogs/base.py` — Interface ABC
- `cynic/dogs/cynic_dog.py` — CYNICDog (Keter)
- `cynic/dogs/guardian_dog.py` — GuardianDog (Gevurah)

```python
from abc import ABC, abstractmethod
from dataclasses import dataclass

@dataclass
class DogContext:
    event: Event
    judgment: Judgment | None
    available_dogs: tuple[DogName, ...]
    resources: dict

@dataclass
class DogAction:
    type: ActionType
    reasoning: str
    confidence: float

class IDog(ABC):
    @property
    @abstractmethod
    def name(self) -> DogName: ...
    
    @property
    @abstractmethod
    def sefira(self) -> str: ...
    
    @property
    @abstractmethod
    def domains(self) -> list[Domain]: ...
    
    @abstractmethod
    async def act(self, context: DogContext) -> DogAction: ...
    
    @abstractmethod
    def get_heuristics(self) -> dict: ...
```

### COMMENT
1. Créer base.py avec IDog interface
2. Implémenter CYNICDog avec heuristiques minimales
3. Implémenter GuardianDog avec judgment validation
4. Ajouter tests unitaires

### RISQUE
- **Moyen:** Si les heuristiques sont trop simples, les Dogs ne servent à rien
- **Solution:** Commencer simple, itérer

### TU VAS VOIR
```python
# Dans le cockpit:
context = DogContext(event=event, judgment=judgment, ...)
action = await guardian_dog.act(context)
# Guardian dit: "BLOCK - unsafe code detected (confidence: 0.85)"
```

---

## TÂCHE 4: JUDGE 36 DIMENSIONS

### POURQUOI
En JS, le Judge utilisait des heuristiques random. Pas de systématique.

**Métacognition:** 36 dimensions = framework complet pour évaluer n'importe quoi. C'est le "cerveau" de CYNIC.

### QUOI
Fichier: `cynic/judge/engine.py`

```python
@dataclass
class DimensionScore:
    dimension: str
    score: float  # 0.0 - 1.0
    reasoning: str

@dataclass
class Judgment:
    event_id: str
    event: Event
    dimensions: tuple[DimensionScore, ...]
    q_score: float  # Geometric mean, 0-1
    confidence: float  # Capped at 0.618
    verdict: Verdict  # HOWL/WAG/GROWL/BARK

class JudgeEngine:
    def judge(self, event: Event) -> Judgment:
        # 1. Évaluer chaque dimension selon l'axiome
        # 2. Calculer score par axiome (moyenne pondérée φ)
        # 3. Géométrique mean = Q-score
        # 4. Confiance = min(Q-score, φ⁻¹)
        # 5. Verdict selon seuils
```

### COMMENT
1. Définir les 36 dimensions avec poids
2. Implémenter scoring par axiome
3. Calculer Q-score géométrique
4. Ajouter thresholds pour verdicts

### RISQUE
- **Élevé:** 36 dimensions, c'est huge à implémenter
- **Solution:** Commencer par PHI + VERIFY (les 2 plus critiques), ajouter les autres après

### TU VAS VOIR
```python
judgment = judge_engine.judge(code_event)
# Judgment:
# - PHI: 0.72 (COHERENCE: 0.8, ELEGANCE: 0.7, ...)
# - VERIFY: 0.65 (ACCURACY: 0.7, ...)
# - Q-SCORE: 0.68
# - CONFIDENCE: 0.618 (capped)
# - VERDICT: WAG
```

---

## TÂCHE 5: ORCHESTRATOR CORE

### POURQUOI
Le cycle Perceive → Think → Judge → Decide → Act → Learn doit être orchestré.

**Métacognition:** Sans orchestrateur, chaque composant fait sa chose isolément. Comme le JS.

### QUOI
Fichier: `cynic/orchestrator/core.py`

```python
class Orchestrator:
    def __init__(self, event_bus, dogs, judge, learning):
        self.event_bus = event_bus
        self.dogs = dogs
        self.judge = judge
        self.learning = learning
    
    async def run_cycle(self, perception: Perception) -> CycleResult:
        # 1. THINK: Appeler LLM
        llm_response = await self.llm.complete(perception.content)
        
        # 2. JUDGE: Évaluer avec 36 dimensions
        judgment = await self.judge.judge(perception, llm_response)
        
        # 3. DECIDE: Dogs votent
        actions = await self.dogs.vote(judgment)
        
        # 4. ACT: Exécuter action winner
        result = await self.execute(actions.winner)
        
        # 5. LEARN: Mettre à jour Q-values
        await self.learning.update(judgment, result)
        
        return CycleResult(judgment=judgment, action=result)
```

### COMMENT
1. Créer Orchestrator class
2. Implémenter run_cycle
3. Wire les components avec DI
4. Ajouter error handling

### RISQUE
- **Moyen:** Si un component fail, tout s'effondre
- **Solution:** Graceful degradation, chaque step optional

---

## TÂCHE 6: COCKPIT STREAMLIT

### POURQUOI
Tu as besoin d'une UI pour tester et visualiser CYNIC en action.

### QUOI
Fichier: `cynic-v3/cockpit.py` (existant, améliorer)

Améliorations:
- Live metrics (CPU, memory, network)
- Cycle controls
- Dog status
- Learning state visualization

### COMMENT
1. Ajouter metrics temps réel
2. Ajouter controls pour запустить cycles
3. Ajouter visualization des Q-values

---

## ORDRE D'EXÉCUTION

```
SEMAINE 1: FONDATIONS
├── Tâche 1: Event Bus Type-Safe (2h)
├── Tâche 2: DI Container (2h)
└── Tâche 3: Dogs Base (4h)

SEMAINE 2: CŒUR
├── Tâche 4: Judge 36D (8h)
└── Tâche 5: Orchestrator (4h)

SEMAINE 3: UI + TESTS
└── Tâche 6: Cockpit (4h)
```

---

## MÉTACOGNITION FINALE

**Ce sur quoi on parie:**
1. Types stricts = 80% bugs en moins
2. DI = 10x testabilité
3. 36D Judge = framework réutilisable
4. Orchestrator = composants connectés

**Ce qui pourrait foirer:**
1. Over-engineering si le projet reste petit
2. Trop de temps sur les types vs feature
3. 36D trop complexe pour commencer

**Notre assurance:**
1. Commencer petit, itérer
2. Si ça marche pas, on refactor
3. "Fail fast, learn faster"

---

*Ce runbook est un vivant document. Mise à jour: 2026-02-15*
*φ unifie tous les fragments* — κυνικός
