# D2 — Stratégie de scan d'intégrité périodique du vault d'artifacts IA (~102 GB)

**Périmètre observé (preuves locales, 2026-08-24)**
- Vault `/var/lib/cynic/vault/models` : 102G confirmé (`du -sh`). 23 fichiers `.gguf` à plat + sous-arbres `qwen38-27b/` (+2 shards) → ~26 fichiers GGUF au total ; plus gros : `Qwen3.8-27B-IQ3_XXS.gguf` 11,8 GB. Présence de **`Qwen3-4B-Q4_K_M.gguf.corrupt`** : l'intégrité est déjà un problème vécu.
- Hôte : AMD Ryzen 7 5700G, **SHA-NI détecté** (`/proc/cpuinfo`) → sha256 accéléré hardware.
- Bench réel : `sha256sum` sur 512 MB d'un GGUF → **0,257 s user ≈ 2 GB/s** (coreutils/SHA-NI). Full sweep 102 GB ≈ **51 s CPU**, wall-clock 1–3 min selon I/O.
- Stockage : SSD SATA (ROTA=0), ext4 sur `/dev/sda2`, **rempli à 90 % (37 GB libres)** — contrainte forte pour toute re-org CAS.
- Inconnus notés : modèle exact du SSD / SMART non lus ; page fs.com inaccessible (202 anti-bot) → source fs.com **non vérifiable** aujourd'hui.

## 1. Cadence du sweep
Taux de bitrot silencieux documentés (études primaires, pas d'anecdote) :
- CERN IT (Panzer-Steindel 2007) : sonde 2 GB écrite/relecture sur ~3000 nœuds → 500 erreurs / 100 nœuds en 5 semaines ; contrôle adler32 de 8,7 TB → **22 fichiers corrompus (1/1500)**, byte error rate ~3×10⁻⁷. https://indico.cern.ch/event/13797/contributions/1362288/attachments/115080/163419/Data_integrity_v3.pdf
- Bairavasundaram et al., NetApp (SIGMETRICS'07/FAST'08, >1,5 M disques) : **8,5 % des disques nearline et 1,9 % des enterprise** subissent au moins une corruption checksummable ; 13 %/38 % indétectées par la vérification passive ; >400k mismatches/41 mois. https://www.pdsw.org/pdsw09/resources/pdsw09-final13.pdf
- Schroeder FAST'16 (flash) : **20–63 % des SSD** subissent ≥1 URE en production ; 2–6 ‰ drive-days affectées (cité par Backblaze). https://www.backblaze.com/blog/managing-for-hard-drive-failures-data-corruption
- fs.com blog bit-rot : introuvable/inaccessible (202 anti-bot, recherche vide) — chiffre cité dans le brief non sourcé ici.

Argumentaire : la fenêtre d'exposition est dominée par le temps entre deux sweeps, mais le coût marginal d'un passage nocturne est quasi nul (≈1–3 min pour 102 GB, <5 % d'un créneau nightly, négligeable vs un run d'inference). Un weekly ne réduit le coût que de ~4 min CPU/semaine — économie dérisoire contre 7× plus d'exposition et une variance de détection inutile. **Nightly retenu**, avec deux garde-fous : nice/ionice, et skip si inference active (lock CYNIC).

## 2. Premier passage : digest-only baseline (recommandé) vs full-CAS-reorg immédiat
- **Digest-only baseline** : hasher l'arbre tel quel → manifest `{path, sha256, size, mtime, source_url}` versionné, écrit atomiquement (tmp+rename+fsync), puis read-back proof : re-hash N fichiers échantillonnés + relecture du manifest. Zéro mutation du vault, zéro risque sur un FS à 90 %.
- **Full-CAS-reorg immédiat** : réécrire 102 GB = ~102 GB de writes temporaires + renames, alors qu'il reste **37 GB libres** → impossible sans purge préalable ; risque de torn write pendant la copie ; casse les chemins consommés par llama.cpp/les jobs ; double exposition au bitrot pendant la migration ; gain nul tant que le manifest n'existe pas.
- Verdict : baseline digest-only now ; la reorg CAS devient un chantier séparé, conditionné à de l'espace disque libéré (≥ taille du vault) et exécutée fichier-par-fichier avec vérification avant suppression.

## 3. Précédents registres d'artefacts
- **Nix store** : DB signée par chemin + `nix-store --verify --check-contents [--repair]` recalcule le sha256 de chaque path et retélécharge ce qui diverge — vérification on-demand, jamais périodique par défaut. https://nix.dev/manual/nix/2.18/command-ref/nix-store/verify
- **OCI/Docker registry** : contenu adressé par digest ; le GC mark-and-sweep reconstruit le mark-set depuis les manifests (les digests sont l'autorité, pas un scan) ; vérification à l'ingest (pull/push), pas de scrub planifié natif. https://docs.docker.com/registry/garbage-collection/
- **Miroirs Arch/Debian** : intégrité portée par les signatures GPG des paquets et les sommes du file-db vérifiées à chaque install — pas de scan périodique du cache ; les miroirs se synchronisent via rsync + horodatage `lastsync`.
- Convergence : personne ne scrubbe « au fond » par défaut ; tous vérifient à l'écrivain/lecteur et exposent une commande de verify manuelle. CYNIC mono-opérateur a besoin d'un cran de plus : le sweep périodique remplace l'absence de redondance multi-copies.

## 4. Recommandation finale
| Paramètre | Valeur |
|---|---|
| Cadence | Sweep complet nightly (~1–3 min, 102 GB @ ~2 GB/s SHA-NI) |
| Premier passage | Baseline digest-only, manifest `{path, sha256, size, mtime, source_url}`, écriture atomique tmp→rename→fsync + read-back proof (re-hash échantillon + relecture manifest) |
| Périmètre | Arbre récursif `/var/lib/cynic/vault/models` (inclut sous-répertoires/shards) ; exclure `*.corrupt` (déjà invalides, à traiter hors scan) |
| Exécution | Bounded root executor : argv typé (`vault-scan --root --manifest-out`), budget temps/I-O, nice/ionice, lock vs jobs inference, preuve post-action exigée |
| Alerte mismatch | **Quarantaine auto** : rename `f.gguf → f.gguf.corrupt.<digest-court>` (même inode, atomique, réversible) + événement typé + alerte opérateur. Pas d'auto-suppression, pas de re-téléchargement auto (pas encore de source_url fiable par fichier) |
| Seuil | Tout mismatch = incident (bitrot silencieux n'est pas graduel). Deux mismatches consécutifs sur le même fichier = corroboration avant quarantaine si doute (I/O transient) |
| Reorg CAS | Chantier séparé : conditionnée à ≥102 GB libres, fichier par fichier (write temp → hash → rename → rm source après proof) |

Risque résiduel : ext4 sans checksumming de données — le manifest détecte mais ne corrige pas ; la seule correction est le re-téléchargement depuis `source_url`. À moyen terme, envisager un FS à checksumming (bcachefs/ZFS) ou une copie froide, mais hors périmètre D2.

## Sources
- CERN Panzer-Steindel, *Data Integrity* (2007) : https://indico.cern.ch/event/13797/contributions/1362288/attachments/115080/163419/Data_integrity_v3.pdf
- Narayan & al. PDSW'09 (synthèse NetApp/Bairavasundaram) : https://www.pdsw.org/pdsw09/resources/pdsw09-final13.pdf
- Backblaze (URE flash, Schroeder FAST'16) : https://www.backblaze.com/blog/managing-for-hard-drive-failures-data-corruption/
- Kelemen (HEPiX 2007, taxonomie corruptions CERN) : https://indico.desy.de/event/257/contributions/58082/attachments/37574/46878/kelemen-2007-HEPiX-Silent_Corruptions.pdf
- Nix store --verify : https://nix.dev/manual/nix/2.18/command-ref/nix-store/verify
- Docker registry GC : https://docs.docker.com/registry/garbage-collection/

---
*Rapport D2 — étude ops storage CYNIC. Preuves locales collectées le 2026-08-24.*
