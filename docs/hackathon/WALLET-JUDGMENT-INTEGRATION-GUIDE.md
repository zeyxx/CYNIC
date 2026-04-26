# Wallet Judgment Integration Guide

**Purpose:** Complete integration architecture for Option C anti-Sybil gate. For T. (kernel implementer) and S. (B&C backend developer).

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│ Blitz & Chill Backend (/mint-permit endpoint) │
├─────────────────────────────────────────────────┤
│ 1. Verify Ed25519 sig ✓                         │
│ 2. Check games_completed >= 5 ✓                 │
│ 3. Build enriched_context from LocalGameData   │
│    - Use cynic_kernel::domain::wallet_enrichment
│ 4. POST /judge to CYNIC                        │
│    - stimulus.domain = "wallet-judgment"        │
│    - stimulus.context = enriched_context       │
│    - inject_crystals = true                     │
│ 5. Parse verdict.consensus.mean_q_score        │
│    - If >= 0.618: PASS (mint permitted)        │
│    - If < 0.618: FAIL (403 Forbidden)          │
│ 6. Rate-limit: 1 mint/wallet/hour               │
│ 7. Upload Arweave, return mint permit          │
└─────────────────────────────────────────────────┘
              ↓
┌──────────────────────────────────────────────────┐
│ CYNIC Kernel (/judge endpoint)                  │
├──────────────────────────────────────────────────┤
│ Pipeline flow (unchanged):                       │
│ • Embed stimulus.content                        │
│ • Check cache                                   │
│ • Search crystals (semantic + domain)           │
│ • Build enriched_context (crystals + sessions)  │
│ • Construct Stimulus:                           │
│   - content: "Validate wallet..."               │
│   - context: [user_context | crystals | sess]   │
│   - domain: "wallet-judgment"                   │
│ • Call evaluate_progressive() → Dogs            │
│ • Return verdict + dog_scores                   │
│                                                  │
│ Dogs (5 total):                                 │
│ • deterministic-dog: pseudocode (in-kernel)    │
│ • qwen-7b-hf: LLM (fast)                        │
│ • qwen35-9b-gpu: LLM (nuanced)                  │
│ • gemma-4-e4b: LLM (conservative)               │
│ • gemini-cli: LLM (pattern matching)            │
│                                                  │
│ Each Dog scores 6 axioms (FIDELITY, PHI,       │
│ VERIFY, CULTURE, BURN, SOVEREIGNTY)            │
│ → q_score (0-1)                                 │
│ → verdict_kind (HOWL/WAG/GROWL/BARK)           │
│                                                  │
│ Consensus:                                      │
│ • mean_q_score = avg of 5 Dogs' q_scores       │
│ • If mean >= 0.618: option_c_gate = PASS       │
│ • Else: option_c_gate = FAIL                    │
│                                                  │
│ CCM (Crystal Coherence Machine):                │
│ • observe_crystal_for_verdict()                 │
│   Input: verdict (from any Dog)                 │
│   Output: crystal (if quorum + epistemic gate)  │
│   Future: "honest_wallet" crystals inject      │
│   into future wallet evaluations (K15)         │
└──────────────────────────────────────────────────┘
```

---

## Implementation Checklist

### T. (CYNIC Kernel — /home/user/Bureau/CYNIC/cynic-kernel/)

- [ ] **Deterministic Dog** (cynic-kernel/src/domain/dog/)
  - Implement pseudocode from cynic-kernel/domains/wallet-judgment-dogs.md
  - Gates: games >= 5 (mandatory), suspicious_cluster OR replay_risk (terminal)
  - Return DogScore with q_score + axiom_scores + verdict_kind
  - Register in Judge::available_dogs()

- [ ] **Wallet Judgment Domain** (already created)
  - cynic-kernel/domains/wallet-judgment.md ✓ (6 axioms)
  - cynic-kernel/domains/wallet-judgment-dogs.md ✓ (prompts)

- [ ] **Wallet Enrichment Utility** (already created)
  - cynic-kernel/src/domain/wallet_enrichment.rs ✓
  - WalletEnrichmentBuilder: input LocalGameRecord → formatted markdown
  - Export for B&C to use (or T. provides as shared lib)

- [ ] **No Pipeline Changes Needed**
  - Existing pipeline.rs already handles wallet-judgment domain
  - B&C provides enriched_context → stimulus.context → Dogs
  - evaluate_progressive() routes to Dogs normally

- [ ] **Optional: Gate at evaluate_progressive()**
  - If domain=="wallet-judgment" and games_completed < 5:
    - Return BARK verdict immediately (safety, no Dog wasted)
  - Or: let Dogs detect and penalize in their scoring

- [ ] **Optional: CCM Crystal Loop**
  - Once verdict volume exists, observe_crystal_for_verdict() will produce "honest_wallet" crystals
  - Inject into future wallet evaluations (K15 acting consumer test)

### S. (Blitz & Chill Backend)

- [ ] **Build /nonce endpoint**
  - Generate 32-byte random nonce
  - Store in Redis/memory, TTL 5 minutes
  - Return { nonce, nonce_hash, expires_at }

- [ ] **Build /mint-permit endpoint**
  - Input: { wallet, nonce, archetype, ed25519_sig }
  - Verify Ed25519 sig (use nacl or curve25519 library)
  - If invalid: return 401 Unauthorized
  - Lookup games_completed from LocalCompletedGame store
  - If < 5: return 400 Bad Request
  - **Build enriched context:**
    ```javascript
    const enrichment = buildWalletContext(wallet, localGames);
    // Or use T.'s utility:
    // const enrichment = cynic_kernel::domain::wallet_enrichment::WalletEnrichmentBuilder::new(...)
    ```
  - **Call CYNIC /judge:**
    ```javascript
    const response = await fetch(`${CYNIC_REST_ADDR}/judge`, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${CYNIC_API_KEY}`,
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        stimulus: {
          content: "Validate wallet for Personality Card mint",
          domain: "wallet-judgment",
          context: enrichment,
          wallet_address: wallet
        },
        inject_crystals: true
      })
    });
    ```
  - **Parse verdict:**
    ```javascript
    const verdict = await response.json();
    const meanQScore = verdict.consensus.mean_q_score;
    const gatePass = meanQScore >= 0.618;
    
    if (!gatePass) return 403; // Sybil detected
    ```
  - Rate-limit check: 1 mint per wallet per hour
  - Upload Arweave metadata
  - Return 200 + { mint_permit, arweave_uri }

- [ ] **Rate-Limiter**
  - Track { wallet, mint_timestamp, arweave_uri }
  - Before successful mint, check no other mint in last 3600s
  - If exists: return 429 Too Many Requests

- [ ] **Arweave Upload**
  - Create Personality Card metadata JSON
  - Upload via Irys SDK
  - Get Arweave URI
  - Store for audit trail

### Fallback (B&C ships without CYNIC)

If CYNIC wallet-judgment is not ready by May 1:

```javascript
async function validateWalletMinimal(wallet, nonce, archetype, sig) {
  // Verify Ed25519 sig
  const isValid = nacl.sign.detached.verify(...);
  if (!isValid) return { passed: false, reason: "invalid_sig" };
  
  // Check games >= 5
  const games = await db.query("SELECT COUNT(*) FROM completed_games WHERE wallet = ?", [wallet]);
  if (games < 5) return { passed: false, reason: "insufficient_games" };
  
  // PASS: cryptographic + game gate only
  return { passed: true, mode: "ed25519-only", confidence: null };
}
```

This allows shipping J6-7 without anti-Sybil. Add CYNIC gating post-hackathon (May 12+).

---

## Fallback Timeline

| Date | If CYNIC Ready | If Not Ready |
|------|---|---|
| Apr 26-27 | S. builds /nonce + /mint-permit | S. builds /nonce + /mint-permit |
| Apr 28 | T. ships deterministic dog + wiring | T. continues, not blocking S. |
| May 1-4 | S. integrates CYNIC /judge call | S. uses fallback (Ed25519-only) |
| May 4 | Registration (both teams ready) | Registration (S. can mint) |
| May 11 | Submission (Option C fully live) | Submission (Ed25519 + game gate) |
| May 12+ | CCM looping on wallet verdicts | T. ships wallet-judgment as post-launch hardening |

---

## Testing Checklist

**Unit Tests (T.):**
- [ ] deterministic-dog on 5-game wallet (85% archetype, age 24 days, no red flags) → HOWL or WAG
- [ ] deterministic-dog on <5 games → BARK (early return)
- [ ] deterministic-dog on replay_risk=true → BARK (terminal)
- [ ] WalletEnrichmentBuilder on real LocalGameRecord → formatted context

**Integration Tests (T. + S.):**
- [ ] End-to-end: B&C sends POST /judge → CYNIC returns verdict → S. checks mean_q_score >= 0.618
- [ ] Fallback: B&C calls /mint-permit without CYNIC → uses Ed25519 + game gate
- [ ] Rate-limit: second mint from same wallet within 1 hour → 429

**Manual Testing (S. + T.):**
- [ ] Mint a Personality Card with valid Ed25519 + 5 games + CYNIC PASS
- [ ] Attempt mint with 4 games → 400 Bad Request
- [ ] Attempt mint with invalid Ed25519 → 401 Unauthorized
- [ ] Attempt second mint within 1 hour → 429 Too Many Requests

---

## Epistemic Status

**Observed:**
- B&C LocalCompletedGame store exists, J3-5 shipped
- Option C decision confirmed via Slack
- Token-judgment domain works (calibration baseline from 2026-04-22)

**Deduced:**
- Wallet behavior can be scored via 6-axis framework (same as token-judgment)
- Archetype consistency is strong proxy for autonomous decision-making
- Time variance detects bot-like patterns

**Inferred (not yet tested):**
- Dogs will change verdicts based on enriched context (K15 impact unknown)
- φ⁻¹ (0.618) is right confidence threshold (set by design, not validated)
- Gameplay patterns will be stable enough to detect coordination (falsifiable when crystal volume exists)

**Critical Unknown (future):**
- Can CCM loop sustain "honest_wallet" crystals? (depends on verdict volume + persistence)
- Will Dogs actually use enriched context to change scores? (K15 acting consumer test, deferred to when crystal volume exists)

---

## References

- cynic-kernel/domains/wallet-judgment.md — domain evaluation criteria
- cynic-kernel/domains/wallet-judgment-dogs.md — Dog prompts + deterministic algorithm
- cynic-kernel/src/domain/wallet_enrichment.rs — enrichment utility (T. provides to B&C)
- docs/hackathon/B2C-CYNIC-INTEGRATION-SPECIFICATION.md — integration spec for S.
- CLAUDE.md § K15 — Consumer law (every producer needs an acting consumer)
- Hackathon timeline: May 4 registration, May 11 submission
