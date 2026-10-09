# Design Spec: Plugin Antigravity pour Hermes Agent

**Date :** 2026-09-05  
**Auteur :** Gemini & Sovereign Operator  
**Statut :** Validé / En cours de spécification  
**Référence Pi :** `Rahularya01/pi-antigravity` (v0.7.2) pour `@earendil-works/pi`  
**Cible Hermes :** `nousresearch/hermes-agent`  

---

## 1. Contexte & Objectifs

Dans le harness **Pi** (`earendil-works/pi`), le plugin tiers `pi-antigravity` permet d'utiliser l'infrastructure Google Antigravity / Cloud Code Assist comme provider d'inférence agentique pour exécuter des modèles tels que Gemini (Gemini 2.5, Gemini 3.x Flash/Pro) et Claude (Claude 3.5 / 3.7 Sonnet) avec support du streaming, du tool calling et du reasoning/thinking.

L'objectif est d'implémenter un plugin équivalent pour le harness **Hermes Agent** (`nousresearch/hermes-agent`), en exploitant nativement le système officiel de plugins `model-providers` de Hermes sans modifier le code source du dépôt principal.

---

## 2. Emplacement & Cycle de Vie du Plugin

Le plugin est déployé en tant que plugin utilisateur découplé dans :
```
~/.hermes/plugins/model-providers/antigravity/
```
Ce répertoire est automatiquement découvert par `providers.__init__._discover_providers()` au premier appel à `get_provider_profile()` ou `list_providers()`.

### Avantages de cet emplacement :
- Survit aux mises à jour Git du dépôt `hermes-agent`.
- S'intègre immédiatement dans les commandes CLI (`hermes model`, `hermes --provider antigravity`, configuration via `~/.hermes/config.yaml`).
- Isolation complète et possibilité de packaging ultérieur sous forme de dépôt Git ou paquet pip autonome.

---

## 3. Structure des Fichiers

```
~/.hermes/plugins/model-providers/antigravity/
├── plugin.yaml           # Manifeste du plugin
├── __init__.py           # Enregistrement du ProviderProfile et hook create_client
├── auth.py               # Authentification hybride (tokens locaux + OAuth 2.0 PKCE)
├── client.py             # Client synchrone OpenAI-compatible pour Hermes AIAgent
├── translator.py         # Mappage bidirectionnel des requêtes/outils et streaming SSE
└── models.py             # Découverte dynamique de modèles et catalogue statique
```

---

## 4. Spécifications Détaillées des Composants

### 4.1. Manifeste (`plugin.yaml`)
```yaml
name: antigravity
kind: model-provider
version: 1.0.0
description: Google Antigravity / Cloud Code Assist inference provider for Hermes Agent
author: Sovereign Operator
```

### 4.2. Profil Provider & Point d'Entrée (`__init__.py`)
- Définit `AntigravityProfile(ProviderProfile)`.
- Enregistre le profil via `register_provider(antigravity_profile)`.
- Configure :
  - `name = "antigravity"`
  - `aliases = ("google-antigravity", "agy", "cloudcode")`
  - `api_mode = "chat_completions"`
  - `auth_type = "oauth_external"`
  - `supports_vision = True`
  - `supports_vision_tool_messages = True`
  - `default_max_tokens = 65536`
  - `default_aux_model = "gemini-3.8-flash"`
- Surcharge `create_client(**kwargs)` :
  - Résout les identifiants d'accès (access token, project ID).
  - Retourne une instance de `AntigravityClient` compatible avec l'interface `client.chat.completions.create` consommée par `AIAgent`.
- Surcharge `fetch_models()` :
  - Délègue à `models.fetch_available_models()` pour interroger en direct le endpoint `/v1internal:fetchAvailableModels`.

### 4.3. Module d'Authentification Hybride (`auth.py`)
Gère la résolution et le cycle de vie des jetons Google.

1. **Constantes & Empreinte OAuth** (alignées sur `pi-antigravity`) :
   - `AUTH_URL` : `https://accounts.google.com/o/oauth2/v2/auth`
   - `TOKEN_URL` : `https://oauth2.googleapis.com/token`
   - `REDIRECT_URI` : `http://localhost:51121/oauth-callback`
   - `CLIENT_ID` : `1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com`
   - `CLIENT_SECRET` : `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`
   - `SCOPES` :
     - `https://www.googleapis.com/auth/aicode`
     - `https://www.googleapis.com/auth/cloud-platform`
     - `https://www.googleapis.com/auth/userinfo.email`
     - `https://www.googleapis.com/auth/userinfo.profile`
     - `https://www.googleapis.com/auth/cclog`
     - `https://www.googleapis.com/auth/experimentsandconfigs`

2. **Résolution en cascade (Auto-Discovery)** :
   - Variables d'environnement : `ANTIGRAVITY_ACCESS_TOKEN`, `ANTIGRAVITY_REFRESH_TOKEN`.
   - Token Antigravity CLI local : `~/.gemini/antigravity-cli/antigravity-oauth-token` (ou session SQLite/JSON associée).
   - Cache d'authentification Pi : `~/.pi/agent/auth.json` (champ `antigravity`).
   - Fichier de persistance dédié : `~/.hermes/antigravity-auth.json`.

3. **Flux interactif OAuth 2.0 PKCE** :
   - Génération de `code_verifier` et `code_challenge` SHA-256 base64url.
   - Serveur HTTP local éphémère (`http.server`) écoutant sur `127.0.0.1:51121`.
   - Ouverture automatique du navigateur ou affichage dans le terminal de l'URL pour saisie manuelle (mode headless).
   - Échange du code contre `access_token` et `refresh_token`.

4. **Rafraîchissement automatique (Auto-Refresh)** :
   - Fonction `get_valid_credentials()` vérifiant l'expiration (`expires_at`).
   - Si expiration dans moins de 5 minutes, appel POST à `TOKEN_URL` avec `grant_type=refresh_token`.

5. **Découverte du Project ID (`loadCodeAssist`)** :
   - Appel POST à `/v1internal:loadCodeAssist` avec payload `{ "metadata": { "ideType": "ANTIGRAVITY" } }`.
   - Extraction de `cloudaicompanionProject` ou `projectId`.
   - Fallback stable déterministe dérivé de l'email du compte.

### 4.4. Module Client (`client.py`)
Fournit la façade consommée par `AIAgent` dans `hermes-agent` :

- Expose `AntigravityClient` avec un sous-objet `chat.completions`.
- Méthode `create(model, messages, stream=True, tools=None, tool_choice=None, temperature=None, max_tokens=None, **kwargs)` :
  - Prépare le payload Antigravity complet via `translator.py`.
  - Effectue la requête HTTP POST en streaming (`alt=sse`) vers les endpoints candidats avec basculement automatique :
    - `https://daily-cloudcode-pa.googleapis.com` (défaut)
    - `https://daily-cloudcode-pa.sandbox.googleapis.com`
    - `https://cloudcode-pa.googleapis.com`
  - Headers HTTP stricts :
    - `Authorization: Bearer <access_token>`
    - `Content-Type: application/json`
    - `User-Agent: antigravity/cli/1.1.23 (aidev_client; os_type=linux; arch=amd64; cl=974125021; auth_method=consumer)`
  - Si `stream=True`, renvoie un générateur itérable d'objets `ChatCompletionChunk`.
  - Si `stream=False`, accumule le générateur et renvoie un `ChatCompletion`.

### 4.5. Traducteur Bidirectionnel (`translator.py`)

#### Requête sortante (`to_antigravity_payload`) :
- **System Prompt** : Séparé dans `systemInstruction: { role: "user", parts: [{ text: ... }] }`.
- **Historique des messages** :
  - `user` $\rightarrow$ `role: "user"`, `parts: [{ text: ... }]` (ou `inlineData` pour vision en base64).
  - `assistant` avec texte $\rightarrow$ `role: "model"`, `parts: [{ text: ... }]`.
  - `assistant` avec tool calls $\rightarrow$ `role: "model"`, `parts: [{ functionCall: { id, name, args } }]`.
  - `tool` (résultats d'exécution) $\rightarrow$ `role: "user"`, `parts: [{ functionResponse: { name, response: { content } } }]`.
- **Outils (Function Declarations)** :
  - Les schémas JSON des fonctions OpenAI sont transposés dans `tools: [{ functionDeclarations: [...] }]`.
- **Reasoning / Thinking** :
  - Pour les modèles supportant le raisonnement (Gemini 2.5/3.x, Claude 3.7), injection de `generationConfig.thinkingConfig = { "thinkingBudget": ... }`.

#### Flux entrant (`parse_sse_stream`) :
- Décode les lignes SSE `data: {...}`.
- Extrait `candidates[0].content.parts` :
  - Si `part.thought == True` : émet un chunk avec `delta.reasoning_content = part.text` (affiché dans la TUI Hermes).
  - Si `part.text` classique : émet un chunk avec `delta.content = part.text`.
  - Si `part.functionCall` : émet un chunk avec `delta.tool_calls = [...]`.
  - Conserve et propage `part.thoughtSignature` si présent pour garantir la cohérence des tours suivants sur Gemini 3.
- Traduit les métadonnées d'usage (`usageMetadata`) vers `chunk.usage` (`prompt_tokens`, `completion_tokens`, `total_tokens`).

### 4.6. Catalogue de Modèles (`models.py`)
- Découverte en direct via `/v1internal:fetchAvailableModels`.
- Catalogue statique de fallback en cas de réseau indisponible :
  - `gemini-3.8-flash` (défaut recommandé pour le coding rapide)
  - `gemini-3.7-flash`
  - `gemini-2.5-pro`
  - `gemini-2.5-flash`
  - `claude-3-7-sonnet`
  - `claude-3-5-sonnet`

---

## 5. Gestion des Erreurs & Résilience

1. **Token expiré (401)** : Détecté automatiquement par le client, rafraîchi via `auth.refresh_token()`, puis rejeu transparent de la requête courante (1 tentative).
2. **Indisponibilité / Capacité (503 / 404)** : En cas d'erreur de capacité sur `daily-cloudcode-pa`, bascule automatique sur `cloudcode-pa.googleapis.com`.
3. **Limites de Quota (429)** : Capture du message d'erreur et formatage propre pour l'utilisateur sans écraser la session Hermes.
4. **Timeouts & Stalls** : Gestion de délais de garde distincts pour l'établissement de la connexion (header timeout : 15s) et la réception des chunks (stall timeout : 30s).

---

## 6. Plan de Validation & Tests

1. **Tests unitaires (`tests/`)** :
   - `test_auth.py` : Validation de la génération PKCE, parsing de tokens locaux, rafraîchissement mocké.
   - `test_translator.py` : Vérification de la conversion des messages (système, user, assistant, tool results) et des déclarations d'outils.
   - `test_streaming.py` : Simulation d'un flux SSE complet avec texte, blocs de réflexion (thinking) et tool call, vérifiant l'émission conforme des objets `ChatCompletionChunk`.
2. **Test d'intégration système** :
   - Vérification de la présence de `antigravity` dans `hermes_cli/auth.py` et `list_providers()`.
   - Lancement d'une requête test avec `--provider antigravity --model gemini-3.8-flash`.
