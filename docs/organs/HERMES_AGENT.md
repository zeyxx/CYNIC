# CYNIC — Framework Integration Doctrine (Hermes Agent)

Ce document établit la base saine, stricte et définitive sur la façon dont l'organisme CYNIC intègre et utilise des frameworks d'agents externes (par défaut : `hermes-agent`).

Suite à la purge de la dette technique, l'architecture repose désormais sur un principe absolu : **La Séparation de la Cognition et de l'Action.**

## 1. Cognition Native (Le domaine du Framework)
Le framework `hermes-agent` est considéré comme un "Cerveau" invité. Nous devons respecter sa physiologie interne pour ne pas briser son fonctionnement.
* **Mémoire native :** Hermes gère sa propre mémoire locale (contexte de session, RAG interne). CYNIC n'essaie plus de fusionner ou de hacker cette mémoire.
* **Éphémérité :** L'instance de Hermes et toute sa mémoire locale sont considérées comme strictement éphémères vis-à-vis de l'organisme.

## 2. Action Souveraine (Le domaine du Kernel CYNIC)
Le cerveau Hermes n'a **aucun bras natif**. 
* **Castration des Outils Natifs :** Tous les outils natifs d'action (exécuteur bash, éditeur de fichier natif) doivent être désactivés.
* **Monopole du MCP :** La seule boîte à outils fournie à Hermes est une connexion client vers le serveur `cynic-mcp`.

## 3. L'Inférence Hybride (Souveraineté vs Commodité)
Hermes est configuré comme un moteur à double injection :
1. **Moteur de Commodité (Modèles Gratuits Cloud) :** Hermes utilise nativement des modèles externes peu coûteux (OpenRouter, Groq) pour sa boucle cognitive interne, la recherche web et la planification.
2. **Socle Souverain (`cynic-inference`) :** Le Kernel expose le proxy d'inférence via l'alias DNS `http://inference.synapse.cynic:3030/v1`. Dès qu'une action nécessite une validation de sécurité ou modifie le code source de l'organisme, le MCP CYNIC délègue en interne l'évaluation à ce socle souverain (qui gère ses propres GPU et le `SlotSemaphore`).

---

## 4. Formalisation des Contrats d'Interface

Au lieu d'utiliser des subprocess fragiles, CYNIC interagit avec Hermes via l'API **Gateway** du framework (`POST /v1/chat/completions`).

### A. Le Contrat d'Entrée ("Message-In" Payload)
Quand le pont CYNIC lit une tâche dans la Mempool, il la convertit en un prompt structuré envoyé à l'API Gateway de Hermes. Le contrat JSON d'injection exige :
* **Un `system` prompt restrictif :** Informant Hermes qu'il est aveugle et sans bras, et qu'il DOIT utiliser le serveur MCP fourni.
* **Un `user` prompt (La Tâche) :** Contenant le JSON brut de la Mempool.
* **Un `tool_choice` forcé :** Optionnellement, forcer Hermes à appeler `cynic_observe` avant de terminer.

### B. Le Contrat de Sortie (Bloodstream / Mémoire)
Hermes ne communique pas sa conclusion en stdout. Il **doit** appeler l'outil MCP `cynic_observe` pour persister son travail. 
**Schema requis (cynic.hermes.execution_result.v1) :**
```json
{
  "domain": "<task_domain>",
  "kind": "task_result_observation",
  "source": {"type": "hermes_gateway", "task_id": "<mempool_id>"},
  "claim": "<résumé de la conclusion cognitive>",
  "evidence": {"<logs ou fichiers modifiés>"},
  "confidence": "inferred"
}
```

---

## 5. Analyse des Lacunes (Ce qu'il manque à l'organisme)

Pour que cette architecture soit pleinement opérationnelle, les éléments suivants manquent actuellement dans CYNIC et doivent être construits :

1. **Le Démon Routeur (`cynic-hermes-bridge`) :** Un petit démon (en Rust ou script robuste) qui lit `/agent-tasks`, formate le *Message-In* et fait la requête HTTP POST à la Gateway Hermes.
2. **Configuration du Nœud Hermes :** Le fichier `~/.hermes/config.yaml` doit être généré sur les machines cibles pour déclarer `cynic-mcp` et désactiver les plugins natifs.
3. **Gating Multi-Couches dans MCP :** Actuellement, `cynic-mcp` obéit aveuglément si le fichier est "claimé". Il manque un middleware dans le MCP qui intercepte les modifications critiques (ex: `replace_file_content`). Ce middleware doit soumettre l'action à **`cynic-judge`** pour une évaluation asynchrone, qui escaladera ensuite vers une **approbation humaine obligatoire via l'interface finale et publique** (Odysseum / Web UI) avant que la modification ne soit réellement écrite sur le disque.
