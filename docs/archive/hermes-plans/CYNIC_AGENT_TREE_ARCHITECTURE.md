# CYNIC Agent Tree Architecture — Formalisation

**Version**: 1.0  
**Date**: 2026-09-22  
**Head**: `2eea86ba`

---

## 1. Principes directeurs

1. **Chaque agent = identité cryptographique** (ed25519 pubkey dérivée)
2. **Chaque tâche = claim** avec scope, budget, TTL, preuve askès
3. **Chaque handoff = preuve SHA-256** liant parent → enfant
4. **L'état est append-only** (SurrealDB ou SQLite, jamais muté)
5. **Le coordinateur = petit LLM local** (notre inference souveraine)

---

## 2. Hiérarchie des agents

```
┌─────────────────────────────────────────────────────────────────┐
│                        COORDINATEUR (META)                       │
│         Petit LLM local (~7B) — gère le bus, les budgets        │
│         Reçoit les plans Hermes → les decompose → dispatch       │
│         Surveillance: stagnation, deadlocks, ICE, régressions     │
└─────────────────────────────────────────────────────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│  AGENT BUILD    │   │  AGENT AUDIT    │   │  AGENT EXTRACT  │
│  (compilation)  │   │  (prismatic)    │   │  (DDD split)    │
│  Builder SSH    │   │  Multi-review   │   │  Domain/Infra   │
│  cargo check    │   │  Adversarial    │   │  Architecture   │
│  make test      │   │  Ground truth   │   │  Boundary tests │
└─────────────────┘   └─────────────────┘   └─────────────────┘
          │                     │                     │
          └─────────────────────┼─────────────────────┘
                                ▼
                     ┌─────────────────────┐
                     │    BUS D'ÉTAT       │
                     │  SurrealDB/SQLite   │
                     │  - tasks            │
                     │  - claims           │
                     │  - proofs (askès)   │
                     │  - messages         │
                     │  - sessions         │
                     └─────────────────────┘
```

---

## 3. Types de tâches (taxonomie data-centric)

### 3.1. Par complexité de coordination

| Type | Description | Agents nécessaires | Handoff |
|------|-------------|-------------------|---------|
| `SOLO` | Fichier unique, scope isolé | 1 | 0 |
| `CHAIN` | Séquentiel A→B→C | 2-3 | N-1 |
| `FANOUT` | Parallèle A→[B,C,D] | 3+ | 1→N |
| `MULTI_PRISM` | Adversarial review | 4+ | Convergence |
| `EXTRACT` | DDD split avec validation | 2 (extract + verify) | Preuve askès |

### 3.2. Par couche architecturale

| Couche | Exemples | Détéction automatique |
|--------|----------|----------------------|
| `DOMAIN_PURE` | Types, value objects, enums | Scan imports std/tokio/reqwest → 0 |
| `PORT` | Traits, interfaces | `pub trait` dans ports/ |
| `ADAPTER` | Implémentations concrètes | Dossier adapters/ |
| `INFRA` | I/O, std::fs, tokio::sync | Scan imports → non-pure |
| `APPLICATION` | Orchestration, services | Dossier application/ |
| `CROSS_CRATE` | Changements multi-crates | Git diff scope > 1 crate |

### 3.3. Par domaine métier

| Domaine | Crate(s) | Coordinateur spécialisé |
|---------|----------|------------------------|
| `PROTOCOL` | cynic-protocol | Agent Protocole (puriste) |
| `KERNEL` | cynic-kernel | Agent Kernel (observabilité) |
| `INFERENCE` | cynic-inference, -runtime | Agent Inference (vertical LLM) |
| `INTEL` | cynic-intel-* | Agent Intel (analyse) |
| `FORENSICS` | cynic-forensics | Agent SOC |
| `DISCOVERY` | cynic-discovery-* | Agent Discovery |
| `INTERFACE` | cynic-cli | Agent CLI (thin surface) |

---

## 4. Workflow de handoff

### 4.1. Contrat de preuve

```rust
struct HandoffProof {
    /// SHA-256 du commit parent
    parent_commit: [u8; 32],
    /// SHA-256 du commit enfant  
    child_commit: [u8; 32],
    /// AgentId du parent (ed25519 pubkey[16])
    parent_agent: AgentId,
    /// AgentId de l'enfant
    child_agent: AgentId,
    /// Portée de la tâche
    scope: Scope,
    /// Budget consommé par le parent
    consumed_budget: Budget,
    /// Résultat du parent (pour reconstruction contexte)
    output_hash: [u8; 32], // SHA-256 du output
    /// Timestamp UNIX
    timestamp_unix_s: u64,
    /// Signature du parent sur le handoff
    signature: [u8; 64], // ed25519
}
```

### 4.2. Chaîne de custody

```
Plan Hermes (intent)
    ↓ proof[0]
Claim A (extract)
    ↓ proof[1]  
Claim B (verify)
    ↓ proof[2]
Commit (final)
    ↓
Askesis log (immutable)
```

Chaque `proof[i]` est ajouté au ledger. La chaîne est vérifiable : toute divergence = dette non résolue.

---

## 5. Règles d'enforcement (déterministes)

### 5.1. Budget

| Agent | Actions max | Wall seconds | Scope max |
|-------|-------------|--------------|-----------|
| Coordinateur | ∞ | ∞ | Global |
| Build | 100 | 7200 | Crate |
| Audit | 50 | 3600 | Module |
| Extract | 30 | 1800 | File |
| Solo | 20 | 900 | File |

### 5.2. Guardrails

1. **Pas de mutation sans claim** — écriture sur disque = claim actif
2. **Pas de commit sans preuve** — SHA-256 askès du diff
3. **Pas de handoff sans budget restant** — sinon blocage
4. **Pas de scope chevauchant** — claim lock par fichier/crate
5. **Pas de boucle infinie** — stagnation detection (3× même output)

### 5.3. Alternance

Pour les workflows multi-prisme :
- Les agents ne peuvent pas s'auto-déléguer
- Un audit ne peut pas se auditer lui-même
- Les reviews adversariales nécessitent ≥2 modèles différents

---

## 6. Bus de messages

### 6.1. Topics

```
/task/create     → Nouvelle tâche soumise
/task/claim      → Agent claim actif
/task/complete   → Tâche terminée (avec preuve)
/task/fail       → Échec (avec raison)
/budget/update   → Consommation budget
/handoff         → Handoff parent→enfant
/proof/new       → Nouvelle preuve askès ajoutée
/health          → Heartbeat agent
/ice_detected    → rustc ICE signalé
/stagnation      → Boucle détectée
```

### 6.2. Format message (inspiré ACP v1.0)

```rust
struct BusMessage {
    id: ulid::Ulid,
    topic: Topic,
    from_agent: AgentId,
    from_session: SessionId,
    timestamp_unix_s: u64,
    payload: Payload,
    proof: Option<HandoffProof>,
}
```

---

## 7. Composants techniques

| Composant | Stack | Backend |
|-----------|-------|---------|
| Ledger | Rust | SurrealDB (existant) ou SQLite |
| Identity | ed25519-dalek | `~/.hermes/agent-identities/` |
| Proofs | SHA-256 (sha2 crate) | Askesis (déjà implémenté) |
| Bus | tokio::sync::broadcast | Mémoire partagée + persistence |
| API | axum | localhost:43110 |
| CLI | clap | `cynic agent-bus` |
| Coordinateur | Petit LLM (~7B) | Inference souveraine |

---

## 8. Relations Parent-Enfant

### 8.1. Hermes (TUI) → Coordinateur

```
Hermes envoie un plan (intent)
    ↓
Coordinateur decompose en tâches
    ↓
Assigne aux agents selon spécialité
    ↓
Supervise l'exécution
    ↓
Valide les preuves askès
    ↓
Commit + log
```

### 8.2. Coordinateur → Agent Build

```
Coordinateur: "compile cynic-kernel, scope crate"
    ↓
Agent Build: claim, rsync, cargo check, report
    ↓
Proof: SHA-256 du diff + output compilation
    ↓
Coordinateur: validation ou réparation
```

### 8.3. Coordinateur → Agents Multi-Prism

```
Coordinateur: "audit PR #123, 4 prismes"
    ↓
[Audit A, Audit B, Audit C, Audit D] en parallèle
    ↓
Convergence: synthèse des findings
    ↓
Si conflit → escalade vers humain (Hermes)
```

---

## 9. Implémentation CYNIC

### 9.1. Crate `cynic-agent-bus`

```
crates/cynic-agent-bus/
├── src/
│   ├── lib.rs              → exports
│   ├── ledger.rs           → SurrealDB tasks/messages/proofs
│   ├── claim.rs            → claim/lease/release lifecycle
│   ├── identity.rs         → AgentId (ed25519)
│   ├── orchestrator.rs     → Multi-turn loop, guardrails
│   ├── proof.rs            → SHA-256 handoff proofs
│   ├── bus.rs              → tokio broadcast topics
│   ├── api.rs              → REST localhost
│   ├── cli.rs              → `cynic agent-bus` commands
│   └── coordinator.rs      → Petit LLM loop
├── tests/
│   └── architecture_boundary.rs
└── Cargo.toml
```

### 9.2. Intégration kernel

Le kernel ne consomme plus `organ/` directement. Il écoute le bus :

```rust
// AVANT (couplé)
use crate::organ::InferenceOrgan;
let organ = InferenceOrgan::new();
organ.degrade_backend(&name, reason);

// APRÈS (bus)
use cynic_agent_bus::Bus;
bus.emit(Topic::TaskFail, payload!(name, reason));
```

### 9.3. Intégration Hermes

Hermes soumet des "intents" au coordinateur via CLI :

```bash
cynic agent-bus task create \
  --agent hermes:main \
  --scope crate:cynic-kernel \
  --intent "Audit organ/ pattern, propose extraction" \
  --budget-actions 50 \
  --budget-seconds 3600
```

---

## 10. Plan de mise en œuvre

| Phase | Livrable | Scope |
|-------|----------|-------|
| **0** | Design doc (ce fichier) | Architecture formelle |
| **1** | `cynic-agent-bus/Cargo.toml` + types | Protocole, Identity, Proof |
| **2** | `ledger.rs` | Persistence SurrealDB |
| **3** | `claim.rs` + `cli.rs` | Claim/lease/release lifecycle |
| **4** | `orchestrator.rs` | Multi-turn + guardrails |
| **5** | `api.rs` | REST localhost |
| **6** | `coordinator.rs` | Petit LLM loop |
| **7** | Intégration kernel | Remplace organ/ |
| **8** | Intégration Hermes | CLI submit intent |

---

## 11. Différence avec AI Bus

| Concept | AI Bus (Node) | CYNIC Agent Bus (Rust) |
|---------|---------------|------------------------|
| Identité | Nom de CLI | ed25519 pubkey |
| Preuve | SHA-256 content | SHA-256 commit + handoff |
| Stockage | SQLite | SurrealDB (déjà dans CYNIC) |
| Coordination | Boucle multi-turn | Petit LLM souverain |
| Agents | Claude/Codex CLI | Hermes, Claude Code, Codex, Build |
| Port | localhost:43110 | localhost:43110 (même port) |

---

## 12. Acceptance criteria

- [ ] `cargo check -p cynic-agent-bus` → 0 erreurs
- [ ] Claim/lease/release lifecycle testé
- [ ] Preuve handoff vérifiable (SHA-256)
- [ ] API `/health` retourne status + version
- [ ] CLI `cynic agent-bus task create/list/show`
- [ ] Stagnation detection fonctionnelle
- [ ] Alternance agents enforcee
- [ ] Kernel ne dépend plus de `organ/`
