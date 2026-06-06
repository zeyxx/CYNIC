# CYNIC DATA CONSTITUTION
**Status:** Canonical Mandate
**Version:** 1.0.0 (2026-06-05)

> "Data is the Moat. We treat it not as a byproduct, but as the sovereign substance of the organism."

---

## 1. PERCEPTION (Input)
Everything begins with capture. A system without perception is a system without truth.
- **Sources:** CSV (`pd.read_csv`), Cookies (`$_COOKIE`), Sessions (`session_start`), Hooks (Event capture).
- **Mandate:** Capture the environment faithfully and exhaustively. Never assume data is "noise" before it is processed.

## 2. TRANSFORMATION (Processing)
Raw data is potential; transformed data is signal.
- **Manipulation:** Selection (`df['col']`), Aggregation (`groupby`), Summarization (`describe`).
- **Cleaning:** 
    - Handle missing values (Interpolation for time-series).
    - Drop columns with high NaN density.
- **Mandate:** Transform raw entropy into actionable signal.

## 3. STRUCTURATION (Modeling)
Structure is the skeleton of persistence.
- **Database:** Adhere to **Third Normal Form (3NF)** to eliminate redundancy.
- **Operations:** Use SQL Joins wisely (Beware of **CROSS JOIN** combinatorial explosion).
- **Logic:**
    - **Triggers:** Automate actions but avoid cascading side-effects.
    - **Procedures/Functions:** Use functions for return values, procedures for actions.
- **Mandate:** Create a stable, non-redundant structure that honors the data's geometry.

## 4. ANALYSIS & COMPREHENSION
Data must be understood before it is used.
- **Statistics:** Always `describe()` the distribution.
- **Visualization:** Use Matplotlib/Seaborn. Visualize histograms and distributions.
- **Mandate:** Extract meaning and interpretability from the signal.

## 5. MACHINE LEARNING (Learning)
Discovery of hidden structures.
- **Clustering:** Use K-Means (unsupervised) to group similar data without labels.
- **Mandate:** Discover the latent patterns that escape human observation.

## 6. RELIABILITY & SURVIVAL (ACID)
The data must survive the system's failures.
- **ACID Principles:**
    - **Atomicité:** All or nothing.
    - **Cohérence:** Valid state transitions.
    - **Isolation:** Independent transactions.
    - **Durabilité:** Persistence is absolute.
- **Commands:** Explicit `COMMIT`, `ROLLBACK`, and `REVOKE`.
- **Security:** `password_hash` for identities.
- **Mandate:** Guarantee the stability and sovereign integrity of the storage.

## 8. FORMAT SOVEREIGNTY (JSON IS KING)
In the CYNIC ecosystem, JSON is not just a format; it is the language of the realm.
- **Inter-Agent Communication:** All reports intended for machine consumption (Handoffs, Audits, Task results) MUST be delivered in JSON.
- **Signals:** Every extracted signal must adhere to a strict JSON schema.
- **Datasets:** JSONL (JSON Lines) is the mandatory format for all training data (L1 & L2).
- **Mandate:** Human-readable text (Markdown) is for the Sovereign (T.); Machine-readable JSON is for the Cortex (Agents) and the Heart (Kernel).

---

## 9. RISK MITIGATION (Anti-Patterns)
The following are considered "Heresies" in the CYNIC Data System:
- **Cascading Triggers:** Unpredictable side-effects.
- **Massive CROSS JOIN:** Data explosion leading to resource exhaustion.
- **High Missing Value Density:** Biased or hollow analysis.
- **Weak Hashing:** Compromised sovereignty.

---

## Data Intelligence Architecture (Hierarchical View)

```text
SYSTÈME DATA INTELLIGENT
│
├── 1. PERCEPTION (INPUT)
│   ├── Capture l’environnement (CSV, Hooks, Sessions)
│
├── 2. TRANSFORMATION (TRAITEMENT)
│   ├── Transformer le brut en signal (Clean, Group, Summarize)
│
├── 3. STRUCTURATION (MODÉLISATION)
│   ├── Créer une structure stable (3NF, Joins, Triggers)
│
├── 4. ANALYSE & COMPRÉHENSION
│   ├── Extraire du sens (Stats, Visualisation)
│
├── 5. APPRENTISSAGE
│   ├── Découvrir des structures cachées (Clustering, K-Means)
│
├── 6. FIABILITÉ (SURVIE SYSTÈME)
│   ├── Garantir la stabilité (ACID, Transactions, Sécurité)
│
└── 7. RISQUES & ANTI-PATTERNS
    ├── Éviter l'effondrement (Cascades, Cross-Joins, NaN)
```
