<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Pitch CYNIC — v3 (Angle Technique)

**Public cible :** Builders, développeurs, protocoles Solana

---

## CYNIC : Infrastructure Souveraine de Jugement

### Le problème technique

Les outils de détection de scam sont des **heuristiques** : règles statiques, seuils arbitraires, confiance 0-100% sans fondement.

Aucun ne :
- Utilise plusieurs modèles indépendants
- Plafonne sa propre confiance
- Apprend de ses erreurs (mémoire inter-session)
- Suspend son jugement face à l'incertitude

### L'architecture CYNIC

```
                    ┌─────────────────┐
                    │   Stimulus      │
                    │  (mint, signal) │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │  Dog #1  │  │  Dog #2  │  │  Dog #3  │
        │ Qwen 7B  │  │ Qwen 27B │  │  Det-Dog │
        └────┬─────┘  └────┬─────┘  └────┬─────┘
             │              │              │
             └──────────────┼──────────────┘
                            │
                    ┌───────▼────────┐
                    │   Judge        │
                    │ φ-bounded      │
                    │ consensus      │
                    └───────┬────────┘
                            │
                    ┌───────▼────────┐
                    │    Verdict     │
                    │ HOWL/BARK/     │
                    │ EPOCHÉ         │
                    └───────┬────────┘
                            │
                    ┌───────▼────────┐
                    │   Crystal      │
                    │ Coherence      │
                    │ Machine        │
                    └────────────────┘
```

### Les 6 Axiomes

Chaque Dog évalue sur 6 axes indépendants :

| Axiome | Question |
|--------|----------|
| FIDELITY | Fidèle à la vérité ? |
| PHI | Structurellement harmonieux ? |
| VERIFY | Testable ? Réfutable ? |
| CULTURE | Respecte les patterns et traditions ? |
| BURN | Efficient ? Minimal waste ? |
| SOVEREIGNTY | Préserve l'agence et la liberté ? |

### Spécifications techniques

- **Kernel** : Rust, ~15K lignes, 340 tests
- **Inference** : LLM locaux (Qwen 7B + 27B) + Dog déterministe
- **Storage** : SurrealDB (cristaux), PostgreSQL (KAIROS)
- **API** : REST + MCP (Model Context Protocol)
- **Souveraineté** : hardware propre, zéro cloud, nftables firewall
- **Réseau** : Tailscale mesh

### K15 — Consumer Law

Chaque capteur a un consommateur qui **agit**. Pas de data lake sans action.

### CCM — Crystal Coherence Machine

Mémoire inter-session : verdicts → patterns → cristaux → prompts Dogs.
Seul mécanisme garanti de persistance entre sessions Claude.

---

## Intégration

```bash
# API REST
curl -X POST cynic:3030/judge \
  -d '{"content":"MINT_ADDRESS","domain":"token-analysis"}'

# MCP Server
# Compatible avec Claude, Cursor, etc.
```

---

## Roadmap technique

1. **Fait** : Kernel, 3 Dogs, CCM, Hermes, KAIROS, 20K+ verdicts
2. **En cours** : Wallet behavioral enricher, mint-permit
3. **Prochain** : Programme Solana (Pinocchio), Dog décentralisé
4. **Vision** : Infrastructure de jugement pour l'agentic economy

---

*Version technique — voir `futardio-pitch-unified-v6.md` pour la version complète.*
