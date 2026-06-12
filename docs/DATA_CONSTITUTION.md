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

## 7. FINANCIAL SOVEREIGNTY & ENCRYPTION
Data relating to trading volumes, balances, proprietary signals, and economic models must be secured.
- **Mandate:** "Encrypt the money." No raw financial indicators, client payloads, or trading datasets may be published in plaintext on public branches. All sovereign data must be encrypted before persistence or transfer outside the trusted environment.

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
├── 7. SOUVERAINETÉ FINANCIÈRE & CHIFFREMENT
│   ├── Protéger les actifs et modèles ("Encrypt the money")
│
├── 8. SOUVERAINETÉ DES FORMATS
│   ├── Communication inter-agents (JSON is King)
│
├── 9. RISQUES & ANTI-PATTERNS
│   ├── Éviter l'effondrement (Cascades, Cross-Joins, NaN)
│
└── 10. EFFICIENCE COGNITIVE (1 Dog = 1 SOT)
    ├── Chaque Dog représente une unique Source de Vérité. Les LLM Dogs sont le dernier recours.
```

---

## 10. COGNITIVE EFFICIENCY (1 Dog = 1 SOT)
In CYNIC, a "Dog" is an epistemic agent encapsulating exactly one Source of Truth (SOT).
- **The Golden Rule:** 1 Dog = 1 SOT.
- **Hierarchy of Resolution:** You MUST route queries through deterministic Dogs before invoking LLM Dogs.
  1. Local Database Dogs (Absolute SOT)
  2. On-Chain / Cryptographic Dogs (SOT)
  3. Deterministic Heuristic Dogs (e.g., `rug-prefilter`, `wallet-judgment` rules)
  4. **LLM Dogs** (Latent SOT — Used ONLY for nuance, ambiguity, or when deterministic Dogs are inconclusive)
- **Mandate:** Never burn GPU cycles to answer a question that a deterministic Dog could answer via a simple SQL query or API call. LLMs are for judgment, not data retrieval.
