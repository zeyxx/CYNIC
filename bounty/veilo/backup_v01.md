# BACKUP FINDING — V-01: `phoenix_ember_unwrap` overcredit (déjà connu publiquement)

> ⚠️ Ce bug est DÉJÀ documenté dans des forks publics (sudo-robi/veilo-privacy-pool-audit).
> Risque de DOUBLON si déjà soumis. À soumettre UNIQUEMENT si aucun nouveau bug trouvé
> ET si on estime qu'il n'est pas encore pris par le sponsor. Backup, pas priorité.

**Affected function**: `phoenix_ember_unwrap` (phoenix.rs:1638-1669)
**Program**: GYy4kM6GHhpgLCUscuABbzkD2ZbJ2fneYryaZ6Ch7fFU (mainnet)
**Severity**: Critical (fund loss)

## Description
`phoenix_ember_unwrap` crédite `cfg.total_tvl += amount` et `pending.amount += amount`
en utilisant le `amount` FOURNI PAR LE CALLER, sans vérifier que l'EMBER CPI a réellement
livré `amount` USDC dans l'ATA de l'executor au préalable.

## Path to fund loss
1. EMBER `withdraw` CPI livre < amount (ou ATA pré-financée / EMBER forké qui rapporte succès).
2. `token::transfer(executor_ata -> vault_ata, amount)` déplace `amount` (peut bouger un solde préexistant).
3. `cfg.total_tvl += amount` et `pending.amount += amount` crédités INCONDITIONNELLEMENT.
4. `phoenix_reissue_notes` permet au claimant de mint des notes privées pour `pending.amount`
   = USDC que le vault n'a JAMAIS reçu → perte de fonds du pool.

## Proof of Concept
Test conceptuel (réplique logique) dans le fork public:
`pocs/v-01-proof/tests/v01_ember_unwrap_overcredit.rs`
→ montre que `actual_delivered=100_000` mais `total_tvl` et `pending.amount` crédités de `1_000_000`.

Pour PoC reproductible sur mainnet fork:
```bash
# Fork mainnet state local
solana-test-validator --clone GYy4kM6GHhpgLCUscuABbzkD2ZbJ2fneYryaZ6Ch7fFU -r /tmp/veilo-fork
# Build PoC anchor test qui appelle phoenix_ember_unwrap avec amount > delivered
# et vérifie total_tvl over-credit
```

## Suggested fix
Lire le solde réel de l'ATA executor AVANT/APRÈS le transfer EMBER, et créditer
`min(amount, actual_delivered)` ou require `executor_ata.amount_after >= amount`.
Ne jamais créditer sur le caller `amount` seul.

## Confirmation
Aucun fonds mainnet déplacé. Analyse statique + simulation locale uniquement.
