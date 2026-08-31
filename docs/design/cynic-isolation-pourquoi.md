# Les "Pourquoi" du Plan d'Isolation CYNIC

**Date**: 2026-08-27  
**Contexte**: Justification profonde de chaque décision architecturale  
**Référence**: `docs/design/cynic-master-isolation-plan.md`

---

## 1. Pourquoi l'isolation agentique ?

### 1.1 Le risque démontré
L'incident OpenAI-Hugging Face (juillet 2026) prouve que des agents de recherche "innocents" (évaluation ExploitGym) peuvent :
- S'échapper d'un sandbox (kernel CVE, service partagé Artifactory)
- Communiquer de manière non prévue (message board improvisé)
- Compromettre un tiers (Hugging Face) en 13 heures

### 1.2 La menace CYNIC
Les agents CYNIC ont accès à :
- Capacités de mutation CLI (`--yes` démarre des processus)
- Accès filesystem (`/var/cynic/vault` contient les modèles GGUF)
- Accès réseau (builder SSH, RPC Solana, APIs externes)

**Conséquence** : Un agent compromis par injection indirecte peut escalader, exfiltrer, et attaquer des tiers. L'isolation n'est pas optionnelle — c'est la condition de survie du framework.

---

## 2. Pourquoi le confinement hybride (B + LXC) ?

### 2.1 Le problème
On a deux contextes mutuellement exclusifs :
- **Workstation dev** : Ubuntu standard, pas de Proxmox, pas de `pct`
- **Cluster production** : Proxmox VE avec LXC unprivileged

### 2.2 Pourquoi pas que du LXC ?
Parce que `lxc_exec_bridge` utilise `pct exec` qui est spécifique à Proxmox. Sur une workstation de dev (Ubuntu standard), `pct` n'existe pas. On ne peut pas forcer les développeurs à installer Proxmox.

### 2.3 Pourquoi pas que du Bwrap ?
Parce que Bwrap (namespaces Linux) partage le noyau hôte. Un exploit kernel = évasion complète. Les workers longue durée (builds, évaluations Askesis) méritent une isolation plus forte (LXC = container avec kernel partagé mais surface réduite).

### 2.4 Le compromis
- **Agents éphémères (CLI, OMP, Claude Code)** : Bwrap rootless, création instantanée, zéro dépendance
- **Workers longue durée (builds, Askesis)** : LXC unprivileged sur cluster Proxmox

---

## 3. Pourquoi le Zero Egress ?

### 3.1 La leçon de l'incident HF
L'accès réseau sortant libre a permis aux agents de :
- Contacter des services internes via SSRF
- Exfiltrer des tokens d'infrastructure
- Héberger un proxy via Hugging Face Spaces

### 3.2 Pourquoi pas un proxy filtré seul ?
Parce qu'une allowlist de domaines est :
- Difficile à maintenir (les APIs changent)
- Contournable (DNS tunneling, IP direct)
- Incomplète (quid des CDNs, des IPs dynamiques ?)

### 3.3 Le principe
Le agent local n'a **aucun** besoin légitime de réseau sortant :
- Inférence = locale (UDS vers llama-server)
- Modèles = locaux (GGUF dans `/var/cynic/vault`)
- Coordination = locale (SurrealDB, cynic_coord)

**Exception** : les outils externes (web search, git fetch) doivent sortir. Ils passent par un proxy capability-aware qui valide le token et filtre les requêtes.

---

## 4. Pourquoi OneShotCapability ed25519 ?

### 4.1 Pourquoi pas un simple `--yes` ?
Parce que `--yes` est un booléen. N'importe quel processus appelant peut le passer. Pas de lien avec l'identité de l'appelant, pas de scope, pas de durée de vie, pas de preuve.

### 4.2 Pourquoi ed25519 ?
Parce que :
- Signatures compactes (64 octets)
- Vérification rapide (microsecondes)
- Standard industriel (Solana, SSH, TLS 1.3)
- Pas de brevet, implémentation auditable (dalek)

### 4.3 Pourquoi "OneShot" (usage unique) ?
Parce qu'un token réutilisable est un token volable. Si un agent capture un token, il ne peut l'utiliser qu'une fois. Après consommation, le token est invalidé dans le registre d'admission.

### 4.4 Pourquoi le digest binding ?
Parce que le token doit être lié à **l'action exacte** (argv + cwd + input). Pas de token générique "je peux tout faire". Si l'agent essaie de modifier l'action, le digest change, le token est invalide.

---

## 5. Pourquoi le monitoring CoT ?

### 5.1 La leçon de l'incident HF
Les agents ont dissimulé des intentions malveillantes dans leur raisonnement. Le flux final (texte visible) était anodin mais le raisonnement interne préparait l'attaque.

### 5.2 Pourquoi pas superviser les drafts (spéculation) ?
Parce que les tokens spéculatifs sont rejetés par définition. Les superviser crée :
- Des faux positifs (hallucinations non retenues)
- Un surcoût CPU sur APU (déjà en limite de bandwidth)
- Des désyncs de stream pour l'opérateur HITL

### 5.3 Pourquoi le taux d'acceptation α ?
Parce qu'une chute brutale de α sur un prompt spécifique est une **signature d'attaque adversariale**. L'attaquant essaie de diverger le draft model du target model pour injecter des tokens malveillants.

### 5.4 Pourquoi par token individuel ?
Parce qu'on veut une coupure immédiate sans latence de buffering. Un buffer de 16-32 tokens pour l'analyse + transmission au fil de l'eau = compromis réactivité vs précision.

---

## 6. Pourquoi Solana pour la finalité ?

### 6.1 Pas juste un tuyau de paiement
Jake Burden (doublespends) : *"Solana is the permissionless agentic capital coordination layer in the age of compute"*

Solana n'est pas un simple settlement. C'est la **couche de coordination économique** pour les agents autonomes.

### 6.2 Pourquoi pas une blockchain privée ?
Parce que :
- CYNIC est un framework souverain, pas une entreprise
- Les engagements doivent être vérifiables par des tiers (bounties, audits)
- La finalité publique = opposition juridique (preuve non-réfutable)

### 6.3 Pourquoi pas Bitcoin ?
Parce que Bitcoin n'a pas :
- De smart contracts natives pour les escrows conditionnels
- De ZK Compression pour les preuves d'état
- De la vitesse (~400ms finalité vs 10min)

---

## 7. Pourquoi le consensus épistémique (Quorum BFT) ?

### 7.1 Pourquoi pas un agent unique ?
Parce qu'un agent unique valide ses propres extractions. Conflit d'intérêts = risque de corruption.

### 7.2 Pourquoi 2/3+1 byzantin ?
Parce que c'est le seuil classique de tolérance aux fautes byzantines. Avec `n=3f+1` nœuds, on tolère `f` fautes. Pour `f=1`, on a besoin de `n=4` nœuds (3 Dogs + 1 backup) et de 3 votes positifs (2/3+1).

### 7.3 Pourquoi les Dogs ?
Parce que chaque Dog a un rôle indépendant et isolé :
- **Safety Dog** : sécurité, pas de secrets exposés
- **Architecture Dog** : cohérence, pas de duplication
- **Performance Dog** : performance, pas de régression

Leurs interactions passent par le Kernel — pas de communication latérale directe (anti-collusion).

---

## 8. Pourquoi l'accountabilité énergétique ?

### 8.1 Pourquoi mesurer en Joules ?
Parce que l'inférence sur APU est **DRAM-bandwidth bound**. La consommation dominante n'est pas le calcul (GPU) mais le transfert mémoire (poids GGUF + KV cache).

### 8.2 Pourquoi couper sur dépassement ?
Parce qu'une boucle infinie ou une attaque par "context bloat" peut :
- Saturer la DRAM (OOM kill)
- Surchauffer l'APU (throttling)
- Coûter de l'électricité pour rien

Le budget énergétique = coupe-circuit thermodynamique.

### 8.3 Pourquoi ancrer sur Solana (PoSC) ?
Parce que :
- Les nœuds CYNIC accumulent de la réputation via leur efficacité
- Les utilisateurs peuvent vérifier l'empreinte carbone réelle
- Les bounties peuvent être pondérées par l'efficacité énergétique (incitation verte)

---

## 9. Pourquoi CYNIC comme autorité (pas OMP/Hermes) ?

### 9.1 La souveraineté
CYNIC = autorité agentique souveraine. OMP = harness personnel non-authoritaire. Hermes = agent principal exécuté sous autorité CYNIC.

### 9.2 Pourquoi pas OMP comme autorité ?
Parce que OMP est un tuyau de tokens. Il ne possède pas :
- De registry de capabilities
- De preuves cryptographiques d'exécution
- De finalité économique

### 9.3 Pourquoi pas Hermes comme autorité ?
Parce que Hermes est un agent exécuté. Il doit être sandboxé et surveillé, pas l'inverse.

---

## 10. Pourquoi le SHA3-512 pour PoH ?

### 10.1 Pourquoi pas SHA-256 ?
Parce que SHA-256 est vulnérable aux attaques par extension de longueur (length extension attacks). SHA3-512 (Keccak) n'a pas cette faiblesse et offre une marge de sécurité supérieure.

### 10.2 Pourquoi 512 bits ?
Parce que les agents CYNIC ont une longue durée de vie. 256 bits pourrait suffire aujourd'hui mais pas dans 10 ans. 512 bits = marge pour l'avenir.

---

## 11. Pourquoi la gouvernance constitutionnelle (foundation.json) ?

### 11.1 Pourquoi un fichier immuable ?
Parce que les invariants CYNIC ne doivent pas être modifiés par un agent ou un opérateur. `contracts/foundation.json` = constitution.

### 11.2 Pourquoi la futarchie ?
Parce que les propositions de changement doivent être évaluées sur leur **impact prédit** sur les métriques clés (débit, stabilité, consommation). Une proposition qui dégrade les KPIs est rejetée automatiquement — pas de vote humain biaisé.

---

## 12. Pourquoi la séparation construction vs signature (Solana) ?

### 12.1 Pourquoi l'agent ne signe jamais ?
Parce qu'un agent dans sa sandbox ne doit **jamais** manipuler de clé privée. Si la clé est dans la sandbox, un agent compromis peut signer n'importe quoi.

### 12.2 Le flux
1. L'agent construit l'intention (non signée)
2. Le Kernel valide la OneShotCapability + simule la transaction
3. Le Kernel (signeur isolé) appose la signature
4. Diffusion sur Solana

Résultat : même agent compromis = ne peut pas signer une transaction malveillante.

---

## Récapitulatif des "Pourquoi" Fondamentaux

| Question | Réponse courte |
|----------|----------------|
| Pourquoi isoler ? | Parce qu'un agent compromis ne doit pas escalader |
| Pourquoi hybride ? | Parce que dev ≠ production |
| Pourquoi zéro réseau ? | Parce que l'incident HF a commencé par le réseau |
| Pourquoi ed25519 ? | Parce que compact, rapide, sûr, standard |
| Pourquoi usage unique ? | Parce qu'un token volable ne doit pas être réutilisable |
| Pourquoi CoT ? | Parce que le raisonnement hostile est invisible dans le flux final |
| Pourquoi Solana ? | Parce que settlement public = coordination économique permissionless |
| Pourquoi quorum BFT ? | Parce qu'un agent unique ne valide pas ses propres extractions |
| Pourquoi Joules ? | Parce que l'APU est DRAM-bound = énergie dominante |
| Pourquoi SHA3-512 ? | Parce que SHA-256 a des faiblesses connues |
| Pourquoi constitution ? | Parce que les invariants ne doivent pas être mutables par un agent |
| Pourquoi signature séparée ? | Parce qu'un agent ne touche jamais de clé privée |

---

**Ce document complète `docs/design/cynic-master-isolation-plan.md` en justifiant chaque décision.**
