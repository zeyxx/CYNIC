# Analyse des Surfaces Agentiques & Généalogie des Structures Émergentes

**Status :** Document d'analyse architecturale et historique (Non-runtime authority — voir `docs/README.md` et `contracts/foundation.json`).

---

## I. Cartographie des Surfaces Agentiques

CYNIC n'interagit pas avec les agents via un "chat libre", mais à travers trois canaux d'interface étanches et typés.

```text
                             ┌──────────────────────────────┐
                             │       OPÉRATEUR / LLM        │
                             └──────────────┬───────────────┘
                                            │
        ┌───────────────────────────────────┼───────────────────────────────────┐
        ▼                                   ▼                                   ▼
┌───────────────────────┐       ┌───────────────────────┐       ┌───────────────────────┐
│ 1. CANAL MCP (STDIO)  │       │ 2. CANAL CLI ORCHEST. │       │ 3. CANAL RÉCURSIF     │
│   (Outils Déclarés)   │       │   (Plans & Preuves)   │       │ (Enveloppes Enfants)  │
│                       │       │                       │       │                       │
│ • cynic-mcp           │       │ • cynic runtime ...   │       │ • agent_recursion.rs  │
│ • agent-exposure.json │       │ • cynic lab ...       │       │ • Budgets monotones   │
│ • 50+ tools typés     │       │ • cynic dev ...       │       │ • Content-addressed   │
│ • Audit de fuite path │       │ • JSON Schema v1      │       │   evidence            │
└───────────────────────┘       └───────────────────────┘       └───────────────────────┘
```

### 1. Le Canal MCP (`cynic-mcp` / Stdio)
* **Contrat d'honnêteté (`infra/agent-exposure.json`)** : Chaque outil exposé à un agent (Codex, Claude, etc.) doit être enregistré et classifié par domaine (`organism_status`, `inference`, `coordination`, `agent_tasks`, `operator_local`, `network_mesh`, `fleet_and_bridge`, `services_and_storage`).
* **Barrière anti-fuite** : Validé par `cynic dev agent-exposure --json`. Interdiction absolue de commit des chemins locaux (`/home/`, `C:\Users\`) ou des secrets dans les configurations d'agents.
* **Séparation Tier Produit vs Baseline Runtime** : Les outils sont explicitement étiquetés (`baseline_runtime`, `operator_local`, `mixed_baseline_and_product`).

### 2. Le Canal CLI Orchestrateur (`cynic-cli`)
* Les commandes CLI ne sont pas des scripts "fire-and-forget", mais des **projections d'actions bornées**.
* **Grammaire universelle** :
  * `--json` : Émission systématique d'un payload JSON versionné (`cynic.<domaine>.<action>.v1`).
  * `plan` vs `apply` : Séparation obligatoire entre l'intention d'action (pure fonction) et la mutation réelle (`--apply` / `--yes`).
  * `preflight` & `read-back proof` : Observation de l'état avant d'agir, vérification de l'état après action.

### 3. Le Protocole de Récursion Agentique (`cynic-protocol::agent_recursion`)
Formalisé dans `crates/cynic-protocol/src/agent_recursion.rs` :
* **Enveloppe de délégation (`cynic.agent_delegation_envelope.v1`)** : Un agent parent délègue un sous-objectif en fixant une membrane stricte.
* **Invariants de budget monotone** :
  $$\text{Budget}_{\text{enfant}} \le \text{Budget}_{\text{parent}} \quad (\text{Tokens}, \text{Actions}, \text{Children}, \text{WallTime})$$
  Un enfant ne peut jamais amplifier ses droits, muter un composant sans mandat explicite, ni dépasser la profondeur maximale de récursion.
* **Enveloppe de complétion (`cynic.agent_completion_envelope.v1`)** : Le retour ne contient pas du texte brut, mais des références de preuves adressées par leur condensat SHA-256 (`AgentEvidenceRef`) et une liste explicite d'*inconnus résiduels* (`unknowns`).

---

## II. La Matrice de Préparation Agentique (*Agent Matrix*)

Tous les sous-systèmes ne sont pas immédiatement automatisables. `crates/cynic-host/src/agent_matrix.rs` catégorise dynamiquement chaque surface en 3 niveaux de maturité :

```text
               ┌──────────────────────────────────────────────────┐
               │              AGENT MATRIX READINESS              │
               └────────────────────────┬─────────────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        ▼                               ▼                               ▼
┌───────────────────────┐   ┌───────────────────────┐   ┌───────────────────────┐
│   AgentifiableNow     │   │ FormalizeNow/Activate │   │  NotYetAgentifiable   │
│  (PilotEnabled: Obs/  │   │ Later (Declared: Plan/│   │ (Inactive: Verrouillé)│
│       Prover)         │   │      Supervisor)      │   │                       │
│                       │   │                       │   │                       │
│ • principal/privilege │   │ • host_activation_plan│   │ • update_lifecycle    │
│ • services/packages   │   │ • host_apply_execution│   │ • framework_bootstrap │
│ • host_bootstrap_ev.  │   │ • bootstrap_constell. │   │ • lab (harness)       │
│ • storage_capabilities│   │ • recursive_multi_ag. │   │ • inference_surface   │
└───────────────────────┘   └───────────────────────┘   └───────────────────────┘
```

* **Règle fondamentale** : Une surface dont la décision de rollback ou la preuve de read-back n'est pas 100% déterministe reste en `NotYetAgentifiable` ou `FormalizeNowActivateLater`. On ne délègue jamais d'action mutatrice aveugle à un LLM.

---

## III. Généalogie Historique : Les Structures Émergentes du Chaos

L'historique des commits et des checkpoints révèle une trajectoire d'auto-organisation remarquable en 4 phases :

```text
TEMPS
  │
  ├─► PHASE 1 : LE CHAOS PRIMITIF (L'ère Python / Organs / Prompts Magiques)
  │   - Des dizaines de "scripts d'organes" Python instables.
  │   - Documentation pléthorique servant d'alibi à un code incomplet.
  │   - Hallucinations acceptées comme vérité ; mutations d'état non tracées.
  │
  ├─► PHASE 2 : LA KÉNOSIS & LA COUCHE 0 (Purification par le compilateur)
  │   - Destruction radicale des faux organes et des axiomes narratifs.
  │   - Instauration de l'ordre d'autorité strict (Runtime > Contrats > Cargo > Crates).
  │   - Lints industriels au niveau rustc/clippy (zéro unsafe, zéro unwrap, zéro dead code).
  │
  ├─► PHASE 3 : LA CRISTALLISATION MODULAIRE (Lois Anti-Monolithes)
  │   - Découpage obligatoire : types → policy/plan → port → adapter → render → CLI.
  │   - Émergence du pattern CYNIC : Desired State → Observe → Reconcile → Action → Proof.
  │   - Les outils et OS deviennent des adaptateurs interchangeables.
  │
  └─► PHASE 4 : L'ORDRE AGENTIQUE TYPÉ (Aujourd'hui)
      - Contrats d'admission typés pour chaque commit, build et délégation.
      - Harnais d'exécution étanches (Lab, Conformance, Metrics).
      - Quantification de l'inconnu résiduel (Null-budget).
```

### 1. Phase 1 : L'illusion de l'organisme textuel (Le Chaos)
Au début, le projet souffrait du travers typique des architectures assistées par IA : **la prolifération de prose non exécutable**. Des concepts métaphoriques (`axioms/`, `organs/`, boucles Python) masquaient l'absence de substrat réel. Les agents validaient des plans dans le vide car il n'existait pas de boucle de rétroaction avec le silicium.

### 2. Phase 2 : Le grand nettoyage (*Kénosis*)
Face aux régressions continues, une purge constitutionnelle a été actée :
* **Interdiction des docs comme loi d'exécution** : `docs/` est placé en quarantaine historique (`docs/README.md`).
* **Le compilateur Rust devient le juge suprême (Layer 0)** : Tout ce qui ne compile pas de manière déterministe avec `make check` n'existe pas.

### 3. Phase 3 : L'émergence des invariants structurels
Pour empêcher les agents de réintroduire des monolithes ingérables de 2000 lignes, des **règles géométriques** ont été gravées dans `AGENTS.md` et dans les audits automatisés (`cynic dev surface-audit`, `cynic dev kernel-audit`) :
* **Plafond de 400 lignes** par fichier de domaine sans extraction de sous-module ou de port.
* **Interdiction du shell arbitraire** : Toute interaction système passe par un port d'exécution typé (`ArgvRunner`).
* **Isolation du harnais de test** : Le lab de test (`lab`) ne doit jamais fuiter dans l'autorité de runtime de production.

### 4. Phase 4 : L'agent comme charge de travail bornée (*Bounded Workload*)
L'état final observable aujourd'hui :
* Un agent n'est plus un "collaborateur qui discute", mais une **fonction pure de calcul et d'inspection**.
* L'agent reçoit une preuve d'entrée (`context_refs`), produit une mutation atomique, et doit retourner une preuve de sortie vérifiable (`read-back proof`).
* L'incertitude n'est plus camouflée : si une observation échoue, elle remonte comme `unknown` explicite dans le rapport de métriques (`cynic runtime metrics --json`).

---

## IV. Les 4 Lois Durables Extraites de l'Expérience

L'évolution de CYNIC a dégagé quatre lois invariantes pour le développement futur :

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. LOI D'ASYMÉTRIE VÉRITÉ / INTENTION                                      │
│    L'intention descend (plans, requêtes) ; la vérité monte uniquement par   │
│    l'observation du substrat (read-back). Une commande réussie sans preuve  │
│    n'est pas une convergence.                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. LOI DE MONOTONIE DES PRIVILÈGES                                          │
│    Dans toute récursion d'agents, autorité, budgets et capacités décroissent│
│    strictement avec la profondeur. Aucun sous-agent ne peut s'auto-octroyer │
│    des droits supérieurs à son créateur.                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. LOI DE PÉRENNITÉ DES PORTS VS DISPOSITION DES ADAPTATEURS               │
│    Les technologies d'infrastructure (systemd, uv, Proxmox, Git, WireGuard)│
│    sont des adaptateurs jetables. Seuls les ports et les preuves sont       │
│    durables. Remplacer un outil ne doit jamais réécrire l'orchestration.    │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. LOI DE L'INCONNU RÉSIDUEL QUANTIFIÉ                                      │
│    Le 100% de couverture théorique est une illusion. Un système industriel  │
│    robuste est un système qui quantifie exactement son budget d'inconnu     │
│    résiduel sans jamais le convertir en faux succès.                        │
└─────────────────────────────────────────────────────────────────────────────┘
```
