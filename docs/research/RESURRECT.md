# How to Resurrect Archived Python Code

This document explains how to bring back code from the cynic-python archive if needed.

## Background

On 2026-05-05, cynic-python underwent a burn audit (see `.claude/plans/burn-audit-cynic-python.md`). Result:
- 13 files actively wired to systemd/kernel (Tier 2 INFRASTRUCTURE)
- 57 salvageable files needing consumer + systemd unit (Tier 1 EXPERIMENTAL)
- 54 files archived with valuable research findings
- 0 files deleted (no code was bad enough to trash)

To reduce metabolic cost (context waste in future sessions), salvageable files were tagged with death dates and archived findings were moved to docs/research/.

## When to Resurrect

Resurrect code when:
1. A future session needs the functionality
2. A Tier 2 consumer is identified (systemd service, kernel integration, or test)
3. You can commit to wiring it within 1-2 days

Do NOT resurrect just because "the code looks good." Dead code that's not wired → still dead.

## How to Resurrect

### Step 1: Find the File
Each archived file has a corresponding entry in this document (see Archives section below). Example:

```
ARCHIVE: benchmarks/judge_axiom_quality.py
Location: cynic-python/benchmarks/judge_axiom_quality.py
Tier: 1 EXPERIMENTAL (needs consumer)
Effort to wire: 3 hours (needs cynic-benchmark.timer)
Last commit: 2026-04-27
Death date if not promoted: 2026-05-27 (30 days from archive)
```

### Step 2: Restore the File

```bash
# Find the commit where it was archived
cd ~/Bureau/CYNIC
git log --all --pretty=format:"%H %s" | grep -i "burn.*archive"

# Example output:
# abc123def456 fix(burn): archive 105 dead cynic-python modules

# Restore the file from that commit
COMMIT=abc123def456
git show $COMMIT:cynic-python/benchmarks/judge_axiom_quality.py > cynic-python/benchmarks/judge_axiom_quality.py
```

### Step 3: Identify Consumer

Before committing, identify WHERE this code will run:
- Systemd service/timer? (document in [Service] section)
- Imported by kernel? (document the REST endpoint)
- Called from a script? (document the call chain)
- Test framework? (document the test file)

**If no consumer can be identified, STOP. Re-archive.**

### Step 4: Update Docstring

Add Tier classification:

```python
"""
Tier 2 INFRASTRUCTURE: Measure convergence of inference Dogs.

Phase: Periodic calibration (monthly)
Consumer: cynic-calibration.timer calls this
Systemd: hermes-calibration.service [to be created]
Promotion date: 2026-05-XX (from archive)
Last successful run: [your date]

Metrics: Reports convergence delta to /health.dog_convergence_<dog_name>
Error handling: Logs failures to journal, continues
"""
```

### Step 5: Wire to Systemd (If Applicable)

Create the service/timer:

```ini
# ~/.config/systemd/user/hermes-calibration.service
[Unit]
Description=CYNIC Hermes Calibration — Measure Dog convergence
After=cynic-kernel.service

[Service]
Type=oneshot
WorkingDirectory=/home/user/Bureau/CYNIC
ExecStart=/usr/bin/python3 cynic-python/benchmarks/judge_axiom_quality.py
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

```ini
# ~/.config/systemd/user/hermes-calibration.timer
[Unit]
Description=CYNIC Hermes Calibration — Monthly Dog convergence check

[Timer]
OnCalendar=monthly
OnBootSec=30min
Persistent=true

[Install]
WantedBy=timers.target
```

Then:
```bash
systemctl --user daemon-reload
systemctl --user enable hermes-calibration.timer
systemctl --user start hermes-calibration.timer
systemctl --user status hermes-calibration.timer
```

### Step 6: Test

```bash
# Test the service
systemctl --user start hermes-calibration.service
journalctl --user -xeu hermes-calibration.service -n 50

# Verify output/metrics
curl -s http://<TAILSCALE_CORE>:3030/health | jq .dog_convergence
```

### Step 7: Commit

```bash
git add cynic-python/benchmarks/judge_axiom_quality.py ~/.config/systemd/user/hermes-calibration.*
git commit -m "feat(calibration): resurrect judge_axiom_quality, wire to monthly timer

- Restored from 2026-04-27 baseline measurement script
- Added Tier 2 INFRASTRUCTURE tag + consumer documentation
- Created hermes-calibration.timer (monthly) + hermes-calibration.service
- Metrics: /health.dog_convergence_{dog_name}
- Purpose: Monthly validation that Dogs haven't diverged

Closes: K15-convergence-monitoring"
```

---

## Archives

### benchmarks/ (9 research scripts, 50KB total)

**Key Finding**: Qwen3.5-9B generates reasoning tokens by default, routing them to `reasoning_content` field with empty `content`. Measured throughput: ~6.5-7.7 tok/s actual vs 35 tok/s expected. This drove kernel Dog model selection and context budget decisions.

Files:
- `judge_axiom_quality.py` — Measure Dog axiom consistency (Tier 1, effort 3h)
- `hardware_profiler.py` — Snapshot GPU/CPU/memory (Tier 1, effort 2h)
- `tuner.py` — Bottleneck analysis + prioritized recommendations (Tier 1, effort 3h)
- `convergence.py` — Before/after measurement (Tier 1, effort 2h)
- `analysis.py` — Trend detection framework (Tier 1, effort 2h)
- `judge_axiom_quality_local.py` — Local variant (Tier 1, effort 1h)
- `judge_via_kernel.py` — Test Dogs via kernel /judge (Tier 1, effort 2h)
- `convergence_local.py` — Local convergence (Tier 1, effort 1h)
- `apu_capacity_research.py` — APU capacity findings (Archive only, no code resurrection needed)

**How to Resurrect**: Create `cynic-benchmark.timer` that runs monthly, calls `judge_axiom_quality.py`, collects output into reports/.

### behavioral/ (1 salvageable, 1 archive)

**Key Finding**: LSTM trained on 538K human browser events. Model captures behavioral signature (typing speed, scroll patterns, click timing). Valuable for anomaly detection.

Files:
- `behavior_simulator.py` — LSTM wrapper, clean interface (Tier 1, effort 2d)
- `behavior_ml_train.py` — Training pipeline (Archive, findings baked into model.pt)

**How to Resurrect behavior_simulator.py**: Import from hermes-behavior.service as human simulation layer for behavioral routing.

### domains/ (4 salvageable, 18 archive)

**Key Finding**: TF-IDF + spectral clustering on 2,287 tweets yields 7 coherent clusters. Cluster 3 = Solana tokens, Cluster 6 = LLM inference, Bridge signals (12% of corpus) represent highest-value cross-domain synthesis.

Also: Human HomeTimeline clicks (D2 Inference + D5 Sovereignty dominant) differ systematically from organ-x SearchTimeline farming (D1 Tokens over-mined). This inversion drove behavioral routing decisions.

Salvageable:
- `domain_verdict_builder.py` — Build domain verdicts (Tier 1, effort 1d)
- `domain_iterator.py` — Domain loop protocol (Tier 1, effort 1h)
- `curation_domain_classifier.py` — Keyword heuristic D1-D6 (Tier 1, effort 1h)
- `k15-routing/v1/domain_router_v1.py` — Domain-aware routing (Tier 1, effort 1d, has tests)

Archive:
- `archive/` — 3 versions of clustering experiments (v0, v1, v2)
- `behavioral-analysis/v1/` — 5 behavioral click mining scripts
- `domain-discovery/` — 8 TF-IDF + bridge detection scripts
- `k15-routing/v0/` — Earlier routing attempts

**How to Resurrect domain_router_v1.py**: Wire as middleware in k15_observation_consumer.py (before task dispatch) to add domain-aware routing.

### inference_organ/ (6 salvageable, 17 archive)

**Key Finding**: Python reimplementation of what cynic-kernel (Rust) already does. Framework is well-architected but superseded. ML findings from measure_track2.py are valuable (v2 heuristics Δq > 0.05 on 2,271-tweet dataset).

Salvageable:
- `inference_organ.py` — Core routing logic (Tier 1, effort TBD)
- `backends.py` — Dog backend definitions (Tier 1, effort TBD)
- `dogs.py` — Dog configuration (Tier 1, effort TBD)
- `monitoring.py` — SLA enforcement (Tier 1, effort TBD)
- `recovery.py` — Self-healing logic (Tier 1, effort TBD)
- `app/runner.py` — Run loop (Tier 1, effort TBD)

Archive:
- `measure_baseline.py`, `measure_track2.py` — v1 vs v2 heuristic A/B results
- `validate_organism.py` — Organism state validation
- `adapters/`, `domain/`, `profiles/` — Hexagonal layer (8 files)

**How to Resurrect**: Only if kernel needs Python fallback (unlikely). Otherwise keep archived.

### kenosis/ (1 archive with valuable finding)

**Key Finding**: Wu-Wei / KENOSIS pattern discovered empirically in Gemini CLI decision logs. The 7th axiom (non-action, strategic silence) was NOT explicitly designed—it emerged from session data. This is epistemically significant for the organism's self-awareness model.

Files:
- `kenosis_mining.py` — Sessions log miner (Archive with findings in KENOSIS_FINDINGS.md)

**How to Resurrect**: Only for research continuity. Code itself has limited production use, but FINDINGS are foundational.

### heuristics/ (21 wired, 15 salvageable, 1 archive)

**Key Finding**: Heuristics are CORE infrastructure (all K15 consumers depend on twitter_dog + twitter_signal_extractor). Measurement scripts revealed critical divergence: Domain-naive Dogs vs domain-aware routing produced different verdicts on 28 token test set.

Salvageable:
- `measure_token_calibration.py` — Token heuristic calibration (Tier 1, effort 2h)
- `measure_conviction_only.py` — Simple baseline (Tier 1, effort 1h)
- `measure_multi_modal.py` — Fusion measurement (Tier 1, effort 2h)
- `measure_twitter_calibration.py` — Twitter heuristic calibration (Tier 1, effort 2h)
- And 11 others (see audit report)

Archive:
- `create_mock_dataset.py` — Mock data only (no production use)

**How to Resurrect measurement scripts**: Wire to monthly `cynic-heuristic-calibration.timer` to continuously validate Dogs haven't diverged.

### validation/ (0 wired, 7 salvageable, 4 archive)

**Key Finding**: Phase 2 hypothesis test (wallet authenticity filtering) produced measurable results. These validation scripts are the methodology backbone for corpus building.

Salvageable:
- `phase2_human_filtering_measurement.py` — Phase 2 protocol (Tier 1, effort 1d)
- `wallet_corpus_builder.py` — Corpus collection (Tier 1, effort 2h)
- `real_wallet_corpus_builder.py` — Multi-source builder (Tier 1, effort 2h)
- And 4 others

Archive:
- `kenosis_mining.py` — Moved to kenosis/
- Spectrum files (data-only, 3 files)

**How to Resurrect phase2_human_filtering_measurement.py**: Wire to monthly measurement cron to continuously track whether filtering criteria remain effective.

---

## Tier 1 → Tier 2 Promotion Checklist

When resurrecting, apply this checklist:

- [ ] Code has explicit docstring (Tier 2 tag + consumer documented)
- [ ] Consumer identified (systemd service, kernel endpoint, or test)
- [ ] Systemd unit written (if applicable)
- [ ] Error handling documented (what happens on failure?)
- [ ] Metrics defined (/health endpoint or log output)
- [ ] Test: Run K15 falsification (consumer actually triggers?)
- [ ] No hardcoded paths (use env vars or repo-relative)
- [ ] Committed to git with clear commit message

If any checkbox fails: do NOT resurrect. Re-archive.

---

## Cleanup Checklist (For Future Sessions)

After 30 days, if code was NOT resurrected:
- [ ] Confirm: no consumer identified?
- [ ] Confirm: no systemd service created?
- [ ] Final commit: "trash(burn): delete [module], 30 days archived without promotion"
- [ ] Remove from docs/research/ OR keep findings only?

---

## Questions Before Resurrecting

**Q: This code looks good, why was it archived?**
A: Good code without a consumer is still dead code. It's a liability: future sessions waste tokens understanding what it is and why it doesn't run. Archive protects the codebase from rot while preserving the option to resurrect.

**Q: Should I resurrect everything?**
A: No. Resurrect only:
1. Code with clear consumer identified (systemd service, kernel integration, test)
2. Code you can wire within 1-2 days
3. Code whose findings directly inform current work

**Q: What if I resurrect and then abandon it again?**
A: Document in the commit why. Example: "Resurrected for Phase 2, abandoned when Phase 3 made it unnecessary." Then re-archive with updated notes.

**Q: The code was last edited 60 days ago. Is it stale?**
A: Yes. Before resurrecting, verify:
- Dependencies haven't changed (import paths, APIs)
- No hardcoded paths to moved files
- Tests still pass
- Interface matches where it would be called

If major changes needed: treat as a rewrite, not resurrection.

---

## Contact

Questions about archives? Check:
- `.claude/plans/burn-audit-cynic-python.md` — Full audit results
- `.cortex/rules/python-lifecycle.md` — Tier system + governance
- `.cortex/memory/` — Project memory entries on specific modules
