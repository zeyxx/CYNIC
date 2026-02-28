# CYNIC LIVE MAP — La Vérité du Code (2026-02-28)

Ce document liste uniquement les composants qui sont **importés, instanciés et actifs** dans l'architecture unifiée. Tout le reste appartient à l'histoire (docs/archaeology).

## 1. LE CŒUR (Immaculé)
- **Mathématiques φ** (`cynic/core/phi.py`) : Constantes de confiance et de seuils.
- **Axiomes** (`cynic/core/axioms.py`) : Les 5 filtres (Fidelity, Phi, Verify, Culture, Burn).
- **Event Bus** (`cynic/core/event_bus.py`) : Système nerveux central. Unique source de mouvement.
- **StateManager** (`cynic/organism/state_manager.py`) : Mémoire unifiée (RAM/DB/Fichier).

## 2. LE CERVEAU (Cognition)
- **Orchestrator** (`cynic/cognition/cortex/orchestrator.py`) : Cycle en 7 étapes.
- **Consensus PBFT** (`cynic/core/unified_state.py`) : Algorithme de vote des Chiens.
- **11 Chiens (Dogs)** : Agents de jugement (Scholar, Sage, Oracle, Cynic, etc.).
- **Q-Table** (`cynic/learning/qlearning.py`) : Apprentissage par renforcement du feedback.

## 3. LE CORPS (Métabolisme & Actuateurs)
- **GASdf Executor** (`cynic/integrations/gasdf/executor.py`) : Exécution on-chain des verdicts.
- **ClaudeCodeRunner** (`cynic/metabolism/runner.py`) : Capacité à modifier son propre code.
- **Sensors** (`cynic/senses/workers/`) : Git, Health, Market, Solana, Social (en cours de branchement).

## 4. LES INTERFACES (Limbes)
- **FastAPI Server** (`cynic/api/server.py`) : Point d'entrée unique pour le monde extérieur.
- **Governance Bot** (`governance_bot/bot.py`) : Client Discord du cerveau CYNIC.

---
**Verdict de Stabilité :** 
L'unification a soudé les liens entre (1), (2) et (4). 
Le point de friction actuel est la **visibilité** des données traversant le point (3).
