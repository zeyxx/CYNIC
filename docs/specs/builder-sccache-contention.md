# Diagnostic : le builder ne peut plus exécuter de tests

**Date** : 2026-10-20
**Machine** : builder `10.10.10.119` (`cynic-builder-ssh`)
**Symptôme** : tout `cargo test` sur le builder meurt avant de compiler quoi que ce soit.

```text
error: process didn't exit successfully: `sccache rustc -vV` (exit status: 101)
--- stderr
thread 'main' panicked at src/jobserver.rs:111:18:
failed to spawn helper thread: Os { code: 11, kind: WouldBlock,
message: "Resource temporarily unavailable" }
```

## Ce que le symptôme N'EST PAS

| Hypothèse | Test | Verdict |
|---|---|---|
| La stack de 8 GiB est trop grosse | 256 MiB → **même panic** | ÉLIMINÉE |
| Le noyau refuse les commits | `pthread_create` → `Success` ; `Committed_AS` 795 Go / `CommitLimit` 16,5 Go (47×) mais surcommit `0` | ÉLIMINÉE |
| Le serveur sccache est wedgé | `--show-stats` répond, un nouveau client `rustc -vV` répond en 1 s | ÉLIMINÉE |
| Le cache sccache est corrompu | stats cohérentes : 21 886 requêtes, 12 912 hits, 3 758 misses | ÉLIMINÉE |

## Ce que le symptôme EST

**Le serveur sccache se dégrade sous la charge tierce et ses clients s'empilent.**

Mesures :

```text
serveur sccache  : PID 264353, 2774 s, NLWP 71 -> 33 threads (il PERD des threads)
clients en attente : 11 (4 root, 7 cynic)
avancement       : 21883 -> 21886 requêtes en 10 s (il avance, mais à peine)
```

Les clients bloqués portent un temps CPU de `4123153216` s (≈ 130 ans) : ce sont des
enfants non-moissonnés, pas des processus vivants. Ils pointent tous vers le **même**
serveur.

### Le mécanisme

`crates/cynic-node/src/ports/build_runner_adapter.rs:465` pose :

```rust
argv.push(format!("RUSTC_WRAPPER={}", default_sccache_bin().display()));
argv.push(format!("SCCACHE_DIR={dir}"));
```

Le serveur sccache est **unique pour toute la machine**. Le pipeline CYNIC (`SCCACHE_DIR=/var/lib/cynic/sccache`)
et les builds root tiers (`/root/.cache/sccache`, ou leur propre socket) **partagent le même
processus serveur**. Sur une machine déjà saturée, ce serveur perd des threads ; ses clients
restent en attente ; `cargo` interprets l'attente comme un échec de fork et lève
`WouldBlock`.

Le message est donc **trompeur** : ce n'est pas « pas de mémoire », c'est « le cache Compiler
partagé ne répond plus ».

### Le facteur aggravant mesuré

18 à 20 `cargo build` **root** issues d'un checkout de travail tiers (`/tmp/cynic-msg`,
commit `b9b56a37`) tournent en parallèle, avec un target dir séparé — donc **zéro
contamination disque**, mais une **contention totale** sur le serveur sccache et sur le
`shared-target`.

## La correction de code déjà faite (et vérifiée)

`crates/cynic-build/src/build/stack_budget.rs` réserve la stack **par crate** au lieu de
8 GiB pour le workspace entier (6 crates sur 48 l'exigent). C'est un vrai défaut — mais
**ce n'est pas la cause de ce panic** : 256 MiB reproduit le même échec.

## Ce qu'il faut décider

Ce n'est pas une dette de code : c'est de l'**exclusion de machine**. Trois agents
partagent un builder de 8 Go sans aucune isolation.

1. **Isoler le cache par agent** : `SCCACHE_DIR` par branche/agent au lieu d'un serveur
   unique. Supprime la contention, mais chaque agent a son cache (plus de cache hits
   croisés).
2. **Réserver le builder** : un seul agent à la fois, les autres attendent. Simple, mais
   sérialise tout le monde.
3. **Déployer sccache par worker** (`SCCACHE_SERVER_PORT` distinct) : même cache disque,
   serveurs distincts. Le plus proche du vrai multi-tenancy.

Le build root tiers n'a **pas** été tué : l'opérateur a tranché deux fois qu'on attend la
fin, sans feu explicite.
