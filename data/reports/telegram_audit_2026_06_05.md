# Rapport d'Audit : Organe Telegram (Cortex-Handoff)
**Date :** 2026-06-05
**Agent :** Gemini (Cortex)
**Destinataire :** Hermes Agent / T.

## 1. État des Lieux Technique

L'organe Telegram est scindé en deux composants distincts et complémentaires, respectant la séparation Organe (Ingestion) / Agent (Interaction).

### A. L'Organe d'Ingestion (MTProto Userbot)
- **Localisation :** `cynic-python/organs/telegram/`
- **Moteur :** Telethon (Userbot). Permet d'écouter n'importe quel canal où le compte personnel est présent, sans les limitations de l'API Bot standard.
- **Stockage :** SQLite local (`~/.cynic/organs/telegram/messages.db`).
- **Pipeline :**
    - **Classification :** Basée sur une cartographie de `channel_id` (40+ canaux pré-configurés) et des patterns regex (20+ filtres de "BARK" structurel : buy-bots, raids, shills).
    - **Buffering :** Agrégation temporelle et par auteur pour éviter la fragmentation des signaux.
- **Fiabilité :** 32 tests unitaires validés (couvrant buffer, config, listener, pipeline, schema).

### B. L'Agent d'Interaction (@CynicOracle)
- **Localisation :** `cynic-python/agents/telegram_organ_bot.py`
- **Moteur :** `python-telegram-bot`.
- **Fonctions :** `/observe` (enregistrement manuel d'observations), `/status` (santé du kernel).
- **Statut :** MVP opérationnel. Une version squelette (`cynic-python/agents/telegram_bot/bot.py`) existe pour le futur `/judge`.

---

## 2. Analyse Axiomatique (Scoring CYNIC)

| Axiome | Score | Commentaire |
| :--- | :--- | :--- |
| **FIDELITY** | 0.618 | Capture brute intégrale vers SQLite. Heartbeats réguliers vers le kernel. |
| **PHI** | 0.550 | Structure Organ/Agent propre, mais le pipeline de classification par regex est rigide. |
| **VERIFY** | 0.618 | Excellente couverture de tests. Idempotence de la DB vérifiée. |
| **CULTURE** | 0.600 | Respecte les patterns CYNIC (Tier 2, SQLite, Kernel POST). |
| **BURN** | 0.580 | Filtrage agressif du "BARK" avant traitement LLM (économie de tokens). |
| **SOVEREIGNTY** | 0.618 | Utilisation de MTProto (compte personnel) + Extraction locale via Qwen 7B sur Core. |

---

## 3. Le "Gap" Hermes (Travail Restant)

Hermes a déjà complété sa Phase 2 pour X.com (apprentissage des poids/mots-clés). Pour Telegram, le travail est en suspens :

1.  **Consommation des Extractions :** Le script `extract_batch.py` génère une table `extractions`. Hermes ne consomme pas encore cette table pour mettre à jour ses `learned_weights.json`.
2.  **Boucle de Feedback Organique :** Sur X, Hermes mesure la précision par rapport aux likes de T. Sur Telegram, il doit apprendre de ce que T. considère comme un signal (via les messages envoyés au bot ou les interactions dans les canaux surveillés).
3.  **Unification :** Hermes doit traiter les signaux Telegram comme une source de données au même titre que X, en utilisant le même mécanisme de scoring temporel et sémantique.

---

## 4. Recommandations pour Hermes (Directives)

1.  **Phase 3 TG - Extraction Continue :** Transformer `extract_batch.py` en un daemon ou un cron intégré à l'agent Hermes-Telegram.
2.  **Phase 4 TG - Apprentissage Croisé :** Utiliser les signaux extraits de Telegram pour enrichir le profil de mots-clés (`learned_weights.json`) utilisé sur X.
3.  **Interface de Jugement :** Activer le handler `/judge` dans le bot pour permettre à Hermes de soumettre des signaux Telegram au verdict des Dogs.

## Verdict Global : GROWL (Satisfaisant, mais passif)
L'infrastructure est solide et souveraine, mais elle reste une "archive morte" sans l'activation de la boucle de raisonnement de Hermes.

---
*Fin du rapport d'audit.*
