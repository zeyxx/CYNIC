# May 2-3 Execution Guide — Wallet Behavior Corpus Collection

**Status:** Ready for production execution ✅

**Timeline:** May 2 (collection) → May 3 (validation) → May 5+ (impact measurement)

**Falsification criterion:** ROC-AUC > 0.7 on real corpus of 10H + 8-10S wallets

---

## Pre-Execution Checklist (May 2, morning)

Before running the collection script, verify:

```bash
# 1. Confirm API key is available in environment
# Should be loaded from ~/.cynic-env or similar secure location
env | grep -q HELIUS_API_KEY && echo "✓ API key loaded" || echo "✗ API key missing"

# 2. Verify all Python modules are importable
python3 cynic-python/wallet_behavior_validator.py 2>&1 | grep "All modules"

# 3. Confirm corpus addresses file exists
ls -lh cynic-python/CORPUS_ADDRESSES.json

# 4. Verify script is executable
ls -lh scripts/collect-validation-corpus.sh
```

---

## Execution (May 2, 10 AM - 12 PM)

### Step 1: Set API Key and Run Collection Script

```bash
# Load API key from secure location (e.g., ~/.cynic-env)
# Then run the automated collection script
bash scripts/collect-validation-corpus.sh

# Expected output file:
# → cynic-python/validation_corpus_real.json
```

### Step 2: Monitor Progress

The script will:
1. Load CORPUS_ADDRESSES.json (10 humans + 10 sybils)
2. Fetch profiles from Helius API (10-15 minutes)
3. Skip synthetic test wallets (marked as skipped, not counted as failures)
4. Save JSON corpus to validation_corpus_real.json
5. Display summary: N wallets collected, N humans, N sybils

**Costs:**
- Human profiles: 10 × 120 credits = 1,200 credits
- Sybil profiles: ~8-10 × 120 credits = 960-1,200 credits
- **Total:** ~2,160-2,400 credits (~$0.54-$0.60)

**Rate limits:** Helius allows 100 RPS; collection is conservative (~10 RPS)

### Step 3: Validate Output File

After script completes:

```bash
# Check file exists and has data
jq '.[] | {wallet_address, is_human, wallet_age_days, single_token_pct}' \
  cynic-python/validation_corpus_real.json | head -20

# Quick stats
jq 'group_by(.is_human) | map({is_human: .[0].is_human, count: length})' \
  cynic-python/validation_corpus_real.json
```

---

## Validation (May 3, 9 AM)

### Run Validator on Real Corpus

```bash
python3 cynic-python/wallet_behavior_validator.py \
  cynic-python/validation_corpus_real.json

# Expected output:
# Validation Results
# ==================
# Corpus: 18-20 wallets (10H + 8-10S)
#
# ROC-AUC: ?
# Accuracy: ?
# TP/TN/FP/FN: ?
# 
# Separation: ?
# Specificity: ?
# Sensitivity: ?
```

### Falsification Decision (May 3, noon)

**PASS:** ROC-AUC > 0.7
- Proceed to Phase 2 (May 5-6): measure CYNIC impact on token verdicts
- Heuristic generalizes to real data ✓

**FAIL:** ROC-AUC ≤ 0.7
- Debug: which scores are colliding? (age/diversity/temporal/anomalies)
- Revise weights in wallet_behavior_scorer.py (lines 60-80)
- Re-collect and re-validate (cost: another ~$0.60)
- Decision point: abandon Phase 1 or iterate?

---

## If PASS → Phase 2 (May 5-6)

### Measure CYNIC Impact on Token Verdicts

```bash
# 1. Score 20-30 tokens with all Dogs (baseline)
python3 scripts/measure_token_impact.py --mode baseline

# 2. Score same tokens, filtering by verified humans only
python3 scripts/measure_token_impact.py --mode filtered_by_humans

# 3. Measure Δ in verdict distribution
python3 scripts/compare_distributions.py \
  baseline_verdicts.json \
  filtered_verdicts.json
```

**Falsification:** Δ > 5% in verdict distribution between baseline and filtered

---

## Data Integrity Checks

### Before May 2 Collection

1. **Addresses are live on mainnet:** Helius will fetch even if dormant (activity span historical)
2. **No private keys stored:** All addresses are public Solana programs/known accounts
3. **Epistemic status:** Observed (public data, on-chain verified, documented archives)

### After May 2 Collection

1. **Verify no nulls in required fields:**
   ```bash
   jq '.[] | select(.wallet_age_days == null or .token_count == null)' \
     cynic-python/validation_corpus_real.json | wc -l
   # Should output: 0
   ```

2. **Check score distribution:**
   ```bash
   jq '.[] | .authenticity_score' \
     cynic-python/validation_corpus_real.json | sort -n
   # Humans should cluster ≥ 0.6
   # Sybils should cluster ≤ 0.2
   ```

3. **Verify label fidelity:**
   ```bash
   jq 'group_by(.is_human) | map({
     is_human: .[0].is_human,
     avg_score: (map(.authenticity_score) | add / length),
     min: (map(.authenticity_score) | min),
     max: (map(.authenticity_score) | max)
   })' cynic-python/validation_corpus_real.json
   ```

---

## Troubleshooting

### Issue: "HELIUS_API_KEY not set"
- Solution: Load from secure location (e.g., ~/.cynic-env) before running script
- Ensure environment variable is properly exported before execution

### Issue: "address not found on mainnet"
- Expected for synthetic test wallets (script skips automatically)
- If real address fails: network issue or address migrated. Log and skip.

### Issue: "Too many requests" (429)
- Script backs off automatically
- Helius free tier: 100 RPS. Script uses ~10 RPS (conservative)
- If persistent: wait 1 min, retry single wallet to verify network

### Issue: ROC-AUC < 0.7 on real corpus
- Check: is age/diversity threshold too strict? (weights at lines 60-80)
- Check: are sybils actually generating low diversity scores? (profile_helius.py enrichment)
- Debug: plot distribution of scores by is_human to see overlap
- Revise and re-test (cost: ~$0.60 per iteration)

---

## Commit & Handoff

After validation (May 3):

```bash
# Commit corpus to git (for audit trail)
git add cynic-python/validation_corpus_real.json
git commit -m "feat(wallet): real corpus (10H + 8S) ROC-AUC=$(cat validation_result.txt)"

# Push to origin
git push origin main

# Update TODO.md with result
# → Falsification Test 2: PASS/FAIL
```

---

## Success Markers

**May 2 (Collection):**
- [ ] Script runs without errors
- [ ] 10 human profiles fetched
- [ ] 8-10 sybil profiles fetched
- [ ] validation_corpus_real.json contains valid JSON with all required fields
- [ ] Cost was ~$0.60
- [ ] No API throttling or network issues

**May 3 (Validation):**
- [ ] ROC-AUC computed and displayed
- [ ] Confusion matrix generated (TP/TN/FP/FN)
- [ ] ROC-AUC > 0.7 OR debugging plan documented
- [ ] Commit pushed to origin/main with result

**May 5-6 (Impact Measurement):**
- [ ] Token impact script runs successfully
- [ ] Baseline vs filtered verdict distributions measured
- [ ] Δ computed (target > 5%)
- [ ] Decision: iterate heuristic or proceed to Phase 3?

---

**Next action:** May 2, 10 AM. Set HELIUS_API_KEY and run the script.
