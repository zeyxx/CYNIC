# CYNIC Quickstart

> **5 minutes → Votre premier jugement**
>
> *🐕 κυνικός | "Loyal to truth, not to comfort"*

---

## Prérequis

| Outil | Version | Installation |
|-------|---------|--------------|
| Python | ≥3.11 | `python --version` |
| PostgreSQL | ≥14 | `docker compose up -d postgres` |
| Ollama | latest | [ollama.ai](https://ollama.ai) |

---

## Étape 1: Clone & Setup (2 min)

```bash
# Clone le repository
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC

# Setup Python environment
cd cynic
pip install -e .

# Setup PostgreSQL (Docker)
docker compose up -d postgres

# Pull Ollama model
ollama pull qwen2.5:14b
```

---

## Étape 2: Premier Jugement (2 min)

```python
from cynic import CYNICKernel

# Initialize kernel
kernel = CYNICKernel(
    storage='postgres://localhost:5432/cynic',
    llm='ollama://qwen2.5:14b'
)

# Judge du code
verdict = kernel.judge("""
def calculate_total(items):
    total = 0
    for item in items:
        total += item.price * item.quantity
    return total
""")

print(f"Q-Score: {verdict.q_score}")
print(f"Verdict: {verdict.verdict}")
print(f"Confidence: {verdict.confidence:.1%}")
```

**Sortie attendue:**

```
Q-Score: 68
Verdict: WAG
Confidence: 58.0%
```

---

## Étape 3: Comprendre le Résultat (1 min)

### Q-Score (0-100)

| Score | Verdict | Signification |
|-------|---------|---------------|
| 82-100 | HOWL | Excellent, production-ready |
| 61-82 | WAG | Bon, acceptable |
| 38-61 | GROWL | Moyen, amélioration suggérée |
| 0-38 | BARK | Problématique, retravailler |

### φ-Bounded Confidence

**Jamais plus de 61.8%** - C'est le principe fondateur:

```
max_confidence = φ⁻¹ = 0.618 = 61.8%
```

Pourquoi? Parce que la certitude absolue n'existe pas.

---

## Prochaines Étapes

1. **[installation.md](./installation.md)** - Setup complet avec toutes les options
2. **[../03-reference/architecture.md](../03-reference/architecture.md)** - Comprendre l'architecture
3. **[../03-reference/axioms.md](../03-reference/axioms.md)** - Les 5 principes fondateurs

---

## Problèmes Courants

### PostgreSQL Connection Error

```bash
# Vérifier que PostgreSQL tourne
docker ps | grep postgres

# Redémarrer si nécessaire
docker compose restart postgres
```

### Ollama Model Not Found

```bash
# Lister les modèles disponibles
ollama list

# Pull le modèle requis
ollama pull qwen2.5:14b
```

### Import Error

```bash
# Réinstaller le package
pip install -e . --force-reinstall
```

---

## Valider l'Installation

```bash
# Lancer les tests E2E
pytest cynic/test/test_kernel_e2e.py

# Devrait afficher: X passed
```

---

*🐕 Prêt à juger? Votre premier verdict attend.*