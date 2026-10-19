# Audit architecture + sécurité PR 137 — Verdict reviewer : REQUEST CHANGES (1 finding) → synthèse : APPROVE + dette suivie

Prisme : architecture / DAG / sécurité. Head `2601eef2`.

## Vert (reviewer + confirmé)

DAG strict (4 arêtes vers le bas, aucun cycle), aucun secret/endpoint en dur, CLI nested conforme, arêtes fondées Cargo, 9 fantômes réellement absents, huggingface L2 pertinent, scripts bootstrap cohérents CLI actuelle.

## Finding unique : `knowledge_cache_path()` impure — HORS PÉRIMÈTRE

`crates/cynic-types/src/paths.rs:116` (`create_dir_all` dans le domaine) viole FCIS. **Pré-existant** : introduit par `c6e511f4` sur `main`, aucun des 9 picks ne le touche. Même classe que les violations pré-existantes notées en #136 (synthesis.rs, vector_cache.rs).

**Décision** : ne bloque pas la tranche. Dette suivie : purifier `knowledge_cache_path()` (retourner le candidat, création côté adapter) dans un lot dédié.
