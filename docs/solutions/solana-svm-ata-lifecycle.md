# ADR-002: SVM Ephemeral ATA Lifecycle & Rent Exemption Recovery

## Status
Accepted & Implemented (GASdf Commit `1f35f18c`, PR #29)

## Context
On Solana, creating an Associated Token Account (ATA) requires an immutable rent exemption deposit of 0.00203928 SOL (~2,039,280 lamports).
In a gasless relay model (GASdf) where network fees and state allocations are sponsored by the fee-payer (Kora):
- If users paying subscriptions in USDC do not hold an existing $ASDF ATA, the relay must allocate one to receive the swapped tokens before burning them.
- Sponsoring these account creations linearly drains the fee-payer's SOL reserves (e.g. 50 new users = 0.102 SOL lost purely to state rent).

## Decision
Implement conditional on-chain account lifecycle management in `atomic-tx-builder.ts`:
1. **State Inspection:** Query `rpc.getAccountInfo(userAsdfAta)` prior to transaction assembly.
2. **Atomic Swap & Full Burn:** Swap input token to $ASDF and execute an exact `BurnChecked` instruction.
3. **Conditional Recovery:**
   - If the user had NO prior $ASDF ATA, append a `CloseAccount` instruction post-burn with `destination: feePayerAddr`.
   - The rent exemption deposit is immediately refunded to the Kora fee-payer in the exact same transaction block.
   - If the user already held an $ASDF ATA, the account is left intact to avoid `NonNativeHasBalance` errors or disrupting user balances.

## Consequences
- **Net State Cost:** 0 SOL per new user subscription (transient rent advanced and reclaimed atomically).
- **Execution Cost:** Only nominal signature fees (~5,000 to 10,000 lamports) are permanently consumed.
