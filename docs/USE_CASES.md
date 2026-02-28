# CYNIC — Core Use Cases & Proofs of Concept

L'architecture unifiée de CYNIC (V3.1) n'est pas conçue pour tout faire. Elle est mathématiquement ancrée dans 4 réalités fondamentales (définies dans `kernel/core/consciousness.py`). Chaque réalité correspond à un Use Case spécifique, avec un niveau de conscience et une priorité métabolique dédiés.

---

## 1. Réalité : CODE (Le Bâtisseur Sécurisé)
**Use Case : Audit, Refactoring et Sécurité de Codebases (DevSecOps).**
*   **La Logique Sous-Jacente** : CYNIC lit un fichier source, active ses 11 Chiens (dont l'Architecte et l'Analyste) en mode **MACRO** (Haute priorité : 0.9). Il cherche les vulnérabilités, le code mort, ou les violations de design.
*   **L'Action (Métabolisme)** : Si le Q-Score est bas (BARK/GROWL), il utilise le `ClaudeCodeRunner` pour générer un correctif ou lever une alerte CI/CD.
*   **La Preuve (Déjà établie)** : Le script `scripts/dna_chain_demo.py` prouve ce cas d'usage en simulant la détection d'une vulnérabilité (RCE) et son traitement par l'Axiome PHI.

## 2. Réalité : SOCIAL (Le Juge de Gouvernance)
**Use Case : Modération de DAOs, Résolution de Conflits et Vote Décentralisé.**
*   **La Logique Sous-Jacente** : CYNIC lit une proposition issue de Discord ou Telegram. Il se place en mode **MICRO** (Priorité : 0.6) pour délibérer rapidement. Le `GovernanceAgent` utilise la `Q-Table` pour comparer la proposition aux valeurs historiques de la communauté (Axiome CULTURE).
*   **L'Action (Métabolisme)** : Si le consensus est HOWL (Approbation forte), l'Organisme déclenche le `gasdf_executor` pour valider la proposition *on-chain* (NEAR).
*   **La Preuve (À construire)** : Nous devons créer `scripts/dna_social_demo.py` pour simuler l'ingestion d'une proposition discord jusqu'à sa validation simulée.

## 3. Réalité : MARKET (Le Sceptique Financier)
**Use Case : Analyse de Sentiment, Détection de FUD/FOMO et Trading Défensif.**
*   **La Logique Sous-Jacente** : Les "Senses" de CYNIC absorbent des flux de données externes (prix, tweets). Il se place en mode **MICRO** (Priorité : 0.8). Il utilise l'Axiome BURN pour éviter les actions impulsives et l'Axiome VERIFY pour exiger des preuves cryptographiques avant de recommander un mouvement.
*   **L'Action (Métabolisme)** : Émettre un signal de `MARKET_ALERT` sur le Bus, qui peut déclencher un swap automatique ou une alerte communautaire.
*   **La Preuve (Latente)** : Les capteurs existent (`senses/workers/market.py`), mais le pont avec l'action finale doit être éprouvé.

## 4. Réalité : CYNIC (L'Auto-Amélioration)
**Use Case : Optimisation d'Infrastructure et Auto-Réparation (Self-Healing).**
*   **La Logique Sous-Jacente** : C'est le niveau le plus profond. CYNIC s'observe lui-même via le `SymbioticStateManager` et le `SelfProber`. Il tourne en mode **MACRO/META** (Priorité absolue : 1.0). Si son propre taux d'erreur augmente (entropie), il génère une `ActionProposer` pour se corriger.
*   **La Preuve (Établie)** : Les traces de notre console montrent l'organisme émettant `ACTION_PROPOSED: type=ALERT priority=2 → BUILD EScore=82.0`. Il est conscient de ses propres alertes.

---

### Comment prouver l'avancée de ces Use Cases ?
Pour ne plus retomber dans le chaos, chaque avancée sur un Use Case doit être matérialisée par un **Script DNA** dans le dossier `scripts/` (ex: `dna_code_audit.py`, `dna_dao_vote.py`). 

*Si un script DNA s'exécute de bout en bout sans erreur et met à jour la Q-Table, le Use Case est validé et prêt pour la production.*
