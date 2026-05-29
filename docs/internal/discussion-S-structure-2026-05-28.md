<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Discussion Structure SAS — 2026-05-28

**Participants :** T., S. (+ notes Claude)
**Sujet :** Structuration juridique B&C en SAS

---

## Contexte

Préparation Futardio. Besoin de clarifier la structure juridique avant de pitcher aux investisseurs.

## Contributions de chaque membre

### Par projet

- **T.** → CYNIC (fondateur, 100% kernel Rust), GASdf, HolDex (50/50 avec G.)
- **G.** → ASDelegate (owner), HolDex (50/50 avec T.), ASDForecast, CultScreener
- **S.** → B&C (owner), intégration communauté, business dev

### Répartition technique (commits estimés)

| Projet | Commits | Langages | Contributeur principal |
|--------|---------|----------|----------------------|
| CYNIC | 1500+ | Rust, Python | T. |
| B&C | ~500 | TypeScript | S. |
| HolDex | 766 | JavaScript | T. + G. (50/50) |
| ASDelegate | ~200 | JavaScript | G. |
| CultScreener | ~150 | JavaScript | G. |
| GASdf | ~300 | TypeScript | T. |
| KAIROS | ~400 | Python | T. |

---

## Options de structure

### Option A : SAS classique
- Capital social : 1 000€ minimum
- Parts sociales entre T., G., S.
- Président : à définir

### Option B : SAS avec pacte d'associés
- SAS + pacte définissant les rôles, vesting, gouvernance
- Plus flexible pour intégrer un investisseur plus tard

### Option C : Attendre
- Pas de structure pour l'instant
- Présenter le projet comme collectif/association
- Risque : moins crédible pour investisseurs

---

## Points de discussion

### Equity split
- **Répartition?** À définir. Tenir compte: CYNIC = T., B&C = S., CultScreener = G., HolDex = 50/50 T./G.
- Critères : contribution technique, propriété intellectuelle, rôle futur
- Question : est-ce que tous les projets entrent dans la SAS ou seulement certains ?

### Rôles
- **Président SAS** : probablement T. (projet central = CYNIC)
- **DG** : S. (business dev, communauté)
- **Associé technique** : G. (contribution HolDex + CultScreener)

### Propriété intellectuelle
- Les projets personnels (ASDelegate pour G., B&C pour S.) restent-ils en dehors de la SAS ?
- Le code CYNIC (open source sur GitHub) — comment protéger la PI ?
- Le script `anteriorite-snapshot.sh` comme preuve d'antériorité

### Vesting
- Proposer un vesting 4 ans avec cliff 1 an (standard startup)
- Accélération en cas de cession/exit

---

## Décisions prises

1. **Structure :** SAS avec pacte d'associés (Option B)
2. **Timing :** Après Futardio, basé sur les retours investisseurs
3. **Nom :** B&C (nom de S.) comme entité parapluie
4. **Périmètre :** CYNIC + KAIROS + Hermes dans la SAS, projets personnels hors périmètre
5. **Répartition :** À finaliser après discussion à trois (T., G., S.)

---

## Actions

- [ ] T. : préparer le pitch unifié (CYNIC + B&C)
- [ ] S. : identifier un avocat pour la SAS
- [ ] G. : confirmer son engagement et ses attentes
- [ ] Tous : définir la répartition equity avant Futardio
- [ ] T. : anteriorite-snapshot.sh pour preuve d'antériorité

---

*Document interne — ne pas diffuser.*
