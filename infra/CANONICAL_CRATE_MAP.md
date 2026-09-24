# CYNIC — CANONICAL CRATE MAP v2

> **Supersède** : CYNIC_EXTRACTION_GLOBALE.md, CYNIC_CONSTELLATIONS_HEXAGONALES.md
> **Autorité** : ce document est la source de vérité pour la taxonomie des crates.
> **Pattern** : `cynic-<famille>-<rôle>` — sans exception.
> **Validé par** : audit workspace Agave/Solana (117 crates), Bevy (60 crates), Substrate (500+ crates).

---

## 1. PRINCIPE DIRECTEUR

```
cynic-<famille>              → singleton légitime (cross-cutting ou isolé)
cynic-<famille>-<rôle>      → membre d'une constellation

Règles :
  R1. Tout crate avec au moins un sibling dans sa famille DOIT suivre le pattern.
  R2. Le rôle nomme CE QUE LA CRATE FAIT, pas sa couche technique.
  R3. Pas de préfixes de couche (-domain, -ports, -adapter, -application).
      Rôles conventionnels admis (pattern Agave) :
        -types     → types purs seulement (ex: cynic-rpc-types)
        -callback  → traits/callbacks seulement (ex: cynic-svm-callback)
        -interface → définition protocole seulement
  R4. Domaine pur d'abord — types + ports avant adapters.
  R5. DAG strict descendant. Jamais de cycle. kernel ne dépend pas de node.
  R6. no_std + alloc pour toute crate dont les deps le permettent.
      La frontière no_std/std IS la frontière domain/adapter.
      Enforcement : compilateur (pas un test, pas une convention).
```

---

## 2. ÉTAT ACTUEL — LOC RÉELS (2026-09-23)

```
cynic-host          89 754    MONOLITHE
cynic-node          58 087    MONOLITHE
cynic-protocol      54 687    GOD OBJECT (feuille zéro dep)
cynic-cli           37 050    MONOLITHE
cynic-inference     26 846    MONOLITHE
cynic-intel         10 864    constellation partielle ✓
cynic-observer       8 558    ORPHELIN (famille inconnue)
cynic-build          7 836    OK
cynic-workload       6 193    cross-cutting
cynic-kernel         6 290    OK (inversions à corriger)
cynic-lifecycle      3 091    mauvais nom (→ cynic-host-lifecycle)
cynic-domain-ops     2 892    constellation ✓
cynic-pkg            2 920    mauvais nom (→ cynic-host-pkg)
cynic-storage        2 481    cross-cutting
cynic-network        2 201    cross-cutting
cynic-vcs            1 654    cross-cutting
cynic-lxc            1 175    mauvais nom (→ cynic-host-lxc)
cynic-host-probes    1 129    ✓ pattern OK
cynic-market         1 059    ORPHELIN (→ fusion cynic-intel-futarchy)
cynic-sandbox        1 013    mauvais nom (→ cynic-host-sandbox)
cynic-intel-escore     945    constellation ✓
cynic-forensics        802    mauvais nom (→ cynic-host-forensics)
cynic-agent-bus        789    constellation ✓
cynic-secrets          783    cross-cutting
cynic-pool             744    ORPHELIN (→ cynic-agent-pool)
cynic-mcp              776    singleton projection ✓
cynic-intel-soc        597    constellation ✓
cynic-release          595    mauvais nom (→ cynic-host-release)
cynic-domain-cloudflare 555   constellation ✓
cynic-agent-bus-coordinator 449  constellation ✓
cynic-domain-porkbun   432    constellation ✓
cynic-zeroclaw-solana  401    mauvais nom (→ cynic-solana-zeroclaw)
cynic-discovery        352    mauvais nom (→ cynic-node-discovery)
cynic-askesis          303    mauvais nom (→ cynic-kernel-askesis)
cynic-channel          283    ORPHELIN (→ cynic-agent-channel)
cynic-consensus        251    mauvais nom (→ cynic-node-consensus)
cynic-discovery-host   220    mauvais nom (→ cynic-host-discovery)
cynic-intel-futarchy   213    constellation ✓
cynic-identity         159    singleton ✓ (feuille pure)
cynic-agent-bus-server  104   constellation ✓
cynic-domain-askesis    67    constellation ✓
```

---

## 3. TAXONOMIE CIBLE — PAR FAMILLE

### FAMILLE : agent
*Coordination multi-agent, économie, transport.*
```
cynic-agent-bus              ✓ existant — types, ports, ledger, claim
cynic-agent-bus-server       ✓ existant — projection REST
cynic-agent-bus-coordinator  ✓ existant — orchestrateur fleet
cynic-agent-channel          ← renommer cynic-channel (économie bilatérale HTLC)
cynic-agent-pool             ← renommer cynic-pool (PoUW, PPLNS, récompenses)
cynic-agent-coord            → CRÉER — coordination_lease, registry_claim_audit
                                        (extraire de cynic-protocol)
cynic-agent-session          → CRÉER — agent_session/ (extraire de cynic-protocol)
```

### FAMILLE : eval
*Scoring, jugement, verdict, crystals.*
```
cynic-eval-dog               → CRÉER — eval/dog, dog_health, wisdom (de protocol)
cynic-eval-judgment          → CRÉER — judgment, kyon/ (de protocol)
cynic-eval-evidence          → CRÉER — evidence, evidence_journal, evidence_projection,
                                        observation_record (de protocol)
```

### FAMILLE : kernel
*Orchestrateur souverain, bus de coordination.*
```
cynic-kernel                 ✓ existant — résidu ~5K après corrections
cynic-kernel-askesis         ← renommer cynic-askesis
```

### FAMILLE : host
*Substrat OS, hyperviseur, adapters système.*
```
cynic-host                   ✓ existant — résidu orchestrateur host
cynic-host-probes            ✓ existant — hardware probes
cynic-host-lxc               ← renommer cynic-lxc
cynic-host-pkg               ← renommer cynic-pkg
cynic-host-sandbox           ← renommer cynic-sandbox
cynic-host-lifecycle         ← renommer cynic-lifecycle
cynic-host-release           ← renommer cynic-release
cynic-host-forensics         ← renommer cynic-forensics
cynic-host-discovery         ← renommer cynic-discovery-host
cynic-host-substrate         → CRÉER — host_substrate/ cpu/disk/memory/hypervisor
                                        (extraire de cynic-protocol)
```

### FAMILLE : node
*Nœud unique, supervision, autonomie.*
```
cynic-node                   ✓ existant — résidu supervisor
cynic-node-consensus         ← renommer cynic-consensus
cynic-node-discovery         ← renommer cynic-discovery
cynic-node-protocol          → CRÉER — node_activation, node_enrollment, node_control,
                                        node_identity, machine_enrollment, pre_os
                                        (extraire de cynic-protocol)
cynic-node-autonomy          → CRÉER — emergence, local_autonomy
                                        (extraire de cynic-protocol)
```

### FAMILLE : runtime
*Placement, graine, takeover.*
```
cynic-runtime-placement      → CRÉER — runtime_placement, runtime/ (de protocol)
cynic-runtime-seed           → CRÉER — runtime_seed (de protocol)
cynic-runtime-takeover       → CRÉER — runtime_takeover, runtime_takeover_activation
                                        (de protocol)
```

### FAMILLE : fleet
*Multi-nœuds, heartbeat.*
```
cynic-fleet-heartbeat        → CRÉER — fleet_heartbeat, fleet_discovery (de protocol)
```

### FAMILLE : federation
*Multi-organisation, bootstrap fédéré.*
```
cynic-federation-membership  → CRÉER — federation_membership (de protocol)
cynic-federation-authz       → CRÉER — federation_authz, federation_envelopes (de protocol)
cynic-federation-bootstrap   → CRÉER — bootstrap_federation (de protocol)
```

### FAMILLE : inference
*LLM souverain, routing, verdict cache.*
```
cynic-inference              ✓ existant — résidu router
cynic-inference-llm          → CRÉER — llm_port, fleet_inference/ (de protocol)
```

### FAMILLE : vcs
*Source control, Git, Forgejo.*
```
cynic-vcs                    ✓ existant — résidu types
cynic-vcs-git                → CRÉER — vcs/git_topology, git_hygiene, source_control,
                                        source_worktree* (de protocol)
cynic-vcs-forgejo            → CRÉER — vcs/forgejo, forgejo_token (de protocol)
```

### FAMILLE : storage
*Persistance, ports.*
```
cynic-storage                ✓ existant — résidu ports + absorbe storage/ de protocol
```

### FAMILLE : network
*Mesh, transport.*
```
cynic-network                ✓ existant — absorbe network.rs, transport_* de protocol
```

### FAMILLE : build
*CI/CD, harness de test.*
```
cynic-build                  ✓ existant
cynic-build-harness          → CRÉER — harness.rs, harness_authority (de protocol)
```

### FAMILLE : intel
*Intelligence, scoring, marchés.*
```
cynic-intel                  ✓ existant — absorbe cynic-market
cynic-intel-escore           ✓ existant
cynic-intel-futarchy         ✓ existant — absorbe cynic-market
cynic-intel-soc              ✓ existant
```

### FAMILLE : solana
*Intégration Solana, observer, plugins.*
```
cynic-solana-observer        ← renommer cynic-observer (solana_archive = 80%)
cynic-solana-zeroclaw        ← renommer cynic-zeroclaw-solana
cynic-solana-anchor          → DEFERRED — anchor_client.rs purged 2026-09-24
                                        (251 LOC, 10/10 symbols zero external
                                        consumers). Recreate only when an
                                        on-chain branch is actually wired.
cynic-solana-pay             ✓ CRÉÉ 2026-09-24 (ex-solana/ de protocol:
                                        constants + ops — payment/SOP
                                        vocabulary, not anchor). AllowanceGrant
                                        stays in protocol residue (shared
                                        vocabulary via agentic_tool_call);
                                        payment behavior lives here as
                                        AllowanceGrantPaymentExt.
```

### FAMILLE : domain
*Opérations DNS, providers.*
```
cynic-domain-ops             ✓ existant
cynic-domain-cloudflare      ✓ existant
cynic-domain-porkbun         ✓ existant
cynic-domain-askesis         ✓ existant
```

### SINGLETONS CROSS-CUTTING
*Légitimes sans famille. Pas de split prévu.*
```
cynic-protocol               → résidu types atomiques ~3K (après toutes extractions)
cynic-identity               ✓ feuille pure (ed25519, blake3, NodeId)
cynic-secrets                ✓ cross-cutting (6 consommateurs, pas de famille claire)
cynic-workload               ✓ cross-cutting (6 consommateurs)
cynic-mcp                    ✓ projection agent, singleton
cynic-cli                    → résidu thin ~3K (après extractions)
cynic-pool                   → cynic-agent-pool (voir famille agent)
```

---

## 4. VERDICTS PAR ACTION

### RENOMMER (Cargo.toml + tous les consommateurs)
```
cynic-askesis          → cynic-kernel-askesis
cynic-consensus        → cynic-node-consensus
cynic-discovery        → cynic-node-discovery
cynic-discovery-host   → cynic-host-discovery
cynic-forensics        → cynic-host-forensics
cynic-lifecycle        → cynic-host-lifecycle
cynic-lxc              → cynic-host-lxc
cynic-pkg              → cynic-host-pkg
cynic-release          → cynic-host-release
cynic-sandbox          → cynic-host-sandbox
cynic-channel          → cynic-agent-channel
cynic-pool             → cynic-agent-pool
cynic-observer         → cynic-solana-observer
cynic-zeroclaw-solana  → cynic-solana-zeroclaw
```

### FUSIONNER (absorber puis supprimer la source)
```
cynic-market           → dans cynic-intel-futarchy (marché futarchique = extension)
```

### CRÉER — extraire de cynic-protocol (voir §5 ordre d'extraction)
```
cynic-eval-evidence
cynic-eval-dog
cynic-eval-judgment
cynic-agent-coord
cynic-agent-session
cynic-node-protocol
cynic-node-autonomy
cynic-host-substrate
cynic-runtime-placement
cynic-runtime-seed
cynic-runtime-takeover
cynic-fleet-heartbeat
cynic-federation-membership
cynic-federation-authz
cynic-federation-bootstrap
cynic-inference-llm
cynic-vcs-git
cynic-vcs-forgejo
cynic-build-harness
cynic-solana-anchor
```

### CRÉER — extraire de cynic-host
```
(dépend des renommages ci-dessus — les crates cynic-host-* existent déjà
 comme destinations ; le code qui les concerne est encore dans cynic-host/src/)
```

---

## 5. ORDRE D'EXTRACTION — BOTTOM-UP PAR TIER

```
RÈGLE : ne jamais extraire B avant que toutes ses dépendances soient propres.

TIER 0 — Feuilles absolues (zero dep workspace)
  cynic-protocol, cynic-identity, cynic-kernel-askesis (post-rename)

SESSION 1 : Éclater cynic-protocol
─────────────────────────────────────────────────────────
  VAGUE A — modules sans import intra-protocol (feuilles internes)
    vcs/         → cynic-vcs (absorber) + créer cynic-vcs-git + cynic-vcs-forgejo
    storage/     → cynic-storage (absorber)
    forensics.rs → cynic-host-forensics (absorber)
    peer/        → cynic-node-consensus (absorber)
    network.rs + transport_* → cynic-network (absorber)
    harness.rs   → cynic-build (absorbé 2026-09-24 avec harness_authority.rs;
                   schemas cynic.build.harness_*)
    solana/ + anchor_client.rs → cynic-solana-pay (créé 2026-09-24);
                   anchor_client purged (dead code, DEFERRED)
    ✅ VAGUE A TERMINÉE 2026-09-24 (commit 06c19797)
    Leçons: (1) SecurityEvidenceState reste au résidu — vocabulaire de preuve
    partagé 40 fichiers/5 crates; (2) AllowanceGrant = vocabulaire cross-family
    (agentic_tool_call) → résidu, behavior en extension trait dans la crate
    domaine (orphan rule); (3) coordination_{fault,scaling}_harness purgés
    (0 consommateur externe).

  VAGUE B — importent uniquement des feuilles-A
    evidence.rs (→ digest_json)           → cynic-eval-evidence (créer)
    convergence/ (→ evidence)             → cynic-node-consensus (absorber)
    node_identity/enrollment/control/activation → cynic-node-protocol (créer)
    runtime_placement/seed/takeover        → cynic-runtime-* (créer)
    local_autonomy + emergence             → cynic-node-autonomy (créer)
    host_substrate/ (cpu/disk/memory)      → cynic-host-substrate (créer)

  VAGUE C — importent B
    eval/ + judgment + kyon               → cynic-eval-dog + cynic-eval-judgment
    agent_session/ + agent_coordination   → cynic-agent-coord + cynic-agent-session
    fleet_heartbeat/discovery             → cynic-fleet-heartbeat
    llm_port + fleet_inference/           → cynic-inference-llm

  VAGUE D — importent C (plus couplés)
    federation_* + bootstrap_federation   → cynic-federation-* (créer)
    organic_opportunity + framework       → cynic-cli (absorber, projections)

  RÉSIDU cynic-protocol : ~3K LOC
    error_catalog, digest_json, sanitize, integrity, residual_unknown,
    phi, secret_reference, freshness, sbom, temporal, convergence types

SESSION 2 : Renommages (Cargo.toml workspace + consommateurs)
─────────────────────────────────────────────────────────────
  14 renommages listés au §4. Mécaniques, sans déplacement de code.
  Valider : cargo check --workspace après chaque rename.

SESSION 3 : Éclater cynic-host (89K → ~10 crates)
─────────────────────────────────────────────────────────────
  Après protocol propre. Les crates-destinations existent déjà (post-rename).
  Déplacer les modules host/src/ vers leurs crates cynic-host-* respectives.

SESSION 4 : Éclater cynic-node (58K → ~4 crates)
─────────────────────────────────────────────────────────────
  Après host propre.
  cynic-node-protocol, cynic-node-autonomy, cynic-node-consensus absorbent.

SESSION 5 : Corriger les inversions kernel
─────────────────────────────────────────────────────────────
  kernel → node  : SUPPRIMER (inversion architecturale)
  kernel → host  : REMPLACER par traits HostPort dans cynic-protocol résidu
  Résidu cynic-kernel : ~5K LOC, orchestrateur pur.

SESSION 6 : Éclater cynic-inference + cynic-cli
─────────────────────────────────────────────────────────────
  En dernier — ces crates dépendent de tout le reste.
```

---

## 6. DAG CIBLE (simplifié)

```
Feuilles (zero dep) :
  cynic-protocol  cynic-identity  cynic-kernel-askesis

Tier 1 (→ protocol only) :
  cynic-workload  cynic-secrets  cynic-network  cynic-storage
  cynic-vcs  cynic-host-forensics  cynic-host-lifecycle  cynic-host-release
  cynic-host-lxc  cynic-host-pkg  cynic-host-sandbox  cynic-agent-bus
  cynic-node-consensus  cynic-eval-evidence  cynic-node-protocol
  cynic-host-substrate  cynic-runtime-*  cynic-node-autonomy
  cynic-fleet-heartbeat  cynic-agent-channel  cynic-agent-pool

Tier 2 (→ T1) :
  cynic-build  cynic-inference  cynic-intel-soc
  cynic-eval-dog  cynic-eval-judgment  cynic-agent-coord
  cynic-agent-session  cynic-vcs-git  cynic-vcs-forgejo

Tier 3 (→ T2) :
  cynic-intel  cynic-host-lifecycle  cynic-domain-ops

Tier 3b (→ intel) :
  cynic-intel-escore  cynic-intel-futarchy  cynic-intel-soc
  cynic-solana-observer  cynic-solana-zeroclaw  cynic-solana-anchor

Tier 4 (→ T1-T3) :
  cynic-host (résidu)  cynic-federation-*

Tier 5 (→ T4) :
  cynic-node (résidu)
  cynic-kernel (résidu, sans dep sur node post-correction)

Tier 6 (→ tout) :
  cynic-cli (résidu)  cynic-mcp
```

---

## 7b. no_std — STRATIFICATION PAR DESTINATION

```
TIER no_std + alloc  (on-chain Solana, sandboxes, WASM)
  Deps compatibles : ed25519-dalek ✓  blake3 ✓  serde ✓  sha2 ✓  hex ✓  thiserror v2 ✓

  AUJOURD'HUI (zéro changement de logique) :
    cynic-identity        → #![no_std] prêt immédiatement
    cynic-agent-channel   → #![no_std] prêt (sha2 + serde)
    cynic-agent-pool      → #![no_std] prêt (serde + sha2)

  À LA CRÉATION (extraction Session 1) :
    cynic-protocol résidu
    cynic-eval-evidence
    cynic-eval-dog
    cynic-eval-judgment
    cynic-agent-types     (types purs extraits de cynic-agent-bus)

  À VÉRIFIER (peuvent avoir des deps std cachées) :
    cynic-workload        (serde_json nécessite alloc → ok)
    cynic-node-protocol   (async_trait → bloque no_std, à isoler)
    cynic-runtime-*       (à vérifier module par module)

TIER std + feature-gated  (off-chain pur, accès contrôlé)
  Pattern Agave : feature "cynic-internal" pour items privés aux tests
  #[cfg_attr(feature = "cynic-internal", pub)]
  Crates : cynic-agent-coord, cynic-agent-session, cynic-node-protocol

TIER std complet  (adapters, applications, I/O)
  cynic-host-*  cynic-vcs-git  cynic-vcs-forgejo  cynic-kernel
  cynic-inference  cynic-storage  cynic-network  cynic-mcp  cynic-cli
  cynic-node (résidu)
```

```toml
# Cargo.toml workspace — ajouter après extraction Session 1
[[bans.deny]]
name = "cynic-protocol"
# Interdit d'importer le god object résiduel directement
# depuis les crates std. Forcer l'import via les crates extraites.
wrappers = ["cynic-protocol"]  # seul le résidu autorisé en interne
```

## 7. RÈGLES D'ENFORCEMENT

```
E1. Pattern naming
    Tout nouveau crate DOIT suivre cynic-<famille>-<rôle>.
    Violation → PR bloquée.

E2. DAG acyclique
    Toute arête montante (kernel→node, host→kernel) → cargo deny.
    Chaque crate a tests/architecture_boundary.rs.

E3. Ratchet LOC
    Tout crate > 40K LOC → WARN dans cynic dev crate-audit.
    Pas de nouveau code dans les monolithes en cours d'extraction.

E4. Orphelin
    Tout crate sans consommateur workspace → WARN.
    Exception : binaires (main.rs), crates de projection finale (cli, mcp).

E5. Module mal placé
    Tout module dont le langage ubiquitaire appartient à une autre famille
    → issue ouverte, extraction planifiée dans ce doc.

E6. Zéro re-export global
    lib.rs ne fait pas pub use autre_crate::*.
    Les imports sont explicites.
```

---

## 8. MODULES AMBIGUS — DÉCISIONS PRISES

| Module (dans protocol) | Appartient à | Raison |
|------------------------|--------------|--------|
| `harness.rs` | cynic-build | schémas `cynic.build.*`, scénarios de test CI |
| `organic_opportunity` | cynic-cli | read-only, "no I/O", projection opérateur |
| `framework.rs` | cynic-cli | auto-description machine, sortante |
| `emergence.rs` | cynic-node-autonomy | coordination sans kernel, domaine NODE |
| `local_autonomy.rs` | cynic-node-autonomy | couplé à emergence (re-export mutuel) |
| `emergence_gallery.rs` | cynic-eval-evidence | importe EvidenceEnvelope + CynicErrorCode |
| `sensory.rs` | cynic-host-probes | SensoryPort trait, captures snapshot |

---

*v2 — 2026-09-23. Ajout : no_std stratification, feature-gating pattern Agave,*
*cargo-deny sur monolithe résiduel. Validé par benchmark Agave/Bevy/Substrate.*
