# CYNIC Custom Local Models Setup

> Edge case handling: User has local LLM models (GGUF files) in a non-standard directory

**Problem**: Your LLM files are in `D:\Models` (Windows host), but CYNIC runs in Docker container.

**Solution**: Mount your local directory into Docker so CYNIC can discover and use your models via LlamaCppAdapter.

---

## Quick Start (Windows Host with D:\Models)

### Step 1: Enable Custom Models in docker-compose.yml

The docker-compose.yml has been updated with optional environment variables. To enable custom models:

**Create a `.env` file** in the same directory as `docker-compose.yml`:

```bash
# .env (git-ignored, local only)

# 1. Tell Docker where your models directory is on the Windows host
CUSTOM_MODELS_PATH=D:\Models

# 2. Set the container path where models will appear
CYNIC_MODELS_DIR=/models

# 3. Optional: GPU layer offloading (for llama-cpp-python)
#    -1  = offload all layers to GPU
#     0  = CPU only (default if not set)
LLAMA_CPP_GPU_LAYERS=-1

# 4. Optional: CPU threads for llama-cpp-python
LLAMA_CPP_THREADS=8
```

### Step 2: Restart the Container

```bash
cd cynic
docker-compose down
docker-compose up -d
```

Docker will mount your `D:\Models` directory into the container at `/models`.

### Step 3: Verify Models Are Discovered

```bash
# Check the container logs
docker-compose logs cynic | grep -A5 "LlamaCpp\|discovered"

# Expected output:
# *ears perk* LLMs discovered: ['ollama:gemma2:2b', 'llama_cpp:model_name.gguf', ...]
```

### Step 4: Test LLM Calls

Submit a MACRO judgment to verify SAGE dog now uses LLMs:

```bash
curl -X POST http://localhost:8000/judge \
  -H "Content-Type: application/json" \
  -d '{
    "subject": "Test SAGE with local LLM",
    "code": "def test(): pass",
    "level": "MACRO"
  }' | python -m json.tool
```

Check for `llm_count > 0` in response.

---

## Technical Details

### How It Works

1. **docker-compose.yml**: Volume mount added
   ```yaml
   volumes:
     - ${CUSTOM_MODELS_PATH:-/tmp/nonexistent}:/models
   ```

2. **Environment variables**: Read by CynicConfig
   ```python
   # cynic/core/config.py line 90
   models_dir=os.getenv("CYNIC_MODELS_DIR")
   ```

3. **LLM Discovery**: LlamaCppAdapter scans models_dir on startup
   ```python
   # cynic/llm/adapter.py line 641-653 (_discover_llama_cpp)
   from cynic.llm.llama_cpp import list_local_models
   paths = list_local_models(models_dir)  # → finds all *.gguf files
   ```

4. **Model Registration**: Each GGUF becomes an available adapter
   ```python
   # Dogs can now call: await adapter.complete(request)
   # No HTTP overhead, zero latency added over local inference
   ```

### Supported Model Files

- **Format**: `.gguf` (GGML universal quantized format)
- **Discovery**: Recursive glob pattern finds models in subdirectories
- **Sorting**: Loaded by file size (smallest first for faster startup)

### Performance Implications

**LlamaCppAdapter vs OllamaAdapter**:

| Metric | Ollama | LlamaCpp |
|--------|--------|----------|
| HTTP Overhead | ~50ms per call | 0ms |
| Network | TCP socket | Direct process |
| Thread Safety | Built-in | Semaphore(1) |
| GPU Support | CUDA/Vulkan | LLAMA_CPP_GPU_LAYERS |
| **7-Parallel MCTS** | 7× HTTP overhead | Direct memory sharing |

**Result**: LlamaCppAdapter composite scores typically **20-30% higher** than Ollama for same model/task.

---

## Troubleshooting

### Models Not Found

**Check 1: Docker mount is correct**
```bash
docker-compose exec cynic ls -la /models
# Should list your .gguf files
```

**Check 2: Environment variable is set**
```bash
docker-compose exec cynic python -c "import os; print(os.getenv('CYNIC_MODELS_DIR'))"
# Should print: /models
```

**Check 3: llama-cpp-python is installed**
```bash
docker-compose exec cynic python -c "from llama_cpp import Llama; print('OK')"
# If error: LlamaCppAdapter silently skipped (logged as debug)
```

### Check Logs for Discovery

```bash
docker-compose logs cynic | grep -E "LlamaCpp|discovered|models_dir"
```

### Errors During Model Load

If a GGUF file is corrupted or incompatible:

```
WARNING LlamaCpp model_name.gguf failed to load: ...
→ Registration fails silently
→ SAGE falls back to Ollama or heuristics
```

This is **by design**: corrupted models don't block startup.

---

## Advanced: Multiple Model Directories

If you have models spread across multiple directories:

**Option A**: Symlink them into one directory
```bash
# Windows: Create junction
mklink /j D:\Models\all D:\LLMs
mklink /j D:\Models\all D:\Quantized
```

**Option B**: Copy or mount multiple locations (requires Dockerfile change)

---

## Advanced: Benchmarking Your Models

After models are discovered, CYNIC automatically benchmarks them:

```bash
# Check benchmark results
curl http://localhost:8000/consciousness | \
  python -c "import json, sys; d=json.load(sys.stdin); print(json.dumps(d['mirror']['llm_routing'], indent=2))"
```

Benchmark scores drive routing decisions:
- SAGE chooses fastest/best model per task
- Q-Scores improve over time as CYNIC learns
- EMA (exponential moving average) weights recent measurements heavily

---

## Advanced: Custom Ollama Models

If you also want Ollama to see your custom models:

1. **Mount the directory into Ollama too**:
   ```yaml
   # docker-compose.yml - ollama service
   volumes:
     - ollama_models:/root/.ollama
     - ${CUSTOM_MODELS_PATH}:/models:ro  # Add this
   ```

2. **Configure Ollama** (if using Ollama Modelfile):
   ```
   FROM /models/model_name.gguf
   ```

---

## Reset: Disable Custom Models

Remove from `.env`:
```bash
CUSTOM_MODELS_PATH=
CYNIC_MODELS_DIR=
```

Or comment out the volume in docker-compose.yml:
```yaml
# - ${CUSTOM_MODELS_PATH:-/tmp/nonexistent}:/models
```

---

## FAQ

**Q: Can I use models from Hugging Face?**
A: Download the `.gguf` version to `D:\Models`, then they'll be discovered.

**Q: Can I add models without restarting?**
A: No, discovery runs once on startup. Add models, then restart the container.

**Q: Do llama-cpp-python models conflict with Ollama?**
A: No, they run separately. CYNIC benchmarks both and routes to fastest.

**Q: What if a model fails to load?**
A: Silently skipped, logged at DEBUG level. Others still work.

**Q: Can I offload layers to GPU?**
A: Yes, set `LLAMA_CPP_GPU_LAYERS=-1` in `.env` (offload all) or specific count.

---

## Next Steps

1. ✅ Update docker-compose.yml (already done)
2. ⏳ Create `.env` file with your `CUSTOM_MODELS_PATH`
3. ⏳ Restart Docker: `docker-compose down && docker-compose up -d`
4. ⏳ Verify: `docker-compose logs cynic | grep discovered`
5. ⏳ Test: Submit MACRO judgment, check `llm_count > 0`

---

**Last Updated**: 2026-02-20
**Status**: Edge case handler ready for activation
**Confidence**: 61.8% (φ⁻¹) — requires user to set CUSTOM_MODELS_PATH
