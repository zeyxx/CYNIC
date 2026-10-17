# TLS fleet-wide — design Phase 0

Statut : PROPOSITION (aucun changement tant que non validé).
Objectif : corriger à la racine le transport chiffré/authentifié entre tous
les composants CYNIC, retirer les béquilles (socat loopback, HTTP management
en clair), et donner au node-control la voie TLS qu'exige déjà sa loi.
Autorités supérieures : runtime observé > foundation.json > registry.
Ce doc devient des types purs + adapters une fois validé.

---

## 1. Cartographie observée (2026-08-26) — qui parle à qui, comment

### 1.1 Plans réseau déclarés (infra/registry.json)

| Plan | CIDR | Canonique | Usage réel |
|---|---|---|---|
| synapse | 10.10.10.0/24 | oui | services inter-nœuds |
| overlay WG | 10.20.0.0/24 | **non** | chiffré full-mesh, watchdogs L4 |
| management | 192.168.100.0/24 | non | proxmox, kernel_url builder actuel |

### 1.2 Services & transports observés

| Service | Endpoint actuel | Scheme déclaré | TLS réel | Consommateurs |
|---|---|---|---|---|
| cynic-kernel REST | cortex :3030 TCP + UDS `/run/cynic/kernel.sock` | http | **NON** | cynic-node (tous), CLI, watchdog |
| cynic-node control | n/a (node = client) | — | — | — |
| forgejo web/API | 10.10.10.101 :3000 http / :443 https déclaré | https (api-tls service) | **à vérifier** (curl https → 000 depuis cortex) | git SSOT, webhook |
| artifact store | storage :8443 https déclaré | https | **NON joignable** (000) | build/release pipeline |
| SurrealDB | storage :8000 http | http | **NON** | kernel (storage), coord |
| headscale | 10.10.10.40:8080 http | http | **NON** (self-hosted, souverain) | mesh coordination |
| SSH | port 22 partout | ssh | oui (par définition) | enrollment seed, admin |
| WireGuard | :44196 full mesh | — | chiffré L4 | overlay |

### 1.3 Béquilles actuelles à retirer après TLS

- `cynic-kernel-proxy.service` (builder) : socat `127.0.0.1:3030 → 192.168.100.2:3030`
  — contourne la loi loopback/TLS du node-control ;
- `kernel_url` builder pointe sur l'IP management en clair.

### 1.4 Loi existante qui exige TLS (déjà codée)

- `http_node_control.rs:86` : « node control requires TLS or a loopback
  transport » — refuse tout HTTP distant en clair. C'est CETTE loi qui bloque
  le service node sur le builder. Elle est juste ; c'est le monde qui manque.
- UDS kernel déjà propre : socket groupé `cynic`, mode 0660 — modèle local
  correct à conserver tel quel.
- Classes de secrets typées (`SecretClass`, `SecretProvider`) prêtes à porter
  les références de matériel TLS (`RuntimeMachineIdentity`, provider
  `PrincipalOwnedFile`).

### 1.5 Dépendances disponibles

- reqwest 0.13 + native-tls dans les trois crates clientes (déjà compilé).
- rustls-pki-types transitif présent côté kernel.
- axum (server) présent ; il manque uniquement le terminateur TLS
  (`axum-server` + `tokio-rustls` ou `axum::serve` + rustls acceptor).

---

## 2. Design cible — PKI interne minimale, sans vendor lock

### 2.1 Autorité de certification

- **UNE CA racine CYNIC**, clé privée hors-ligne (générée sur machine air-gap
  ou au moins jamais sur un nœud runtime), classe secret `HumanAdmin`
  (bootstrap only).
- **CA intermédiaire par rôle** (kernel-primary / nodes / services), signée
  par la racine, hébergée :
  - clé intermédiaire sur l'hôte pve (management) OU via l'artifact store
    signé — décision D-TLS2 ci-dessous ;
  - durée 5 ans racine, 1 an intermédiaire, rotation documentée.
- SAN obligatoires : `cynic-kernel.synapse.cynic`,
  `cynic-builder.synapse.cynic`, etc. (DNS synapse = source des noms).
- Algorithme : ed25519 pour les CA (cohérent avec l'identité ed25519 déjà
  utilisée par MachineEnrollment), P-256 accepté si compat exigée.

### 2.2 Certificats de nœud (leaf)

- Émis par l'intermédiaire, TTL court (30 jours) avec auto-renouvellement
  node-local (même mécanique que le renewal L2 d'enrollment : seuil % vie,
  boucle tick existante).
- Clés leaf générées LOCALEMENT sur chaque nœud (la privée ne voyage jamais,
  même loi que L1 enrollment).
- CSR vers le kernel (= autorité d'enrollment TLS), même vocabulaire que
  node_enrollment : `cynic.tls_csr.v1` → admission par policy → cert signé
  → read-back. Le endpoint csr-enroll_handler déjà présent côté kernel
  devient LA voie canonique (D5 inversé : il se réactive).

### 2.3 Qui fait confiance à qui (matrice mTLS)

| Client → Serveur | Vérif serveur | Vérif client |
|---|---|---|
| cynic-node → kernel REST | CA racine + SAN | **cert node** (mTLS) |
| CLI opérateur → kernel | CA + SAN | token humain (pas de cert, garde HumanAdmin simple) |
| kernel → SurrealDB | CA + SAN | cert kernel |
| kernel/node → artifact store | CA + SAN | bearer token (déjà en place) |
| node → forgejo API | CA + SAN (ou public si relay) | token session |
| headscale adapter → headscale | CA + SAN (via son propre TLS) | clé join existante |

Règle : mTLS partout où les DEUX bouts sont des composants CYNIC sous
principal ; auth applicative conservée EN PLUS (défense en profondeur),
jamais remplacée par le seul certificat.

### 2.4 Config surface (registry data, pas branches de code)

```jsonc
// infra/registry.json — par service
"transport_security": {
  "schema": "cynic.transport_security.v1",
  "mode": "mtls",                    // mtls | tls | insecure_loopback_only
  "ca_ref": "secret://runtime-machine-identity/ca-bundle",
  "cert_ref": "secret://runtime-machine-identity/tls-cert",
  "key_ref": "secret://runtime-machine-identity/tls-key",   // PrincipalOwnedFile
  "san_dns": ["cynic-builder.synapse.cynic"],
  "renewal_before_expiry_s": 604800
}
```

Le mode `insecure_loopback_only` reste une valeur LÉGALE (tests locaux,
lab) mais le registre de production ne doit plus le contenir — audité par
un check dédié.

### 2.5 Chemin Rust

- Kernel : `axum-server` avec `RustlsConfig` (nouvelle dep unique) sur le
  listener TCP existant ; UDS reste tel quel (local trust).
- Clients : basculer reqwest sur feature `rustls-tls` (supprimer native-tls
  progressivement) + charger l'identity depuis les refs secrets.
- Types purs : `cynic_protocol::transport_security` existe DÉJÀ (module
  détecté) — l'étendre plutôt que créer un parallèle (§481).

---

## 3. Audit des gaps — classés, avec owner de correction

| # | Gap | Sévérité | Couche owner | Correction structurelle |
|---|-----|----------|--------------|------------------------|
| G1 | Kernel REST sans TLS sur synapse/management | **haute** | kernel server.rs + PKI | axum-server rustls + certs §2.2 |
| G2 | Node-control bloqué par sa propre loi (service down sur builder) | haute (symptôme de G1) | config registry | pointer sur https + mTLS après G1 ; PAS de contournement |
| G3 | Forgejo HTTPS déclaré mais non joignable (000) | haute | infra forgejo (hors crates) | activer/réparer le vhost TLS forgejo-api-tls (port 443 déjà déclaré) |
| G4 | Artifact store 8443 injoignable | moyenne | infra storage | vérifier unit + firewall ; cert requis §2.1 |
| G5 | SurrealDB en http clair | moyenne | storage_primary | TLS surreal (start --tls) + cert, puis bascule scheme registry |
| G6 | headscale en http | moyenne | mesh adapter | activer TLS headscale ou passer par overlay WG (déjà chiffré) — décision explicite à documenter |
| G7 | Béquille socat kernel-proxy | basse (après G1/G2) | infra builder | suppression après bascule node-control |
| G8 | Pas de CA, pas de matériel certificat nulle part | **bloquant amont** | nouveau surface PKI | §2.1 — génération hors-ligne + distribution |
| G9 | native-tls (openssl) dans les 3 crates | basse | Cargo features | migration rustls-tls (supprime la dep système openssl) |
| G10 | Secrets TLS sans refs typées | moyenne | registry + SecretReference | ajouter les refs §2.4, audit Registry-Secret Consistency étendu aux certs |
| G11 | csr-enroll_handler kernel orphelin/non testé e2e | moyenne | kernel REST | devenir la voie officielle §2.2 (D5 inversé) |
| G12 | Renewal cert absent (TTL 30j exige auto-renewal) | moyenne | node tick | étendre run_membership_loop/renewal loop avec renewal TLS avant expiry |

Ordre critique dépendant : G8 → G3/G4/G5 → G1 → G2 → G7 → G6/G9/G10 → G12.

---

## 4. Décisions ouvertes (D-TLS1..n) à trancher avant code

D-TLS1. Racine CA : génération hors-ligne manuelle (HumanAdmin, script
   one-shot audité) vs service interne signant via OneShotCapability ?
   Recommandation : hors-ligne manuelle pour la RACINE (simple, rare),
   intermédiaire+leaf automatisés par le kernel (MachineEnrollment class).
D-TLS2. Où vit l'intermédiaire : pve host (management, déjà root-owned)
   vs kernel primary ? Recommandation : kernel primary en steady state
   (zéro HumanAdmin), import initial fait par geste humain borné.
D-TLS3. ed25519 pure vs P-256 pour compat ? Recommandation : ed25519
   partout (rustls le supporte ; cohérence avec l'identité existante).
D-TLS4. Basculer reqwest native-tls → rustls-tls dans ce chantier ou
   après ? Recommandation : après (G9 séparé, ne pas coupler).
D-TLS5. Forgejo : cert interne CA CYNIC ou Let's Encrypt (il a un domaine
   public relay) ? Recommandation : LE pour forgejo (public), CA CYNIC
   pour tout l'internel.
D-TLS6. Le proxy socat : détruit ou gardé en fallback explicitement taggé?
   Recommandation : supprimé (une béquille gardée est une béquille utilisée).

---

## 5. Ordre de réalisation (proposition, tests d'abord)

1. **PKI bootstrap** : script one-shot audité (racine hors-ligne) +
   types purs `transport_security` étendus (refs, SAN, policy renewal) +
   tests table-driven. Aucun hôte touché.
2. **Kernel serveur TLS** : axum-server/rustls derrière feature flag,
   dual-stack temporaire (http loopback + https synapse), read-back
   `/health` en https. Tests avec certs fixture générées in-test.
3. **CSR enrollment TLS** : activer/tester csr-enroll_handler e2e
   (fixture), policy admission côté kernel, renewal loop côté node.
4. **Basculer les clients un par un** : node-control d'abord (débloque le
   service builder), puis watchdog/CLI, puis storage/artifact consumers.
5. **Infra services tiers** : forgejo LE, surreal TLS, artifact store,
   headscale (décisions D-TLS5/6).
6. **Retrait béquilles** : socat supprimé, modes insecure absents du
   registry prod (audit check), docs sync.

Chaque étape : preuve read-back (`curl --cacert`, test unitaire, journal),
commit atomique par chemin explicite. Jamais de bascule sans plan de retour
(l'ancien port http reste lié en loopback pendant la transition).

---

## 6. Ce que ce design N'EST PAS

- Un remplacement de WireGuard : l'overlay WG reste le transport L4
  chiffré inter-nœuds ; TLS ici authentifie les SERVICES au-dessus.
- Une refonte du node_enrollment : le TLS enrollment RÉUTILISE le même
  pattern (token/admission/grant/read-back) avec un autre type de credential.
- Un chantier de ce soir : G8 (CA) est bloquant amont et demande ton geste
  HumanAdmin pour la racine.
