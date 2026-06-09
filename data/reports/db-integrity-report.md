# Rapport d'Intégrité : Base de Données (SurrealDB 3.x)

## Problème Identifié
Lors de la montée en version vers SurrealDB 3.x, une régression a été identifiée dans la couche de stockage du kernel (`cynic-kernel`). Les requêtes SQL sélectionnant des champs spécifiques (Subset Selection) tout en ordonnant par un champ absent de cette sélection (`ORDER BY created_at`) échouaient avec l'erreur :
`Parse error: Missing order idiom created_at in statement selection`.

Ce bug empêchait le kernel de charger la **fenêtre EPOCHÉ** au démarrage, rendant le système aveugle aux anomalies de désaccord entre Dogs pendant les premières heures d'opération.

## Correctif Appliqué (v26.5.15)
Le fichier `cynic-kernel/src/storage/surreal/verdicts.rs` a été patché pour inclure explicitement le champ de tri dans la clause `SELECT`.

```rust
// Avant
format!("SELECT max_disagreement FROM verdict ORDER BY created_at DESC LIMIT {capped}");

// Après (Fix SurrealDB 3.x)
format!("SELECT max_disagreement, created_at FROM verdict ORDER BY created_at DESC LIMIT {capped}");
```

## État de la Compilation
*   **Release (opt-level=3)** : Échec par SIGSEGV (Dette de RAM sur la machine de dev).
*   **Debug (dev profile)** : Succès. Le binaire a été testé et valide le chargement correct des 1000 échantillons de la fenêtre EPOCHÉ.

## Recommandations pour Hermes Agent (Maintenance DB)
1.  **Strict Selection** : Toujours inclure les champs utilisés dans `ORDER BY` au sein du `SELECT`.
2.  **Indexation** : Vérifier périodiquement l'existence des index via `INFO FOR TABLE verdict;`. L'index `verdict_created_idx` est critique pour les performances de démarrage.
3.  **Migration Linux** : Le pivot Linux de cet après-midi doit inclure une vérification de la version de SurrealDB (Cible: 3.x stable) pour maintenir cette compatibilité.

---
*Audit scellé dans le Proof-of-History le 2026-06-05.*
