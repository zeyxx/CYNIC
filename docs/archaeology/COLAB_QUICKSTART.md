# Google Colab Fine-Tuning — Quick Start Guide

**Phase 2: Train Mistral 7B on Google Colab (Free T4 GPU)**

---

## 1. Open the Notebook

Click to open in Google Colab:
[CYNIC_Mistral_Finetune_Colab.ipynb](CYNIC_Mistral_Finetune_Colab.ipynb)

Or manually:
1. Go to https://colab.research.google.com
2. Click **File → Open notebook**
3. Go to **GitHub** tab
4. Paste repo URL (if uploaded to GitHub), or manually upload the `.ipynb` file

---

## 2. Enable GPU

**CRITICAL**: Ensure you have a GPU selected.

1. Click **Runtime** (top menu)
2. Click **Change runtime type**
3. Set **Hardware accelerator** to **T4 GPU**
4. Click **Save**

---

## 3. Run Cells in Order

Execute each cell from top to bottom:

### Cell 1: Check GPU ✓
```python
# Verifies GPU is available (should show T4)
```

### Cell 2: Install Dependencies ✓
```python
# Installs unsloth, transformers, datasets, trl (takes ~2 min)
```

### Cell 3: Upload Training Data ✓
**Choose ONE option:**

**Option A** (Recommended): Upload `governance_v1.jsonl`
- You already generated this locally
- File location: `~/.cynic/training/governance_v1.jsonl`
- Click upload button in Cell 3

**Option B**: Generate in Colab
- Run the commented code in Cell 3B
- Creates 14 synthetic examples automatically

### Cell 4: Load Model ✓
```python
# Loads Mistral 7B (4-bit quantized)
# Takes 1-2 minutes
```

### Cell 5: Prepare Dataset ✓
```python
# Tokenizes training data
# Prepares for training
```

### Cell 6: Train Model 🚀
```python
# Main training loop: 3 epochs, ~1.5 hours
# You can watch progress in real-time
# Training metrics logged every 5 steps
```

### Cell 7: Verify Results ✓
```python
# Checks that training completed successfully
# Lists LoRA adapter files (should be ~48MB total)
```

### Cell 8: Download ✓
```python
# Creates ZIP of LoRA adapters
# Downloads to your machine
# You'll use this in Phase 3
```

---

## 4. After Training Completes

### You'll have:
- LoRA adapters (`adapter_config.json`, `adapter_model.bin`, etc.)
- Training artifacts
- ZIP ready to download

### Next Steps (Phase 3 — Local Machine):

```bash
# 1. Extract the ZIP file locally
# $ unzip cynic-mistral-7b-qlora.zip

# 2. Copy LoRA adapters to ~/.cynic/models/
# $ mkdir -p ~/.cynic/models/cynic-mistral-7b-qlora
# $ cp -r cynic-mistral-7b-qlora/* ~/.cynic/models/cynic-mistral-7b-qlora/

# 3. Run Phase 3 export
# $ python -m cynic.training.export_ollama \
#     --model-dir ~/.cynic/models/cynic-mistral-7b-qlora

# 4. Verify in Ollama
# $ ollama list | grep cynic-mistral

# 5. Run Phase 4 benchmark
# $ python -m cynic.training.benchmark_model
```

---

## Troubleshooting

### "No GPU detected"
- **Fix**: Go to Runtime → Change runtime type → T4 GPU
- Restart runtime after changing

### "Out of memory"
- **Normal on first epoch** — GPU memory usage increases as training progresses
- If it crashes, reduce batch size in Cell 6:
  ```python
  per_device_train_batch_size=1,  # Was 2
  ```

### "Module not found" errors
- Make sure **Cell 2** completed successfully
- Wait for `✓ Installation complete!` message
- Don't skip Cell 2

### Training too slow
- Normal on T4 (slower than RTX 4090)
- 3 epochs takes ~1.5 hours on T4
- Can't speed this up without premium GPU

### Can't upload file in Cell 3
- Make sure you're in Google Colab (not Jupyter locally)
- File upload only works in Colab
- Alternatively, use **Option B** to generate in Colab

---

## Monitoring Training

**During Cell 6 (training):**
- Watch for training loss to decrease
- Normal values: loss drops from ~2.5 → ~0.5 over 3 epochs
- Logs appear every 5 steps
- Total time: ~1.5 hours

**Example output:**
```
Steps: 10%|██        | 5/50 [00:45<07:15, ...]
loss = 1.234
Steps: 20%|████      | 10/50 [01:30<06:30, ...]
loss = 0.876
...
Training completed!
```

---

## Estimated Timeline

| Step | Time | What Happens |
|------|------|---|
| 1. Setup GPU | 1 min | Enable T4 GPU |
| 2. Install deps | 2 min | Download + install packages |
| 3. Upload data | 1 min | Upload governance_v1.jsonl (or generate) |
| 4. Load model | 2 min | Download Mistral 7B |
| 5. Tokenize | 1 min | Prepare dataset |
| 6. **Train** | **90 min** | 3 epochs on governance proposals |
| 7. Verify | 1 min | Check results |
| 8. Download | 2 min | Download ZIP of adapters |
| **Total** | **~100 min** | **~1.5 hours** |

---

## Cost

**Free** (within Colab free tier limits):
- T4 GPU access: unlimited per notebook
- Training time: unlimited
- Storage: limited to Colab session

**After training:**
- Download adapters (~48MB)
- Delete notebook (free up space)
- Use adapters locally (no ongoing cost)

---

## Next: Phase 3 (Local Machine)

Once you download the LoRA adapters:

```bash
# Export to Ollama
python -m cynic.training.export_ollama

# Benchmark
python -m cynic.training.benchmark_model

# Integrate with CYNIC
# (Automatic — LLMRegistry will discover it)
```

See `MISTRAL_FINETUNE_PLAN.md` for complete details.

---

## Questions?

- **About Colab**: https://colab.research.google.com/notebooks/intro.ipynb
- **About Unsloth**: https://github.com/unslothai/unsloth
- **About Mistral**: https://mistral.ai
- **CYNIC**: See `cynic/training/README.md`
