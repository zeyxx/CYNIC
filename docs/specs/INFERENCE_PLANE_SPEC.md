# Spécification d'Ingénierie Système : Le Plan d'Inférence (*Inference Plane*)

**Status :** Normatif & Contractuel (Layer 2 Substrate Execution — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. CONTRATS & PORTS (crates/cynic-inference/src/ports/)                     │
│    • inference_port.rs, chat_port.rs, embedding_port.rs, local_runtime_port │
│    • Invariant : Interfaces asynchrones pures découplées des moteurs.       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. DOMAINE MÉTIER (crates/cynic-inference/src/domain/)                      │
│    • slot_semaphore.rs : Gestion de la concurrence VRAM / Slots.            │
│    • gguf_meta.rs      : Preflight et extraction des métadonnées de tenseurs│
│    • model_registry.rs : Catalogue des backends (LlamaCpp, OpenAICompat).   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. SERVICES D'APPLICATION (crates/cynic-inference/src/application/)         │
│    • local_runtime_service.rs : Cycle de vie des processus d'inférence.     │
│    • chat_service.rs          : Orchestration des tours de conversation.    │
│    • hf_discovery.rs          : Téléchargement et catalogage HuggingFace.   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. ADAPTATEURS D'INFRASTRUCTURE (crates/cynic-inference/src/adapters/)      │
│    • local/   : Adaptateurs SystemdUserLocalRuntime & ProcessLocalRuntime.  │
│    • hf/      : Adaptateur HfCliAdapter.                                    │
│    • ods/     : Synchronisation avec l'environnement Osmantic ODS.          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Modèles)

### 1. Invariants d'Ingénierie
1. **Isolation Métabolique** : Le crash d'un moteur d'inférence (ex: `llama-server` OOM sur GPU) ne doit jamais faire crasher le `cynic-kernel` ni corrompre le journal d'état (`Proof of History`).
2. **Allocation Concurrente Bornée (Slot Semaphores)** :
   * Chaque modèle chargé dispose d'un pool de slots fini.
   * `SlotSemaphore` garantit qu'aucune requête n'est soumise à un moteur saturé ; les requêtes excédentaires attendent dans une file FIFO bornée (`InferenceQueue`).
3. **Validation Preflight GGUF** :
   * Avant tout démarrage de serveur, `gguf_meta::preflight_gguf_for_engine` inspecte le header binaire du fichier `.gguf` (architecture des tenseurs, taille de contexte, nombre de couches) pour refuser un chargement incompatible avec la VRAM observée.

---

## III. HOW (Comment s'exécute le cycle d'inférence ?)

```text
 1. Demande d'Inférence (CLI / MCP / Agent)
            │
            ▼
 2. Résolution du Modèle (`resolve_chat_model_id`)
    ├── Recherche dans le Vault local (`/var/lib/cynic/models`)
    └── Fallback backend externe si configuré (`CYNIC_BACKEND_*`)
            │
            ▼
 3. Vérification des Slots Disponibles (`SlotSemaphore::acquire`)
    ├── Si slot libre : Permis accordé
    └── Si saturé : Mise en file d'attente bornée
            │
            ▼
 4. Routage & Exécution (`LocalInferenceRuntimePort`)
    ├── Processus Local (`llama-server`) via Unix Socket / HTTP
    └── Stream SSE vers le client (`StreamChunk`)
            │
            ▼
 5. Libération du Permis & Émission de la Preuve d'Inférence
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi extraire l'inférence hors de `cynic-kernel` ?**
   * Le kernel a pour mission de garantir la **cohérence cryptographique et la vérité d'état**.
   * Les moteurs d'inférence sont des dépendances lourdes (C++, CUDA, Metal, ROCm) sujettes à des évolutions rapides et des instabilités mémoire. Les isoler dans `cynic-inference` préserve la robustesse de couche 0 du Kernel.
2. **Pourquoi supporter à la fois `Systemd` et `Process` ?**
   * En production sur serveur dédié, `systemd` user units assure la persistance et le redémarrage automatique après reboot.
   * En conteneur de test, en Closed Lab ou sous Windows, le mode `ProcessLocalRuntime` assure l'exécution sans dépendre de l'init systemd.
3. **Pourquoi le preflight binaire GGUF ?**
   * Évite les boucles de crash-restart infinies (`CrashLoopBackOff`) en rejetant immédiatement les fichiers corrompus ou surdimensionnés.
