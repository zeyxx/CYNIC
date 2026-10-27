# CYNIC MCP — Décision d'architecture

Statut : proposé. Sans ratification souveraine, les vagues en cours restent
en suspens et aucun nouvel outil n'est ajouté.

## 1. Analyse de l'échec précédent

Le `cynic-mcp` supprimé en f07e9827 (24 août 2026) faisait 70 fichiers et
22 000 lignes, avec axum, reqwest, tokio full, tracing, chrono, glob et regex.
Un god-crate HTTP. Sa suppression comme dette monolithe est confirmée bonne :
la forme a tué le projet, pas le but.

Trois faits aggravent le dossier :

- Son `handler.rs` de 2 221 lignes ne contenait qu'un seul `#[tool`. L'ancien
  serveur ne satisfaisait déjà pas la règle que l'audit exige aujourd'hui.
- Le remplacement annoncé (`cynic_surface_audit_read_back`, 70271bc0) vivait
  dans le crate supprimé. Il est mort avec lui.
- Depuis, 109 outils déclarés dans `infra/agent-exposure.json` contre zéro
  implémentation, avec deux audits qui comparaient l'exposition à elle-même
  (couverture 100 % par construction). Le vide peint en vert.

Les quatre tranches récentes (`cynic-mcp` stdio, 9 outils) ont été construites
sans lire ni le commit de suppression ni le registre de dette. Le résultat
est architecturalement l'inverse de ce qui est mort (stdio seul, 3
dépendances, ~700 lignes), mais obtenu par instinct. La méthode est rejetée
même quand le résultat est bon.

## 2. Architecture cible

Un seul crate `cynic-mcp`, borné :

- Transport unique : stdio JSON-RPC via rmcp 3.1.0 (version déjà déclarée
  par l'exposition, déjà dans le cache hors-ligne). Pas de HTTP, pas
  d'axum, pas de reqwest, pas de tokio full. Le jour où un transport
  réseau est prouvé nécessaire, il passe derrière un port avec sa propre
  décision — jamais par ajout de dépendance opportuniste.
- Dépendances autorisées : `cynic-protocol` (types purs), `cynic-host`
  (ports et observation), rmcp, serde, tokio minimal. Toute autre
  dépendance exige une preuve qu'aucune frontière existante ne l'absorbe.
- Structure interne : `lib.rs` (observation), `server.rs` (outils),
  `main.rs` (façade serve). Au-delà de ~400 lignes dans `server.rs`,
  découper par domaine (un module par domaine d'exposition), jamais par
  couche technique.
- Interdit : état mutable partagé, magasin de coordination, orchestration
  bash, chemins machine en dur, succès simulés.

## 3. Utilité

Aujourd'hui, les agents Hermes invoquent les outils MCP via un proxy externe
dont la contrepartie dans le repo n'existe pas. L'exposition déclare, rien
n'exécute. Le serveur en arbre donne :

- une contrepartie exécutable et lisible à chaque outil déclaré, dans le
  même workspace que les contrats qu'il projette ;
- un gradient mesurable (compteur de fantômes) qui remplace les jugements
  d'avancement par une lecture ;
- un binaire `cynic-mcp` correspondant exactement au `command` déjà déclaré
  dans l'exposition, vérifiable par handshake live en test.

Non-objectif : remplacer le proxy externe, servir du HTTP, exécuter des
charges mutantes sans admission.

## 4. Périmètre et ordre des vagues

Vague 1 (livrée) : sondes lecture seule sans dépendance — ping, versions,
inventaire. Vague 2 (livrée) : contrats, harness, observation de binaire.
Vague 3 (livrée) : registre et IPAM.

Règle d'admission d'un outil : un exécuteur réel existe aujourd'hui dans
`cynic-protocol` ou `cynic-host`, ou en observation directe du workspace.
Sinon l'outil n'est pas implémenté et reste fantôme explicite. Sont
exclus jusqu'à leur propre décision : outils à magasin (coordination),
outils à mutation (admission requise), outils à live distant (mesh, fleet,
activation), outils produit sans exécuteur (jugement, métabolisme).

## 5. Position CLI : extraction, pas réécriture

Le CLI fait 208 fichiers et 36 000 lignes, avec des façades à 600-850
lignes qui violent le seuil dur et la loi de finesse. La réécriture de zéro
est rejetée : elle jetterait 36 000 lignes de comportement prouvé sans
spécification exécutable du bon, contre la loi d'extraction outillée.
Le régime : chaque retouche CLI déplace de la logique vers le crate
propriétaire et fait baisser les compteurs des audits (surface, ownership,
crate, formality). Les fichiers au-dessus de 400 lignes sans raison de
rôle sont de la dette d'extraction nommée, traitée un par un.

## 6. Portes et gradient

- `cynic dev surface-audit` reste bloqué tant qu'un outil déclaré manque
  de contrepartie. Le compteur de fantômes est la mesure d'avancement.
- Chaque vague : `#[tool]` + exécuteur réel + déclaration + test live.
  Test qui épinglait le faux vert converti en gradient (plancher source,
  zéro non-déclaré, fantômes = déclarés − implémentés).
- Condition de sortie du confinement : zéro fantôme — restaurer l'assertion
  tout-vert.

## 7. Inconnues résiduelles

- Dérive `forgejo` : clé réseau `local` sans CIDR déclaré (trouvée par
  l'outil IPAM lui-même). Convention non documentée ou dérive du registre.
  Trancher : documenter le plan ou corriger l'entrée.
- Propriété `proof_history` et surfaces `askesis` : propriétaire prévu
  fantôme dans les contrats normatifs. Réassignation souveraine en suspens.
- Transport réseau futur : aucune preuve d'utilité à ce jour.

## 8. Preuves exigées par vague

Tests unitaires des exécuteurs, test live stdio contre le binaire
(handshake + appel, sans mock), suite lib `cynic-cli` verte, suite
`cynic-mcp` verte, `rustfmt --check`, `git diff --check`. Une vague qui
ne fait pas baisser le compteur de fantômes n'est pas une vague.
