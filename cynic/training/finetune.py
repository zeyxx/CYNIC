"""
Mistral 7B Fine-Tuning with Unsloth QLoRA

Fine-tunes Mistral 7B Instruct model for CYNIC governance judgment using:
- Unsloth: 4x faster than standard HF, uses 4-bit QLoRA
- LoRA rank=16: Good balance between quality and model size
- Target modules: All linear layers (q, k, v, o, gate, up, down)

Hardware requirements:
- GPU: 8GB+ VRAM (4-bit quantization uses ~8GB for 7B)
- Training time: ~1 hour on single GPU (3 epochs, 200-500 examples)

Output:
- LoRA adapters: ~/.cynic/models/cynic-mistral-7b-qlora/
- Ready for export_ollama.py to convert to GGUF
"""

import json
import logging
import sys
from pathlib import Path
from typing import Optional

import torch
from datasets import load_dataset
from transformers import TrainingArguments

logger = logging.getLogger("cynic.training.finetune")


def setup_unsloth():
    """Verify Unsloth is installed and importable."""
    try:
        from unsloth import FastLanguageModel
        from unsloth import is_bfloat16_supported
        logger.info("✓ Unsloth loaded successfully")
        return FastLanguageModel, is_bfloat16_supported
    except ImportError as e:
        logger.error(f"Unsloth not installed. Install with: pip install unsloth[colab-new]")
        raise


def load_model_and_tokenizer(
    model_name: str = "unsloth/mistral-7b-instruct-v0.3-bnb-4bit",
    max_seq_length: int = 2048,
):
    """
    Load Mistral 7B model and tokenizer with 4-bit quantization.

    Args:
        model_name: HuggingFace model ID (pre-quantized)
        max_seq_length: Max sequence length for training

    Returns:
        (model, tokenizer) ready for training
    """
    FastLanguageModel, is_bfloat16_supported = setup_unsloth()

    logger.info(f"Loading model: {model_name}")
    logger.info(f"Max sequence length: {max_seq_length}")
    logger.info(f"4-bit quantization enabled")

    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=model_name,
        max_seq_length=max_seq_length,
        dtype=None,  # Auto-detect
        load_in_4bit=True,
    )

    # Add LoRA adapters
    logger.info("Adding LoRA adapters (rank=16)...")
    model = FastLanguageModel.get_peft_model(
        model,
        r=16,  # LoRA rank
        lora_alpha=16,  # LoRA alpha (learning rate scale)
        lora_dropout=0,  # No dropout in training (this is fine)
        bias="none",
        use_gradient_checkpointing="unsloth",
        random_state=42,
        target_modules=[
            "q_proj", "k_proj", "v_proj", "o_proj",  # Attention
            "gate_proj", "up_proj", "down_proj",      # MLP
        ],
    )

    return model, tokenizer


def prepare_dataset(
    jsonl_path: Path,
    tokenizer,
    max_seq_length: int = 2048,
):
    """
    Load and prepare training dataset from JSONL file.

    Args:
        jsonl_path: Path to training JSONL file
        tokenizer: Tokenizer for encoding
        max_seq_length: Max tokens per example

    Returns:
        HuggingFace Dataset with tokenized examples
    """
    logger.info(f"Loading dataset from {jsonl_path}")

    # Load JSONL
    def load_jsonl(path):
        data = []
        with open(path) as f:
            for line in f:
                if line.strip():
                    data.append(json.loads(line))
        return data

    raw_data = load_jsonl(jsonl_path)
    logger.info(f"Loaded {len(raw_data)} examples from {jsonl_path}")

    # Convert to text format for training
    def format_messages(example):
        """Convert messages to text format."""
        messages = example.get("messages", [])
        text = ""

        for msg in messages:
            role = msg.get("role", "")
            content = msg.get("content", "")

            if role == "system":
                text += f"<s>[INST] <<SYS>>\n{content}\n<</SYS>>\n\n"
            elif role == "user":
                text += f"{content} [/INST] "
            elif role == "assistant":
                text += f"{content}</s>"

        return {"text": text}

    # Apply formatting
    formatted_data = [format_messages(ex) for ex in raw_data]

    # Create dataset
    from datasets import Dataset
    dataset = Dataset.from_list(formatted_data)

    # Tokenize
    def tokenize_func(example):
        return tokenizer(
            example["text"],
            max_length=max_seq_length,
            truncation=True,
            padding="max_length",
            return_tensors=None,
        )

    logger.info("Tokenizing dataset...")
    tokenized = dataset.map(
        tokenize_func,
        batched=True,
        remove_columns=["text"],
        desc="Tokenizing",
    )

    logger.info(f"Prepared {len(tokenized)} tokenized examples")
    return tokenized


def train(
    train_dataset,
    model,
    tokenizer,
    output_dir: Path,
    num_epochs: int = 3,
    batch_size: int = 2,
    gradient_accumulation_steps: int = 4,
    learning_rate: float = 2e-4,
):
    """
    Fine-tune model with LoRA.

    Args:
        train_dataset: HuggingFace Dataset
        model: Model with LoRA adapters
        tokenizer: Tokenizer
        output_dir: Where to save trained LoRA adapters
        num_epochs: Number of training epochs
        batch_size: Per-device batch size
        gradient_accumulation_steps: Gradient accumulation (effective batch = batch_size × accum)
        learning_rate: Learning rate
    """
    from transformers import DataCollatorForLanguageModeling
    from trl import SFTTrainer

    logger.info(f"Starting training for {num_epochs} epochs")
    logger.info(f"Batch size: {batch_size}, Gradient accumulation: {gradient_accumulation_steps}")
    logger.info(f"Learning rate: {learning_rate}")
    logger.info(f"Output directory: {output_dir}")

    output_dir.mkdir(parents=True, exist_ok=True)

    # Training arguments
    training_args = TrainingArguments(
        output_dir=str(output_dir),
        per_device_train_batch_size=batch_size,
        gradient_accumulation_steps=gradient_accumulation_steps,
        warmup_steps=10,
        num_train_epochs=num_epochs,
        learning_rate=learning_rate,
        fp16=not torch.cuda.is_available(),
        bf16=torch.cuda.is_available(),
        logging_steps=5,
        optim="paged_adamw_8bit",
        weight_decay=0.01,
        lr_scheduler_type="cosine",
        seed=42,
        save_strategy="epoch",
        save_total_limit=2,
    )

    # Trainer
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=train_dataset,
        dataset_text_field="text",
        args=training_args,
        packing=False,
        max_seq_length=2048,
    )

    # Train
    logger.info("🚀 Starting training...")
    trainer.train()

    logger.info(f"✓ Training complete! LoRA adapters saved to {output_dir}")
    return model, trainer


def main(
    jsonl_path: Optional[Path] = None,
    output_dir: Optional[Path] = None,
    num_epochs: int = 3,
    dry_run: bool = False,
):
    """
    Main training pipeline.

    Args:
        jsonl_path: Path to training JSONL (default: ~/.cynic/training/governance_v1.jsonl)
        output_dir: Where to save LoRA adapters (default: ~/.cynic/models/cynic-mistral-7b-qlora/)
        num_epochs: Number of training epochs
        dry_run: If True, just load data and model, don't train
    """
    # Setup logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(name)s - %(levelname)s - %(message)s",
    )

    # Default paths
    if jsonl_path is None:
        jsonl_path = Path.home() / ".cynic" / "training" / "governance_v1.jsonl"
    if output_dir is None:
        output_dir = Path.home() / ".cynic" / "models" / "cynic-mistral-7b-qlora"

    # Verify training data exists
    if not jsonl_path.exists():
        logger.error(f"Training data not found: {jsonl_path}")
        logger.error("First run: python -m cynic.training.data_generator")
        sys.exit(1)

    # Load model and tokenizer
    model, tokenizer = load_model_and_tokenizer()

    # Prepare dataset
    train_dataset = prepare_dataset(jsonl_path, tokenizer)

    # Check GPU availability
    if torch.cuda.is_available():
        logger.info(f"✓ GPU available: {torch.cuda.get_device_name(0)}")
        logger.info(f"  VRAM: {torch.cuda.get_device_properties(0).total_memory / 1e9:.1f} GB")
    else:
        logger.warning("⚠ No GPU detected. Training will be very slow on CPU.")

    if dry_run:
        logger.info("DRY RUN: Model and data loaded successfully")
        logger.info(f"Model: {model}")
        logger.info(f"Dataset size: {len(train_dataset)}")
        return

    # Train
    model, trainer = train(
        train_dataset,
        model,
        tokenizer,
        output_dir,
        num_epochs=num_epochs,
    )

    logger.info(f"✓ Fine-tuning complete!")
    logger.info(f"Next: python -m cynic.training.export_ollama --model-dir {output_dir}")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Fine-tune Mistral 7B for governance judgment")
    parser.add_argument("--data", type=Path, help="Training JSONL file")
    parser.add_argument("--output", type=Path, help="Output directory for LoRA adapters")
    parser.add_argument("--epochs", type=int, default=3, help="Number of epochs")
    parser.add_argument("--dry-run", action="store_true", help="Load but don't train")

    args = parser.parse_args()

    main(
        jsonl_path=args.data,
        output_dir=args.output,
        num_epochs=args.epochs,
        dry_run=args.dry_run,
    )
