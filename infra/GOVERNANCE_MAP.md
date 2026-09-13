# CYNIC Governance — Carte du formalisé

Objet : mettre la doctrine à l'épreuve en recensant tout ce qui prétend
gouverner, son statut réel, sa contrepartie vivante et ses contradictions.
Méthode : les fichiers eux-mêmes. Pas de prose sans pointeur.

## 1. Échelle d'autorité en vigueur (AGENTS.md)

État observé > `contracts/foundation.json` > `infra/runtime-authority.json` >
`infra/registry.json` > `Cargo.toml` > crates canoniques. `protocols/` n'y
figure pas. `docs/` est mis en quarantaine comme prose historique. Tout
document de cette carte qui réclame un rang sans y figurer est une
prétention, pas un rang.

## 2. Prétentions concurrentes — le problème central

| Document | Prétention | Statut réel |
|---|---|---|
| `protocols/agent-protocol.md` (708 l.) | « complete executable specification », « supersedes all previous », tranche les désaccords | Prose sans rang. Référence un `agent-git-protocol.md` inexistant. Prétention maximale, fondation nulle. |
| `protocols/agent-coordination-protocol.md` (332 l.) | « normative », « supersedes the prose-based coordination section » | Chevauche la précédente sans mécanisme de départage entre elles. |
| `protocols/agent-constitution.md` (296 l.) | « normative constitution » | Auto-limitée : subordonnée aux contrats, « the Rust types win », loi seulement une fois transposée. Honnête. |
| `protocols/cynic-government-architecture.md` | « proposition normative en révision » | Au moins étiquetée proposition. Cinq pouvoirs + trésor + registre + magistratures. |
| `CONSTITUTION.md` (racine) | Layer 0, amendement par le Souverain seul | Subordonnée aux contrats par son propre en-tête. Deux constitutions coexistent sans lien de succession écrit. |
| `protocols/agent-degradation-and-execution-authority.md` | « design validé, AUCUNE autorité runtime créée » | Exemplaire : cite sa chaîne, son observation source, ses contrats à étendre. |

Verdict : trois documents se déclarent vainqueurs du même concours sans
arbitre désigné, et l'arbitre désigné (AGENTS.md) ne les reconnaît pas.
Tant que ce n'est pas tranché, chaque nouvelle proposition ajoute un
prétendant au lieu d'une loi.

## 3. Contreparties vivantes par sujet

| Sujet | Types (protocole) | Exécution | Verdict |
|---|---|---|---|
| Mandats, budgets, fencing | `peer/accounts.rs`, `agent_recursion.rs` | Partielle (vertical build) | Loi outillée par un seul vertical |
| Charges, réclamations, achèvements | `agent_workload/*` (13 modules) | Vertical build + magasin fichier | Le plus vivant du lot |
| Baux baseline | `coordination_lease` + prédicats | Magasin hôte réel | Coopératif, pas exécutif |
| Scheduler général | `agent_scheduler.rs`, step UC 98 l. | Un pas, pas un moteur | Décor avec un orteil vivant |
| Transactions pair | `peer/engine.rs:149` | Zéro appelant | Décor pur |
| Citoyenneté, sessions | `agent_citizenship.rs`, `agent_session` | Handshake MCP externe | Déclaré hors arbre |
| Portes de mutation | `agent_mutation_gates.rs` | À vérifier | Inconnu, pas zéro |
| Isolation | `agent_isolation.rs` + design worktree | Pratique d'agents, pas de moteur | Convention, pas contrainte |
| Énergie/tokens | Aucun type | Rien | Trou béant, doc proposée |

## 4. Formalisé implicite — jamais déclaré, toujours appliqué

`dead_code = deny` (tueur d'orphelins), `rustfmt --check`, lints workspace
comme enforcement couche 0, `cynic_validate`, hygiène de branches,
`git add` par chemins explicites. C'est le droit coutumier : personne ne
l'a voté, tout le monde le subit, et il mord plus que trois constitutions.

## 5. Épreuve de la doctrine énergie/gouvernement

La doctrine « intention descend, preuves remontent, bornes monotones »
est formalisée en types (délégation, budgets, achèvements) et en prose
redondante (trois vainqueurs). Elle est exécutée sur un seul vertical.
L'énergie proposée (`AGENT_ENERGY.md`) comble la mesure, pas le moteur :
sans scheduler, les enregistrements de consommation n'ont pas de
consommateur autre que l'audit. Une capacité d'observation sans porte
de décision est un thermomètre, pas un gouvernement.

## 6. Questions tranchantes

1. Quel document unique tranche entre `agent-protocol`,
   `agent-coordination-protocol` et `agent-constitution`, ou lesquels
   sont abrogés ?
2. `agent-protocol.md` perd sa référence morte et sa prétention
   d'exécutabilité, ou gagne l'exécutable qu'elle annonce ?
3. `peer/engine.rs` entre en service ou sort du contrat ?
4. Le pas scheduler existant grandit en moteur ou reste une marche ?
5. L'énergie s'observe pour l'audit seul (thermomètre assumé) ou avec
   sa porte de décision dès la phase 1 ?
