# Alphas & Subtilités du Compilateur Rust — CYNIC

STATUS: Spécification canonique des propriétés profondes de `rustc`, `LLVM` et de la chaîne de compilation pour les agents autonomes (Hermes, Codex, Talaria) et développeurs CYNIC.
AUTORITÉ: Transpose les leçons des crashes réels (2026-08-18 / 2026-08-30), `contracts/foundation.json` (§472, §481, §489) et `infra/SYSTEM_AGENTIC_MAPPING.md`.

---

## 1. La Frontière de Visibilité : Binaire (`main.rs`) vs Bibliothèque (`lib.rs`)

En Rust, une crate peut exposer simultanément une cible bibliothèque (`src/lib.rs`) et un binaire (`src/main.rs`). Leurs modèles d'analyse de code mort (`dead_code`) sont **fondamentalement opposés** :

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                    MODÈLE DE VISIBILITÉ DU COMPILATEUR RUST                  │
├──────────────────────────┬───────────────────────────────────────────────────┤
│ Bibliothèque (src/lib.rs) │ Modèle de l'API Ouverte :                         │
│                          │ Tout élément `pub` est considéré comme une API    │
│                          │ exportée. Il n'est JAMAIS marqué comme dead_code, │
│                          │ même s'il n'est pas appelé en interne.            │
├──────────────────────────┼───────────────────────────────────────────────────┤
│ Binaire (src/main.rs)    │ Modèle de l'Arbre Clos :                          │
│                          │ Tout symbole doit être strictement atteignable    │
│                          │ depuis `fn main()`. Tout élément non atteint est  │
│                          │ marqué en dead_code, même s'il est `pub`.         │
└──────────────────────────┴───────────────────────────────────────────────────┘
```

### Invariant Architectural : *Library-First Pattern* (§472)
- **Tout le code de domaine, les usecases, les ports et les adaptateurs appartiennent à `src/lib.rs`**.
- `src/main.rs` ne doit être qu'une coquille vide de 20 à 40 lignes (Thin Facade) qui parse les arguments CLI (via `clap`) et appelle la fonction publique de sa propre bibliothèque :
  ```rust
  fn main() -> std::process::ExitCode {
      let args = Cli::parse();
      cynic_node::node_control_runtime::run(&args.config, args.check)
  }
  ```
- **Anti-pattern formellement interdit** : Ne jamais déclarer de sous-modules dans `main.rs` avec `#[path = "..."]` pour tenter de contourner la visibilité de `lib.rs`.

---

## 2. Monomorphisation & Saturation de Pile LLVM (DWARF Frame Emitter)

Sur les bases de code volumineuses (ex: `cynic-host` >80k LOC, `cynic-kernel` >50k LOC) contenant des macros dérivées récursives (`serde`, `tokio`, `tracing`), le compilateur peut s'interrompre avec `SIGSEGV` (signal 11).

```text
rustc crash stack trace signature:
llvm::MCExpr::evaluateAsRelocatableImpl
llvm::MCDwarfFrameEmitter::Emit
llvm::legacy::PassManagerImpl::run
```

### La Cause Réelle
Le crash n'est pas une erreur de syntaxe ou de logique, mais un **dépassement de pile récursive** du générateur de tables de débogage DWARF de LLVM combiné à la profondeur d'instanciation des templates de monomorphisation.

### Les Règles d'Exécution
1. **Désactiver la génération de debuginfo en cycle de test/check** :
   ```bash
   RUSTFLAGS="-C debuginfo=0" cargo check
   ```
   `-C debuginfo=0` supprime 90% des allocations de table DWARF tout en conservant 100% de la rigueur du typage et des lifetimes.
2. **Dimensionner la pile du compilateur pour les tests complets** :
   ```bash
   ulimit -s unlimited
   RUST_MIN_STACK=8589934592 RUSTFLAGS="-C debuginfo=0" cargo test -p <crate> --lib <module>
   ```

---

## 3. Les Pièges d'Implémentation de Serde (`#[serde(tag = "...")]`)

### Le Piège du Tag Interne et des Visiteurs Scalaires
Lorsqu'un enum dérive `Deserialize` avec un tag interne :
```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event { ... }
```
La macro `serde_derive` génère un désérialiseur interne complet avec des méthodes `visit_u64`, `visit_i64`, `visit_str`.

### L'Incompatibilité Mathématique `Eq` vs `f64`
En Rust standard :
- Les flottants `f64` n'implémentent **PAS** le trait `Eq` car en norme IEEE 754, $NaN \neq NaN$ (l'égalité n'est pas réflexive).
- `f64` implémente uniquement `PartialEq`.
- **Règle** : Toute structure contenant un champ flottant (taux de redémarrage, pourcentages, métriques CPU/RAM) doit dériver :
  ```rust
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] // SANS Eq
  ```
  Tenter de dériver `Eq` sur une structure portant un `f64` provoque une rupture de contrat immédiate (`trait bound f64: Eq is not satisfied`).

---

## 4. Trait Objects (`dyn Trait`) & Supertraits

Lorsqu'un port ou service manipule des références dynamiques `Arc<dyn StoragePort>` :
- Les traits objets perdent l'accès aux traits qui ne sont pas explicitement exigés comme **supertraits** dans la définition du trait.
- Si `StoragePort` n'exige pas `std::fmt::Debug`, aucune structure contenant un `Arc<dyn StoragePort>` ne peut dériver automatiquement `#[derive(Debug)]`.

### Règle d'Or des Ports CYNIC
Tous les traits de ports doivent déclarer leurs supertraits requis :
```rust
pub trait StoragePort: std::fmt::Debug + Send + Sync {
    // ...
}
```
Si un trait objet externe ne peut pas implémenter `Debug`, la structure hôte doit implémenter `std::fmt::Debug` manuellement :
```rust
impl std::fmt::Debug for BuildScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildScheduler").finish()
    }
}
```

---

## 5. Le `dead_code` comme Capteur Épistémique Continu

Dans CYNIC, `--deny=dead_code` n'est pas une contrainte bureaucratique, c'est un **capteur de vérité du substrat**.

| Observation `dead_code` | Diagnostic Architectural | Action Conforme |
|---|---|---|
| Champ de struct jamais lu | Donnée récoltée pour rien ou calculée puis écrasée par `0` | Réparer le calcul ou supprimer le champ |
| Fonction `pub(crate)` jamais appelée | Port écrit mais non raccordé au runtime/scheduler | Raccorder au usecase ou exposer en API `pub` |
| Constante/Type orphelin | Vestige d'une ancienne version ou prototype abandonné | Supprimer (`delete weightless code`) |
| Double de test dans la prod | Fixture égarée hors du module de test | Déplacer sous `#[cfg(test)]` |

**Règle absolue** : Ne jamais masquer un warning avec `#![allow(dead_code)]` global sans avoir au préalable arbitré l'appartenance du code à l'une des catégories ci-dessus.
