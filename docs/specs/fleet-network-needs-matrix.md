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

## Matrice

| ID | Besoin | État | Fait observé / sonde |
|---|---|---|---|
| NET-01 | Zone canonique `service.cynic` servie fleet-wide | MANQUANT | `dig @10.10.10.1 forgejo.service.cynic` → vide ; seul cortex se sert (dnsmasq :1053 local). Bloqueur : root PVE pour `service.hosts` + `service.conf` + restart |
| NET-02 | Résolution homogène (même résolveur fleet) | DIVERGENT | cortex → 127.0.0.1:1053 ; builder → Freebox 192.168.0.254 (hosts file) ; CT → DHCP 10.10.10.1. Repoint builder = root manquant |
| NET-03 | Artefacts DNS générés (header, régénérables) | MANQUANT | `/etc/cynic/dns/synapse.hosts` = drop manuel du 4 sept, sans header, rien ne le régénère ; PVE n'a pas `service.hosts` du tout |
| NET-04 | Vocabulaire registre unifié (fin `synapse.cynic`) | DIVERGENT | 9 refs `synapse.cynic` vs 16 `service.cynic` dans `infra/registry.json`. APRÈS NET-01 (jamais avant — cf. ordre de migration) |
| NET-05 | Transport source via registre (zéro DNS système) | EN PLACE | prouvé builder : clone `ssh://forgejo@forgejo.synapse.cynic` OK, cargo lancé (commit a7a082a6) |
| NET-06 | DHCP+DNS co-localisés PVE | EN PLACE | dnsmasq hôte sur `vmbr0`, range `.50-.200`, `dhcp-option dns-server 10.10.10.1`. Force à garder |
| NET-07 | TLS intra-fleet | MANQUANT | zéro listener TLS déclaré ; test `forgejo-api-tls` échoue. Prérequ : trancher D-TLS1..n de `tls-fleet-design` |
| NET-08 | Firewall : policy déclarée | MANQUANT | `network_policy` = {planes, reservations, canonical} — pas de section firewall. Générateur nftables existe mais sans policy qui le nourrit |
| NET-09 | Firewall : état live observé | INCONNU | `nft`/`iptables` refusés sans root (builder + PVE). Sonde : `nft list ruleset` avec root |
| NET-10 | Identification des listeners fantômes | INCONNU | PVE `:80/:443/:4000/:3128/:85`, builder `:8081` loopback — propriétaires inconnus. Sonde : `ss -tlnp` avec root. AUCUNE règle avant ça |
| NET-11 | Liveness overlay WireGuard (handshakes) | INCONNU | non probé. Sonde : `wg show <iface> dump` + `latest-handshakes` avec root |
| NET-12 | Synchro temps (NTP/chrony) | INCONNU | non probé. Sonde : `timedatectl` / `chronyc tracking` par hôte. Fondamental distribué : certs, handshakes, preuves datées |
| NET-13 | Section `observed.nodes` tenue par sondes | INCONNU | à vérifier : qui écrit `observed`, fraîcheur, preuve |
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
