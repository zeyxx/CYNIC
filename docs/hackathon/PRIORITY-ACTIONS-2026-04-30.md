# Priority Actions — April 30, 2026

> **Ground truth crystallized.** Two separate hackathon competitions. Different deadlines. Clearest path forward is defined.

---

## CRITICAL (Do First)

### 1. S. — Confirm B&C Registration Plan
**What:** Blitz & Chill has registration hard gate **May 4 23:59 UTC**. No extensions. After this date: project ineligible for awards.

**Action:** 
- [ ] Confirm Colosseum platform access (can log in, can register)
- [ ] Mark May 4 calendar (do not miss)
- [ ] Reply: "Ready to register" or "Need help with platform"

**Why:** If missed, cannot compete. Period.

**Falsify:** Colosseum registration form submitted by May 4 23:59.

---

### 2. S. — Decide: Full Integration or Solo?
**What:** CYNIC wallet-judgment is code-ready but untested on real B&C game data.

**Decision points:**
- **Option A (Full Integration):** Send 3-5 WalletProfile JSON samples by **May 1 23:59**
  - T. validates (30min), adds integration test
  - You integrate CYNIC /judge call into B&C /mint-permit by May 1-4
  - Submit with full anti-Sybil gate

- **Option B (Solo/Ed25519):** Skip CYNIC integration entirely
  - Use Ed25519 signature + games ≥5 gate (already implemented)
  - No new work needed
  - Ship on time guaranteed

**Action:**
- [ ] Decide: Option A (send data) or Option B (Ed25519-only)?
- [ ] Reply by **May 1 00:00 UTC**

**Why:** T. needs to know if wallet-judgment integration test is needed by May 1 23:59.

**Falsify:** S. sends WalletProfile JSON samples OR confirms "proceeding with Ed25519-only."

See: `MESSAGE-TO-S-2026-04-30.md` for details.

---

## HIGH PRIORITY (Hackathon Delivery)

### 3. T. — Record CYNIC Demo Video
**What:** Hackathon submission requires video demo (2-3 min, 4 scenes).

**Scenes:**
1. Kernel logs + `/health` endpoint (circuit breaker visible)
2. `curl /judge` with chess content → deterministic-dog returns q_score
3. UI rendering verdict + axiom breakdown chart
4. (Optional) B&C integration OR recovery endpoint demo

**Action:**
- [ ] Record when rested (no artifacts, clean state)
- [ ] Use existing script: `docs/hackathon/VIDEO-DEMO-SCRIPT.md`
- [ ] Setup: start tunnel (`cloudflared tunnel --url ...`), kernel live, Dogs responding
- [ ] Upload to submission platform

**Deadline:** May 10 23:59 PDT

**Falsify:** Video uploaded to Colosseum. Users can play it.

**Note:** Video is optional to demo immediate urgency, but required for final submission. Can be done any time before May 10.

---

### 4. T. — Update Colosseum Submission Description
**What:** Submission requires written description + video + GitHub + deployed URL.

**Action:**
- [ ] Update `docs/hackathon/COLOSSEUM-SUBMISSION-FULL.md` (currently marked as TODO)
- [ ] Copy/refine sections: Problem, Solution, Differentiation, Why It Matters, Status
- [ ] Include: K15 validation results, Dogs voting mechanism, on-chain proof, axiom soundness

**Deadline:** May 10 23:59 PDT

**Falsify:** Description submitted on Colosseum platform.

---

## MEDIUM PRIORITY (If Time Allows)

### 5. T. — Ship Conviction-Only Token Baseline
**What:** Token calibration pipeline validated (100% accuracy on 28 tokens). Ready to ship.

**Action:**
- [ ] Commit: `token_dataset_ingester_conviction_only.py` + `measure_conviction_only.py`
- [ ] Add to CI/CD or kernel calibration pipeline
- [ ] (Next session) Measure Dogs agreement on live token set

**Deadline:** No hard deadline (post-hackathon acceptable per TODO).

**Falsify:** Conviction-only model integrated; next session measures Dogs vs conviction correlation.

---

### 6. T. — Install Hermes Crons (Unblock K15)
**What:** Three missing systemd timers: gemini-briefing (4h), feedback-loop (1h), hermes-agent-executor (service).

**Why:** Currently K15 violation. Hermes produces data but no one consumes it. Crons will fix this.

**Action:**
- [ ] Wire three systemd timers
- [ ] Verify: `systemctl list-timers` shows 3 active

**Deadline:** Soft (post-demo OK).

**Falsify:** `systemctl list-timers` shows 3 Hermes timers, all running.

---

## IF S. SENDS DATA (Conditional)

### 7. T. — Validate Wallet-Judgment & Add Integration Test
**What:** S. provides WalletProfile samples. T. validates deterministic_dog output.

**Action:**
- [ ] Parse WalletProfile JSON samples
- [ ] Call deterministic_dog(profile) on each
- [ ] Verify verdicts: legit → WAG/GROWL (≥0.618), suspicious → BARK (<0.382)
- [ ] Add integration test to `cynic-kernel/tests/integration_wallet_judgment.rs`
- [ ] Run test, confirm passing
- [ ] Signal: "Wallet-judgment integration ready"

**Deadline:** May 1 23:59 UTC (S. needs this to decide final integration by May 4)

**Falsify:** Integration test in `tests/`, all passing.

---

## Timeline Summary

```
Today: Apr 30
├─ S. confirms registration + decides Option A/B
├─ T. records demo (flexible, can do anytime before May 10)
│
May 1
├─ S. sends WalletProfile data (Option A only)
├─ T. validates + adds integration test (30min, if data received)
│
May 1-4
├─ S. optionally integrates CYNIC /judge (if Option A + T. confirms)
│
May 4 23:59 ⏰ HARD GATE
├─ B&C registers on Colosseum platform
│
May 10 23:59 ⏰ HARD GATE
├─ CYNIC video demo + description submitted
│
May 11 23:59 ⏰ HARD GATE
├─ Both projects final submissions close
```

---

## Reference Documents

- `HACKATHON-GROUND-TRUTH-2026-04-30.md` — Two competitions, dates, deadlines, risk assessment
- `MESSAGE-TO-S-2026-04-30.md` — Data request for wallet-judgment integration
- `COLOSSEUM-SUBMISSION-FULL.md` — CYNIC submission description
- `VIDEO-DEMO-SCRIPT.md` — Demo recording scenes
- `B2C-CYNIC-INTEGRATION-SPECIFICATION.md` — Endpoint contract

---

## Next Sync with S. and T.

**Message S.:**
- Share `HACKATHON-GROUND-TRUTH-2026-04-30.md` (clarifies May 4 registration)
- Share `MESSAGE-TO-S-2026-04-30.md` (asks for WalletProfile samples or Option B decision)
- Ask: "Can you register by May 4? Will you send integration test data or proceed with Ed25519-only?"

**Brief T.:**
- Update on conviction-only baseline (ready to ship, 100% validated)
- Confirm demo recording plan (before May 10)
- Note: wallet-judgment integration test is blocking if S. sends data by May 1

