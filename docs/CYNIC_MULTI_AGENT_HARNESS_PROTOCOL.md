# CYNIC : Protocole de Coordination Multi-Agents & Matrices de Harnais

**Status :** Document contractuel d'architecture agentique (Normatif pour la coordination multi-LLM — voir `contracts/foundation.json`).

---

## I. Contexte & Problématique

Dans l'écosystème CYNIC, plusieurs instances d'agents (Codex, Claude, Grok, agents de build locaux) travaillent simultanément sur le même organisme. 

Sans protocole strict, le travail multi-agents génère 3 modes de défaillance destructeurs :
1. **Écrasement d'état concurrent** : Deux agents modifiant simultanément les mêmes surfaces sans synchronisation.
2. **Amplification de privilèges** : Un sous-agent s'auto-octroyant des droits ou des budgets supérieurs à son mandat.
3. **Pollution de contexte** : Échange de bavardages non structurés saturant la mémoire des modèles et créant des hallucinations croisées.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                      LES 4 PILIERS DU PROTOCOLE CYNIC                       │
├──────────────────────────┬──────────────────────────────────────────────────┤
│ 1. Baux de Coordination  │ Verrouillage de surfaces avec TTL & LeaseGuard   │
│ 2. Enveloppes Monotones  │ Budgets décroissants (Tokens, Actions, Children) │
│ 3. Évidence Adressée     │ Échange par hash SHA-256 (AgentEvidenceRef)      │
│ 4. Isolation des Harnais │ Cloisonnement Lab / Conformance / Production     │
└──────────────────────────┴──────────────────────────────────────────────────┘
```

---

## II. Pilier 1 : Les Baux de Coordination (*Coordination Lease Fencing*)

Formalisé dans `crates/cynic-protocol/src/coordination_lease.rs` :
* **Prise de bail obligatoire (*Claim*)** : Avant d'entamer une tâche mutatrice sur un domaine (ex: `crates/cynic-inference/`), l'agent doit émettre un `cynic_coord_claim`.
* **Règles d'arbitrage** :
  * Si la surface est libre : le bail est accordé avec un `LeaseGrant` (identifiant unique, expiration en millisecondes).
  * Si la surface est déjà réservée : l'agent reçoit un `LeaseConflict` immédiat et doit soit attendre la libération (`cynic_coord_release`), soit basculer sur un domaine orthogonal.
* **Fencing anti-stale** : Tout commit ou action exécuté avec un bail expiré est rejeté par le kernel.

---

## III. Pilier 2 : La Récursion & Les Budgets Monotones

Formalisé dans `crates/cynic-protocol/src/agent_recursion.rs` :

```text
               SUPERVISEUR (Depth 0 - Budget Global)
                          │
            ┌─────────────┴─────────────┐
            ▼                           ▼
      SOUS-AGENT A                SOUS-AGENT B
   (Depth 1 - Budget/2)        (Depth 1 - Budget/2)
            │
            ▼
      SOUS-AGENT A.1
   (Depth 2 - Budget/4)
```

* **Règle constitutionnelle de monotonie** :
  $$\text{Tokens}(C) \le \text{Tokens}(P), \quad \text{Actions}(C) \le \text{Actions}(P), \quad \text{Depth}(C) = \text{Depth}(P) + 1$$
* **Enveloppe de délégation (`cynic.agent_delegation_envelope.v1`)** :
  * `objective` : Intention concise et bornée.
  * `capabilities` : Liste blanche stricte des outils MCP autorisés.
  * `expected_proofs` : Schéma de résultat attendu.
* **Enveloppe de complétion (`cynic.agent_completion_envelope.v1`)** :
  * Retourne `status` (`completed`, `blocked`, `failed`), `consumed_budget` et `evidence_refs`.

---

## IV. Pilier 3 : Échange par Évidence Adressée par le Contenu

Pour éviter l'inflation de contexte :
* **Interdiction du bavardage brut** : Les agents ne s'envoient pas de longs historiques de chat.
* **Passage par référence SHA-256 (`AgentEvidenceRef`)** :
  ```rust
  pub struct AgentEvidenceRef {
      pub schema: String,
      pub digest: String, // SHA-256
      pub path: Option<String>,
  }
  ```
  Un agent publie son résultat dans un artifact JSON typé, et ne transmet à ses pairs que le hash et le chemin. L'agent récepteur ne charge que les sections strictement nécessaires à son calcul.

---

## V. Pilier 4 : La Matrice des Harnais d'Isolation (*Harness Matrix*)

Les agents doivent adapter leur niveau d'agressivité au harnais d'exécution déclaré :

| Harnais | Nature | Droit de Mutation | Risque Réseau / Hôte | Autorité |
|---|---|---|---|---|
| **`lab`** | Sandbox local fermé | Mocks / Bundles VM libvirt | **Zéro** (pas de mutation hôte) | Test & Exploration |
| **`conformance`** | Sonde d'inspection | Lecture seule stricte | **Zéro** (observation pure) | Qualification de Nœud |
| **`staging`** | Environnement miroir | Bounded apply (`--apply`) | Isolé (VM/LXC dédié) | Release Candidate |
| **`canary`** | Nœuds pilotes réels | Bounded apply sur sous-ensemble | Contrôlé avec rollback auto | Validation Flotte |
| **`production`** | Organisme vivant | Soumis à validation opérateur | Réel | SSOT Runtime |

---

## VI. Protocole d'Action Recommandé pour les Sessions Agents

Pour toute session de travail avec un agent CLI ou un LLM distant :
1. **Étape 1 : Diagnostic & Prise de bail** :
   ```bash
   # Vérifier les baux actifs
   cynic-mcp: cynic_coord_who
   # Déposer un claim sur le domaine ciblé
   cynic-mcp: cynic_coord_claim {"domain": "audit_engine", "ttl_ms": 3600000}
   ```
2. **Étape 2 : Exécution bornée** :
   * Ne modifier que les fichiers relevant du domaine réservé.
   * Valider en continu avec `make check` et `cynic dev audit-v2 --json`.
3. **Étape 3 : Libération du bail & Publication de preuve** :
   * Libérer la surface pour les autres agents (`cynic_coord_release`).
   * Émettre l'enveloppe de preuve d'action.
