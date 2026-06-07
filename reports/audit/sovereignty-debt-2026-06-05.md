# Audit: Dette de Souveraineté (Sovereignty Debt)
**Date**: 2026-06-05T07:25:00Z
**Framework**: ARCHITECTURE D’UN SYSTÈME DATA

## 1. PERCEPTION (INPUT)
*   **Dette**: Dépendance aux outils `google_web_search` et `web_fetch`. Les agents propriétaires "perçoivent" le web en temps réel.
*   **Pont**: Migration vers `hermes-x` (capture locale X/Web) et intégration de `Tavily` ou `SearxNG` local comme sens de remplacement.
*   **Objectif**: Rendre la perception autonome vis-à-vis des infrastructures Google/Anthropic.

## 2. TRANSFORMATION (TRAITEMENT)
*   **Dette**: Fenêtre de contexte de 1M+ tokens. Les agents propriétaires "lisent" tout le repo sans effort.
*   **Pont**: Utilisation de `vLLM` avec `PagedAttention` et implémentation de **RAG local** (via `cynic-kernel` embedding port).
*   **Objectif**: Transformer le repo massif en signaux digestes pour un modèle 27B (fenêtre 64k-128k).

## 3. STRUCTURATION (MODÉLISATION)
*   **Dette**: Raisonnement Zero-Shot complexe. Les modèles propriétaires "comprennent" des instructions Markdown denses sans exemples.
*   **Pont**: **Guided Decoding** (Outlines/XGrammar). Forcer la structure JSON au niveau du moteur d'inférence pour éviter les erreurs de parsing (le bloqueur 0.05).
*   **Objectif**: Stabiliser la structure de sortie pour garantir l'intégrité du Proof-of-History.

## 4. ANALYSE & COMPRÉHENSION
*   **Dette**: Orchestration d'agents "Sub-agents" native.
*   **Pont**: Utilisation du **Mempool CYNIC** comme bus de coordination. Les agents OS ne se parlent pas directement ; ils lisent/écrivent des `Observations` de domaine `mempool`.
*   **Objectif**: Découpler l'intelligence de l'orchestration.

## 5. APPRENTISSAGE
*   **Dette**: Fine-tuning propriétaire invisible.
*   **Pont**: Utilisation des verdicts **HOWL** (Q-Score > 85) comme dataset de calibration pour le Fine-Tuning local (LoRA/DPO).
*   **Objectif**: Aligner le "Cerveau" souverain sur les axiomes CYNIC par l'exemple, pas seulement par le prompt.

## 6. FIABILITÉ (SURVIE SYSTÈME)
*   **Dette**: Sécurité "Cloud" (Google Cloud Safety).
*   **Pont**: Pivot vers **Native Linux (Ubuntu 24.04)**. Suppression de la couche Windows/DLL pour une gestion directe des ressources GPU (RTX 4060 Ti).
*   **Objectif**: Garantir que le système ne peut pas être coupé par un fournisseur tiers.

## 7. RISQUES & ANTI-PATTERNS
*   **Risque**: "Prompt Bloat". Envoyer 13k de prompt à un modèle 7B/9B sature le KV cache et dégrade le raisonnement.
*   **Anti-pattern**: Vouloir simuler un Cortex 1M context sur une carte 16GB VRAM sans stratégie de compression.
*   **Mitigation**: Décomposition des tâches en micro-verdicts (Pattern: Pipeline).

---
**Verdict Audit**: **GROWL** (42/100). La dette est gérable mais nécessite une refonte de la couche d'orchestration avant de pouvoir débrancher les agents propriétaires.
