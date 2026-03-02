# 🛡️ GEMINI.md — Charte de Rigueur Architecturale CYNIC

Ce document définit les mandates, principes et interdits de l'agent Gemini CLI au sein du projet CYNIC. Il prime sur toute impulsion d'exécution rapide.

## 🧠 1. Mandat Global
Transformer CYNIC d'un prototype de recherche en une **Infrastructure Industrielle Fractale** capable de supporter une infinité de données et d'instances (10k+ TPS). 

## 🏗️ 2. Les 9 Lentilles d'Ingénierie (Standards)
Chaque modification doit être validée mentalement par ces experts :
1.  **AI Infra** : Inférence scalable, pas de Dogs en dur, gestion de queue LLM.
2.  **Backend** : Injection de dépendances, pooling HTTP, asynchronisme maîtrisé.
3.  **ML Platform** : Lignage de données (Trace ID), feedbacks loops fermées.
4.  **Data Engineer** : Persistence réactive (Event Sourcing), ring-buffers, pas de leaks.
5.  **Security** : Zero-Trust, isolation multi-tenant stricte, modèles scellés (Frozen).
6.  **SRE** : Backpressure, ABS (Flow Control), ports dynamiques, auto-healing.
7.  **Blockchain** : Déterminisme du consensus (phi-BFT), auditabilité totale.
8.  **Robotics** : Précision motrice, couplage somatique œil/main, budget métabolique.
9.  **Solutions Architect** : Vision bout-en-bout, scalabilité à 10 ans, simplicité noble.

## ☢️ 3. Les Hérésies Interdites (BURN)
*   **Silence** : Interdiction des `except: pass`. Chaque erreur doit être un événement `ANOMALY`.
*   **Flou** : Interdiction des `Any` et des `Optional` par défaut. Le type doit être strict.
*   **Couplage** : Interdiction des singletons et des accès directs `os.getenv` dans le noyau.
*   **Gâchis** : Interdiction de recréer des clients HTTP. Tout passe par le `VascularSystem`.

## 🛠️ 4. Workflow de Travail (Strikt)
1.  **Analyse** : Explorer le contexte fractal avant de proposer un design.
2.  **Design** : Présenter les changements via les 9 lentilles.
3.  **Exécution** : Chirurgie précise, pas de remplacements globaux par script.
4.  **Validation** : `ruff` doit être vert sur tous les fichiers modifiés.
5.  **Preuve** : `verify_fractal_trace.py` doit passer avec 0 fuite.

## 🌌 5. Vision de l'Infini
CYNIC ne doit pas être "fini", il doit être "fondé". Les rails (CI/CD, Auth, Scaling) passent avant les fonctionnalités de jeu. L'objectif est un système capable de s'auto-analyser et de signaler sa propre douleur.

---
*Signé : Gemini CLI - 01/03/2026*
