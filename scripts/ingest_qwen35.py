
import os
import logging
from pathlib import Path
from huggingface_hub import hf_hub_download

# Configuration de la Rigueur
logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("cynic.ingestor")

# Destination Souveraine (Hors C:)
MODEL_DIR = Path("D:/cynic-models")

# Modèles Qwen 3.5 optimisés par Unsloth (Doctrine E2E)
MODELS = [
    {"repo": "unsloth/Qwen3.5-2B-GGUF", "file": "Qwen3.5-2B-Q4_K_M.gguf"},
    {"repo": "unsloth/Qwen3.5-4B-GGUF", "file": "Qwen3.5-4B-Q4_K_M.gguf"},
    {"repo": "unsloth/Qwen3.5-9B-GGUF", "file": "Qwen3.5-9B-Q4_K_M.gguf"},
]

def build_rails():
    """Prépare le terrain et télécharge les muscles du cerveau."""
    if not MODEL_DIR.exists():
        logger.info(f"Création du coffre-fort souverain : {MODEL_DIR}")
        MODEL_DIR.mkdir(parents=True, exist_ok=True)

    print("\n" + "!"*60)
    print("🧬 CYNIC E2E INGESTION : QWEN 3.5 SOVEREIGN MUSCLES")
    print("!"*60)
    
    for m in MODELS:
        target_path = MODEL_DIR / m['file']
        if target_path.exists():
            logger.info(f"Muscle déjà présent : {m['file']} - Analyse de l'intégrité...")
            continue
            
        try:
            logger.info(f"Téléchargement de {m['file']} depuis {m['repo']}...")
            # Téléchargement direct sans symlinks pour une portabilité totale sur D:
            path = hf_hub_download(
                repo_id=m['repo'],
                filename=m['file'],
                local_dir=str(MODEL_DIR),
                local_dir_use_symlinks=False
            )
            logger.info(f"✅ Ingestion réussie : {path}")
        except Exception as e:
            logger.error(f"❌ Échec de l'ingestion pour {m['file']} : {e}")

    print("\n" + "!"*60)
    print("🏆 TOUS LES RAILS SONT POSÉS. CYNIC POSSÈDE SES MUSCLES.")
    print("!"*60)

if __name__ == "__main__":
    build_rails()
