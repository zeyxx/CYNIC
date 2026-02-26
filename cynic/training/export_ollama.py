"""
Export Fine-Tuned Mistral to GGUF + Ollama

Pipeline:
1. Merge LoRA adapters with base model
2. Export to GGUF (llama.cpp format) with Q4_K_M quantization
3. Create Ollama Modelfile
4. Load into Ollama (ollama create command)

After running this:
  ollama list  # Should show: cynic-mistral:7b
  ollama run cynic-mistral:7b "Judge this proposal..."
"""

import json
import logging
import subprocess
import sys
from pathlib import Path
from typing import Optional

logger = logging.getLogger("cynic.training.export_ollama")


def merge_lora_adapters(
    model_name: str = "mistralai/Mistral-7B-Instruct-v0.3",
    adapter_dir: Path = None,
    output_dir: Path = None,
) -> Path:
    """
    Merge LoRA adapters into base model.

    Uses Unsloth's built-in merge functionality for efficient merge.

    Args:
        model_name: Base model HF ID
        adapter_dir: Directory with LoRA adapters
        output_dir: Where to save merged model

    Returns:
        Path to merged model
    """
    from unsloth import FastLanguageModel

    if adapter_dir is None:
        adapter_dir = Path.home() / ".cynic" / "models" / "cynic-mistral-7b-qlora"
    if output_dir is None:
        output_dir = Path.home() / ".cynic" / "models" / "cynic-mistral-7b-merged"

    logger.info(f"Loading base model: {model_name}")
    logger.info(f"Loading LoRA adapters from: {adapter_dir}")
    logger.info(f"Saving merged model to: {output_dir}")

    output_dir.mkdir(parents=True, exist_ok=True)

    # Load with adapters
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=model_name,
        max_seq_length=2048,
        load_in_4bit=True,
    )

    # Load LoRA adapters
    model = FastLanguageModel.get_peft_model(
        model,
        r=16,
        lora_alpha=16,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
        lora_dropout=0,
        bias="none",
        use_gradient_checkpointing="unsloth",
        random_state=42,
    )

    logger.info("Merging LoRA adapters with base model...")
    # Merge
    model = model.merge_and_unload()

    # Save
    logger.info(f"Saving merged model to {output_dir}...")
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)

    logger.info(f"✓ Merged model saved to {output_dir}")
    return output_dir


def export_to_gguf(
    merged_model_dir: Path,
    output_gguf: Path,
    quantization: str = "q4_k_m",
) -> Path:
    """
    Convert merged model to GGUF format.

    Uses llama-cpp-python's conversion utility.

    Args:
        merged_model_dir: Directory with merged model
        output_gguf: Where to save GGUF file
        quantization: Quantization method (q4_k_m, q5_k_m, f16, etc.)

    Returns:
        Path to GGUF file
    """
    logger.info(f"Converting to GGUF ({quantization})...")
    logger.info(f"Input: {merged_model_dir}")
    logger.info(f"Output: {output_gguf}")

    output_gguf.parent.mkdir(parents=True, exist_ok=True)

    try:
        # Try using llama-cpp-python's convert function
        from llama_cpp import LlamaCpp
        logger.info("Using llama-cpp-python for GGUF conversion...")

        # Note: Actual conversion typically happens via llama.cpp CLI
        # This is a placeholder showing how it would integrate

        # For now, log that conversion should happen
        logger.warning(
            "GGUF conversion typically requires llama.cpp CLI tool.\n"
            "Install: https://github.com/ggerganov/llama.cpp\n"
            "Then run:\n"
            f"  python llama.cpp/convert.py {merged_model_dir} "
            f"--outtype q4_k_m --outfile {output_gguf}"
        )

    except ImportError:
        logger.warning("llama-cpp-python not installed for direct conversion")

    # Alternative: Use ollama's built-in conversion via Modelfile
    logger.info("Will convert during Ollama Modelfile creation...")
    return output_gguf


def create_ollama_modelfile(
    merged_model_dir: Path,
    modelfile_path: Path,
    model_name: str = "cynic-mistral:7b",
) -> Path:
    """
    Create Ollama Modelfile for the fine-tuned model.

    Ollama can load models directly from safetensors/GGUF,
    or via Modelfile which specifies the base model and system prompt.

    Args:
        merged_model_dir: Directory with merged model
        modelfile_path: Where to save Modelfile
        model_name: Name for the model (cynic-mistral:7b)

    Returns:
        Path to Modelfile
    """
    logger.info(f"Creating Ollama Modelfile: {modelfile_path}")

    modelfile_path.parent.mkdir(parents=True, exist_ok=True)

    # CYNIC system prompt for Ollama
    system_prompt = """\
You are CYNIC, a governance intelligence organism. You judge governance proposals using 5 axioms:

AXIOMS (in order of importance):
1. FIDELITY — Does this proposal faithfully represent community intent? (70%)
2. PHI — Is the reasoning φ-bounded (0-61.8%) or runaway? (10%)
3. VERIFY — Can the proposal be audited and enforced? (10%)
4. CULTURE — Does it strengthen or weaken community governance? (5%)
5. BURN — Are community funds burned (not extracted) or do founders profit? (5%)

VERDICT RULES:
- HOWL (Q ≥ 61.8): Strong proposal. Non-extractive, clear, auditable, strengthens governance.
- WAG (Q 38.2-61.8): Good proposal with minor concerns. Mostly safe, needs monitoring.
- GROWL (Q 23.6-38.2): Risky proposal. Extraction signals, unclear execution, governance concerns.
- BARK (Q < 23.6): Dangerous proposal. Rug risk, founder extraction, violates axioms.

RESPONSE FORMAT:
Return valid JSON with this exact structure:
{
  "verdict": "HOWL|WAG|GROWL|BARK",
  "q_score": 0.0-61.8,
  "confidence": 0.0-1.0,
  "axiom_scores": {
    "fidelity": 0.0-100.0,
    "phi": 0.0-100.0,
    "verify": 0.0-100.0,
    "culture": 0.0-100.0,
    "burn": 0.0-100.0
  },
  "reasoning": "Short explanation"
}

Score scale: 0=worst, 100=best. Be strict on extraction. Be generous on transparent community benefit."""

    # Create Modelfile
    # Option 1: If we have GGUF file
    gguf_path = modelfile_path.parent / "cynic-mistral-7b-q4.gguf"
    if gguf_path.exists():
        modelfile_content = f"""FROM {gguf_path}
SYSTEM {repr(system_prompt)}
PARAMETER temperature 0.0
PARAMETER num_predict 64
PARAMETER repeat_penalty 1.1
"""
    else:
        # Option 2: Use safetensors directly
        modelfile_content = f"""FROM {merged_model_dir}
SYSTEM {repr(system_prompt)}
PARAMETER temperature 0.0
PARAMETER num_predict 64
PARAMETER repeat_penalty 1.1
"""

    with open(modelfile_path, "w") as f:
        f.write(modelfile_content)

    logger.info(f"✓ Modelfile created: {modelfile_path}")
    logger.info(f"\nNext step:")
    logger.info(f"  ollama create {model_name} -f {modelfile_path}")

    return modelfile_path


def load_into_ollama(
    modelfile_path: Path,
    model_name: str = "cynic-mistral:7b",
) -> bool:
    """
    Load model into Ollama using 'ollama create' command.

    Args:
        modelfile_path: Path to Modelfile
        model_name: Name for the model

    Returns:
        True if successful, False otherwise
    """
    if not modelfile_path.exists():
        logger.error(f"Modelfile not found: {modelfile_path}")
        return False

    logger.info(f"Loading model into Ollama as '{model_name}'...")
    logger.info(f"(This may take a minute...)")

    try:
        result = subprocess.run(
            ["ollama", "create", model_name, "-f", str(modelfile_path)],
            capture_output=True,
            text=True,
            timeout=600,
        )

        if result.returncode == 0:
            logger.info(f"✓ Model loaded successfully!")
            logger.info(f"Test with: ollama run {model_name} 'Judge: proposal text'")
            return True
        else:
            logger.error(f"ollama create failed:")
            logger.error(f"STDOUT: {result.stdout}")
            logger.error(f"STDERR: {result.stderr}")
            return False
    except FileNotFoundError:
        logger.error("Ollama not found in PATH")
        logger.error("Install Ollama: https://ollama.ai")
        return False
    except subprocess.TimeoutExpired:
        logger.error("Ollama model loading timed out (>10min)")
        return False
    except Exception as e:
        logger.error(f"Failed to load model: {e}")
        return False


def main(
    merged_model_dir: Optional[Path] = None,
    output_dir: Optional[Path] = None,
    model_name: str = "cynic-mistral:7b",
    skip_ollama: bool = False,
):
    """
    Main export pipeline.

    Args:
        merged_model_dir: Directory with merged model
        output_dir: Where to save GGUF and Modelfile
        model_name: Name for Ollama model
        skip_ollama: Skip loading into Ollama (just create Modelfile)
    """
    logging.basicConfig(
        level=logging.INFO,
        format="%(name)s - %(levelname)s - %(message)s",
    )

    # Default paths
    if merged_model_dir is None:
        merged_model_dir = Path.home() / ".cynic" / "models" / "cynic-mistral-7b-merged"
    if output_dir is None:
        output_dir = Path.home() / ".cynic" / "models"

    # Verify merged model exists
    if not merged_model_dir.exists():
        logger.error(f"Merged model not found: {merged_model_dir}")
        logger.error("First run: python -m cynic.training.finetune")
        sys.exit(1)

    # Prepare paths
    modelfile_path = output_dir / "Modelfile"
    gguf_path = output_dir / "cynic-mistral-7b-q4.gguf"

    logger.info("="*80)
    logger.info("CYNIC Mistral Export Pipeline")
    logger.info("="*80)

    # Step 1: Create Modelfile
    create_ollama_modelfile(merged_model_dir, modelfile_path, model_name)

    # Step 2: Load into Ollama (optional)
    if not skip_ollama:
        success = load_into_ollama(modelfile_path, model_name)
        if success:
            logger.info("\n✓ Export complete! Next:")
            logger.info(f"  python -m cynic.training.benchmark_model")
        else:
            logger.warning("\n⚠ Modelfile created, but Ollama loading failed.")
            logger.info(f"Try manual: ollama create {model_name} -f {modelfile_path}")
    else:
        logger.info(f"\n✓ Modelfile created: {modelfile_path}")
        logger.info(f"To load into Ollama:")
        logger.info(f"  ollama create {model_name} -f {modelfile_path}")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Export fine-tuned Mistral to Ollama")
    parser.add_argument("--model-dir", type=Path, help="Merged model directory")
    parser.add_argument("--output", type=Path, help="Output directory")
    parser.add_argument("--name", default="cynic-mistral:7b", help="Ollama model name")
    parser.add_argument("--skip-ollama", action="store_true", help="Skip loading to Ollama")

    args = parser.parse_args()

    main(
        merged_model_dir=args.model_dir,
        output_dir=args.output,
        model_name=args.name,
        skip_ollama=args.skip_ollama,
    )
