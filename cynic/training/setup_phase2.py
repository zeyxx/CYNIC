#!/usr/bin/env python
"""
Phase 2 Setup: Complete workflow for fine-tuning CYNIC

This script walks through all steps needed to set up Phase 2 fine-tuning:
1. Extract real proposals from Phase 1B governance_bot database
2. Combine with synthetic examples if needed
3. Generate JSONL training file
4. Verify training data
5. Print next steps (finetune, export, benchmark)
"""

import json
import logging
from pathlib import Path
from typing import Optional

from .phase1b_integration import generate_training_jsonl
from .data_generator import SYNTHETIC_PROPOSALS

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


def print_header(title: str):
    """Print a formatted header."""
    print(f"\n{'='*70}")
    print(f"  {title}")
    print(f"{'='*70}\n")


def step_1_extract_real_proposals():
    """Step 1: Extract proposals from governance_bot database."""
    print_header("STEP 1: Extract Real Proposals from Phase 1B")

    print("Looking for governance_bot.db...")
    db_paths = [
        Path("governance_bot/governance_bot.db"),
        Path.home() / ".cynic" / "governance_bot.db",
    ]

    db_path = None
    for p in db_paths:
        if p.exists():
            db_path = p
            break

    if not db_path:
        print("[!] Could not find governance_bot.db")
        print("  Checked:")
        for p in db_paths:
            print(f"    - {p}")
        print("\n  Proceeding with synthetic examples only.")
        return None

    print(f"[+] Found governance_bot.db at {db_path}")
    print("\nExtracting proposals...")

    try:
        proposals = []
        import sqlite3
        conn = sqlite3.connect(db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            SELECT COUNT(*) as total FROM proposals
            WHERE judgment_verdict IS NOT NULL
        """)
        total_count = cursor.fetchone()["total"]

        if total_count == 0:
            print("[!] No proposals with judgments found in database")
            print("  (Governance bot hasn't generated judgments yet)")
            return None

        cursor.execute("""
            SELECT
                proposal_id, title, category, impact_level, approval_status,
                judgment_verdict, judgment_q_score, yes_votes, no_votes
            FROM proposals
            WHERE judgment_verdict IS NOT NULL
            ORDER BY created_at DESC
            LIMIT 20
        """)

        proposals = [dict(row) for row in cursor.fetchall()]
        conn.close()

        print(f"[+] Extracted {len(proposals)} proposals")
        print(f"\nSample proposals:")
        for prop in proposals[:3]:
            print(f"  - [{prop['judgment_verdict']}] {prop['title'][:50]}")

        return proposals

    except Exception as e:
        logger.error(f"Failed to extract proposals: {e}")
        print(f"[!] Error: {e}")
        return None


def step_2_verify_training_data(output_path: Path):
    """Step 2: Verify generated training data."""
    print_header("STEP 2: Verify Training Data")

    if not output_path.exists():
        print(f"[X] Training file not found: {output_path}")
        return False

    print(f"[+] Found training file: {output_path}")
    print(f"  Size: {output_path.stat().st_size / 1024:.1f} KB")

    # Count examples
    example_count = 0
    verdict_counts = {}

    with open(output_path) as f:
        for line in f:
            if not line.strip():
                continue
            try:
                example = json.loads(line)
                if "messages" in example and len(example["messages"]) == 3:
                    example_count += 1
                    verdict = json.loads(example["messages"][2]["content"]).get("verdict")
                    verdict_counts[verdict] = verdict_counts.get(verdict, 0) + 1
            except json.JSONDecodeError:
                pass

    print(f"\n[+] Training examples: {example_count}")
    print(f"\nVerdict distribution:")
    for verdict in ["HOWL", "WAG", "GROWL", "BARK"]:
        count = verdict_counts.get(verdict, 0)
        pct = (count / example_count * 100) if example_count > 0 else 0
        print(f"  {verdict:6s}: {count:3d} ({pct:5.1f}%)")

    if example_count < 5:
        print(f"\n[!] Warning: Only {example_count} examples. Recommended minimum: 20")
        return False

    print(f"\n[+] Training data verified!")
    return True


def step_3_print_next_steps(output_path: Path):
    """Step 3: Print next steps."""
    print_header("STEP 3: Next Steps")

    print("Phase 2 setup complete! Training data is ready.")
    print(f"\nTraining file: {output_path}")

    print("\n" + "="*70)
    print("PHASE 2: Fine-tuning on GPU (1-2 hours)")
    print("="*70)
    print(f"""
Option A: Fine-tune locally (requires GPU with 8GB+ VRAM)
$ python -m cynic.training.finetune --data {output_path}

Option B: Fine-tune on Google Colab (free T4 GPU)
1. Download {output_path.name}
2. Open: CYNIC_Mistral_Finetune_Colab.ipynb
3. Upload {output_path.name}
4. Run all cells (takes ~1.5 hours)
5. Download cynic-mistral-7b-qlora.zip
6. Extract locally

""")

    print("="*70)
    print("PHASE 3: Export to Ollama")
    print("="*70)
    print("""
After fine-tuning:
$ python -m cynic.training.export_ollama

This will:
  1. Merge LoRA adapters with base Mistral 7B
  2. Create Ollama Modelfile
  3. Register: cynic-mistral:7b

Test it:
$ ollama run cynic-mistral:7b "Judge: proposal text"
""")

    print("="*70)
    print("PHASE 4: Benchmark Against Baselines")
    print("="*70)
    print("""
After exporting:
$ python -m cynic.training.benchmark_model

This will:
  1. Test cynic-mistral:7b vs gemma2:2b
  2. Compare accuracy, latency, cost
  3. Store results: ~/.cynic/benchmark_results.json

Example output:
  [BEST] Best model: cynic-mistral:7b (score: 0.847)
  Next: Update LLMRegistry to route governance calls there

""")

    print("="*70)
    print("Configuration")
    print("="*70)
    print("""
Edit ~/.cynic/llm/adapter.py to prefer cynic-mistral:7b:

PREFERRED_MODELS = {
    "SOCIAL:JUDGE": "ollama:cynic-mistral:7b",
}

On next CYNIC startup, it will auto-benchmark and route there.
""")


def main():
    """Run complete Phase 2 setup."""
    print_header("PHASE 2 SETUP: Fine-Tuning & Model Learning")

    print("""
Phase 1B (just completed):
  [+] Governance bot deployed
  [+] Learning loop closes (Q-Table updates from outcomes)
  [+] Community rates proposals (1-5 stars)
  [+] Real governance data collected

Phase 2 (this setup):
  [>] Extract training data from Phase 1B
  [>] Fine-tune Mistral 7B with governance examples
  [>] Deploy via Ollama
  [>] Benchmark against baselines
  [>] Auto-route production traffic

Result: CYNIC judges improve from real feedback!
""")

    # Step 1: Extract proposals
    real_proposals = step_1_extract_real_proposals()

    # Step 2: Generate training JSONL
    print_header("Generating Training File")
    try:
        output_path = generate_training_jsonl()
        if not output_path:
            print("[X] Failed to generate training file")
            return False
    except Exception as e:
        logger.error(f"Failed to generate training file: {e}", exc_info=True)
        print(f"[X] Error: {e}")
        return False

    # Step 3: Verify data
    if not step_2_verify_training_data(output_path):
        print("\n[!] Warning: Training data has issues. Continue? (y/n)")
        response = input("> ").strip().lower()
        if response != "y":
            return False

    # Step 4: Print next steps
    step_3_print_next_steps(output_path)

    print("\n" + "="*70)
    print("[+] Phase 2 Setup Complete!")
    print("="*70)
    print(f"\nNext: python -m cynic.training.finetune")
    print("\nFor detailed instructions, see: cynic/training/README.md")


if __name__ == "__main__":
    main()
