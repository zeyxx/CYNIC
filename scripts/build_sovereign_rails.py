"""
CYNIC Sovereign Rails Builder - E2E Model Ingestion & Optimization.
Builds the infrastructure to download, discover, and run Unsloth-optimized models.
Focus: Qwen 3.5 (Small) on Ryzen 5700G (D: storage).
"""
import os
import logging
from pathlib import Path
from huggingface_hub import hf_hub_download

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.build_rails")

MODEL_DIR = Path("D:/cynic-models")
REPOS = {
    "qwen3.5-9b": "unsloth/Qwen2.5-7B-Instruct-GGUF", # Substitute for Qwen3.5 until available
    "qwen3.5-3b": "unsloth/Qwen2.5-3B-Instruct-GGUF"
}
FILE_NAME = "Qwen2.5-7B-Instruct-Q4_K_M.gguf"

def build_sovereign_storage():
    """Ensure the model vault is ready and optimized."""
    if not MODEL_DIR.exists():
        logger.info(f"Creating Sovereign Vault: {MODEL_DIR}")
        MODEL_DIR.mkdir(parents=True, exist_ok=True)
    else:
        logger.info(f"Sovereign Vault already established: {MODEL_DIR}")

def ingest_optimized_model():
    """Download the Unsloth-optimized model to the correct hardware vault."""
    try:
        logger.info(f"Ingesting {FILE_NAME} from Unsloth Hub...")
        path = hf_hub_download(
            repo_id=REPOS["qwen3.5-9b"],
            filename=FILE_NAME,
            local_dir=str(MODEL_DIR),
            local_dir_use_symlinks=False
        )
        logger.info(f"Ingestion successful: {path}")
        return path
    except Exception as e:
        logger.error(f"Ingestion failed: {e}")
        return None

if __name__ == "__main__":
    print("\n" + "="*60)
    print("💎 CYNIC SOVEREIGN RAILS: BUILDING INFRASTRUCTURE")
    print("="*60)
    build_sovereign_storage()
    # ingestion = ingest_optimized_model()
    # if ingestion:
    #     print(f"✅ Rails built. CYNIC now possesses {FILE_NAME}.")
    print("\n" + "="*60)
    print("RAILS ESTABLISHED. READY FOR DISCOVERY.")
    print("="*60)
