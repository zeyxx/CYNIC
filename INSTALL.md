# Installation de CYNIC

> *"φ distrusts φ"* - Loyal to truth, not to comfort

---

## Installation Rapide (Recommandé)

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC
./scripts/install.sh
```

Le script vérifie les prérequis, installe les dépendances et configure tout automatiquement.

---

## Installation Manuelle

### Prérequis

- **Node.js** >= 20.0.0
- **Claude Code** (CLI d'Anthropic) - [Installation](https://docs.anthropic.com/en/docs/claude-code)
- Git

### Étape 1 : Cloner le repo

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC
```

---

### Étape 2 : Installer les dépendances

```bash
npm install
```

---

### Étape 3 : Configuration de l'environnement

```bash
cp .env.example .env
```

Pour le développement local, les valeurs par défaut suffisent généralement.

Pour la production, configurer :
- `CYNIC_DATABASE_URL` - PostgreSQL
- `CYNIC_REDIS_URL` - Redis (optionnel)

---

### Étape 4 : Configurer le MCP Server

Le MCP Server est le "cerveau" de CYNIC - il fournit les outils de jugement, mémoire et analyse.

```bash
cp .mcp.json.example .mcp.json
```

Modifier le chemin `cwd` dans `.mcp.json` pour pointer vers ton installation :

```json
{
  "mcpServers": {
    "cynic": {
      "command": "node",
      "args": ["packages/mcp/bin/mcp.js"],
      "cwd": "/chemin/vers/CYNIC",
      "env": {
        "MCP_MODE": "stdio",
        "NODE_ENV": "development"
      }
    }
  }
}
```

---

### Étape 5 : Activer le Plugin Claude Code

Le dossier `.claude/` contient le plugin qui donne à Claude l'identité CYNIC.

### Option A : Automatique (recommandé)

Ouvrir Claude Code dans le dossier CYNIC - le plugin se charge automatiquement :

```bash
cd /chemin/vers/CYNIC
claude
```

### Option B : Installation globale

Pour avoir CYNIC disponible partout :

```bash
claude mcp add cynic -s user -- node /chemin/vers/CYNIC/packages/mcp/bin/mcp.js
```

---

### Étape 6 : Vérifier l'installation

Lance Claude Code :

```bash
claude
```

Et salue CYNIC :

```
> bonjour
```

Si tu vois un *tail wag* et que CYNIC répond avec sa personnalité de chien cynique, l'installation est réussie !

---

## Structure du projet

```
CYNIC/
├── .claude/           # Plugin Claude Code (identité CYNIC)
│   ├── plugin.json    # Manifest du plugin
│   ├── cynic-consciousness.md  # Instructions système
│   ├── hooks/         # Hooks de session
│   └── agents/        # Agents spécialisés
├── packages/
│   ├── mcp/           # Serveur MCP (cerveau)
│   ├── node/          # Noeud P2P
│   ├── protocol/      # Protocole PoJ
│   └── persistence/   # Stockage
├── CLAUDE.md          # Instructions d'identité
├── .mcp.json          # Config MCP locale
└── .env               # Variables d'environnement
```

---

## Dépannage

### CYNIC ne répond pas comme un chien

Vérifier que :
1. Tu es dans le dossier CYNIC quand tu lances `claude`
2. Le fichier `.claude/plugin.json` existe
3. Le fichier `CLAUDE.md` est présent à la racine

### Erreur MCP "command not found"

Vérifier que :
1. Node.js >= 20 est installé : `node --version`
2. Le chemin dans `.mcp.json` est correct
3. Les dépendances sont installées : `npm install`

### Les outils brain_* ne fonctionnent pas

Le serveur MCP n'est pas connecté. Vérifier :
1. Le fichier `.mcp.json` est configuré
2. Relancer Claude Code après modification de `.mcp.json`

---

## Les 4 Axiomes

CYNIC opère selon 4 axiomes fondamentaux :

| Axiome | Principe |
|--------|----------|
| **PHI** | Tous les ratios dérivent de φ (1.618...). Confiance max = 61.8% |
| **VERIFY** | Don't trust, verify. Scepticisme systématique |
| **CULTURE** | Culture is a moat. Les patterns définissent l'identité |
| **BURN** | Don't extract, burn. Simplicité avant tout |

---

## Ressources

- [README.md](./README.md) - Vue d'ensemble du protocole
- [ROADMAP.md](./ROADMAP.md) - Feuille de route
- [docs/](./docs/) - Documentation technique

---

*🐕 κυνικός | Loyal to truth, not to comfort | φ⁻¹ = 61.8% max*
