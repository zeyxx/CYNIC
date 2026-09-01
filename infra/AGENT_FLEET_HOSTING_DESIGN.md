# Hébergement agentique hors Cortex — design de convergence

- Statut : design proposé ; aucune mutation de fleet, de réseau, de secrets ou de services n’est autorisée par ce document.
- Décision de direction : Cortex/Ubuntu devient un poste d’inférence et une console opérateur optionnelle. Il ne doit plus être l’hôte durable des sessions, workspaces ou superviseurs agentiques.
- Problème racine : lancer plusieurs agents dans le même checkout Cortex partage le `git-dir`, le `HEAD` et l’index. Le changement de branche d’un processus modifie le contexte de tous les autres.

## Faits observés

| Élément | Observation | Conséquence |
|---|---|---|
| Cortex | 27 GiB RAM, 16 CPU, Docker et Bubblewrap présents ; pas de contrôle Proxmox local | ressource d’inférence à préserver ; pas la frontière de fleet |
| Proxmox | provider déclaré dans `infra/registry.json`; accès de contrôle et inventaire déjà modélisés | candidat de placement, non preuve de capacité disponible |
| Builder | rôle `build_runner` observé, LXC historique `lxc/105` | le build fédéré a déjà une frontière de placement, mais son runtime Git/queue est dégradé |
| Isolation | types `BwrapEphemeral`/`LxcUnprivileged` et ports sandbox présents ; adaptateurs concrets encore absents | aucun sandbox runtime prouvé aujourd’hui |
| Notifications | quelques mentions Telegram historiques, aucun port/receipt de notification agentique observé | Telegram ne doit pas être traité comme un bus d’exécution |

## Décision de sécurité : VM primaire, LXC/bwrap secondaires

La frontière primaire recommandée est une **VM Proxmox dédiée `cynic-agent-host`**, non une LXC qui monterait le dépôt commun.

Raisons :
- les agents peuvent recevoir du code, des dépôts et des instructions externes non fiables ;
- une VM sépare aussi le noyau de Cortex et protège la disponibilité de l’inférence ;
- elle reste active lorsque Cortex est éteint et rend RustDesk facultatif pour l’opération agentique ;
- LXC partage le noyau de l’hyperviseur : il peut servir à un worker borné, mais pas être présumé équivalent à une barrière VM.

Étagement retenu :

```text
Proxmox hyperviseur
  └── VM cynic-agent-host                  frontière primaire, persistante
       ├── supervisor agent/node-owned     durable, sans checkout humain
       ├── mirror Git privé par workload    stockage privé, accès Forgejo borné
       ├── sandbox bwrap par agent          PID/mount/net/user namespaces
       │    └── workspace + branche privés  seul Git RW accessible à l’agent
       └── notification outbox              preuves et états seulement

  └── LXC cynic-builder                     compilation d’un SHA Forgejo exact
  └── LXC/VM workers futurs                 selon profil de risque et budget

Cortex / Ubuntu
  ├── inference locale                     optionnelle, jamais précondition d’agent
  └── RustDesk / console humaine            observation/intégration, pas control plane
```

Un LXC unprivileged peut devenir un worker à coût réduit uniquement après preuve de son profil : pas de privilège, pas de socket Proxmox/Docker, racine readonly, réseau default-deny, stockage/worktree exclusifs, quotas CPU/RAM/disque et read-back. Les agents ayant egress arbitraire, code non revu ou secret de runtime restent dans la VM primaire ou une VM éphémère dédiée.

Le Builder LXC n'est pas exempté : compiler un PR exécute aussi `build.rs` et des proc-macros. Chaque compilation candidate doit donc s'exécuter dans un sandbox éphémère non privilégié, rootfs readonly et réseau default-deny **à l'intérieur** du LXC Builder. Le LXC ne constitue pas, seul, la preuve d'isolation du workload de build.

## Invariant sandbox-first

```text
admission workload
  → VM/worker sélectionné et profil observé
  → sandbox attestée
  → phase bootstrap : mirror privé alimenté par le supervisor ou credential Forgejo read-only éphémère
  → transition attestée : credential retiré et egress Forgejo fermé
  → branche + workspace privés
  → phase agent : réseau default-deny (ou inference locale explicitement whitelistée)
  → lancement agent
```

Aucune commande Git agent — y compris `clone`, `fetch`, `worktree add`, création de branche ou checkout — ne s’exécute hors du périmètre isolé.

Le checkout Cortex humain, le dépôt Builder par défaut, les worktrees voisins et les credentials Proxmox sont absents de la sandbox. Un bind mount RW du checkout partagé produit `blocked_shared_git_admin`. Aucun token Forgejo écrivable, clé SSH persistante ou credential Proxmox ne peut être monté dans la phase agent.

## Autorités par plan

| Plan | Autorité | Interdit |
|---|---|---|
| Source | Forgejo : refs, PR, SHA candidat | Builder ou agent créant/réécrivant `main` |
| Workspace | agent-host : sandbox/mirror/worktree éphémère liés au workload | checkout humain partagé |
| Build | builder : checkout propre du SHA PR + preuve | compilation Cortex comme preuve promotionnelle |
| Déploiement | policy/humain distinct après artifact attesté | build vert = déploiement |
| Notification | outbox durable + adaptateur Telegram | commande, capability, secret, ou consentement via Telegram |

## Telegram : notification, jamais contrôle

Un futur adaptateur Telegram ne reçoit que des projections issues d’un receipt durable : état de workload, URL/ID de PR, SHA abrégé, verdict build, blocage, lien de preuve et résidu non sensible.

Il ne peut jamais :
- créer ou renouveler une lease ;
- admettre une capability, une mutation, un merge, un push ou un déploiement ;
- accepter des commandes shell ;
- transporter token, HMAC, secret Forgejo/Proxmox ou contenu de worktree.

Une réponse Telegram peut au plus créer une **demande** au format typé, ensuite validée dans le control plane avec identité, consentement explicite et lecture indépendante. Cette capacité est différée après la chaîne C.0–C.2.

## Contrats à formaliser avant provisionnement

1. `AgentHostDesired` : rôle, harness, principal, capacité, profils permis, politique d’egress, autonomie power-state de Cortex.
2. `AgentSandboxAllocation` : workload/agent, VM ou worker sélectionné, isolation digest, TTL/budgets, mirror Git privé, base SHA Forgejo, branche/workspace, mounts et réseau.
3. `AgentSandboxObservation` : VM identity, supervisor, cgroup/PID, mounts, git-dir/cwd, branch/HEAD, egress observé et residuals.
4. `AgentSandboxReceipt` : allocation, lancement, fin, quotas observés, nettoyage/quarantaine/retention, preuves sans secrets.
5. `NotificationOutboxEntry` : événement référencé, canal, classification de contenu, idempotency key, delivery receipt ; aucune payload d’autorité.

Ces contrats appartiennent au protocole pur. Les observations/provisionnements Proxmox et sandbox sont des adapters host ; la corrélation de workload est node-owned ; CLI/MCP/Telegram projettent seulement.

## Phases d’implémentation

### F0 — observation sans mutation

- Inventorier Proxmox : capacité, stockage, réseau, VMID libres, templates, bridge, quotas et politique d’egress.
- Observer les capacités réelles de l’hyperviseur et du futur guest, sans présumer `pvesh`, LXC ou template utilisable.
- Produire un rapport `ready | blocked | unknown` pour la VM agent-host.
- Vérifier que Cortex peut s’éteindre sans affecter Forgejo, builder, stockage, control plane ou observabilité.

### F1 — contrats et plan pur

- Définir les cinq contrats ci-dessus et leurs prédicats fail-closed.
- Refuser workspace hors sandbox, git-dir partagé, branche absente, base SHA non Forgejo, profile digest divergent, mount RW excessif ou egress non prouvé.
- Créer la décision de placement VM/LXC sur capacité et profil de risque, jamais par nom de machine.
- Lier quota par sandbox et quota total agent-host à chaque allocation ; l'expiration produit une disposition `retain`, `quarantine` ou `eligible_for_human_review`, jamais un effacement implicite.
- Tests purs : refus et happy-path simulé ; aucune commande Proxmox.

### F2 — harness conformance VM

- Créer une VM candidate seulement sous plan explicite `--apply`, VMID/ressources/réseau déclarés, rollback et stop condition.
- Installer un principal non privilégié, un runtime CYNIC attesté, puis un supervisor durable.
- Read-back : identité VM, provenance image, principal, cgroup, disques, réseau, service state et capacité.
- Pas de migration Hermes productive pendant F2.

### F3 — sandbox/runtime workspace

- Implémenter/adopter Bwrap réel dans la VM ; rootfs readonly, tmpfs borné, user/mount/pid/net namespaces.
- Créer mirror et workspace par workload dans le stockage de la VM, jamais dans Cortex. Le mirror est alimenté pendant une phase bootstrap Forgejo à egress/credential bornés, puis ces deux capacités sont retirées avant l'exécution agent.
- Tester deux agents simultanés : checkout de l’un ne peut pas modifier le HEAD, index, cwd ou mounts de l’autre.
- Retenir les receipts et conserver les workspaces expirés selon une policy explicite ; aucun `--force` automatique.

### F4 — source/build fédéré

- PR Forgejo obligatoire, head SHA lu après push.
- Builder crée son checkout privé/éphémère au même SHA et produit une BuildProofBundle durable ; le cargo du PR s'exécute dans un sandbox éphémère interne au Builder LXC.
- VM agent-host reçoit seulement le receipt/projection ; Cortex reste facultatif.

### F5 — notification outbox

- Outbox durable, redaction/classification, retry idempotent et receipt de livraison.
- Adapter Telegram opt-in, credentials injectés dans la VM via référence secrète, jamais Git.
- Notification de lecture seule ; aucune commande entrante n’est actionnée automatiquement.

## Gates / stop conditions

- Sans observation de capacité Proxmox, F2 est `blocked`.
- Sans isolation lue après lancement, aucun agent ne reçoit de Git.
- Sans SHA `Forgejo = workspace = builder = proof`, aucune promotion source.
- Sans receipt durable, aucune notification n’est envoyée.
- Workload, build trigger et preuve circulent uniquement dans des envelopes node-owned et des références d'évidence adressées par contenu ; aucun broker/ledger central supplémentaire n'est créé.
- Sans consentement explicite, aucune VM/LXC, réseau, secret, service ou migration n’est modifié.
- Le système reste fonctionnel quand Cortex est éteint ; sinon le design est incomplet.

## Critères d’acceptation finaux

1. Deux agents parallèles possèdent chacun VM/sandbox/worktree/Git admin indépendants et prouvés.
2. Cortex peut être arrêté ; Forgejo, agent-host, builder et leurs preuves restent observables.
3. Un agent ne peut pas voir les checkouts voisins ni exécuter `git checkout` sur un dépôt partagé.
4. Chaque branche candidate passe par PR et build d’un SHA exact sur builder.
5. Telegram reçoit un résumé redacted avec receipt, sans devenir une autorité d’exécution.
