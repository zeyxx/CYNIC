# Métadonnées machines CYNIC — clarifications & besoins (2026-08-25)

## Confusion commise aujourd'hui (et correction)

J'ai mélangé deux machines dans mon narration :
- **L'AX16 Pro (Ryzen 7 5700U)** = machine future du parc, mentionnée en
  mémoire comme « futur worker Vulkan ». Elle **n'est pas** cette station.
- **Cette station** = hostname `ubuntu`, Ryzen 7 **5700G**, mgmt
  `192.168.100.2`, machine-id `56f4f22...` = **cynic-host-cortex** dans le
  registry. C'est ici que tournent les builds locaux et les serveurs
  llama-server du combat.

## Cartographie réelle (tirée du registry, pas de ma mémoire)

| node_id | IP synapse | mgmt | Rôle | État observé |
|---|---|---|---|---|
| cynic-host-proxmox | 10.10.10.1 | 192.168.100.1 | hyperviseur | — |
| forgejo | 10.10.10.101 | — | git SSOT | — |
| cynic-lxc-surrealdb | 10.10.10.103 | — | storage | — |
| **cynic-builder** | 10.10.10.119 | — | LXC 105 sur proxmox, build_runner | sain, disque à nettoyer |
| local-kernel | 10.10.10.100 | — | kernel_primary | — |
| **cynic-host-cortex** (ICI) | **aucune IP synapse** | 192.168.100.2 | host de travail | instable (kernel -30) |

### Gaps métadonnées identifiés

1. **hostname générique** : `ubuntu` ne dit rien ; le registry dit
   `cynic-host-cortex`. Le hostname devrait porter l'identité du node
   (`cynic-cortex`) ou au minimum les agents doivent résoudre via registry,
   jamais via hostname.
2. **Pas d'IP synapse pour cortex** : `reservations` donne `synapse: None`.
   Soit la machine n'est pas encore enrollée sur l'overlay WireGuard, soit
   le registry est incomplet — à trancher (observation : pas d'interface
   10.20.0.x active malgré `overlay_ip: 10.20.0.3` déclaré).
3. **Modèle matériel absent du registry** : nulle part le registry ne dit
   « cortex = 5700G, builder = i5-6500T VM ». Pour qu'un agent choisisse
   où compiler/exécuter sans deviner, il faut un champ hardware par node
   (cpu_model, cores, ram, gpu) — observation, pas hardcode.
4. **AX16 Pro absente du registry** alors que des décisions lui sont
   attribuées (« futur worker Vulkan »). Si elle entre dans le parc, elle
   doit y figurer avec son rôle; sinon elle doit sortir des mémoires.

## Besoin exprimé : gestion disque builder propre

Constat : LXC 105 était à 100% (323 MiB libres). Nettoyage manuel fait
aujourd'hui : /tmp/cynic-* (4.5G) + cargo clean repo (8.8G target) →
66% utilisé, 13G libres.

Ce nettoyage a été **manuel et réactif** — exactement ce que CYNIC veut
éviter. Il manque :

1. **Un observer disque** sur chaque node (le port `ObservationPort::
   observe_disk` existe côté cynic-host) exposé en report périodique;
2. **Une policy pure** seuils → action (`disk_policy`: warn à 75%,
   clean artefacts re-générables à 85%, alerte bloquante à 92%);
3. **Un bounded apply** : le clean n'exécute QUE les chemins déclarés
   re-générables (`target/`, `/tmp/cynic-*`), jamais source/, avec
   `--apply` explicite et journal du volume libéré.

C'est le pattern coherence standard : desired state (seuils), observe
(df + du par chemin déclaré), reconcile (pure), bounded action
(cargo clean / rm ciblés), read-back (df après).

## Prochaines actions proposées

A. Registry : ajouter `hardware` (cpu_model/cores/ram/gpu) aux nodes +
   statut overlay pour cortex; renommer hostname → cynic-cortex.
B. Builder 119 : portage du disk-watchdog CYNIC (observer+policy+clean)
   en cron systemd timer borné, avec journal JSONL des cleans.
C. Cortex : boot test kernel -28 quand opérateur disponible (tranche
   la question stabilité locale); en attendant, gros builds → 119.
