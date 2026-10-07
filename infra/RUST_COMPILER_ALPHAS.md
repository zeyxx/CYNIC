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
---

## 6. Le Modèle MIXTE : un crate qui expose `lib.rs` ET `src/main.rs`

### 6.1. Pourquoi §1 ne suffit pas

Le §1 décrit deux modèles comme *opposés* :

- **bibliothèque** (`src/lib.rs`) : API ouverte, `pub` n'est jamais `dead_code`
- **binaire** (`src/main.rs`) : arbre clos, tout doit être atteignable depuis `fn main()`

Il existe un **troisième cas** que ce document ne traitait pas : un crate qui expose
les deux cibles dans le même paquet Cargo. C'est le cas de `cynic-cli`, dont le
`src/main.rs` est :

```rust
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! `cynic` binary — thin CLI facade conforming to Library-First Pattern (§472).

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    cynic_cli::run_main().await
}
```

C'est un `main.rs` conforme au pattern Library-First, donc le modèle *devrait* être
celui de la bibliothèque : `pub` exporté, jamais `dead_code`.

### 6.2. Ce que la compilation sait réellement

**Mesuré sur le workspace (48 crates, 12 binaires déclarés) :**

| Cible binaire | Chemin | Crate a `lib.rs` ? | Modèle effectif |
|---|---|---|---|
| `cynic` | `src/main.rs` | oui | **MIXTE** |
| `cynic-kernel` | `src/main.rs` | oui | **MIXTE** |
| `cynic-mcp` | `src/main.rs` | oui | **MIXTE** |
| `cynic-node` | `src/main.rs` | oui | **MIXTE** |
| `cynic-pool` | `src/main.rs` | oui | **MIXTE** |
| `cynic-topology` | `src/bin/cynic-topology.rs` | oui | racine séparée |
| `cynic-host-exec` | `src/bin/cynic-host-exec.rs` | oui | racine séparée |
| `cynic-lxc-exec-bridge` | `src/bin/cynic-lxc-exec-bridge.rs` | oui | racine séparée |
| `cynic-node-enroll` | `src/bin/enroll_grant.rs` | oui | racine séparée |
| `zeroclaw-solana-demo` | `src/bin/demo.rs` | oui | racine séparée |
| `cynic-network-actuator` | `src/actuator_main.rs` | oui | racine déclarée explicitement |

**5 binaires sur 11 sont MIXTE** (mesuré avant la suppression de `cynic-audit`,
qui a réduit le total de 12 à 11). Le `dead_code` est analysé par *cible*, pas par
*paquet* : la bibliothèque est un arbre ouvert même quand sa racine binaire est un
arbre clos. Un `pub fn` de `lib.rs` que seul `main.rs` n'atteint pas reste exporté et
ne produit **aucun** warning.

### 6.3. Risque : le MIXTE masque la dette par construction

An agent working on a MIXED target can believe the compiler verifies the code in
`lib.rs`. It only does so when that code is genuinely `pub`.
La conséquence opérationnelle est celle du §5 : **`dead_code` ne peut pas être
l'unique capteur de vérité sur une cible MIXTE**, parce qu'une part du code échappe
structurellement à l'analyse. Les tests restent le seul capteur de cette zone.

**Règle** : sur toute cible MIXTE, un symbole `pub` sans appelant ni test doit être
traité comme du `dead_code` et subir l'arbitrage du §5 — le compilateur ne le fera
pas à notre place.

---

## 7. `codegen-units` est une variable de crash, pas un réglage de performance

### 7.1. Ce que le workspace déclare aujourd'hui

```toml
[profile.dev]
codegen-units = 16
debug = false
[profile.dev.package.cynic-protocol]
codegen-units = 1

[profile.test]
codegen-units = 16
incremental = false
debug = false
```

**Le §2 de ce document ne mentionne `codegen-units` nulle part.** C'est pourtant le
paramètre le plus instable de la chaîne de compilation CYNIC, et les commentaires du
profil en documentent l'instabilité :

```toml
# WHY: codegen-units=1 was a mitigation for nightly-2026-04-06 LLVM fan-out SIGSEGV.
# As of Rust 1.95.0 / LLVM 22.1.2, codegen-units=1 itself triggers non-deterministic
# SIGSEGV in FastISel::selectInstruction on random crates (yoke-derive, zerovec,
# regex-syntax, serde_json). codegen-units=16 (default) passes 685 tests reliably.
#
# PATCH (local ice isolation): clippy-deny groups + codegen-units=16 → MIR overflow
# SIGSEGV on cynic-protocol serde_json graph (rustc 1.96.0). Drop to 1 for the host
# axis (protocol/host/node/cli) to avoid parallel-codegen OOM/SIGSEGV locally.
```

### 7.2. Le problème : la valeur est une réaction, pas une propriété

La séquence documentée est **oscillante par construction** :

1. `codegen-units=1` évite un SIGSEGV LLVM fan-out → on l'utilise
2. `=1` provoque un SIGSEGV non déterministe sur `serde_json` → on passe à `16`
3. `=16` provoque un MIR overflow sur le graphe `serde_json` de `cynic-protocol` → on
   repasse à `1` **pour ce seul crate**, alors que le commentaire annonce un drop
   « pour l'axe host (protocol/host/node/cli) » alors que **seul `cynic-protocol`
   reçoit la valeur 1** dans le manifeste.

Le commentaire annonce quatre crates ; le manifeste n'en configure qu'un.

**Aucun de ces crashs n'est rattaché à une condition reproductible.** Ils sont décrits
comme « non-déterministes » et « sur des crates aléatoires », ce qui est un signal
d'absence de diagnostic : une valeur qui fluctue au hasard ne peut pas être gouvernée
par une règle.

### 7.3. Les versions citées ne sont pas celles du pin

Le pin effectif est `1.95.0` (`rust-toolchain.toml`). Les commentaires du profil
c Evoquent `1.96`, `1.96.0` et `1.98`. Un comment qui pilote un diagnostic vers une
version que le workspace n'utilise pas coûte plus cher qu'il ne rapporte : il fait
perdre le temps de chercher un crash d'une version qui n'est pas celle qui compile.

**Règle** : toute référence à une version `rustc` dans un commentaire de profil doit
être vérifiable contre `rust-toolchain.toml`. Une version citée qui ne correspond pas
au pin est une affirmation périmée, pas une observation.

---

## 8. SSOT de la toolchain : trois réponses, deux inutilisées

### 8.1. Les sources déclarées

| Source | Contenu | Lue par |
|---|---|---|
| `rust-toolchain.toml` (racine) | `1.95.0` | **rustup, par CWD** |
| `crates/*/rust-toolchain.toml` (6 fichiers) | 1.95.0 / 1.96.0 | rustup, par CWD |
| `infra/rust-toolchain.json` | table par crate + défaut `*` | `cynic-host::toolchain::specs`, `build_runner_bootstrap`, `tests/divergence_audit.rs` |
| `[profile.dev]` (Cargo.toml) | `codegen-units`, `debug`, `incremental` | cargo |

### 8.2. Ce qui se passe réellement à la compilation

**Mesuré :** rustup résout la toolchain depuis le **répertoire courant**, pas depuis
`-p <crate>`.

```
depuis la racine du workspace :                 rustc 1.95.0
depuis crates/cynic-protocol (pin 1.96.0) :     rustc 1.96.0
depuis crates/cynic-host (pin 1.96.0) :         rustc 1.96.0
```

Or le `Makefile` construit **toujours depuis la racine** avec `-p`. Conséquence :

> **Les 6 pins par crate ne s'appliquent à aucun build du `Makefile`.**

42 crates sur 48 n'ont pas de pin et héritent de la racine (1.95.0).

### 8.3. La contradiction mesurée entre le registre et les fichiers

| Crate | `rust-toolchain.toml` | `infra/rust-toolchain.json` | Verdict |
|---|---|---|---|
| `cynic-cli` | 1.96.0 | 1.96.0 | OK |
| `cynic-host` | 1.96.0 | 1.96.0 | OK |
| `cynic-inference` | 1.96.0 | **1.95.0** | **CONFLIT** |
| `cynic-kernel` | 1.95.0 | 1.95.0 | OK |
| `cynic-node` | 1.96.0 | **1.95.0** | **CONFLIT** |
| `cynic-protocol` | 1.96.0 | **1.95.0** | **CONFLIT** |
| `<défaut "*">` | racine = 1.95.0 | **1.96.0** | **CONFLIT** |

Quatre divergences. Le registre décrit une règle — « `*` vaut 1.96.0 » — qu'aucun
chemin de construction n'implémente : cargo prend le fichier **le plus proche**, donc
la racine l'emporte pour les 42 crates sans pin.

### 8.4. La contradiction la plus lourde

Le registre affirme pour `cynic-host` :

> `toolchain: 1.96.0`, `reason: "1.95.0 SIGSEGV on proc-macro2 build script (rustc ICE); 1.96.0 resolves"`

Si c'est vrai, **le `Makefile` déclenche ce SIGSEGV sur chaque build**, parce qu'il
construit depuis la racine donc sous 1.95.0. Or `make check` passe. Une seule
explication est possible : le pin par crate est **mort**, et la raison « 1.95.0
SIGSEGV » est soit périmée, soit n'a jamais été observée par ce chemin de
construction.

Tant que cette question n'est pas tranchée, le registre décrit un monde qui n'est pas
celui qui compile.

### 8.5. Ce que le registre contient déjà de périmé

`infra/rust-toolchain.json` déclare une entrée `cynic-observer` — crate supprimé
(13 340 LOC retirés, cette session). C'est exactement le défaut que les audits de
cette session traitent ailleurs : un registre qui décrit une structure disparue
continue de passer tant que personne ne le confronte au disque.

### 8.6. Le capteur existe, il a trouvé, et il est en mode baseline

`crates/cynic-host/tests/divergence_audit.rs` contient `toolchain_divergences()`, dont
le commentaire d'en-tête décrit précisément ce défaut :

> Three answers existed at once — rust-toolchain.toml (1.95.0),
> infra/rust-toolchain.json (1.95.0 and 1.96.0) and the CI using `@stable`.
> The fallback was the pinned-but-broken version, and CI followed neither.

Le test qui l'appelle est `the_compiler_has_one_pinned_version_every_consumer_agrees_on`.
Il **passe**. Il passe parce que l'assertion est :

```rust
const BASELINE_TOOLCHAIN: usize = 3;
assert!(divergent.len() <= BASELINE_TOOLCHAIN, "divergence grew from 3 to {}: ...");
```

C'est un **plafond de dette connue, pas une assertion de vérité**. Les trois
divergences acceptées aujourd'hui sont :

1. `infra/rust-toolchain.json` déclare `1.96.0` alors que `rust-toolchain.toml` épingle `1.95.0`
2. la CI installe un toolchain flottant — `.github/workflows/ci.yml:24` utilise
   `dtolnay/rust-toolchain@stable`
3. (la troisième n'a pas été isolée ; le plafond compte des valeurs distinctes)

Donc le capteur **voit** la divergence, **la compte**, et **la tolère**. C'est le
comportement voulu d'un ratchet : il interdit la croissance. Ce qu'il ne fait pas, et
ce que son nom laisse croire, c'est exiger la convergence.

Deux weaknesses du capteur lui-même, à corriger dans le même mouvement :

- **`read()` avale les fichiers absents.** `fs::read_to_string(...).ok()` rend `None`,
  et la branche CI `if let Some(ci) = read(".github/workflows/ci.yml")` saute
  silencieusement. Un test de cohérence qui ne se plaint pas quand le fichier qu'il
  surveille a disparu mesure l'absence de conflit, pas l'absence de conflit réel.
- **Les pins par crate ne sont pas comparés au registre**, seulement au pin racine.
  Les trois conflits `cynic-inference`, `cynic-node`, `cynic-protocol` (§8.3) sont
  invisibles au capteur : il compare l'ensemble des valeurs déclarées au pin racine,
  pas chaque crate à sa propre déclaration.

### 8.7. Une dette déjà payée mais non enregistrée

`no_source_is_sitting_on_disk_uncompiled` **échoue** :

```
402 LOC on disk are unreachable from a compilation root, workspace-wide.
  left: 402
 right: 557
```

`BASELINE_UNCOMPILED_LOC = 557` est le plafond ; l'observé est **402**. La dette a
**baissé de 155 LOC** — `cynic-observer` supprimé en `0680c23e`. Mais l'assertion est
`assert_eq!`, pas `<=`.

**Conséquence** : la suppression d'`audit_engine` vers un crate dédié, puis des
orchestrateurs, réduira ce chiffre à chaque tranche. Le test échouera à chaque fois
avec `left` décroissant et `right` figé. Il faut **rafraîchir la baseline à chaque
tranche qui réduit la dette**, sinon le workspace est bloqué par une amélioration.

C'est le défaut le plus coûteux des quatre que j'ai trouvés ici : un ratchet
défectueux punit celui qui paie sa dette.

### 8.8. Règles

1. **Un seul pin fait foi** : `rust-toolchain.toml`. Un registre par crate ne peut être
   qu'un *seed* d'observation (il se déclare comme tel dans `specs.rs`), jamais une
   autorité exécutée. Tant que les deux existent et divergent, le registre est une
   dette, pas une source.
2. **Toute entrée de registre doit correspondre à une crate sur disque.** Une entrée
   sans crate est périmée ; la supprimer est le seul mouvement autorisé (les registres
   sont shrink-only).
3. **Un pin par crate qui n'est pas sur le chemin de construction est une
   documentation trompeuse.** Écrire « le pin vaut X » sans écrire « cargo le lit
   depuis le CWD » produit exactement la divergence du §8.4.
4. **`toolchain_divergences()` doit être consulté avant d'ajouter une source de
   pin.** Le test existe pour cela ; l'ignorer est la cause racine du §8.4.