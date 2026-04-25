# CYNIC Colosseum Video Demo Script (2-3 min)

## Scene 1: Token Input (~15s)
**Narration:** "We submit a token to CYNIC for judgment."
- Show curl request to `/judge` endpoint
- Token: USDC (known legitimate token)
- Request includes 6 axioms framework

## Scene 2: Dogs Deliberate (~45s)
**Narration:** "Five independent Dogs score the token on six axioms: Fidelity, Phi (structure), Verify (verifiability), Culture (tradition), Burn (efficiency), Sovereignty (agency)."
- Show kernel logs as each Dog responds
- qwen-7b-hf (HF Inference)
- qwen35-9b-gpu (GPU Dog, 55 tok/s, most thorough)
- gemma-4-e4b-core (CPU Dog)
- deterministic-dog (heuristic baseline, instant)
- gemini-cli (optional, may be skipped)
- Display scoring in real time (axiom scores 0.0-1.0 per Dog)

## Scene 3: Crystal Coherence & Verdict (~40s)
**Narration:** "CYNIC crystallizes disagreement into a single verdict through the CCM (Crystal Coherence Machine). The verdict isn't an average — it's a learned consensus boundary."
- Show max_disagreement calculation across axioms
- Display final verdict: HOWL/WAG/GROWL/BARK score
- Timestamp and reasoning (why this score)

## Scene 4: (Optional) On-Chain Submission (~30s)
**Narration:** "The verdict lands on-chain via Pinocchio, immutable and verifiable."
- Show Solana transaction
- Devnet address: Program `A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx`
- Community PDA: `8Pyd1hqd6jTX2jR8YvCAjnd3cyP5qB7XaxzwAGtHCSFD`

## Key Stats for Narration
- Detection rate: 75.5% on 53 confirmed rug-pulls (baseline)
- Latency: ~10-15 seconds (3-4 Dogs respond before verdict)
- Dogs working: 5/5 (qwen7b, qwen35-gpu, gemma-core, deterministic, gemini)
- Axioms: 6 (FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY)
