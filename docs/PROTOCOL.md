# CYNIC Development Protocol — The Anti-Chaos Doctrine

*"La prolifération des entités sans nécessité est le début de la corruption."*

Ce protocole doit être lu et respecté par tout développeur ou agent IA (Claude Code, Gemini, Cursor) avant de modifier une seule ligne de CYNIC.

## 1. Topologie Immuable (La Règle des 5 Systèmes)
CYNIC est un organisme, pas un script. Tout nouveau code **DOIT** trouver sa place dans l'un de ces 5 organes. **Il est strictement interdit de créer un dossier à la racine de `cynic/`.**

1.  `kernel/` : Le Logos (Mathématiques, Config, Bus, État). Modifiez avec extrême prudence.
2.  `brain/` : L'Intelligence (Chiens, LLM, DNA, Q-Table). 
3.  `perception/` : Les Sens (Capteurs, Fédération, Intégrations externes).
4.  `metabolism/` : L'Action et la Survie (Runners, Outils de santé).
5.  `interfaces/` : La Peau (API, Bots, CLI). Les interfaces ne contiennent **aucune logique métier**.

## 2. Règle d'Évolution (Axiome BURN)
Avant d'ajouter un nouveau fichier ou une nouvelle fonction :
*   **Vérifiez l'existant** : Existe-t-il déjà un organe qui fait cela ? (ex: ne pas créer `unified_learning.py` si `qlearning.py` existe).
*   **N'isolez jamais** : Si vous créez un nouvel organe (ex: un capteur), vous **devez** le brancher au système nerveux (`EventBus`) et à la mémoire (`OrganismState`). Un organe isolé est une tumeur.

## 3. Gestion des Branches (Fin du Multivers)
*   **Pas de worktrees parallèles massifs**. 
*   Si vous lancez une IA autonome (comme Claude Code), elle doit travailler sur une branche de **courte durée** (feature branch) et être fusionnée dans `master` sous 24 heures.
*   Si une branche diverge trop de la topologie de `master`, elle doit être brûlée, pas fusionnée de force.

## 4. Validation Obligatoire (Axiome VERIFY)
Aucune session de code ne se termine sans exécuter :
1.  `pytest tests -v` : Tous les tests doivent passer.
2.  `python scripts/cynic_trace.py` : L'organisme doit pouvoir s'éveiller et boucler son cycle.

Si ces deux commandes échouent, le code n'est pas valide, quelle que soit la beauté de l'idée sous-jacente.
