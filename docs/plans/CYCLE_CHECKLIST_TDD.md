
## Unification / Canonicalisation — 3 échelles
Chaque cycle doit vérifier aux 3 échelles, pas seulement déplacer du code :
1. **Intra-crate (FCIS / Hexagonal / DDD)** : domaine pur au centre, ports autour, adapters en périphérie. Patterns uniques dans la crate (ARCH-002, ARCH-006).
2. **Inter-crates** : doublons entre crates du même niveau -> UNE SSOT (ex. SSH -> cynic-ssh, ARCH-003). Choisir LA crate canonique.
3. **DAG (couches L0/L1/L2)** : types communs remontés vers L0 (cynic-protocol/types), implémentations redescendues vers L2. Jamais de dépendance vers le haut (ARCH-004).
Checklist par cycle :
- [ ] Intra-crate : FCIS pur ? patterns hexagonaux ? DDD respecté ?
- [ ] Inter-crates : doublons trouvés et unifiés vers une SSOT ?
- [ ] DAG : dépendances vers le haut supprimées ? types au bon niveau ?

## Gates avant merge (discipline)
- [ ] cargo fmt --all (appliquer avant commit, jamais après)
- [ ] cargo fmt --all --check (gate CI)
- [ ] cargo clippy --workspace -- -D warnings (warnings = erreurs, rustup component add clippy)
- [ ] cargo test --workspace (vert, échecs préexistants documentés par comparaison pristine)
- [ ] Audit CYNIC-ARCH-* : 0 violation sur les crates touchées
