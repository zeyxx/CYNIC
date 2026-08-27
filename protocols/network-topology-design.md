# Plans réseau — alignement, métadonnées, SSOT — design Phase 0

Statut : PROPOSITION EN RÉVISION après observation runtime. Grille d'analyse : skill `cynic-layer-model` (L0–L5,
fail-closed, observabilité par surfaces existantes).
Complément de `protocols/tls-fleet-design.md` (le TLS est UN attribut des
plans ; ici on traite la fragmentation des plans eux-mêmes).

Read-back autoritaire : `infra/network/topology-observation.json`.
Attention : les hypothèses antérieures qui proposaient `10.10.10.100` comme
endpoint actuel du kernel sont réfutées : cette adresse n'est affectée sur
aucune interface cortex observée et est injoignable depuis les pairs. Ne pas
les mettre en œuvre. `infra/network/topology.json` est le nouvel emplacement
SSOT proposé, avec un modèle multi-rôles et les faits non résolus marqués
`disputed`; il n'est pas encore une topologie désirée admise.

---

## 1. Cartographie observée (2026-08-26)

### 1.1 Les plans physiques (réalité substrate)

| Plan | CIDR | Porteur | Observé |
|---|---|---|---|
| management | 192.168.100.0/24 | enp42s0 (cortex), NICs bare-metal | vivant |
| synapse | 10.10.10.0/24 | via passerelle pve | vivant (route statique) |
| overlay WG | 10.20.0.0/24 | wg0 :44196 full-mesh | vivant (handshakes frais) |

### 1.2 Les SOURCES DE VÉRITÉ qui décrivent ces plans (le problème central)

| # | Source | Contenu | Consommateurs |
|---|--------|---------|---------------|
| S1 | `infra/registry.json` network_policy | planes + reservations + wireguard | tout le monde (théoriquement) |
| S2 | `infra/verticals/network/intent.json` | DUPLIQUE S1 presque mot pour mot (+ code_contract) | surfaces vertical intent |
| S3 | `infra/synapse-nodes.seed.json` | nodes/ip/role « généré par machine enrollment » | peer_resolver, registry_claim_audit, mesh_resolver |
| S4 | `infra/known-peers.<ts>.json` signé | nodes/ip/role/probe_ports (rolling fallback) | mesh_sync, registry_mesh_state, surrealdb adapter |
| S5 | `/etc/cynic/dns/synapse.hosts` (dnsmasq :1053) | 4 lignes hosts | résolution DNS fleet |
| S6 | `/etc/hosts` manuels par machine | entrées à la main (dont une ligne `jeanterre13`) | résolution locale |

**Six sources pour décrire trois réseaux.** Chacune a dérivé :

### 1.3 Divergences mesurées entre les sources

| Objet | S1 registry | S3 seed | S4 peers | S5 dns | Constat |
|---|---|---|---|---|---|
| local-kernel IP | 10.10.10.100 | **127.0.0.1** | **127.0.0.1** | **ABSENT** | 3 valeurs différentes pour le nœud central |
| local-kernel rôle | kernel_primary | **cortex_primary** | **cortex_primary** | — | deux vocabulaires de rôle |
| cortex (hôte) | management only | absent | absent | absent | hôte physique sans identité service |
| kernel.synapse.cynic | déclaré | — | — | **ne résout PAS** | le nom canonique du service central n'existe pas dans le DNS fleet |
| aliases organ-* | déclarés | — | — | présents partiellement | drift mineur |
| probe_ports | absents de S1 | absents | 8000/1053 etc. | — | métadonnée qui n'existe que dans S4 |

### 1.4 Écarts de configuration runtime observés

- Kernel REST écoute sur **0.0.0.0:3030** (toutes interfaces, y compris
  management ET synapse ET toute future iface) — pas de bind sélectif.
- Bearer auth kernel : dépend de `role_keys.has_any_authentication_authority()`
  — si aucune clé configurée, warning seulement.
- `/etc/hosts` cortex contient une entrée personnelle (`jeanterre13`) —
  pollution hors registre.
- Le builder consomme le kernel via management en clair (cf. doc TLS G2).

---

## 2. Lecture par le layer model (L0–L5 appliqué au réseau)

| Layer | Question réseau | État | Verdict |
|---|---|---|---|
| L0 matière | fichiers orphelins/dérivés ? | known-peers daté, hosts manuels | présent, à canaliser |
| **L1 substrat** | les plans physiques existent et routent ? | oui (mgmt/synapse/WG vivants, dnsmasq tourne) | **OK** |
| **L2 enrollment/lifecycle** | chaque nœud a-t-il une identité PAR PLAN déclarée et convergée ? | non : 6 sources désynchronisées, rôles divergents | **GAP MAJEUR** |
| L3 CLI gateway | surface déterministe d'observation/réconciliation ? | partielle (`cynic transport matrix`, preflight, mesh probe) mais aucune ne réconcilie S1–S6 | gap moyen |
| L4 preuve runtime | read-back que DNS == registry == listeners réels ? | inexistant (aucun check cross-source) | **GAP MAJEUR** |
| L5 coordination/SSOT | QUI fait autorité sur la topologie ? | indéterminé : 6 prétendants | **GAP RACINE** |

Verdict layer model : L1 sain, tout ce qui est au-dessus a dérivé faute de
SSOT unique (L5) et de boucle de réconciliation (L3/L4). C'est exactement le
pattern « fail-open par dispersion » que le modèle interdit.

### 2.1 Ne pas confondre les couches réseau

Le modèle utilise deux axes indépendants :

| Axe | Question | Exemples réellement observés |
|---|---|---|
| contexte d'exécution | dans quel namespace vit l'adresse ? | hôte cortex, CT LXC builder, process kernel dans le namespace cortex, Docker/libvirt local |
| réalisation de transport | comment ce contexte atteint-il le plan ? | interface/routage host, vmbr0 + veth CT, wg0 WireGuard, loopback |

Un plan (`synapse`, `management`, `overlay`) est seulement le domaine
d'adressage et l'intention d'accès. La même adresse loopback est différente
dans chaque namespace. Un service doit donc déclarer le triplet :
`execution_context × network_attachment × endpoint`; jamais seulement un
CIDR ou une IP. `RegistryNetworkLink` possède déjà les primitives de bas
niveau (`provider`, `interface`, `connection`, `address`, `gateway`,
`routes`) et doit être étendu/réutilisé, pas doublé.

---

## 3. Design cible

### 3.1 Principe directeur (L5)

**UNE seule source éditable par concern**, tout le reste est PROJETÉ et SIGNÉ :

```
infra/network/topology.json        ← SSOT éditable (humain/agent via PR)
   ├─(généré)→ infra/network/dnsmasq.synapse.conf   (projeté, signé)
   ├─(généré)→ infra/network/known-peers.signed.json (projeté, signé)
   ├─(généré)→ /etc/cynic/dns/synapse.hosts          (déployé, signé)
   └─(consommé)→ cynic-protocol types purs
```

- `registry.json` garde un POINTEUR (`network_topology_ref`) vers la SSOT,
  il cesse d'être lui-même le contenu (préférence utilisateur : fragments
  typés plutôt que blob).
- `verticals/network/intent.json` ne duplique plus les données : il référence
  la SSOT + porte uniquement le contrat de code (types/modules requis).

### 3.2 Schéma de la SSOT (métadonnées alignées)

```jsonc
{
  "schema": "cynic.network.topology.v1",
  "planes": [
    { "id": "synapse",   "cidr": "10.10.10.0/24", "canonical": true,
      "dns_zone": "synapse.cynic", "dns_provider": "fleet-dnsmasq" },
    { "id": "management", "cidr": "192.168.100.0/24", "canonical": false,
      "purpose": "bare-metal mgmt + hypervisor control" },
    { "id": "overlay",   "cidr": "10.20.0.0/24", "canonical": false,
      "purpose": "wireguard encrypted mesh", "transport": "wireguard",
      "listen_port": 44196 }
  ],
  "nodes": [{
    "node_id": "cynic-builder",
    "role": "build_runner",
    "addresses": {
      "synapse":    { "ip": "10.10.10.119", "dns": ["cynic-builder.synapse.cynic"] },
      "management": null,
      "overlay":    { "ip": "10.20.0.5", "endpoint_plane": "synapse" }
    },
    "services": [ { "name": "cynic-node", "ports_probe": [22] } ],
    "ssh_access": { "method": "direct", "target": "root@10.10.10.119",
                    "scope": "substrate_only" }
  }]
}
```

Métadonnées NOUVELLES alignant les plans (répond au « aligner les
métadonnées ») :
- `plane.canonical` + `purpose` partout (déjà à moitié là) ;
- `addresses` PAR NŒUD PAR PLAN (fini les champs optionnels hétérogènes) ;
- `dns[]` co-localisé avec l'IP qu'il nomme (une seule place) ;
- `probe_ports` remonte de known-peers vers la SSOT (c'était le seul endroit
  qui savait comment sonder) ;
- `ssh_access.scope` explicite (substrate_only = loi layer-model règle 3) ;
- `transport_security` (doc TLS) se greffe sur le même arbre.

### 3.3 Résolution du cas kernel (la divergence la plus grave)

Trois valeurs pour une même vérité. Décision proposée :
- Le SERVICE `cynic-kernel` vit dans la CT/VM cortex, joignable sur synapse
  → `kernel.synapse.cynic = 10.10.10.100` (registre avait raison) ;
- `127.0.0.1` dans seed/peers était la vue LOCALE cortex projetée à tort
  comme topologie fédérale (erreur de classe : vue locale ≠ topologie) ;
- cortex-hôte garde management only + son rôle de porteur (pas d'identité
  service propre) ;
- `kernel_primary` devient le seul nom de rôle (migration `cortex_primary`
  → deprecated, alias de lecture pendant une release).

Et le DNS fleet sert enfin `kernel.synapse.cynic` (gap fermé par projection
de la SSOT).

### 3.4 Boucle de réconciliation (L3/L4)

Nouvelle surface mince (préférence naming flat, une responsabilité) :
```
cynic network audit --json
```
Projections comparées en lecture seule :
1. SSOT (fichier) → attendu
2. dnsmasq synapse.hosts déployé → résolutions réelles (dig)
3. listeners réels des services déclarés (ss/netstat via observer existant)
4. WG peers réels (`wg show` — adapter déjà écrit aujourd'hui)

Rapport typé `cynic.network_topology_audit.v1` : chaque divergence =
entrée {source, expected, observed, severity}. Fail-closed : `unknown`
lorsqu'une source est illisible, jamais de guess. `--apply` séparé
(projection régénérée + signature) reste une commande distincte.

### 3.5 Signature & distribution (anti-drift L5)

- `topology.json` versionné + hashé ; les projections embarquent
  `topology_hash` (même mécanique que Registry-Secret Consistency SHA256).
- `known-peers.signed.json` regénéré depuis la SSOT (fin des timestamps
  manuels), signature ed25519 clé machine existante.
- Audit alerte si `topology_hash(déployé) != topology_hash(SSOT)`.

---

## 4. Audit des gaps numérotés

| # | Gap | Sévérité | Layer | Correction |
|---|-----|----------|-------|------------|
| N1 | 6 sources pour 3 plans, aucune autorité désignée | **racine** | L5 | SSOT unique §3.1 + projections signées |
| N2 | kernel: 3 IPs différentes selon la source | haute | L2/L5 | décision §3.3 + migration rôle |
| N3 | `kernel.synapse.cynic` non résolvable | haute | L1/L4 | projection DNS depuis SSOT |
| N4 | rôles doubles kernel_primary/cortex_primary | moyenne | L2 | renommage + alias déprécié |
| N5 | kernel écoute 0.0.0.0 sans besoin | moyenne | L1/L4 | bind sélectif (synapse + loopback) après TLS |
| N6 | probe_ports connus seulement de known-peers | moyenne | L2/L4 | montée en SSOT + audit check |
| N7 | aucune preuve cross-source (DNS==registry==listeners) | **majeur** | L4 | `cynic network audit` §3.4 |
| N8 | duplication registry ↔ verticals intent | moyenne | L5 | pointer + contrat seul |
| N9 | pollution /etc/hosts manuelle (jeanterre13…) | basse | L0 | nettoyage + règle audit |
| N10 | ssh_access sans scope explicite | basse | L2 | champ scope=substrate_only |
| N11 | known-peers timestampé à la main | moyenne | L5 | génération signée automatique |
| N12 | pas de métadonnée de santé par plan (stale/dead watchdogs orphelins) | moyenne | L4 | rattacher aux plans dans SSOT (L3 membership les consomme déjà) |

---

## 5. Questions ouvertes (avec POURQUOI, à trancher)

D-NET1. **Emplacement SSOT** : `infra/network/topology.json` vs garder
`registry.json.network_policy` ?
Pourquoi le premier : ta préférence durable anti-blob (fragments typés),
la SSOT est éditée plus souvent que le reste du registry, et un fichier
par concern rend les diffs de PR lisibles. Registry garde un pointeur.

D-NET2. **IP canonique du kernel** : 10.10.10.100 (synapse) confirmé ?
Pourquoi : le service doit être joignable PAR LA FLOTTE (loopback ne l'est
pas, management n'est pas le plan canonique), et le DNS synapse existe
pour ça. La valeur 127.0.0.1 était une vue locale mal projetée.

D-NET3. **Nom de rôle** : `kernel_primary` seul, ou fusionner vers un
vocabulaire neutre (`coordination_primary`) ?
Pourquoi : deux noms pour un rôle garantit une troisième divergence ;
`kernel_primary` est déjà dans registry + foundation (coût de migration
minimal vs inventer un troisième terme).

D-NET4. **cortex dans la SSOT** : nœud à part entière (porteur de plans,
sans IP synapse) ou seulement ses workloads (local-kernel) ?
Pourquoi : le layer model veut que le SUBSTRAT soit représenté (sinon
on perd le lien WG peer ↔ hôte), mais lui donner une IP service serait
en inventer une fausse.

D-NET5. **DNS fleet** : dnsmasq par nœud (statu quo projeté) vs un vrai
serveur DNS autoritaire (pve/CoreDNS) ?
Pourquoi le statu quo projeté : il existe déjà, marche, et la SSOT+
signature supprime sa dérive ; un serveur central serait un SPOF de
plus tant que pve n'a pas de HA. Réévaluer à N>10 nœuds.

D-NET6. **Migration** : big-bang (toutes sources remplacées d'un coup)
vs strangle (SSOT ajoutée, projections basculent une par une, anciennes
marquées deprecated puis supprimées) ?
Pourquoi le strangle : le builder tourne, les agents travaillent dans ce
repo ; un big-bang couperait tout le monde pour un gain invisible.

D-NET7. **Bind kernel** : restreindre à synapse+loopback dès maintenant
ou après TLS ?
Pourquoi après : changer bind + chiffrement + auth en même temps rend
chaque échec indiagnosticable ; TLS d'abord (doc dédiée), bind ensuite,
et l'audit N7 vérifie.

---

## 6. Ordre de réalisation (strangle, tests d'abord)

1. Types purs `cynic_protocol::network_topology.v1` (parse + validate +
   projection DNS/peers) + tests table-driven — aucun fichier touché.
2. Générer la SSOT initiale DEPUIS registry.json (source la plus complète),
   revue humaine ligne à ligne (c'est LE moment où on tranche N2/N4).
3. `cynic network audit --json` (read-only) : rapporte les divergences
   S1–S6 contre la SSOT. Il listera immédiatement N3/N9 etc.
4. Projections : dnsmasq conf + known-peers.signed générés et déployés
   (--apply séparé, signés, hash vérifié).
5. Déprécier S2/S3/S4 (headers deprecated + consumers migrés un à un).
6. Bind kernel + retrait béquilles (après chantier TLS).

Chaque étape committable isolément, preuve par test ou read-back, retour
arrière = git revert d'un commit.
