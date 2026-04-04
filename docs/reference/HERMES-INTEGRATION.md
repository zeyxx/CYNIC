<!-- AI-CONTEXT
role: reference
audience: humans, AI agents
purpose: Guide on how to connect Hermes Agent to CYNIC MCP Server
-->

# Connecter Hermes Agent à CYNIC

CYNIC expose nativement ses capacités (Jugement, Inférence, Coordination, CCM) via le **Model Context Protocol (MCP)**. Hermes Agent (par NousResearch) étant nativement compatible MCP, la connexion se fait sans avoir besoin de modifier le code de CYNIC.

## 1. Pré-requis

Assurez-vous que `cynic-kernel` est compilé et accessible dans votre `PATH` (généralement `~/bin`).

```bash
# Depuis la racine du projet CYNIC
make check
cp target/release/cynic-kernel ~/bin/cynic-kernel
```

> [!NOTE]
> `make deploy` place automatiquement `cynic-kernel` dans `~/bin/`.
> Le même binaire sert en REST (sans flag) et en MCP (avec `--mcp`).

## 2. Configuration MCP

CYNIC dispose déjà d'un fichier `.mcp.json` à la racine du projet. Ce fichier suit le standard des clients MCP (comme Claude Desktop ou Hermes Agent) :

```json
{
  "mcpServers": {
    "cynic": {
      "command": "cynic-kernel",
      "args": ["--mcp"]
    }
  }
}
```

## 3. Lancer Hermes Agent avec CYNIC

Si vous lancez Hermes Agent en CLI, vous pouvez lui passer directement le chemin vers le fichier de configuration de CYNIC, ou bien fusionner cette configuration dans votre `hermes.json` global.

**Exemple d'utilisation avec Hermes Agent :**
```bash
hermes-agent --mcp-config "$(git rev-parse --show-toplevel)/.mcp.json"
```
*(Adaptez la commande exacte selon la version d'Hermes Agent que vous utilisez).*

## 4. Ce que Hermes pourra faire

Une fois connecté, Hermes Agent découvrira automatiquement et pourra utiliser les outils (tools) suivants exposés par le noyau CYNIC :

- **`cynic_judge`** : Soumettre une analyse aux Dogs de CYNIC pour évaluation épistémique (retourne le Q-Score).
- **`cynic_infer`** : Utiliser l'IA souveraine locale de CYNIC sans coût d'API.
- **`cynic_health`** : Vérifier l'état du système et des bases de données.
- **`cynic_crystals` / `cynic_verdicts`** : Consulter la mémoire du système (CCM) et les anciens jugements.
- **Outils de Coordination** : `cynic_coord_register`, `cynic_coord_claim` pour réserver des fichiers ou s'annoncer lors d'une session.

> [!IMPORTANT]
> Pour respecter les règles de souveraineté et d'architecture, toute exécution LLM demandée par Hermes (via l'outil `cynic_infer`) passera par le composant `InferPort` de CYNIC plutôt que de requêter directement l'API externe depuis Hermes.
