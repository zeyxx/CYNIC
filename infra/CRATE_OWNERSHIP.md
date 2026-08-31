# Crate Ownership & Extraction — CYNIC

STATUS: draft de formalisation. Ce document est la grille d'ownership
applicable à TOUTES les crates. Le kernel est le premier gros cas
traiter, pas la seule cible. Il sert de modele pour extraire host, node,
protocol, cli, inference.

AUTHORITE: ce document ne cree AUCUNE regle nouvelle. Il transpose en
spec ce que `contracts/foundation.json` et `contracts/framework.json`
disent deja:

- foundation.json §481 `one_boundary_one_reason`:
  "Add a crate, module, adapter, or contract only when an existing
  boundary cannot absorb the responsibility cleanly."
- foundation.json §489 (non-goal):
  "centralize all responsibilities in one orchestrator crate" — INTERDIT.
- foundation.json `generalized_ownership` (required_proof):
  "typed owner, custodian, authority, duties, and retention" +
  "ownership transfer and read-back evidence".
- framework.json §472 `cli_is_adapter`:
  "cynic-cli wires inputs and outputs; it must not own orchestration."
- framework.json §476 `proof_producers_are_low_layer`:
  "Host and node crates produce runtime proofs; higher layers consume."

Le kernel (et host) violent ouvertement §489 aujourd'hui. Ce document
dit COMMENT enfermer la violation.

================================================================
1. PRINCIPE D'OR — ONE BOUNDARY ONE REASON
================================================================

Toute crate repond a UNE question: "quelle responsabilite ne peut pas
etre absorbee par une frontiere existante ?"

Si une crate possede 2+ familles de responsabilites non reliees, elle
doit etre scindee. Si une responsabilite est dupliquee dans 2+ crates,
elle doit etre extraite dans un crate/port commun.

Regle de comptage (machine-checkable, voir §6):
- Une crate > 40k LOC est suspecte (host 85k, kernel 58k, node 52k,
  protocol 51k le sont deja).
- Une crate qui implemente des ports appartenant a un AUTRE domaine
  (ex: kernel implemente coord AND ccm AND dogs AND wallet-judgment)
  viole §481.

================================================================
2. TOPOLOGIE ACTUELLE (mesuree, pas suppose)
================================================================

LOC (src/, hors tests), 2026-08-25:

    84813  cynic-host            <- LE plus gros
    58452  cynic-kernel
    51770  cynic-node
    50791  cynic-protocol
    29269  cynic-cli
    24162  cynic-inference
     8225  cynic-observer
     5753  cynic-askesis
     4236  cynic-intel
      399  cynic-zeroclaw-solana
    ------
   318119  TOTAL

Observation: 4 crates depassent 50k LOC. C'est un signal systemique
d'absence d'ownership, pas un accident kernel.

================================================================
3. GRILLE D'OWNERSHIP GENERALE (toute crate)
================================================================

Pour chaque crate, trois colonnes:
- DOIT POSSDER (raison d'etre, unique)
- DOIT DELEGUER (port vers une autre crate)
- INTERDIT (ce qui viole §489/§481)

Forme canonique (a remplir par crate, dans surface-registry.json):

    {
      "crate": "cynic-X",
      "owns": ["responsabilite unique A", "port P"],
      "delegates_to": ["cynic-Y: port Q", "cynic-Z: adapter R"],
      "forbidden": ["orchestration globale", "doublon de S"]
    }

Cette entree DEVIENT la source de verite pour `cynic dev kernel-audit`
et le garde-fou LOC (§6).

================================================================
4. CAS 1 — KERNEL (premier gros cas)
================================================================

Le kernel est un binaire unique (~58k LOC, pas de separation lib/binary
claire: tout sous main.rs via modules). Il possede aujourd'hui ~14
familles de responsabilites NON reliees. Cartographie vers crates
cibles (proposees, a valider):

DOMAINES ACTUELS DU KERNEL (reels, dirs src/):
- api/          REST + WS + MCP (surface projection — ok, mais trop gros)
- storage/      SurrealDB + memory + reconnectable (SSOT + coord)
- domain/ccm/   credit monitoring (produit, pas kernel)
- domain/dog*   dog health/performance (ML evals)
- domain/wallet_judgment  (ML produit)
- organ/        inference organ routing (== cynic-inference)
- backends/     helius/dexscreener/lofi/mail (adapters Solana)
- probe/        hardware/environment/llm (host substrate)
- senses/       hermes_x/rtk (senses, produit)
- introspection/ moteur interne (GARDER)
- infra/tasks/  runtime loops, healing, convergence (orchestration)
- domain/lifecycle*  activation requests/evidence (host concern)
- infra/probes/fleet*  fleet inventory (host concern)
- domain/wisdom  (cynic-askesis)

CARTE D'EXTRACTION PROPOSEE:

    GARDER dans kernel (raison d'etre unique):
      - coordination authority (leases, fencing)  [foundation: coord]
      - bootstrap + federation membrane (boot.rs, runtime_takeover)
      - introspection engine (interne, observe→reconcile)
      - proof-of-history anchor (ledger immuable signe)
      - REST/WS/MCP surface MINIMAL (facade, pas logique)

    EXTRAIRE vers crate cible:
      ccm                -> cynic-protocol (scoring) ou crate dedie
      dogs/health        -> cynic-askesis (ML evals)
      wallet_judgment / market_data -> cynic-intel [RESOLU: rug_risk, wallet, kscore, MarketDataPort dans cynic-intel]
      organ/inference    -> cynic-inference (DEJA existe ! doublon)
      backends/solana    -> cynic-zeroclaw-solana (adapter) ou crate adapters
      probe/hardware     -> cynic-host::domain (host substrate)
      lifecycle_*        -> cynic-host (lifecycle vertical existe deja)
      fleet probes       -> cynic-host (fleet vertical existe deja)
      wisdom             -> cynic-askesis
      storage/surreal    -> NOUVEAU cynic-storage (SSOT dedie) OU rester
                            kernel si on le traite comme "coordination store"

PREUVE DE DOUBLON (violation §481 concrete):
- organ/inference (kernel) vs cynic-inference (crate 24k LOC) = meme
  responsabilite, deux endroits.
- lifecycle_activation (kernel domain/) vs cynic-host lifecycle vertical
  (infra/verticals/lifecycle/intent.json) = meme responsabilite.
- fleet probes (kernel infra/probes/fleet*) vs cynic-host
  fleet_health_observer / fleet_inventory = meme responsabilite.

================================================================
5. CAS 2..N — MEMES PRINCIPES, AUTRES CRATES
================================================================

cynic-host (85k — LE plus gros):
  DOIT: host substrate observation (disk/mem/cpu/net), build runner,
        forgejo control, lifecycle application, fleet health.
  SUSPECT: 85k LOC + possede lifecycle + fleet + build + forgejo + host
        exec + credential provisioning = trop de familles.
  EXTRAIRE: forgejo_control -> crate cynic-git? ; build_runner ->
        deja partiellement cynic-node? ; fleet -> crate cynic-fleet.

cynic-node (52k):
  DOIT: node-local agent tree, watchdog, observation plan, build
        submission orchestration.
  SUSPECT: node possede fleet_inference + watchdog + announce + network
        surface inventory = chevauche host.
  EXTRAIRE: fleet_inference policy -> cynic-protocol (deja co-owner).

cynic-protocol (51k):
  DOIT: types, ports, policy pure, FrameworkSpec.
  SUSPECT: 51k LOC de types + policy + forgejo + coordination_lease +
        runtime_takeover. Est-ce que "protocol" est devenu un fourre-tout?
  EXTRAIRE: coordination_lease -> crate cynic-coord? (ou rester protocol
        si on le considere comme "protocol de coordination").

cynic-cli (29k):
  DOIT: thin facade (parse -> inject ports -> call crate API -> render).
  VERIFIER (§472 cli_is_adapter): aucune logique de politique dans cli.
  Si cli possede des phase policy tables -> extraire vers crate metier.

cynic-inference (24k):
  DOIT: inference runtime, engine selection, profile bootstrap.
  DEJA cible d'extraction kernel organ/ — bien.

================================================================
6. GARDE-FOU MACHINE-CHECKABLE (DEJA CODE, PAS A REINVENTER)
================================================================

VERIFIE (2026-08-25): les trois audits existent deja dans cynic-cli et
sont la "observe" du pattern CYNIC. Ce document FEED la spec, il n'a
pas a recoder ces gardes-fous.

  - `crates/cynic-cli/src/kernel_boundary_audit.rs`
      KERNEL_AREAS (14 zones) avec current_owner / target_owner /
      extraction_priority / status (extract|stabilize) / next_steps.
      Schema: cynic.cli.kernel_audit.v1. Statut = "blocked" tant qu'une
      zone est extract/stabilize ou que des uncovered_hotspots (>400 LOC)
      restent.
      -> La carte du §4 de ce doc est DEJA codee la-bas, mieux que le
         prose. Garder les deux synchronises.

  - `crates/cynic-cli/src/ownership_audit.rs`
      11 surfaces ownership avec crate_owner + migration_target +
      anti_patterns. Schema: cynic.cli.ownership_audit.v1.
      -> Couvre deja la grille generale du §3 (mais nommee "surfaces",
         pas "crates"). Ce doc etend la grille au LOC global par crate.

  - `crates/cynic-cli/src/crate_audit.rs`
      LOC par crate + hotspots (>400 LOC) + entrypoints bornes.
      Schema: cynic.cli.crate_audit.v1. Seuil CORE_CRATES deja defini.
      -> Couvre deja le garde-fou LOC du §2/§6 (host 85k, kernel 58k,
         node 52k, protocol 51k sortent tous du seuil implicite).

CONTRAT DE SORTIE (existant): les trois schemas *.v1 ci-dessus. La
"reconcile" = ce doc + foundation.json §481/§489. La "bounded action"
= extraction incrementale (§7). La "read-back proof" = les json d'audit
verts (status: "ok").

ACTION RESTANTE (si on veut serrer la vis): ajouter au crate_audit un
seuil LOC EXPLICITE par crate (ex: 40k) emettant "blocked" au lieu de
ne lister les hotspots. Aujourd'hui crate_audit liste les hotspots mais
ne bloque pas sur la taille totale du crate. Voir §9 residual.

================================================================
7. PLAN D'EXTRACTION INCREMENTAL (bottom-up)
================================================================

Pas de grand rewrite. Une responsabilite par slice, comme AGENTS.md
l'exige (un commit par slice coherent, par crate cible).

  Etape 1 [RESOLU Slice A]: organ/inference kernel -> cynic-inference.
           Types canoniques dans cynic-inference::domain::backend_health,
           kernel/organ/registry.rs re-export, organ/mod.rs modularise (1049 -> 17 LOC).
  Etape 2: lifecycle_* kernel -> cynic-host (vertical existe).
  Etape 3: fleet probes kernel -> cynic-host fleet.
  Etape 4: dogs/health -> cynic-askesis.
  Etape 5: ccm/wallet_judgment -> cynic-protocol ou askesis/intel.
  Etape 6: backends/solana -> cynic-zeroclaw-solana adapters.
  Etape 7: storage/surreal -> nouveau cynic-storage OU traitement comme
           coordination store du kernel (decision a prender via spec).

A chaque etape: mettre a jour surface-registry.json (owns/delegates)
et faire tourner `cynic dev kernel-audit --json` en read-back proof.

================================================================
8. FORMALISATION SURFACES / CONTRATS / PROTOCOLES
================================================================

Ce document feed trois artifacts machine-readable existants:

  surface-registry.json: ajouter entree "owns/delegates/forbidden"
    par crate (§3 forme). C'est la projection de ce doc.
  contracts/: ajouter contracts/kernel-extraction.json (ou etendre
    framework.json) listant les responsabilites EXTRAITES avec leur
    crate cible + owner + coverage (foundation §107).
  cynic-protocol: les ports extraits (CoordPort, StoragePort, etc.)
    y vivent deja — l'extraction deplace l'IMPL, pas le port.

Une fois surface-registry.json rempli, `cynic dev agent-exposure
--json` et `cynic dev kernel-audit --json` deviennent les deux gardes
(l'un pour les surfaces agent, l'autre pour l'ownership crate).

================================================================
9. RESIDUAL UNKNOWNS (honnetes)
================================================================

- Decider si storage/surreal reste kernel (coordination store) ou
  devient crate dedie — depend de la decision HA du SPOF audit.
- Le seuil 40k LOC est arbitraire; a calibrer apres etape 1-2.
  (Aujourd'hui crate_audit liste les hotspots mais ne bloque pas sur la
  taille totale du crate — voir §6 action restante.)
- Certains domaines (senses/hermes_x, wisdom) n'ont pas de crate cible
  evident — a creer ou a fusionner dans askesis.
- Ce doc EST cite par AGENTS.md (section "Crate ownership & extraction")
  depuis 2026-08-25 — il est donc autorite, pas doc perimee. Maintenir
  la synchro avec KERNEL_AREAS (kernel_boundary_audit.rs) apres chaque
  extraction.

================================================================
REFERENCES (code live)
================================================================

- crates/cynic-kernel/src/main.rs:7         single #[tokio::main]
- crates/cynic-kernel/src/bootstrap/server.rs:94   single listener
- crates/cynic-kernel/src/storage/surreal/coord.rs  coord in storage
- crates/cynic-kernel/src/organ/            doublon cynic-inference
- crates/cynic-host/src/                    lifecycle/fleet/forgejo
- contracts/foundation.json:481,489         regles violees (one_boundary_one_reason / no centralize)
- contracts/framework.json:472,476          cli_is_adapter, proof_low
- crates/cynic-cli/src/kernel_boundary_audit.rs   KERNEL_AREAS (coded spec)
- crates/cynic-cli/src/ownership_audit.rs        11 ownership surfaces
- crates/cynic-cli/src/crate_audit.rs             per-crate LOC gate
- AGENTS.md "Crate ownership & extraction"  cite ce doc comme spec canonique
