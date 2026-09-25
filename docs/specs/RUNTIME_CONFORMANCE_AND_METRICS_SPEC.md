# Spécification d'Ingénierie Système : Conformité Runtime & Métriques Résiduelles

**Status :** Normatif & Contractuel (Layer 0 Host Qualification — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. CONTRATS DE HARNAIS (crates/cynic-protocol/src/runtime/)                 │
│    • layers.rs, proofs.rs, types.rs : RuntimeProofBundle, ConformanceReport │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. DÉTECTION & CONFORMANCE (crates/cynic-host/src/conformance.rs)           │
│    • observe_host_conformance : Qualification de l'hôte sans mutation.      │
│    • detection_engine/ : Sondes D-Bus, Process, VRAM, Provenance.           │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. CALCULATEUR DE MÉTRIQUES (crates/cynic-host/src/runtime_metrics.rs)      │
│    • host_runtime_metrics_with_external_proof : Agrégation des scores.      │
│    • Quantification du budget d'inconnu résiduel (Residual Unknown).        │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. COMMANDES D'EXPOSITION CLI (crates/cynic-cli/src/runtime_surface/)       │
│    • cynic runtime harness --json                                           │
│    • cynic runtime conformance --json                                       │
│    • cynic runtime metrics --json --proof <bundle.json>                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Modèles)

### 1. La Règle des Métriques Industrielles
* **100% n'est PAS l'objectif global aveugle** : Chaque dimension possède son propre `target_percent`.
* **« Unknown is Data »** : Une sonde qui ne peut pas observer un composant distant (ex: un nœud Proxmox inaccessible dans un Closed Lab) renvoie `null`. La valeur `null` n'est pas falsifiée en succès : elle alimente explicitement le pourcentage de **risque résiduel inconnu** (`residual_unknown_percent`).

### 2. La Hiérarchie des Harnais d'Exécution
```text
  [Lab (Closed Sandbox)]  ──►  [Conformance (Inspection)]  ──►  [Staging (LXC VM)]  ──►  [Canary]  ──►  [Production]
      Zéro Risque Hôte               Zéro Mutation                  Miroir Isolé       Sous-ensemble      SSOT Vivant
```

---

## III. HOW (Comment s'exécute la qualification d'un nœud ?)

```text
 1. Invocation CLI : `cynic runtime conformance --json`
            │
            ▼
 2. Détection Dynamique du Substrat (`detection_engine`)
    ├── Superviseur (Systemd User vs System vs Process)
    ├── Matériel & GPU (Direct Syscall sans shell)
    └── Provenance d'Installation (UvTool vs Distro vs Local)
            │
            ▼
 3. Évaluation des Invariants d'Hôte (`conformance.rs`)
    ├── Séparation du principal (`cynic` vs opérateur)
    ├── Étanchéité du stockage (`/var/lib/cynic`)
    └── Connectivité chiffrée (WireGuard Synapse)
            │
            ▼
 4. Émission du Rapport de Preuve (`RuntimeProofBundle`)
            │
            ▼
 5. Calcul de Maturité & Inconnu Résiduel (`runtime_metrics.rs`)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi séparer le Lab de la Conformance ?**
   * Le `lab` génère des plans et des bundles virtuels pour tester des topologies multi-nœuds sans toucher à la machine réelle.
   * La `conformance` observe la machine physique réelle sans jamais appliquer de mutation destructive.
2. **Pourquoi chiffrer l'Inconnu Résiduel ?**
   * Dans les systèmes critiques, la pire menace est la *fausse certitude*. Quantifier ce qu'on ignore permet aux opérateurs et aux agents de connaître précisément le périmètre d'incertitude avant d'engager une action de déploiement.
