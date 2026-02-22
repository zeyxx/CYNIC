# CYNIC Documentation

> **Entry point unique pour toute la documentation CYNIC**
>
> *🐕 κυνικός | "Loyal to truth, not to comfort"*

---

## Quick Navigation

| Je suis... | Je vais à... |
|------------|--------------|
| 👤 **Nouvel utilisateur** | [01-getting-started/](./01-getting-started/) |
| 🛠️ **Développeur** | [03-reference/](./03-reference/) |
| ⚙️ **DevOps** | [05-operations/](./05-operations/) |
| 🔬 **Chercheur** | [06-research/](./06-research/) |
| 📢 **Marketing/Investisseur** | [08-marketing/](./08-marketing/) |
| 🤖 **LLM/Agent** | [_index.json](./_index.json) |

---

## Structure (Diátaxis Framework)

```
cynic/docs/
├── 01-getting-started/   # Tutorials - Apprendre à utiliser CYNIC
│   ├── quickstart.md     #   5 minutes → premier jugement
│   └── installation.md   #   Setup complet
│
├── 02-how-to/            # Guides - Résoudre des problèmes spécifiques
│   ├── judge-code.md     #   Comment juger du code
│   ├── use-skills.md     #   Utiliser les skills
│   └── deploy-daemon.md  #   Déployer le daemon
│
├── 03-reference/         # Reference - Spécifications techniques
│   ├── architecture.md   #   Architecture complète
│   ├── api.md            #   REST API
│   ├── axioms.md         #   5 axiomes + 36 dimensions
│   ├── kernel.md         #   Kernel Python v2.0
│   └── mcp-tools.md      #   90+ outils MCP
│
├── 04-explanation/       # Explanation - Comprendre les concepts
│   ├── philosophy.md     #   Cynicisme + Kabbale + φ
│   ├── organism-model.md #   CYNIC comme organisme vivant
│   └── learning-loops.md #   11 boucles d'apprentissage
│
├── 05-operations/        # Operations - Déployer et maintenir
│   ├── runbook.md        #   Procédures opérationnelles
│   ├── deployment.md     #   Render/Docker
│   └── monitoring.md     #   Health checks
│
├── 06-research/          # Research - Publications académiques
│   ├── paper-draft.md    #   Papier ICML
│   └── experiments/      #   Protocoles expérimentaux
│
├── 07-project/           # Project - Gestion de projet
│   ├── roadmap.md        #   Timeline φ-fractal
│   ├── kpis.md           #   Métriques vivantes
│   └── decisions.md      #   ADR (Architecture Decisions)
│
├── 08-marketing/         # Marketing - Communication externe
│   ├── one-pager.md      #   1 page, 3 minutes
│   ├── comparison.md     #   vs Copilot/Cursor/etc
│   └── pitch-deck.md     #   10 slides
│
└── 99-archive/           # Archive - Documents historiques
    └── v1-javascript/    #   Documentation v1.0 JS
```

---

## Documents Essentiels (Top 5)

1. **[quickstart.md](./01-getting-started/quickstart.md)** - Commencer ici
2. **[architecture.md](./03-reference/architecture.md)** - Comprendre le système
3. **[axioms.md](./03-reference/axioms.md)** - Les 5 principes fondateurs
4. **[roadmap.md](./07-project/roadmap.md)** - Où nous allons
5. **[kpis.md](./07-project/kpis.md)** - Comment nous mesurons

---

## Pour les LLMs/Agents

Le fichier [_index.json](./_index.json) contient:
- Mapping audience → sections
- Liste des documents canoniques
- Métadonnées de version

Les agents doivent TOUJOURS consulter `_index.json` avant de lire d'autres documents.

---

## Version Actuelle

- **Kernel**: Python v2.0 (bootstrap)
- **Status**: Voir [STATE.md](../../STATE.md)
- **Roadmap**: Voir [todolist.md](../../todolist.md)
- **Changements**: Voir [CHANGELOG.md](../../CHANGELOG.md)

---

## Sources Externes

| Ressource | Lien |
|-----------|------|
| Repository | https://github.com/zeyxx/CYNIC |
| MCP Server | cynic-mcp.onrender.com |
| Token ($asdfasdfa) | Solana mainnet |

---

*Documentation restructurée le 2026-02-22*
*🐕 κυνικός | "La vérité commence par un bon nettoyage"*