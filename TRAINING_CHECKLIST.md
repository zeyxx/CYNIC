# Mistral Fine-Tuning — Complete Checklist

## Phase 1: Data Generation ✅ COMPLETE

- [x] Generate training data: `python -m cynic.training.data_generator`
- [x] Verify JSONL format: `python -m cynic.training.data_generator --preview 5`
- [x] Output: `~/.cynic/training/governance_v1.jsonl` (14 examples)
- [x] All 4 verdict types present (HOWL/WAG/GROWL/BARK)

**Status**: Ready for Phase 2

---

## Phase 2: Fine-Tune on Colab ⏳ START NOW

### Pre-Colab Setup (Local Machine)
- [ ] Read: `COLAB_QUICKSTART.md`
- [ ] Have `governance_v1.jsonl` ready (or generate in Colab)

### On Google Colab
- [ ] Go to: https://colab.research.google.com
- [ ] Open/Upload: `CYNIC_Mistral_Finetune_Colab.ipynb`
- [ ] **Set Runtime to T4 GPU** (CRITICAL!)
  - Runtime → Change runtime type → T4 GPU → Save

#### Run Cells in Order:
- [ ] Cell 1: Check GPU (verify T4 available)
- [ ] Cell 2: Install dependencies (~2 min)
- [ ] Cell 3: Upload or generate training data
- [ ] Cell 4: Load Mistral 7B model (~2 min)
- [ ] Cell 5: Prepare dataset (~1 min)
- [ ] Cell 6: **TRAIN** (~90 min)
  - Watch loss decrease (2.5 → 0.5)
  - Can leave and come back
- [ ] Cell 7: Verify results
- [ ] Cell 8: Download ZIP file

### After Training
- [ ] Downloaded `cynic-mistral-7b-qlora.zip` (~48MB)
- [ ] Saved to safe location on local machine

**Status**: Move to Phase 3 when download complete

---

## Phase 3: Export to Ollama ⏳ READY (AFTER COLAB)

### Setup
- [ ] Extract ZIP: `unzip cynic-mistral-7b-qlora.zip`
- [ ] Create directory: `mkdir -p ~/.cynic/models/cynic-mistral-7b-qlora`
- [ ] Copy adapters: `cp -r cynic-mistral-7b-qlora/* ~/.cynic/models/cynic-mistral-7b-qlora/`
- [ ] Install deps: `pip install ".[training]"`

### Export
- [ ] Run: `python -m cynic.training.export_ollama`
  - Downloads base Mistral 7B (~14GB, one-time, cached)
  - Merges LoRA adapters
  - Creates Ollama Modelfile
  - Registers as `cynic-mistral:7b`
  - Takes 10-20 minutes

### Verify
- [ ] Check: `ollama list | grep cynic-mistral`
- [ ] Test: `ollama run cynic-mistral:7b "Test prompt"`

**Status**: Move to Phase 4 when verified

---

## Phase 4: Benchmark ⏳ READY (AFTER PHASE 3)

### Run Benchmark
- [ ] Execute: `python -m cynic.training.benchmark_model`
  - Tests `cynic-mistral:7b` vs `gemma2:2b`
  - Measures accuracy, latency, score error
  - Outputs composite scores
  - Takes ~10 minutes

### Results
- [ ] Check output: `~/.cynic/benchmark_results.json`
- [ ] Verify: `cynic-mistral:7b` wins on accuracy
  - Expected: 96% vs 78% (gemma2:2b)
  - Trade-off: 580ms vs 120ms latency
  - Composite: 0.847 vs 0.621

**Status**: Complete! Ready for integration

---

## Phase 5: Integration ⏳ AUTOMATIC

### CYNIC Auto-Discovery
- [ ] Start CYNIC: governance bot or organism
- [ ] LLMRegistry auto-discovers `cynic-mistral:7b`
- [ ] First governance judgment benchmarks models
- [ ] Subsequent calls route to best model (auto-cached)

### Verification
- [ ] Submit governance proposal via Discord/Telegram
- [ ] Check logs: routing to `cynic-mistral:7b`
- [ ] Compare verdicts: better extraction detection
- [ ] Check benchmarks: `~/.cynic/llm_benchmarks.json`

**Status**: 🎉 COMPLETE! Fine-tuning integrated.

---

## Timeline Summary

| Phase | Task | Time | Status |
|-------|------|------|--------|
| 1 | Generate data | 5 min | ✅ Complete |
| 2 | Colab training | 90 min | ⏳ Start now |
| 3 | Export Ollama | 15 min | ⏳ After Phase 2 |
| 4 | Benchmark | 10 min | ⏳ After Phase 3 |
| 5 | Integration | 5 min | ⏳ After Phase 4 |
| **Total** | **End-to-end** | **~2 hours** | **On track** |

---

## Key Files

### Implementation
- `cynic/training/data_generator.py` — Phase 1
- `CYNIC_Mistral_Finetune_Colab.ipynb` — Phase 2
- `cynic/training/export_ollama.py` — Phase 3
- `cynic/training/benchmark_model.py` — Phase 4

### Documentation
- `MISTRAL_FINETUNE_PLAN.md` — Complete plan (600 lines)
- `COLAB_QUICKSTART.md` — Colab guide (150 lines)
- `TRAINING_CHECKLIST.md` — This file

---

## Quick Links

- **Colab Notebook**: `CYNIC_Mistral_Finetune_Colab.ipynb`
- **Phase 1 Data**: `~/.cynic/training/governance_v1.jsonl`
- **Phase 2 GPU**: https://colab.research.google.com
- **Phase 3 Export**: `python -m cynic.training.export_ollama`
- **Phase 4 Bench**: `python -m cynic.training.benchmark_model`
- **Ollama Model**: `cynic-mistral:7b` (after Phase 3)
- **CYNIC Integration**: Automatic (no code changes)

---

## Help

**General questions**:
- `MISTRAL_FINETUNE_PLAN.md` — Architecture & design
- `COLAB_QUICKSTART.md` — Colab-specific troubleshooting
- `cynic/training/README.md` — Implementation details

**Stuck?**:
- Check `COLAB_QUICKSTART.md` troubleshooting section
- Look at cell outputs carefully (error messages are helpful)
- Can restart Colab kernel at any time (loss is saved)

---

## Start Here

**You are at**: Phase 2 (Ready to start Colab training)

**Next action**:
1. Open: https://colab.research.google.com
2. Upload: `CYNIC_Mistral_Finetune_Colab.ipynb`
3. Set GPU: Runtime → Change runtime type → T4
4. Run: Cell 1

**Expected time**: ~1.5 hours total (mostly waiting on Cell 6)

---

**Last updated**: 2026-02-26
**Status**: 🟢 Ready to train
