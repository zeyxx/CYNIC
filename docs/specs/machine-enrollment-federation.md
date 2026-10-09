# Machine enrollment & maintien en fédération — design Phase 0

Statut : PROPOSITION (aucun code tant que non validé).
Portée : comment un nœud/VM rejoint la fédération CYNIC, garde son identité,
et reste membre — sans coupler les couches entre elles.
Autorités supérieures : runtime observé > `contracts/foundation.json` >
`infra/runtime-authority.json`. Ce document n'est pas runtime law ; il devient
des types purs dans `cynic-protocol` une fois validé.

## 1. Les quatre couches à ne jamais coupler

Chaque couche répond UNE question, possède son propre cycle de vie, son propre
proof schema. Un nœud peut être à des niveaux différents par couche.

| # | Couche | Question | Preuve | Cycle |
|---|--------|----------|--------|-------|
| L1 | **Identité cryptographique** | Qui suis-je ? | ed25519 subject key + `MachineEnrollment` signé (existant : `cynic.machine_enrollment.v1`) | longue durée, rotation explicite |
| L2 | **Enrollment (admission)** | Ai-je le droit de rejoindre, avec quel rôle/capacités ? | `NodeEnrollmentGrant` signé par l'autorité (existant, kernel REST `/node-enrollment`) | TTL moyen, renouvelable (`generation` monotone) |
| L3 | **Membership (fédération)** | Suis-je membre actif aujourd'hui ? | vue locale gossiped, états `active/suspect/lapsed/expelled` (types purs existants : `federation_membership.rs`, NON câblés) | secondes/minutes, volatile |
| L4 | **Transport** | Par quel canal parlé-je ? | read-back du canal : WG handshake, mTLS, HTTP kernel | adaptateur remplaçable |

Règle centrale : **L1 ne meurt jamais parce que L3 est en panne.** Un nœud
expulsé du membership garde sa clé et son grant ; il est `lapsed`, pas détruit.
Inversement un transport mort (WG down) ne suspend pas le membership tant que
le nœud répond sur un autre plan.

### Découplages explicites (interdits de couplage)

- Enrollment ≠ transport : le bootstrap SSH/provider (seed-only, borné,
  `bootstrap_containment` du registre) doit pouvoir être SUPPRIMÉ après
  enrollment sans invalider l'identité. Un jour: zéro SSH dans le steady state.
- Membership ≠ enrollment : expirer un membership (L3) ne révoque pas le grant
  (L2). Ré-admettre un nœud revenant = renouveler L3, PAS repasser L1.
- Identité ≠ principal Unix : `node_id` + clé ed25519 sont l'identité runtime.
  Le compte OS (`cynic@`) est un détail d'exécution local, jamais un secret
  d'enrollment.
- Attestation ≠ assertion : preuves machine (fingerprints, `/proc`, hyperviseur)
  collectées localement (style SPIRE node attestor) ; le kernel DÉCIDE, il ne
  fabrique pas les preuves.
- Consensus ≠ membership : membership = AP, émergence déterministe (déjà loi :
  `elect_for_role`, pas de leader election type Raft). Le KV fortement cohérent
  reste ailleurs (storage_primary).

## 1bis. L0 — L'ancre d'amorçage hors-bande (théorème œuf-poule)

Aucune confiance ne naît d'un canal non cru : rejoindre exige une ancre
externe pré-existante (constaté identiquement dans k8s, Tailscale, SPIRE,
Serf, Bluetooth pairing, SSH TOFU). Les « fragmentations » du domaine sont
des choix différents d'ancre, pas des problèmes différents.

### Couche L0 explicite

| # | Couche | Question | Preuve | Cycle |
|---|--------|----------|--------|-------|
| L0 | **Amorçage hors-bande** | Qui garantit la machine AVANT toute relation ? | reçu one-shot consommable (token digest, attestation provider, geste console) | unique, TTL court, mort après usage |

Règles L0 :
- L0 produit un REÇU (evidence), consommé une fois par L2 ; L0 n'a aucun
  pouvoir en steady state (loi `runtime_human_secret_dependency`).
- Le join est toujours initié PAR LE NŒUD (pull) : la fédération juge des
  demandes, elle ne force ni ne pénètre aucune machine.
- L'ancre meurt : après read-back du grant, le canal bootstrap est retiré
  (`removal_requires` du registre). Un bootstrap qui doit rester branché
  est un défaut de conception.

### Variantes d'ancre unifiées (registry data, pas nouveaux protocoles)

Une seule forme typée : `EnrollmentVoucher { kind, delivery, ttl_s,
single_use, evidence_refs }` avec variantes :

- `human_console` : geste HumanAdmin (token collé au clavier / staging
  manifest) — cas builder aujourd'hui ;
- `provider_attestation` : l'hyperviseur ATTESTE la ressource qu'il a lui-
  même créée (Proxmox sait que lxc/105 existe, son ID, son réseau) —
  surface `provider-enrollment-plan/apply` déjà présente ;
- `sponsor_node` : un nœud enrollé transmet le voucher par le plan synapse
  (jamais par un canal qu'il crée ad hoc) — l'ancre reste le grant du
  sponsor, borné par ses propres capacités ;
- (futur) `tpm_hardware` : chaîne matérielle — hors périmètre immédiat.

Le choix d'ancre est une donnée du registre par nœud, pas une branche de
code. Première amorce absolue (kernel primaire) = `human_console`
irréductible : il existe toujours un t₀ où un humain (ou la chaîne
physique de la salle serveur) est l'ancre.

## 2. Inventaire de l'existant (observé, pas supposé)

Déjà codé et testé :
- L1 : `cynic_protocol::enrollment::MachineEnrollment` (validation pure,
  signature détachée vérifiée côté host, `authority_key_id`, evidence_refs
  obligatoires).
- L2 : `cynic_protocol::node_enrollment` (desired state, authority policy avec
  TTL/token digest, admission, grant signé + génération, renewal, reconcile) ;
  kernel REST POST `/node-enrollment`, renewal, csr-enroll ; stores SQLite/Surreal
  avec CAS (`replace_grant` + `expected_generation`) et read-back obligatoire ;
  reader côté node (`LocalNodeEnrollmentReader`).
- L3 : types purs `federation_membership.rs` (`FleetRole`, `MemberStatus`,
  `MemberLease`, `plan_federation_membership` → Admit/Update/Renew/Keep/
  Suspend/Expel) — **non câblés** (aucun consommateur runtime).
- Bootstrap fédéré : `bootstrap_federation.rs` (phases, binding, read-back) +
  CLI `cynic bootstrap framework {enrollment-authority,provider-enrollment}-{plan,apply}`.
- Transport : WireGuard full-mesh désiré dans le registre ; watchdog stale/dead
  déclarés (150s/200s) mais non reliés au membership.

Manques identifiés :
1. L3 sans runtime : personne ne calcule `suspect → lapsed → expelled` en
   continu ni ne gossippe l'état. Le registre déclare `watchdog_stale_s/dead_s`
   orphelins.
2. Pas de boucle de RÉCONCILIATION membership : `plan_federation_membership`
   existe, aucun tick ne l'appelle avec observation fraîche.
3. Le lien desired→observed pour le transport (WG peer déclaré vs handshake vu)
   n'est pas un report typé (on l'a fait ad hoc pour builder aujourd'hui).
4. Renouvellement L2 piloté par quoi ? (pas de politique « renew à X % de vie »
   observable côté node.)
5. Deux vocabulaires d'enrollment coexistent (`node_enrollment.rs` grant-based
   vs `machine_identity.rs` spec/observation) : risque de doublon décisionnel
   (violation §481 one_boundary_one_reason) — à fusionner ou subordonner.

Références prod étudiées : kubelet TLS bootstrapping (token one-shot → CSR →
approve → cert courte durée), SPIRE (node attestation → identity courte durée
renouvelée automatiquement), Serf/SWIM (membership AP, suspect indirect,
piggyback gossip, suspicion timer accéléré, Lifeguard).

Choix alignés : L2 = modèle kubelet/SPIRE (grant court renouvelable, jamais de
secret long-lived en runtime) ; L3 = SWIM simplifié (sonde directe puis
indirecte avant suspect ; état diffusé, pas de consensus).

## 3. Flux cibles

### 3.1 Join initial (nœuf neuf)
```
provider crée VM/LXC (désir: registry)
→ seed-only: transport bootstrap borné (SSH/provider, MachineEnrollment class,
  receipt à durée de vie unique — déjà contracté §75 foundation.json)
→ génération paire ed25519 LOCALE au nœud (la privée ne quitte JAMAIS le nœud)
→ POST /node-enrollment (token digest + machine_identity_digest + role)
→ admission (policy: rôle autorisé, TTL, capacités ⊆ allowed_capabilities)
→ grant signé persisté (CAS) + read-back
→ nœud écrit son état enrollment local (/var/lib/cynic/identity/)
→ transport bootstrap marqué removable (condition: read-back service plane OK)
→ entrée membership L3: Admit
```

### 3.2 Steady state (maintien)
```
tick node (période T): 
  observe: grant TTL restant, transport vivant, kernel joignable
  renew L2 si remaining < seuil (POST renewal, generation+1, CAS)
  publie heartbeat membership (état + lease) sur le(s) plan(s) disponibles
tick kernel:
  plan_federation_membership(desired, observed) → Admit/Renew/Keep/Suspend/Expel
  diffuser la vue (gossip piggyback sur les réponses, fanout fixe)
tick node autonome (kernel injoignable):
  mode Autonomous (loi existante decide_autonomy): observe/juge, pas de build/release
  membership local: self → suspect après stale_s, lapsed après dead_s
```

### 3.3 Partition & retour
```
partition: chaque côté garde L1+L2 intacts ; vues L3 divergent (AP assumé)
retour: full sync (Serf-style) sur premier contact → convergence déterministe
  sans élection ; nœud jamais expulsé pour cause de partition seule
  (Expel exige décision d'autorité L2/kernel, jamais un timeout seul)
```

### 3.4 Rotation & révocation
```
rotation clé subject: nouveau enrollment (L1 refait) — ancienne clé révoquée
  par l'autorité, grant lié à l'ancienne clé invalidé (generation reset)
expulsion: Expel(L3) + revoke grant (L2) sont DEUX actions distinctes,
  chacune avec sa preuve ; Expel seul = temporaire
compromission token enrollment: rotation du token digest dans l'authority policy
  (les grants déjà émis restent valides jusqu'à expiration — durée courte)
```

## 4. Contrats de preuve (schemas)

- `cynic.node_enrollment_response.v1` (existe) — L2.
- NOUVEAU `cynic.federation_membership_observation.v1` : {node_id, status,
  lease{expires_at, generation}, transport_readbacks[], observed_at} — L3.
- NOUVEAU `cynic.transport_readback.v1` : {plane, kind(wg_handshake|tcp|http),
  healthy, last_evidence_at, error} — L4, consommable par L3 mais jamais
  producteur de L2.
- Invariant : toute transition L3 cite l'observation qui la motive ; toute
  admission L2 cite le token/receipt ; toute rotation cite l'ancienne empreinte.

## 5. Décisions tranchées (2026-08-25)

D1 = **(a)** Renommage/clarification sans fusion :
    `enrollment.rs` → `machine_enrollment.rs` (nom = type MachineEnrollment) ;
    extraction `SecretClass/SecretProvider/SecretReference` vers un module pur
    `secret_reference.rs` (leur vrai home : gouvernance de matériel secret,
    consommée par storage_identity + kernel config_storage) ;
    `reconcile_machine_identity` déclaré orphelin décisionnel (aucun
    consommateur runtime mesuré) avec condition explicite de câblage ou de
    suppression (`deletion_rule`). Flèche écrite : machine_identity CONSOMME
    l'autorité L2, jamais l'inverse. Fusion forcée rejetée (§481).

D2 = **Pull-only HTTP d'abord** : chaque nœud interroge la vue membership
    (kernel = point de rendez-vous, dégradation Autonomous déjà modélisée).
    Piggyback delta en étape 2 SI mesure d'insuffisance. SWIM/UDP dédié jamais
    tant que N < 50.

D3 = **Policy membership propre** (défauts initiaux, injectables, à calibrer) :
    sonde 10s ; suspect à 40s (réfutable, style k8s lease/grace) ; lapsed à
    180s ; expulsion JAMAIS par timer seul (décision d'autorité L2/kernel).
    Les champs WG watchdog_stale_s=150/dead_s=200 restent propriété exclusive
    du tunnel L4 — aucun couplage.

D4 = **(B) Boucle node-local** : chaque `cynic-node` calcule sa vue membership
    avec les fonctions pures existantes (`plan_federation_membership`,
    `elect_for_role`) dans son tick existant (`LocalAutonomyRuntime`) ; le
    kernel ne fait que stocker/projecter les observations (rôle « coordination »
    de l'authority map). Zéro nouvelle responsabilité kernel (§489 déjà violé
    par kernel/host selon CRATE_OWNERSHIP.md — on n'aggrave pas).

Justification d'ordre : cette première itération de fédération stable est le
levier qui débloque le builder souverain (builds distribués propres), qui
débloque à son tour la restructuration SoC des crates surchargées. Le travail
de fédération n'attend PAS le grand nettoyage ; il le conditionne.

## 6. Ordre de réalisation (tests d'abord, une couche à la fois)

1. L3 pur : étendre `federation_membership.rs` (états suspect/lapsed, seuils
   injectables, tests table-driven) — aucun I/O.
2. L4 typé : `transport_readback.v1` + adapter WG (parse `wg show`) derrière
   port injecté, tests avec fixture argv stub.
3. Boucle kernel L3 : wire `plan_federation_membership` sur observations
   fraîches, store + REST read-back `--json`.
4. Node tick : heartbeat + renew policy (seuil % TTL), tests sur décisions pures.
5. Suppression conditionnelle du transport bootstrap (dernier, quand 1-4 prouvés).

Chaque étape : types purs → adapter port/injecté → runtime → projection CLI
thin → `cargo test -p <crate>` ciblé. Jamais CLI d'abord.
