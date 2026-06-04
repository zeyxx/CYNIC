# Talaria X Monitor — Design Spec
**Date:** 2026-06-02  
**Status:** Approved for implementation  
**Scope:** Real-time detection of @TalariaBuild interactions on X + Telegram Ops alerts  
**Adversarial review:** Gemini CLI (GROWL → fixes intégrés)

---

## Problem

@TalariaBuild (= @CynicOracle rebrandé, même browser session) accumule des notifications X non lues. Aucun pipeline n'observe les interactions entrantes ni les tweets publics MetaDAO/Solana en temps réel. L'équipe Ops ne sait pas quand quelqu'un mentionne Talaria, répond à un post, ou crée une opportunité de promotion.

**Falsification :** 8 notifications visibles dans le browser au 2026-06-02, 0 alerte envoyée au groupe Ops → gap confirmé.

---

## Objectif

Détecter en temps réel :
1. **Notifications @TalariaBuild** — mentions, replies, RTs, likes reçus (via inbox CDP)
2. **Sweep public Solana/MetaDAO** — tout tweet contenant `@TalariaBuild`, `MetaDAO ICO`, `Talaria token`, `$TALARIA` (via search_executor existant)

Chaque interaction détectée → alerte dans `TALARIA_OPS_CHAT_ID` (Telegram).

**Phase 1 : tout alerter, pas de filtre** — data-centric : observer d'abord, filtrer sur données réelles après mesure du volume.

---

## Architecture

```
hermes-browser (CDP, port 40769)
    └── notification_poller.py ──────┐
            navigate /notifications  │  tweet_id.json
            parse DOM → interaction  │──────────────► pending/
                                     │                     │
search_executor.py (existant)        │  tweet_id.json      │
    → labels talaria-*/metadao-* ────┘──────────────► pending/
                                                          │
                                             talaria_alerter.py
                                             lit pending/, envoie Telegram
                                             move → processed/
                                                          │
                                             TALARIA_OPS_CHAT_ID
```

**SSOT par fichier** : chaque interaction = un fichier `{tweet_id}.json` dans `pending/`. Dedup naturel (le fichier existe déjà = déjà vu). 0 collision possible entre producteurs. L'alerter fait `move → processed/` sans réécriture globale.

---

## Composants

### 1. `notification_poller.py` (nouveau)
**Tier 2 INFRASTRUCTURE**

```
Localisation : scripts/hermes-x/core/notification_poller.py
K15 Consumer : talaria_alerter.py (lit pending/)
Systemd      : hermes-notification-poller.timer (toutes les 5min)
```

**Comportement :**
- Connecte à hermes-browser via `HubClient` (CDP, port 40770)
- Navigate sur `x.com/notifications`, attend `[data-testid="notification"]` jusqu'à **6s max** (budget CDP total : 9s < P18/10s)
- Parse chaque notification : tweet_id, type (mention/reply/retweet/like), author handle, follower count si disponible, texte ≤ 280 chars, URL canonique
- Dedup : vérifie si `pending/{tweet_id}.json` ou `processed/{tweet_id}.json` existe avant d'écrire
- Écrit `pending/{tweet_id}.json` pour chaque nouvelle interaction
- **Monitor blind** : maintient `poller_state.json` avec `consecutive_empty_cycles`. Si ≥ 3 cycles avec 0 notification → écrit une entrée spéciale `pending/MONITOR_BLIND_{ts}.json` pour alerte Ops
- Si CDP indisponible → `sys.exit(0)`, le timer retentera (pas de retry interne)

### 2. `talaria_alerter.py` (nouveau)
**Tier 2 INFRASTRUCTURE**

```
Localisation : scripts/hermes-x/core/talaria_alerter.py
K15 Consumer : TALARIA_OPS_CHAT_ID (groupe Telegram)
Systemd      : hermes-notification-alerter.timer (toutes les 5min, OnCalendar=*:0/5:30)
```

**Comportement :**
- Lit tous les fichiers `pending/*.json`
- Traitement en batch de **max 20 messages** par cycle (protection rate limit Telegram : 30 msg/s)
- Pour chaque fichier :
  - Formate le message Telegram
  - Envoie via `sendMessage`
  - Si 200 OK → `move pending/ → processed/`
  - Si 429 (rate limit) → garde dans `pending/`, log, arrête le batch (retry prochain cycle)
  - Si autre erreur → garde dans `pending/`, log, continue avec le suivant
- Pas de réécriture de fichier existant — move atomique uniquement

**Format de message Telegram (interaction normale) :**
```
🔔 @TalariaBuild — {type}
👤 @{author} ({followers}K followers)
💬 "{text}"
🔗 {url}
📡 {source} | {detected_at} UTC
```

**Format monitor blind :**
```
⚠️ MONITOR BLIND — notification_poller
3 cycles consécutifs sans détection
Vérifier: hermes-browser actif ? session X valide ? sélecteur DOM changé ?
📡 {ts} UTC
```

### 3. `search_tasks.jsonl` — extensions (modification fichier existant)

Ajouter au fichier existant `/home/user/.cynic/organs/hermes/x/search_tasks.jsonl` :
```json
{"query": "@TalariaBuild", "domain": "token_launch", "weight": 1.0, "label": "talaria-mention"}
{"query": "MetaDAO ICO Solana", "domain": "token_launch", "weight": 0.9, "label": "metadao-ico"}
{"query": "Talaria token Solana", "domain": "token_launch", "weight": 0.9, "label": "talaria-token"}
{"query": "$TALARIA", "domain": "token_launch", "weight": 1.0, "label": "talaria-cashtag"}
```

Modification minimale dans `search_executor.py` : si le label du résultat commence par `talaria-` ou `metadao-` → append dans `pending/` (même format que le poller), en plus du `search_results.jsonl` existant.

### 4. Services systemd (nouveaux)

```ini
# hermes-notification-poller.service
[Service]
WorkingDirectory=/home/user/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 core/notification_poller.py
EnvironmentFile=%h/.config/cynic/env

# hermes-notification-poller.timer
[Timer]
OnCalendar=*:0/5
AccuracySec=10s

# hermes-notification-alerter.service
[Service]
WorkingDirectory=/home/user/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 core/talaria_alerter.py
EnvironmentFile=%h/.config/cynic/env

# hermes-notification-alerter.timer
[Timer]
OnCalendar=*:0/5:30
AccuracySec=10s
```

**SYS1 compliance** : pas de `User=/Group=` (services user-level).

---

## Structure des répertoires

```
/home/user/.cynic/organs/hermes/x/
├── pending/               ← nouveau (interactions non alertées)
│   └── {tweet_id}.json
├── processed/             ← nouveau (interactions alertées)
│   └── {tweet_id}.json
├── poller_state.json      ← nouveau (consecutive_empty_cycles)
├── search_tasks.jsonl     ← modifié (+4 queries)
└── core/
    ├── notification_poller.py   ← nouveau
    └── talaria_alerter.py       ← nouveau
```

---

## Schéma de données — `{tweet_id}.json`

```json
{
  "schema_version": 1,
  "tweet_id": "1234567890",
  "type": "mention|reply|retweet|like|notification",
  "author": "@handle",
  "author_followers": 12000,
  "text": "Have you seen the MetaDAO ICO? $TALARIA looks legit",
  "url": "https://x.com/handle/status/1234567890",
  "detected_at": "2026-06-02T18:00:00Z",
  "source": "notification_poller|search_sweep",
  "keywords_matched": ["MetaDAO", "$TALARIA"]
}
```

**P17 compliance** : `schema_version: 1` dans chaque fichier. Évolution de schema → incrément de version, nouveau sous-dossier `pending/v2/`.

---

## Gestion des erreurs

| Scénario | Comportement |
|----------|-------------|
| hermes-browser mort | poller exit 0, log, timer retentera |
| CDP timeout (> 9s total) | poller exit 0 (P18) |
| 0 notification × 3 cycles | `MONITOR_BLIND` dans pending/ → alerte Ops |
| Sélecteur DOM cassé | 0 notifications détectées → monitor blind déclenché après 3 cycles |
| tweet_id déjà dans pending/ ou processed/ | skip (dedup par existence fichier) |
| Telegram 429 | alerter arrête le batch, fichiers restent dans pending/, retry prochain cycle |
| Telegram autre erreur | log, continue batch, fichier reste dans pending/ |
| pending/ vide | alerter exit 0 silencieusement |
| Shadow ban du compte | 0 notifications → monitor blind (même traitement que sélecteur cassé) |

---

## Tests

- **Unit `test_notification_poller.py`** — mock CDP/HubClient, vérifie parse DOM → fichier `pending/{tweet_id}.json`, vérifie dedup (fichier existant = skip), vérifie monitor blind après 3 cycles vides
- **Unit `test_talaria_alerter.py`** — mock API Telegram, vérifie `move pending/ → processed/`, vérifie garde en pending/ sur 429, vérifie batch cap 20 messages
- **Integration** — run poller avec hermes-browser live, vérifier 1 fichier dans `pending/` dans les 10s
- **K15 falsification** — injecter `pending/synthetic_test_123.json`, lancer alerter, vérifier message reçu dans Telegram Ops dans les 60s. Si non reçu → consumer mort

---

## Contraintes respectées

| Règle | Compliance |
|-------|-----------|
| R1 | Chemins via `HERMES_X_DIR` (hermes_paths.py) |
| P16 | Vocabulaire `type` partagé entre poller et alerter (même schema JSON) |
| P17 | `schema_version: 1` sur chaque fichier |
| P18 | CDP timeout 9s max, Telegram 429 = transient = retry via timer |
| SYS1 | Pas de User=/Group= dans les services |
| R24 | CDP calls avec timeout explicite (9s), requests Telegram avec `timeout=10` |
| K15 | Consumer (alerter) nommé, falsification test définie |
| Sécurité | `CYNIC_TELEGRAM_BOT_TOKEN` uniquement dans `~/.cynic-env` et `~/.config/cynic/env`, jamais dans le repo |

---

## Non-scope (Phase 1)

- Scoring CYNIC avant alert (Phase 2 — après mesure volume réel)
- Filtre par followers/verified (Phase 2)
- Réponse automatique ou promotion auto (décision humaine)
- Archivage SurrealDB (les crystals peuvent ingérer processed/ via K15 Phase 2)
- Rotation/archivage de processed/ (Phase 2 — à définir sur données réelles)
