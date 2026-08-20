# Audit de sécurité — Veilo `privacy-pool` (`programs/privacy-pool/src/lib.rs`)

**Cible :** `/tmp/veilo-official/programs/privacy-pool/src/lib.rs` (5388 lignes, fichier central : `transact`, `handle_public_amount`, `mark_nullifier_spent`, `initialize`, `transact_swap`/`fund_native_source`, comptes, vérificateur ZK).
**Objectif :** vulnérabilités de **perte de fonds** (retrait non autorisé, vol, sur-crédit, bug comptable, contournement d'autorité/signer, overflow/underflow, collision PDA, réentrance CPI, double-spend sur nullifiers, Merkle root forgée).
**Date de l'audit :** 2026-08-20 | **Auditeur :** pentester Solana (sous-agent).

---

## 1. Résumé (TL;DR)

Le **cœur dépôt/retrait (`transact` + `handle_public_amount`) est cryptographiquement sain** : tous les flux financiers (`public_amount`, `fee`, `refund`, `recipient`, `relayer`, `input_nullifiers`, `output_commitments`, `root`) sont liés par la preuve Groth16 ; le double-spend est bloqué par PDA `init` + `is_spent` ; la vérification de la Merkle root couvre toute l'historique ; les seeds PDA sont uniques ; les retraits vérifient le **solde réel** du vault ; l'arithmétique des frais est bornée ; le chemin swap impose un min-output et une contrainte de solvabilité. **Aucune vulnérabilité de vol/sur-crédit/double-spend directe n'a été trouvée dans la logique on-chain.**

Trois points à corriger / creuser :

| ID | Sévérité | Emplacement | Nature |
|----|----------|-------------|--------|
| **F-1** | **Moyenne** | `initialize_global_config` (lib.rs ~654-667, fn 3416) | Contrôle d'accès absent → le 1er appelant devient admin global (squatting de pool / risque de déploiement) |
| **F-2** | Faible | `transact_swap` (swap.rs 464) | Divergence comptable `total_tvl` sur le chemin "relayer-float" natif (non exploitable pour un vol, mais masque l'insolvabilité) |
| **F-3** | Faible / À confirmer | `transact` (lib.rs 3667-3686) | Burn systématique de 2 nullifiers sur chaque dépôt (coût de rent + **DoS possible si les nullifiers de dépôt sont déterministes dans le circuit** — à vérifier côté circuit/ZK) |

> Note : Aucun des points ci-dessus n'est un vol direct des fonds d'un pool *déjà déployé et correctement initialisé*. F-1 est le plus concret (contrôle d'accès).

---

## 2. Périmètre & méthodologie

- Lecture ligne à ligne de `lib.rs` (entry points, comptes, handlers), `zk.rs` (vérification Groth16 + binding des public inputs), `groth16.rs` (pairing), `merkle_tree.rs` (`is_known_root`, `append`), et des modules délégués `swap.rs` / `positions.rs` / `phoenix.rs` / `predictions.rs` (chemins de débit du vault).
- Focus : (1) binding de la preuve, (2) checks d'autorité/owner, (3) arithmétique de frais, (4) unicité des seeds PDA, (5) bornes du vault/TVL.
- Le circuit ZK lui-même (`*.circom` / witness) n'était pas fourni : les conclusions sur le binding supposent que la VK (`vk_constants.rs`) est cohérente avec l'ordre/signe des public inputs du circuit (vérifié de façon défensive par le code : `public_inputs.len()+1 == vk.vk_ic.len()` fait échouer proprement une VK mal dimensionnée).

---

## 3. Surfaces auditées (et jugées saines)

| Fonction / zone | Lignes | Constat |
|---|---|---|
| `verify_transaction_groth16` | zk.rs 169-231 | 8 public inputs construits et passés au pairing : `root, public_amount, ext_data_hash, mint_address, null0, null1, commit0, commit1`. `require_canonical` + `reduce_to_field` cohérents. |
| `verify_swap_transaction_groth16` | zk.rs 262-329 | 10 public inputs, dont `swap_amount`, `swap_params_hash`, `source_root` — tous liés. |
| `Groth16Verifier` | groth16.rs 75-205 | Préparation `Σ input_i·IC[i+1]+IC[0]` + pairing standard. `CHECK_PUBLIC_INPUTS=true`. Échec propre si nb d'inputs ≠ `vk_ic.len()-1`. |
| `transact` — checks pré-transfert | lib.rs 3492-3641 | nullifiers ≠ 0 / ≠ entre eux ; commitments ≠ 0 ; `ext_data_hash` vérifié ; `deadline` ; relayer autorisé si `public_amount<=0` ; `relayer==ext_data.relayer` ; `recipient==ext_data.recipient` ; `mint_address==cfg.mint_address` ; comptes token désérialisés avec `owner==token::ID` et mint/owner cohérents. |
| `transact` — nullifiers | lib.rs 3657-3686 | `is_known_root` sur l'input tree ; `init` sur les markers (bloque le reuse à la résolution du compte) + `is_spent`. |
| `handle_public_amount` — dépôt natif | lib.rs 4984-4999 | `system_program::transfer(from=relayer, to=vault, deposit_amount)` ; `total_tvl += deposit_amount` borné par `checked_add`. |
| `handle_public_amount` — dépôt SPL | lib.rs 4974-4981 | `token::transfer(user_token→vault, authority=relayer)` ; `user_token.owner==relayer`, pas de delegate. |
| `handle_public_amount` — retrait natif | lib.rs 5113-5154 | Vérifie `vault.lamports() >= withdrawal + rent + 0.01 SOL` ; débit manuel lamports : `vault -= W`, `recipient += W-fee-refund`, `relayer += fee+refund` → conservé ; `total_tvl -= W`. |
| `handle_public_amount` — retrait SPL | lib.rs 5069-5112 | `vault_token.amount >= withdrawal_amount` ; `to_recipient = W-fee-refund`, `to_relayer = fee+refund` ; total = W ; `total_tvl -= W`. |
| `transact_swap` — sortie DEX | swap.rs 849, 868 | `swapped_amount >= min_amount_out` **et** `vault_amount >= swap_params.dest_amount` (anti-sur-crédit de la note de destination). |
| `is_known_root` | merkle_tree.rs 116-148 | Parcours complet du buffer circulaire (256 slots) — aucune racine historique légitime n'est manquée. |
| `initialize` (pool) | lib.rs 600-609 | Bien gardé : `constraint = admin.key() == global_config.admin`. |

---

## 4. Constatations positives (preuves de robustesse)

1. **Binding de la preuve.** `public_amount`, `fee`, `refund`, `recipient`, `relayer`, les nullifiers et les commitments sont tous des public inputs de la preuve. Un retrait (`public_amount<0`) exige donc une preuve valide prouvant la connaissance de notes réelles non dépensées — pas de "mint frauduleux" via un `public_amount` négatif (le circuit l'interdit, et le `from` du dépôt natif est bien le relayer, jamais le vault).
2. **Anti-double-spend.** Markers PDA `seeds=[b"nullifier_v3", mint, nullifier]` avec `init` (échec si déjà existant) **+** `is_spent`. Espace de noms global par `mint` → un même nullifier ne peut être dépensé ni dans `transact` ni dans `transact_swap` sur ce mint.
3. **Retraits basés sur le solde réel.** Natif : `vault.lamports()` ; SPL : `vault_token.amount`. Pas de confiance aveugle en `total_tvl`.
4. **Bornes de frais.** `fee+refund <= withdrawal_amount` et `fee <= max_fee_with_margin` (lib.rs 5019, 5064). Plafond intrinsèque ~`fee_bps`×(1+margin) (voir F-4).
5. **Seeds PDA uniques.** `privacy_config_v3` / `privacy_vault_v3` / `privacy_nullifiers_v3` / `privacy_note_tree_v3`(+tree_id) / `nullifier_v3` / `swap_executor`(+source,dest,null,relayer) — pas de collision. Signer du vault = `[b"privacy_vault_v3", mint, &[vault.bump]]` cohérent avec le PDA du compte.
6. **Pas de réentrance exploitable.** Les débits lamports/token se font *après* vérification + nullifiers + append, sans CPI subséquent dans le chemin retrait.

---

## 5. Vulnérabilités / Findings

### F-1 — `initialize_global_config` sans contrôle d'accès (admin global = 1er appelant)  ·  **Sévérité : Moyenne**

- **Fonction :** `initialize_global_config` (lib.rs 3416-3423) ; struct `InitializeGlobalConfig` (lib.rs 654-667).
- **Mécanisme :** Le compte `global_config` (PDA `b"global_config_v1"`) est créé avec `init` et stocke `global_config.admin = ctx.accounts.admin.key()`. Le champ `admin` est déclaré `/// CHECK` **sans** `has_one`, sans `Signer` contraint, sans whitelist. Comme `init` ne peut s'exécuter qu'une fois, **le premier appelant devient définitivement l'admin global**.
- **Impact / perte de fonds :** Pas un vol direct du vault (les retraits exigent toujours une preuve ZK valide). Mais :
  - Si le `global_config` n'est **pas** pré-initialisé par l'équipe au déploiement, un attaquant front-run et s'arroge l'admin global.
  - Il peut alors `initialize(...)` n'importe quel mint (contrainte `admin == global_config.admin` satisfaite) → **squatting de pool** pour un mint légitime non encore créé, ou phishing (les utilisateurs déposent dans "le" pool de ce mint, qui appartient à l'attaquant).
  - Les pools *déjà* créés gardent leur propre `admin` (fixé à l'init) et ne sont pas hijackables ; l'attaquant ne peut pas non plus modifier la config d'un pool existant (`update_pool_config` = `ConfigAdmin` avec `has_one = admin`). Donc **pas de drain d'un pool existant**, mais risque opérationnel/déploiement réel.
- **PoC conceptuel :**
  1. `initialize_global_config{admin = Attacker}` (1er appel, réussit, `global_config.admin = Attacker`).
  2. `initialize{mint = USDC, admin = Attacker, ...}` → passe (admin == global_config.admin).
  3. Attaquant = admin du pool USDC ; il peut `add_relayer`/`update_pool_config` à sa guise et bloquer toute init ultérieure du mint USDC par l'équipe (PDA déjà prise → DoS de création).
- **Recommandation :** Geler l'admin global à une clé/ multisig d'équipe *à la compilation* (constante) ou exiger que `initialize_global_config` ne soit appelable que par une autorité connue ; rendre `admin` `Signer` + `has_one`/whitelist ; ou ajouter une `initialized` flag avec garde. Idéalement : l'admin global ne doit pouvoir être défini qu'une fois par l'équipe avant toute opération.

### F-2 — Divergence comptable `total_tvl` (chemin "relayer-float" natif)  ·  **Sévérité : Faible**

- **Fonction :** `transact_swap` (swap.rs ~363-466, débit à 391-422, `total_tvl -= swap_amount` à 464).
- **Mécanisme :** Dans le chemin natif **non pré-financé** (`is_prefunded == 0`), c'est le **relayer** qui paie `swap_amount` (`system_program::transfer(from=relayer, to=executor)`, swap.rs 413-422) — le vault source **n'est pas débité**. Pourtant `source_config.total_tvl = total_tvl.checked_sub(swap_amount)` (swap.rs 464) est appliqué. Résultat : `total_tvl` sous-estime le solde réel du vault (le vault garde ses lamports, le relayer a flotté). Dans le chemin pré-financé (`fund_native_source`, swap.rs 210-220), le vault est bien débité → cohérent.
- **Impact / perte de fonds :** Aucun vol direct (le vault conserve ses fonds ; les retraits réels utilisent le solde on-chain, pas `total_tvl`). Risque : `total_tvl` devient non représentatif → masque l'état réel, casse le monitoring/insolvabilité et toute logique tiers qui ferait confiance à `total_tvl`.
- **PoC conceptuel :** Swap natif via le chemin relayer-float d'un montant `S` ; après tx, `source_vault.lamports()` est inchangé mais `source_config.total_tvl` a diminué de `S`.
- **Recommandation :** Ne débiter `total_tvl` que si le vault source a réellement été débité (i.e. dans le chemin pré-financé, ou débiter le vault au lieu du relayer dans le chemin float). Ou traiter `total_tvl` explicitement comme "estimation" et ne jamais l'utiliser comme garde de sécurité.

### F-3 — Burn systématique de 2 nullifiers sur chaque dépôt  ·  **Sévérité : Faible (à confirmer côté circuit)**

- **Fonction :** `transact` (lib.rs 3667-3686) + `mark_nullifier_spent` (lib.rs 4901).
- **Mécanisme :** Sur **tous** les chemins (y compris dépôt `public_amount>0`), deux markers PDA sont créés (`init`, payés par le relayer) pour `input_nullifiers[0]` et `[1]` — bien que le dépôt n'ait pas de note d'entrée réelle. Le commentaire affirme que les nullifiers de dépôt sont "non-zero Poseidon outputs" mais "*deterministic from dummy witnesses*". **Si le circuit produit des nullifiers de dépôt déterministes (identiques à chaque dépôt)**, alors le 2e dépôt échoue (`init` sur un PDA déjà existant) → **DoS : un seul dépôt réussi par pool**. Si les nullifiers incluent une entropie par transaction (plausible, vu le commentaire "fresh per transaction"), aucun impact.
- **Impact / perte de fonds :** Pas de vol. Soit (a) coût de rent pour le relayer à chaque dépôt (mineur), soit (b) **DoS complet des dépôts** si nullifiers déterministes — ce qui bloquerait le fonctionnement du pool (disponibilité, pas perte de fonds existants).
- **PoC conceptuel (si déterministe) :** Dépot #1 OK (crée nullifiers N1,N2). Dépot #2 avec les mêmes N1,N2 → `init` échoue → `transact` entier revert.
- **Recommandation :** **Vérifier le circuit ZK** : confirmer que les nullifiers de dépôt dépendent d'une entrée privée aléatoire par transaction (unicité garantie). Sinon, ne pas créer de markers pour les dépôts, ou utiliser des nullifiers dérivés de l'output note.

### F-4 — Plafond de frais relayer (info, pas une faille)  ·  **Sévérité : Info**

- `MAX_FEE_BPS = 500` (5 %, lib.rs 94) et `fee_error_margin_bps` jusqu'à 5000 (50 %). `max_fee_with_margin = max_fee × (10000+margin)/10000` (lib.rs 5057). Avec 5 % de base + marge 50 %, le relayer peut prélever jusqu'à ~7,5 % d'un retrait. C'est un **plafond de sur-paiement utilisateur**, pas un vol du protocole (borne par `withdrawal_amount`, et `fee` est lié à la preuve via `ext_data_hash`).
- **Recommandation :** Documenter le plafond effectif ; envisager de borner `fee_error_margin_bps` plus strictement (ex. ≤ 1000) si on veut limiter la sur-extracton relayer.

### F-5 — Modules `phoenix.rs` / `perps.rs` / `predictions.rs` (hors périmètre primaire)  ·  **Sévérité : À auditer séparément**

- Les handlers `phoenix_*` (phoenix.rs), `jperp_*` (perps.rs), `prediction_*` (predictions.rs) et `open_position`/`close_position` (positions.rs) suivent le **même pattern vérifié** (check `vault_token.amount >= total_outflow` avant `token::transfer` signé par `[b"privacy_vault_v3", mint, &[vault_bump]]`, ex. phoenix.rs 222-225, 381, 520 ; predictions.rs 136-139, 227, 238).
- Ils délèguent des CPI complexes (Phoenix, Jupiter, Raydium, Pump) : la surface d'attaque spécifique (validation des comptes DEX passés en `remaining_accounts`, slippage, ordre des CPI) n'était **pas** dans le périmètre "dépôt/retrait/handlers principaux" mais mérite un audit dédié. Aucun défaut de contrôle d'autorité du vault n'a été repéré à première vue.

---

## 6. Conclusion

- **Pas de vulnérabilité de perte de fonds directe** (vol, sur-crédit, double-spend, root forgée, overflow) dans la logique on-chain centrale `transact` / `handle_public_amount` / `transact_swap` / `fund_native_source`. Le design lie correctement tous les flux à la preuve ZK et empêche le double-spend.
- **Correctif prioritaire (F-1) :** sécuriser `initialize_global_config` (admin global figé/autorisé) avant tout déploiement mainnet — sinon risque de squatting de pool par le 1er appelant.
- **Correctifs secondaires :** aligner `total_tvl` sur le débit réel (F-2) ; confirmer l'unicité des nullifiers de dépôt côté circuit (F-3).
- **Suivi recommandé :** audit dédié de `phoenix.rs` / `perps.rs` / `predictions.rs` (CPI DEX, validation `remaining_accounts`, slippage) et vérification de la cohérence VK↔circuit (ordre/signe des public inputs) — hors périmètre de ce passage mais nécessaires pour un avis de bounty complet.

*Audit limité au fichier `lib.rs` + modules délégués fournis ; le circuit ZK (witness/circom) et `vk_constants.rs` n'ont pas été fournis et conditionnent la validité du binding (voir F-3 et §3).*
