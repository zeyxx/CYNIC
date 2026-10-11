# CYNIC — Spécification d'Isolation des Agents en Conteneurs LXC

## 1. Contexte & Problématique

L'exécution directe d'agents autonomes sur la station de travail principale (`cortex`) pose des risques d'intégrité :
- Instabilité physique (fautes mémoires DRAM sur bit 46 observées et documentées).
- Concurrence sur les worktrees Git et pollution du système hôte.
- Risque d'élévation de privilèges ou d'écritures parasites hors périmètre.

Pour garantir la souveraineté, la résilience et l'étanchéité des agents, CYNIC délègue l'exécution longue des agents à des conteneurs LXC dédiés hébergés sur l'infrastructure Proxmox VE.

---

## 2. Topologie de l'Infrastructure

| Hôte / Rôle | Identifiant / IP | Substrat | Rôle dans l'architecture |
|---|---|---|---|
| **Hyperviseur PVE** | `pve` / `192.168.0.37` | Proxmox VE 8.x (Linux 6.8) | Orchestrateur matériel, contrôle `pvesh` / `pct` |
| **Builder Canonique** | `cynic-builder` / `10.10.10.119` (CT 105) | Ubuntu 24.04 LXC | Autorité de compilation Rust (`make check`, release) |
| **Agent Host Isolant** | `cynic-agent-host` / `10.10.10.102` (CT 102) | Ubuntu 24.04 LXC | Exécution étanche des agents sous Tmux |
| **Forge Git SSOT** | `organ-forgejo` / `10.10.10.101` (CT 101) | Alpine / Forgejo 11.x | Dépôt Git souverain interne |

---

## 3. Matrice de Sécurité & Privilèges

1. **Utilisateur non-privilégié** : Les sessions d'agents s'exécutent strictement sous le compte `cynic` (`uid=1000`), sans accès `sudo` interactif non consenti.
2. **Isolation Tmux** :
   - Session nommée selon la norme `cynic-agent-{agent_id}-{role}` (`infra/tmux-runtime.json`).
   - Journalisation passive par `pipe-pane` vers `/var/log/cynic/agents/{session_name}.log`.
   - Zéro socket X11 / Wayland partagé avec l'hôte physique.
3. **Contrat de déploiement binaire** :
   - Chemin canonique : `/usr/local/bin/cynic`
   - Hash cryptographique vérifié avant admission : `4640377bc15df45ec525c8b2c6eade805a8736c7abe011692c965bbbe8f5ea6a`
   - Conformité catalogue : `cynic privilege catalog --json` validé (0 bloqueurs).

---

## 4. Preuve Opérationnelle (PoW)

Vérifié le 11 octobre 2026 :
- Conteneur 102 opérationnel sur `10.10.10.102` (switch virtuel vmbr0).
- Clé SSH Forgejo provisionnée et synchronisation git fonctionnelle.
- Test de session `cynic-agent-lxc-hermes` : pipe-pane actif, retour `cynic version` et `uname -a` validé, destruction propre sans déchet.
