# Fleet réelle — matrice des besoins réseau & admin système

> **Status**: `admis` — base de travail bottom-up (observé d'abord).
> **Méthode** : chaque besoin part d'un fait observé cette session (sonde nommée)
> ou est marqué INCONNU avec la sonde qui le trancherait. Aucun besoin déduit
> d'un doc, aucun désir promu en fait.
> **Périmètre** : fleet réelle (~8 nœuds : cortex, PVE, 6 CT, builder).
> Le plan 100-nœuds (MYCELIUM RFC) vient APRÈS la fermeture de cette matrice.

## Légende

| État | Sens |
|---|---|
| EN PLACE | observé vivant, prouvé par sonde |
| MANQUANT | observé absent, sonde nommée |
| DIVERGENT | plusieurs vérités concurrentes, observées |
| INCONNU | non probé ; la sonde manquante est nommée, pas le résultat |

> **Règle sondes (2026-10-15)** : une vraie sonde vérifie en temps réel
> (daemon/service vivant). Timers et crons = polling fragile, pas des
> sondes. `watchdog` actuel = one-shot CLI manuel, aucune fraîcheur
> continue : la liveness réseau continue est donc MANQUANTE partout
> où seul le watchdog la couvrirait.

## Matrice

| ID | Besoin | État | Fait observé / sonde |
|---|---|---|---|
| NET-01 | Zone canonique `service.cynic` servie fleet-wide | EN PLACE | PVE sert les 2 zones (sonde dig 2026-10-14/15) et toute la fleet consomme par défaut (NET-02) |
| NET-02 | Résolution homogène (même résolveur fleet) | EN PLACE | prouvé par défaut le 2026-10-15 : cortex via :1053 local, builder via 10.10.10.1 (drop-in), 6/6 CT via DHCP 10.10.10.1 (`getent` depuis PVE). Zéro hosts file restant |
| NET-03 | Artefacts DNS générés (header, régénérables) | EN PLACE | bundle natif dual-zone (`network dns bundle`, 2026-10-14) : régénérable, installé sur PVE depuis le bundle, plus de drop manuel |
| NET-04 | Vocabulaire registre unifié (fin `synapse.cynic`) | DIVERGENT | 9 refs `synapse.cynic` vs 16 `service.cynic` dans `infra/registry.json`. APRÈS NET-01 (jamais avant — cf. ordre de migration) |
| NET-05 | Transport source via registre (zéro DNS système) | EN PLACE | prouvé builder : clone `ssh://forgejo@forgejo.synapse.cynic` OK, cargo lancé (commit a7a082a6) |
| NET-06 | DHCP+DNS co-localisés PVE | EN PLACE | dnsmasq hôte sur `vmbr0`, range `.50-.200`, `dhcp-option dns-server 10.10.10.1`. Force à garder |
| NET-07 | TLS intra-fleet | MANQUANT | zéro listener TLS déclaré ; test `forgejo-api-tls` échoue. Prérequ : trancher D-TLS1..n de `tls-fleet-design` |
| NET-08 | Firewall : policy déclarée | MANQUANT | `network_policy` = {planes, reservations, canonical} — pas de section firewall. Générateur nftables existe mais sans policy qui le nourrit |
| NET-09 | Firewall : état live observé | INCONNU | `nft`/`iptables` refusés sans root (builder + PVE). Sonde : `nft list ruleset` avec root |
| NET-10 | Identification des listeners fantômes | EN PLACE | résolu 2026-10-15 : `cynic-cloud-api` :4000 = gateway CYNIC déclaré (`cynic-cloud.service`, root pour `pct exec`) ; http.server :8734 ad-hoc servi le repo en 0.0.0.0 → TUÉ ; nginx = projets préexistants ; stock Proxmox OK ; loopbacks OK. Reste 1 question : rpcbind :111 0.0.0.0, besoin NFS à trancher avant toute règle |
| NET-11 | Liveness overlay WireGuard (handshakes) | INCONNU | non probé. Sonde : `wg show <iface> dump` + `latest-handshakes` avec root |
| NET-12 | Synchro temps (NTP/chrony) | INCONNU | non probé. Sonde : `timedatectl` / `chronyc tracking` par hôte. Fondamental distribué : certs, handshakes, preuves datées |
| NET-13 | Section `observed.nodes` tenue par sondes | MANQUANT | clé présente mais vide (`observed.nodes: []` le 2026-10-15) ; rien n'écrit en continu. `network registry observe` existe (one-shot) ; la tenue continue exige un vivant temps réel, pas un cron |
| NET-14 | Distribution secrets (token forgejo, clés) | INCONNU | `token_file` déclaré, présence par hôte non vérifiée. Sonde : existence + perms (jamais le contenu) |
| NET-15 | Accès privilégié borné (membrane/PVE/builder) | MANQUANT | pas de sudo sans mot de passe (builder + PVE) ; `pct` exige priv ; tout apply ci-dessus est bloqué sans chemin root admis |

## Ordre organique (bottom-up, chaque couche prouvée avant la suivante)

1. **Observer** : NET-10, NET-09, NET-11, NET-12, NET-13, NET-14 (lecture seule, avec root admis une fois).
2. **Servir** : NET-01 (+ NET-03 : le servant est GÉNÉRÉ ou il n'est pas), puis NET-02 par client prouvé par requête.
3. **Déclarer** : NET-08 (policy minimale : default-drop + scopes par plan, rien de plus), puis audit live-vs-policy read-only.
4. **Appliquer borné** : firewall (NET-08/09/10), purge vocabulaire (NET-04).
5. **Chiffrer** : NET-07 (D-TLS tranché PUIS code).
6. **Rescoper MYCELIUM** : seules les sections dont chaque besoin ci-dessus est EN PLACE entrent au plan 100-nœuds.

## Portes d'entrée au plan 100-nœuds (toutes EN PLACE exigé)

NET-01, NET-02, NET-04, NET-07, NET-08+NET-09 convergents, NET-10 vide (zéro fantôme),
NET-12 prouvé, NET-15 chemin root admis documenté.

## Non-besoins actés (ne pas construire)

- CoreDNS / vues split-horizon : aucun besoin nomade exprimé ; séparation par fournisseur suffisante.
- DNS en container : SPOF boot sans gain (substrat = hôte PVE).
- Sections MYCELIUM 100-nœuds (relais élus, QUIC streams, télémétrie UDP signée) : après les portes ci-dessus.
