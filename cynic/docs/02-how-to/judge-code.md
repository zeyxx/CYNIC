# How to Judge Code with CYNIC

> **Practical guide for code evaluation**
>
> *🐕 κυνικός | "Judge not, lest ye be judged—actually, judge everything"*

---

## Quick Judgment

```python
from cynic import CYNICKernel

kernel = CYNICKernel()
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
print(f"Dimensions: {verdict.dimensions}")
```

---

## Understanding the Q-Score

| Score | Verdict | Meaning |
|-------|---------|---------|
| 82-100 | HOWL | Excellent, production-ready |
| 61-82 | WAG | Good, acceptable |
| 38-61 | GROWL | Needs improvement |
| 0-38 | BARK | Problems, rework needed |

---

## The 36 Dimensions

Grouped by axiom:

### PHI (Structure)
- COHERENCE, HARMONY, STRUCTURE, ELEGANCE, COMPLETENESS, PRECISION, EMERGENCE

### VERIFY (Trust)
- ACCURACY, VERIFIABILITY, TRANSPARENCY, REPRODUCIBILITY, PROVENANCE, INTEGRITY, PROOF

### CULTURE (Fit)
- AUTHENTICITY, RELEVANCE, NOVELTY, ALIGNMENT, IMPACT, RESONANCE, IDENTITY

### BURN (Value)
- UTILITY, SUSTAINABILITY, EFFICIENCY, VALUE_CREATION, NON_EXTRACTIVE, CONTRIBUTION, SIMPLICITY

### FIDELITY (Truth)
- COMMITMENT, DEPTH, RIGOR, CONSISTENCY, SELF_AWARENESS, COURAGE, DOUBT

---

## Improving Your Code

After judgment, check low-scoring dimensions:

```python
# Get detailed breakdown
for dim, score in verdict.dimensions.items():
    if score < 60:
        print(f"⚠️ {dim}: {score} - needs improvement")
```

Common improvements:
- **SECURITY < 60**: Add input validation
- **PERFORMANCE < 60**: Check algorithm complexity
- **MAINTAINABILITY < 60**: Add documentation

---

## Multi-File Judgment

```python
# Judge a whole file
with open('my_code.py') as f:
    verdict = kernel.judge(f.read())

# Judge a directory
import os
for root, dirs, files in os.walk('src/'):
    for file in files:
        if file.endswith('.py'):
            path = os.path.join(root, file)
            with open(path) as f:
                v = kernel.judge(f.read())
                print(f"{path}: {v.q_score} ({v.verdict})")
```

---

## Providing Feedback (Learning)

```python
# Mark judgment as correct
kernel.feedback(verdict_id=verdict.id, correct=True)

# Mark as incorrect
kernel.feedback(verdict_id=verdict.id, correct=False, 
                reason="Missed security issue")
```

This improves future judgments via Q-Learning.

---

*🐕 κυνικός | "Every judgment teaches"*