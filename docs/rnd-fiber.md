# Fiber — R&D note for CYNIC

Source: solana-foundation/fiber (research prototype, unaudited, unsafe).

## Key patterns

1. Two-phase close — finalize (~19 CU, advances watermark) then distribute (transfers, permissionless).
2. Distribution hash — 16-byte truncated Blake3 hash instead of full config (378 bytes). Channel account: 42 bytes.
3. Batch instructions — batch_open, batch_finalize, batch_distribute to amortize framing (~21 bytes/instruction).
4. fastlane! — zero-cost macro mixing compile-time offsets (19 CU) and runtime parsing (50+ CU).

## Relevance to CYNIC

- Computation market: two-phase close + batch instructions for metered payment channels (AI inference, API metering).
- pay-kit: Fiber completes pay-kit with session intent (streaming/metered) vs charge (one-time).
- fastlane!: macro for high-performance Solana programs in CYNIC (programs/).

## Adoption

Deferred — research prototype, not production. Adopt when computation market is active.
