# CYNIC : Manuel d'Ingénierie Système pour Agents Autonomes
# (Agent Systems Engineering Playbook)

**Target Audience :** Codex CLI, OMP (Oh My Pi), Pi, Claude Code, Droid, OpenCode & Autonomous Harnesses.  
**Invariant Fondamental :** *« Discipline matters more than model capabilities. If your docs don't answer Where, What, How, Why, the agent will guess and make a mess. »*

---

## I. Les 4 Piliers Inviolables de la Recette d'Ingénierie

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. MODULARITÉ                                                               │
│    • Plafond strict de 400 lignes par fichier.                              │
│    • Découpage obligatoire : types → policy → port → adapter → render → CLI.│
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. DOMAIN-DRIVEN DESIGN (DDD)                                               │
│    • Séparation du Domaine Pur (zéro I/O) et des Adaptateurs d'I/O.         │
│    • Ports définis par des Traits Rust, adaptateurs interchangeables.       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. SPÉCIFICATIONS DOULOUREUSEMENT EXPLICITES (Painfully Explicit Specs)      │
│    • Chaque brique répond rigoureusement à : WHERE, WHAT, HOW, WHY.         │
│    • Aucun comportement implicite ou non typé.                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. DOCUMENTATION EXHAUSTIVE (Excessive Documentation)                       │
│    • Schémas JSON versionnés (v1, v2) documentés au niveau des types.       │
│    • Invariants cryptographiques et thermodynamiques explicités.            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. Matrice d'Orientation pour Agents : Où, Quoi, Comment, Pourquoi

| Question | Règle d'Or CYNIC pour l'Agent | Emplacement Canonique |
|---|---|---|
| **WHERE ?** | Ne cherche jamais au hasard. Chaque type et logique a un unique propriétaire de couche. | `crates/cynic-protocol` (Types)<br>`crates/cynic-host` (Substrat)<br>`crates/cynic-inference` (IA)<br>`crates/cynic-intel` (E-Score)<br>`crates/cynic-cli` (Façade) |
| **WHAT ?** | Ne devine jamais un format. Chaque commande émet un JSON structuré valide contre un schéma. | `contracts/foundation.json`<br>`infra/agent-exposure.json`<br>`infra/surface-registry.json` |
| **HOW ?** | Ne lance jamais de commande mutatrice aveugle. Respecte le cycle en 5 temps. | $\text{Desired} \to \text{Observe} \to \text{Reconcile} \to \text{Bounded Action} \to \text{Read-Back}$ |
| **WHY ?** | Comprends la raison avant d'éditer. Pas de refactoring gratuit, pas de suppression de garde. | `docs/specs/*.md`<br>`AGENTS.md` |

---

## III. Protocole Opérationnel d'un Tour d'Agent

Avant d'écrire la moindre ligne de code, tout agent doit exécuter cette séquence d'auto-discipline :

```text
 1. LOCALISER LE DOMAINE (WHERE)
    ├── Vérifier les baux actifs (`cynic_coord_who`)
    └── Réserver la surface (`cynic_coord_claim {"domain": "...", "ttl_ms": 3600000}`)
            │
            ▼
 2. ÉVALUER L'INVARIANT (WHAT & WHY)
    ├── Lire la spécification dans `docs/specs/`
    └── Identifier les types purs dans `cynic-protocol`
            │
            ▼
 3. IMPLÉMENTER PAR COUCHES (HOW)
    ├── Écrire le Trait (Port) dans `ports/`
    ├── Coder la logique métier pure dans `domain/`
    └── Implémenter l'adaptateur I/O dans `adapters/`
            │
            ▼
 4. VÉRIFIER & AUDITER (PROOF)
    ├── `cynic dev audit-v2 --json` (Zéro violation critique, zéro fuite machine)
    ├── `make check` (Compilation stricte Layer 0)
    └── Libérer le bail (`cynic_coord_release`)
```

---

## IV. Répertoire des Spécifications Détaillées

Tout agent intervenant sur le dépôt doit obligatoirement consulter la spécification de son domaine :

1. **Coordination & Baux Multi-Agents** : [`docs/specs/MULTI_AGENT_COORDINATION_SPEC.md`](./MULTI_AGENT_COORDINATION_SPEC.md)
2. **Plan d'Inférence & Gestion VRAM** : [`docs/specs/INFERENCE_PLANE_SPEC.md`](./INFERENCE_PLANE_SPEC.md)
3. **E-Score & Arbre de Merkle 7D** : [`docs/specs/ESCORE_AND_FUTARCHY_SPEC.md`](./ESCORE_AND_FUTARCHY_SPEC.md)
4. **Conformité Hôte & Métriques** : [`docs/specs/RUNTIME_CONFORMANCE_AND_METRICS_SPEC.md`](./RUNTIME_CONFORMANCE_AND_METRICS_SPEC.md)
5. **Anthropologie & Vision Nation** : [`docs/CYNIC_NATION_ANTHROPOLOGY.md`](../CYNIC_NATION_ANTHROPOLOGY.md)
6. **Doctrine du Catalyseur** : [`docs/CYNIC_THE_CATALYST.md`](../CYNIC_THE_CATALYST.md)
