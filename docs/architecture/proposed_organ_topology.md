# Nouvelle Topologie : La Grande Migration des Organs

Cette proposition vise à corriger la fragmentation horizontale actuelle en adoptant une approche **"Everything is a folder"** (Modulaire / Domain-Driven).

## Arborescence Cible

```text
CYNIC/
├── organs/                           # 👈 NOUVEAU REPERTOIRE RACINE
│   │
│   ├── anvil/                        # organ-anvil (Repo lifecycle manager)
│   │   ├── MANIFEST.yaml             # Identité, rôle et dépendances
│   │   ├── README.md                 # ⬅️ ex: docs/organs/organ-anvil.md
│   │   ├── state/                    # ⬅️ ex: infra/organ-anvil/
│   │   │   ├── state.json
│   │   │   ├── audit.jsonl
│   │   │   └── poh.json
│   │   ├── src/                      # ⬅️ ex: scripts/organ-anvil-dispatch.py
│   │   │   ├── dispatch.py
│   │   │   └── run.sh
│   │   └── systemd/                  # ⬅️ ex: infra/systemd/organ-anvil-cron.service
│   │       ├── organ-anvil-cron.service
│   │       └── organ-anvil-cron.timer
│   │
│   ├── telegram/                     # organ-telegram (Messaging surface)
│   │   ├── MANIFEST.yaml
│   │   ├── README.md
│   │   ├── state/                    # ⬅️ ex: infra/organ-telegram/
│   │   ├── src/                      # ⬅️ ex: services/cynic-python/organs/telegram/
│   │   │   ├── listener.py
│   │   │   └── pipeline.py
│   │   └── systemd/                  # ⬅️ ex: infra/systemd/organ-telegram...
│   │
│   ├── prompt-enricher/              # organ-prompt-enricher
│   │   ├── MANIFEST.yaml
│   │   ├── src/                      # ⬅️ ex: services/organ-prompt-enricher/
│   │   └── systemd/
│   │
│   └── obsidian/                     # organ-obsidian (Dashboard Projector)
│       ├── MANIFEST.yaml             # ⬅️ ex: services/cynic-python/organs/obsidian/MANIFEST.yaml
│       ├── README.md                 # ⬅️ ex: infra/organ-obsidian/README.md
│       ├── state/
│       └── src/
│           └── projector.py          # ⬅️ ex: services/cynic-python/organs/obsidian/projector.py
│
├── crates/                           # 🦀 Cœur Rust (cynic-kernel, cynic-mcp, etc.)
├── infra/                            # 🌍 Infra Globale (registry.json, backends.toml)
├── docs/                             # 📚 Doc Globale (architecture globale)
├── scripts/                          # 🛠️ Outils génériques (non liés à un Organ)
└── Makefile                          # 🚀 Orchestrateur global
```

## Les 4 piliers de l'encapsulation (Standardisation)

Chaque sous-dossier dans `organs/` respectera strictement ce contrat :

1. `MANIFEST.yaml` : La carte d'identité de l'Organ (utilisée par le Kernel/Registry).
2. `README.md` : La documentation spécifique à cet organe.
3. `state/` : L'empreinte de la machine d'état (JSON is King).
4. `src/` : Le code source (Python, Bash, TS) qui effectue la perception et l'action.
5. `systemd/` (Optionnel) : Les fichiers de déploiement OS-level.

## Bénéfices

* **Lisibilité absolue** : Pour comprendre l'organe `telegram`, tu n'as qu'à ouvrir `organs/telegram/`. Plus besoin de naviguer entre 4 sous-systèmes du repo.
* **Portabilité** : Un organe devient un "package" qu'on peut zipper, versionner, ou déplacer sans casser des liens relatifs partout.
* **Suppression simple** : Déprécier un organe revient à supprimer son dossier (`rm -rf organs/obsidian`), tout part avec (docs, state, code).
