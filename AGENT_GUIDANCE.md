# AGENT GUIDANCE & STRATEGIC DIRECTIVES FOR PROJECT CYNIC

**Version:** 1.0
**Date:** 2026-03-02
**Auteur:** Gemini CLI

## 1. Mission & Vision d'Ensemble

Ce document est le point de départ pour tout agent IA travaillant sur le projet CYNIC. **Votre lecture et compréhension de `GEMINI.md` est un prérequis absolu avant toute intervention.**

Le projet CYNIC vise à construire une **Infrastructure Industrielle Fractale** : un système logiciel complexe, auto-analytique et résilient, mêlant des concepts d'IA, de systèmes distribués et de blockchain. L'objectif n'est pas de créer une simple application, mais une fondation capable de supporter une charge massive (10k+ TPS) et une complexité croissante.

## 2. Synthèse de l'Audit Architectural (Mars 2026)

Une analyse approfondie a été menée sur la base des 9 "lentilles d'ingénierie" définies par les architectes du projet. Voici la synthèse :

| Domaine | Forces (Ce qui est en place) | Axes d'Amélioration (Où agir) |
| :--- | :--- | :--- |
| **AI Infra** | `LLMRouter` pour l'abstraction des modèles, intégration `ollama`. | Manque d'un VectorDB et d'un pipeline d'embedding de données optimisé. |
| **Backend** | Architecture événementielle (EventBus), API "event-first" (FastAPI). | Centraliser la gestion de configuration, éliminer les appels directs à `os.getenv`. |
| **ML Platform** | Forte culture CI/CD (Docker, GitHub Actions). | Intégrer un framework MLOps (ex: MLflow) pour le suivi et le lignage des données. |
| **Data Eng.** | Bases solides d'Event Sourcing pour la traçabilité. | Intégrer des outils de processing de données à grande échelle (ex: Kafka, Spark). |
| **Security** | L'authentification est une préoccupation. | Implémenter des mécanismes Zero-Trust concrets (gestion de secrets, isolation). |
| **SRE** | Conception axée sur la résilience (`test_cognitive_resilience`). | Standardiser le monitoring (ex: Prometheus, OpenTelemetry), implémenter la backpressure. |
| **Blockchain** | Intégration claire avec NEAR (contrat Rust). | Prouver l'implémentation des concepts de consensus théoriques (phi-BFT). |
| **Robotics** | Le système peut prendre des décisions autonomes (Proposal Executor). | N/A (domaine peu applicable). |
| **Solutions Arch.** | Vision architecturale très riche et documentée. | Le risque principal est la complexité. Favoriser la simplicité pragmatique. |

## 3. Directives Stratégiques (Path Forward)

Votre mission n'est pas seulement d'ajouter des fonctionnalités, mais de faire évoluer CYNIC vers une infrastructure de niveau industriel. Priorisez les actions suivantes :

1.  **Renforcer les Fondations (`Industrialisation`)**: Avant de créer de nouvelles capacités, renforcez celles qui existent. Remplacez les implémentations prototypes par des solutions standards de l'industrie (ex: remplacer un bus d'événements en mémoire par RabbitMQ/Kafka si la charge l'exige, intégrer OpenTelemetry pour le tracing).
2.  **Adhérer à la Charte (`GEMINI.md`)**: Traquez et corrigez activement toute violation des principes de `GEMINI.md`. **La première priorité est l'élimination des "Hérésies Interdites" (ex: accès directs à `os.getenv`, types `Any`).**
3.  **Simplifier et Brûler (`BURN`)**: Conformément à la philosophie du projet, cherchez à réduire la complexité. Si une abstraction n'est pas justifiée, supprimez-la. Trois lignes de code répétées sont préférables à une mauvaise abstraction.

## 4. Première Action Suggérée

L'analyse a révélé des appels directs à `os.getenv` dans le noyau, ce qui viole la charte. La première action à entreprendre est de **refactoriser la gestion de la configuration pour utiliser un pattern d'injection de dépendances strict**, rendant le système plus testable et découplé de son environnement.

---
*Ce document est un artefact vivant. Mettez-le à jour après chaque intervention majeure.*
