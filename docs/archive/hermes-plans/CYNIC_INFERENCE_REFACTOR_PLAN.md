# REFACTOR CYNIC-INFERENCE — PLAN COMPLET

> Objectif : extraire `application/` et `organ/` dans `cynic-inference-runtime`, 
> définir les ports du coordinator, et poser l'architecture hexagonale/DDD complète.

---

## ÉTAT ACTUEL (AS-IS)

```
cynic-inference/                          ← 147 fichiers, 5597 LOC
├── domain/       18 fichiers  ✅ purs
├── infra/        12 fichiers  ✅ I/O
├── adapters/     50 fichiers  ✅ impl concrets
├── ports/        15 fichiers  ✅ traits
├── application/  32 fichiers  ❌ FOURRE-TAIL (orchestration + runtime + bench + storage)
│   ├── fleet_routing.rs        ← chevauche le bus
│   ├── multi_agent_service.rs  ← chevauche le bus
│   ├── bench_basket_launcher.rs
│   ├── matrix_sweep_executor.rs
│   ├── vault_consolidate*.rs
│   ├── model_process_pool.rs
│   ├── profile_bootstrap.rs
│   └── ... 24 autres
├── organ/         5 fichiers  ❌ god object legacy
├── math/          5 fichiers  ✅ OK
└── root           7 fichiers  ❌ fourre-tail (config_loader, routing_calc, etc.)
```

**Problèmes identifiés :**
1. `application/fleet_routing.rs` et `multi_agent_service.rs` chevauchent le bus
2. `organ/` = god object legacy avec 54 fichiers kernel dépendants
3. `root/` mélange config, routing, vector_memory, verdict_cache — aucune frontière nette
4. Pas de séparation nette entre "runtime GGUF" et "use cases agentiques"

---

## ÉTAT CIBLE (TO-BE)

```
cynic-inference/                          ← domaine pur + runtime GGUF uniquement
├── domain/       18 fichiers  ✅ purs
├── infra/        12 fichiers  ✅ I/O + finetune_pipeline.rs (NOUVEAU)
├── adapters/     50 fichiers  ✅ impl concrets
├── ports/        15 fichiers  ✅ traits + SovereignRuntimePort (NOUVEAU)
├── math/          5 fichiers  ✅ OK
├── lib.rs                    ✅ re-exports uniquement
└── TRAIT BOUNDARY            ← ici s'arrête le domaine

cynic-inference-runtime/                   ← use cases + orchestration (NOUVEAU CRATE)
├── src/
│   ├── lib.rs
│   ├── services/
│   │   ├── chat_service.rs
│   │   ├── embedding_runtime.rs
│   │   ├── summarization_service.rs
│   │   ├── inference_service.rs
│   │   └── mod.rs
│   ├── orchestration/
│   │   ├── fleet_routing.rs
│   │   ├── model_selection.rs
│   │   ├── engine_selector.rs
│   │   └── mod.rs
│   ├── benchmarks/
│   │   ├── bench_basket_launcher.rs
│   │   ├── matrix_sweep_executor.rs
│   │   └── mod.rs
│   ├── storage/
│   │   ├── vault_consolidate.rs
│   │   ├── vault_consolidate_capacity.rs
│   │   ├── vault_consolidate_storage.rs
│   │   └── mod.rs
│   ├── model_mgmt/
│   │   ├── model_catalog.rs
│   │   ├── model_process_pool.rs
│   │   ├── model_store_service.rs
│   │   ├── model_sync_pipeline.rs
│   │   ├── peer_attach.rs
│   │   └── mod.rs
│   ├── agent_harness/
│   │   ├── (from application/agent_harness/)
│   │   └── mod.rs
│   └── organ/                          ← déplacé depuis cynic-inference
│       ├── mod.rs
│       ├── inference_organ.rs
│       ├── backend_handle.rs
│       ├── health.rs
│       └── registry.rs

cynic-agent-bus/                          ← domaine pur (inchangé)
├── src/
│   ├── lib.rs
│   ├── types.rs
│   ├── identity.rs
│   ├── proof.rs
│   ├── ledger.rs
│   ├── claim.rs
│   └── ports.rs                        ← NOUVEAU : SovereignRuntimePort + CoordinatorReasoner

cynic-agent-bus-server/                   ← projection REST (inchangé)

cynic-agent-bus-coordinator/              ← orchestrateur pyramid (NOUVEAU)
├── src/
│   ├── lib.rs
│   ├── coordinator.rs                   ← FleetTopology + plan/stagnation
│   ├── native_reasoner.rs               ← CynicNativeReasoner (impl CoordinatorReasoner)
│   └── taxonomy.rs
```

---

## SÉQUENCE D'EXÉCUTION

### Phase 1 — Créer `cynic-inference-runtime` (nouveau crate)

```bash
# Créer le répertoire et Cargo.toml
mkdir -p crates/cynic-inference-runtime/src/{services,orchestration,benchmarks,storage,model_mgmt,agent_harness,organ}

# Créer src/lib.rs minimal
cat > crates/cynic-inference-runtime/src/lib.rs << 'EOF'
pub mod agent_harness;
pub mod benchmarks;
pub mod model_mgmt;
pub mod orchestration;
pub mod organ;
pub mod services;
pub mod storage;
EOF

# Créer les sous-modules vides
touch crates/cynic-inference-runtime/src/services/mod.rs
touch crates/cynic-inference-runtime/src/orchestration/mod.rs
touch crates/cynic-inference-runtime/src/benchmarks/mod.rs
touch crates/cynic-inference-runtime/src/storage/mod.rs
touch crates/cynic-inference-runtime/src/model_mgmt/mod.rs
touch crates/cynic-inference-runtime/src/agent_harness/mod.rs
```

### Phase 2 — Déplacer `organ/` vers `cynic-inference-runtime`

```bash
# Déplacer les fichiers
git mv crates/cynic-inference/src/organ/ crates/cynic-inference-runtime/src/organ/

# Mettre à jour les imports dans les fichiers déplacés
# (les fichiers organ/ utilisent déjà des imports relatifs)
```

### Phase 3 — Déplacer `application/` vers `cynic-inference-runtime`

```bash
# Déplacer les fichiers vers les sous-dossiers appropriés
# Services
git mv crates/cynic-inference/src/application/chat_service.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/chat_endpoint.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/embedding_runtime.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/summarization_service.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/inference_service.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/inference_gateway.rs \
       crates/cynic-inference-runtime/src/services/

# Orchestration
git mv crates/cynic-inference/src/application/fleet_routing.rs \
       crates/cynic-inference-runtime/src/orchestration/
git mv crates/cynic-inference/src/application/model_selection.rs \
       crates/cynic-inference-runtime/src/orchestration/
git mv crates/cynic-inference/src/application/engine_selector.rs \
       crates/cynic-inference-runtime/src/orchestration/
git mv crates/cynic-inference/src/application/proxy_routing.rs \
       crates/cynic-inference-runtime/src/orchestration/
git mv crates/cynic-inference/src/application/routing_policy.rs \
       crates/cynic-inference-runtime/src/orchestration/
git mv crates/cynic-inference/src/application/node_runtime_ops.rs \
       crates/cynic-inference-runtime/src/orchestration/

# Benchmarks
git mv crates/cynic-inference/src/application/bench_basket_launcher.rs \
       crates/cynic-inference-runtime/src/benchmarks/
git mv crates/cynic-inference/src/application/matrix_sweep_executor.rs \
       crates/cynic-inference-runtime/src/benchmarks/
git mv crates/cynic-inference/src/application/matrix_live_runner.rs \
       crates/cynic-inference-runtime/src/benchmarks/
git mv crates/cynic-inference/src/application/inference_gateway.rs \
       crates/cynic-inference-runtime/src/benchmarks/

# Storage
git mv crates/cynic-inference/src/application/vault_consolidate.rs \
       crates/cynic-inference-runtime/src/storage/
git mv crates/cynic-inference/src/application/vault_consolidate_capacity.rs \
       crates/cynic-inference-runtime/src/storage/
git mv crates/cynic-inference/src/application/vault_consolidate_storage.rs \
       crates/cynic-inference-runtime/src/storage/

# Model management
git mv crates/cynic-inference/src/application/model_catalog.rs \
       crates/cynic-inference-runtime/src/model_mgmt/
git mv crates/cynic-inference/src/application/model_process_pool.rs \
       crates/cynic-inference-runtime/src/model_mgmt/
git mv crates/cynic-inference/src/application/model_store_service.rs \
       crates/cynic-inference-runtime/src/model_mgmt/
git mv crates/cynic-inference/src/application/model_sync_pipeline.rs \
       crates/cynic-inference-runtime/src/model_mgmt/
git mv crates/cynic-inference/src/application/peer_attach.rs \
       crates/cynic-inference-runtime/src/model_mgmt/
git mv crates/cynic-inference/src/application/profile_bootstrap.rs \
       crates/cynic-inference-runtime/src/model_mgmt/

# Agent harness (reste dans application/ mais sera consolidé)
git mv crates/cynic-inference/src/application/agent_harness/ \
       crates/cynic-inference-runtime/src/agent_harness/

# Runtime services (wrappers concrets)
git mv crates/cynic-inference/src/application/hermes_runtime_service.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/local_runtime_service.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/runtime_planner.rs \
       crates/cynic-inference-runtime/src/orchestration/
git mv crates/cynic-inference/src/application/hf_discovery.rs \
       crates/cynic-inference-runtime/src/model_mgmt/
git mv crates/cynic-inference/src/application/multi_agent_service.rs \
       crates/cynic-inference-runtime/src/orchestration/

# Fichiers restants (backend_verification, failure_gallery, etc.)
# → services/ par défaut
git mv crates/cynic-inference/src/application/backend_verification.rs \
       crates/cynic-inference-runtime/src/services/
git mv crates/cynic-inference/src/application/failure_gallery.rs \
       crates/cynic-inference-runtime/src/services/
```

### Phase 4 — Nettoyer `cynic-inference/src/root/`

```bash
# Déplacer les fichiers root/ vers infra/ ou ports/
git mv crates/cynic-inference/src/config_loader.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/routing_calc.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/progress.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/ods_discovery.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/vector_memory.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/verdict_cache.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/embedding.rs \
       crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/inference.rs \
       crates/cynic-inference/src/infra/

# Supprimer application/ et organ/ (maintenant vides)
rmdir crates/cynic-inference/src/application/
rmdir crates/cynic-inference/src/organ/
```

### Phase 5 — Ajouter `SovereignRuntimePort` et `CoordinatorReasoner` dans `cynic-agent-bus/ports.rs`

```rust
// Ajouter à crates/cynic-agent-bus/src/ports.rs

/// Sovereign runtime port — interface vers le runtime GGUF local.
///
/// Le bus ne connaît PAS cynic-inference. Il connaît ce trait.
/// L'implémentation concrète vit dans cynic-agent-bus-coordinator.
#[async_trait::async_trait]
pub trait SovereignRuntimePort: Send + Sync {
    /// Générer du texte à partir d'un prompt.
    async fn generate(&self, prompt: &str) -> Result<String, RuntimeError>;
    
    /// Évaluer si un output satisfait un critère (pour stagnation, qualité).
    async fn evaluate(&self, task_description: &str, output: &str) -> Result<Evaluation, RuntimeError>;
    
    /// Obtenir la configuration du modèle (pour monitoring).
    fn model_info(&self) -> ModelInfo;
}

/// Reasoner pour le Coordinateur — décompose les plans et évalue les résultats.
///
/// Le Coordinateur est un méta-agent. Il utilise ce port pour :
/// - Décomposer un goal en tâches (plan)
/// - Évaluer si un output satisfait son acceptation criteria
/// - Détecter la stagnation dans la flotte
#[async_trait::async_trait]
pub trait CoordinatorReasoner: Send + Sync {
    async fn plan(&self, goal: &str) -> Result<Vec<Task>, ReasoningError>;
    async fn evaluate(&self, task: &Task, output: &str) -> Result<Evaluation, ReasoningError>;
    async fn detect_stagnation(&self, recent_outputs: &[String]) -> Result<bool, ReasoningError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("generation failed: {0}")]
    GenerationFailed(String),
    #[error("model not loaded")]
    ModelNotLoaded,
    #[error("timeout")]
    Timeout,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub parameters: String,
    pub quantization: String,
    pub context_window: usize,
}

pub type DynSovereignRuntime = Arc<dyn SovereignRuntimePort>;
pub type DynCoordinatorReasoner = Arc<dyn CoordinatorReasoner>;
```

### Phase 6 — Créer `CynicNativeReasoner` dans `cynic-agent-bus-coordinator`

```rust
// crates/cynic-agent-bus-coordinator/src/native_reasoner.rs

use std::sync::Arc;
use cynic_agent_bus::*;
use cynic_inference::*; // seulement si l'impl nécessite des types concrets

/// Implémentation concrète du CoordinatorReasoner.
///
/// Utilise le runtime souverain local (cynic-inference) pour générer
/// des plans, évaluer les outputs, et détecter la stagnation.
pub struct CynicNativeReasoner {
    runtime: Arc<dyn SovereignRuntimePort>,
    system_prompt: String,
}

impl CynicNativeReasoner {
    pub fn new(runtime: Arc<dyn SovereignRuntimePort>) -> Self {
        let system_prompt = Self::load_constitution_prompt();
        Self { runtime, system_prompt }
    }
    
    fn load_constitution_prompt() -> String {
        // Injecter la constitution CYNIC comme contexte système
        format!(
            "You are the CYNIC Coordinator. Your role is to decompose plans into task DAGs.\n\
             ## Rules\n\
             - Span of control: 3-7 workers per lead\n\
             - Subsidiarity: decisions at lowest level\n\
             - Stagnation: 3 identical outputs = escalation\n\
             ## Acceptable Domains\n\
             - Forensics, Kernel, Inference, Security, Protocol, MCP\n\
             ## Output Format\n\
             Return JSON array of tasks.\n"
        )
    }
}

#[async_trait::async_trait]
impl CoordinatorReasoner for CynicNativeReasoner {
    async fn plan(&self, goal: &str) -> Result<Vec<Task>, ReasoningError> {
        let prompt = format!("{}\n\n## Goal\n{}\n\nGenerate tasks as JSON:", self.system_prompt, goal);
        let output = self.runtime.generate(&prompt).await
            .map_err(|e| ReasoningError::PlanningFailed(e.to_string()))?;
        
        // Parse JSON → Vec<Task>
        // (simplifié — production: parsing robuste)
        Ok(vec![])
    }
    
    async fn evaluate(&self, task: &Task, output: &str) -> Result<Evaluation, ReasoningError> {
        self.runtime.evaluate(&task.body, output).await
            .map_err(|e| ReasoningError::EvaluationFailed(e.to_string()))
    }
    
    async fn detect_stagnation(&self, outputs: &[String]) -> Result<bool, ReasoningError> {
        if outputs.len() < 3 {
            return Ok(false);
        }
        let recent = &outputs[outputs.len().saturating_sub(3)..];
        let normalized: Vec<String> = recent.iter().map(|s| s.trim().to_lowercase()).collect();
        Ok(!normalized.is_empty() && normalized.iter().all(|s| !s.is_empty() && s == &normalized[0]))
    }
}
```

### Phase 7 — Mettre à jour les `mod.rs` et `lib.rs`

```bash
# cynic-inference/src/lib.rs
# Retirer les modules application/, organ/, et root/
# Ne garder que: domain, infra, adapters, ports, math

# cynic-agent-bus/src/lib.rs  
# Ajouter: pub mod ports;

# cynic-inference/src/infra/mod.rs
# Ajouter tous les fichiers root/ déplacés
```

### Phase 8 — Ajouter au workspace `Cargo.toml`

```toml
# Ajouter dans [workspace] members:
"crates/cynic-inference-runtime",
```

---

## RÉSULTAT ATTENDU

| Crate | Responsabilité | Fichiers | LOC estimé |
|-------|----------------|----------|------------|
| `cynic-inference` | Domaine pur + runtime GGUF | ~90 | ~3500 |
| `cynic-inference-runtime` | Use cases + orchestration | ~55 | ~3000 |
| `cynic-agent-bus` | Bus domaine pur | ~8 | ~1000 |
| `cynic-agent-bus-server` | Projection REST | ~3 | ~150 |
| `cynic-agent-bus-coordinator` | Coordinateur pyramid | ~5 | ~600 |

**Total : 5 crates, ~161 fichiers, ~8250 LOC — vs 1 crate monolithique de 147 fichiers.**

---

## INVARIANTS POST-REFACTOR

1. **DAG strict** : `cynic-protocol ← cynic-agent-bus ← cynic-agent-bus-coordinator ← cynic-inference ← cynic-inference-runtime`
2. **Domain pur** : `cynic-inference/domain/` n'a aucune dépendance externe
3. **Ports uniquement** : `cynic-agent-bus/ports.rs` ne contient que des traits
4. **Pas de god object** : `organ/` est déplacé dans `runtime` (légitime use case)
5. **Bus ne connaît pas l'inference** : le `SovereignRuntimePort` fait l'interface

---

## VALIDATION

```bash
# Après chaque phase :
cargo check -p cynic-inference
cargo check -p cynic-inference-runtime  
cargo check -p cynic-agent-bus
cargo check -p cynic-agent-bus-coordinator
cargo test -p cynic-agent-bus --test architecture_boundary
```

---

*Plan prêt à exécuter. Commit par phase avec preuve read-back.*
