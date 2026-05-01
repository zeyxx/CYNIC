#!/usr/bin/env python3
"""
Score text blocks for decision-ness using Qwen 9B on cynic-gpu.
Deploy via llama.cpp HTTP endpoint.
"""

import json
import requests
import re
import os
from pathlib import Path
import logging
from typing import Optional

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

QWEN_ENDPOINT = os.getenv('QWEN_ENDPOINT', 'http://<TAILSCALE_GPU>:8080/v1/chat/completions')

def score_block(text: str, retries: int = 2) -> Optional[float]:
    """
    Score a text block for decision-ness (0-1).
    Returns None if inference fails.
    """
    prompt = f"""Given this text from a decision-making session, score how much it represents an actual decision moment (choosing between options, evaluating trade-offs, axiom reasoning).

Scale:
- 1.0: Clear decision with axiom evaluation or trade-off reasoning
- 0.7: Decision moment but implicit reasoning
- 0.4: Discussion or analysis without clear decision
- 0.1: Meta-commentary, status, or boilerplate

Text:
{text[:500]}

Score (just a number 0.0-1.0):"""

    try:
        response = requests.post(
            QWEN_ENDPOINT,
            json={
                "model": "qwen",
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.1,
                "max_tokens": 10,
            },
            timeout=30,
        )

        if response.status_code == 200:
            result = response.json()
            msg = result.get("choices", [{}])[0].get("message", {})
            # Combine content and reasoning_content
            text = (msg.get("content", "") + " " + msg.get("reasoning_content", "")).strip()
            # Extract first number in 0-1 range
            match = re.search(r'0\.[0-9]+|1\.?0*|0', text)
            if match:
                try:
                    score = float(match.group())
                    return max(0, min(1, score))  # Clamp to 0-1
                except ValueError:
                    return None
            return None
        else:
            log.warning(f"Qwen error: {response.status_code} - {response.text}")
            return None
    except Exception as e:
        if retries > 0:
            log.warning(f"Retry: {e}")
            return score_block(text, retries - 1)
        log.warning(f"Qwen inference failed: {e}")
        return None

def main():
    input_file = Path('/home/user/Bureau/CYNIC/cynic-python/text_blocks_raw.jsonl')

    if not input_file.exists():
        log.error(f"Input file not found: {input_file}")
        log.info("Run extract_all_text_blocks.py first")
        return

    log.info(f"Testing Qwen 9B connection at {QWEN_ENDPOINT}...")
    test_score = score_block("Test decision moment.")

    if test_score is None:
        log.error("Cannot connect to Qwen 9B. Ensure llama-server is running:")
        log.info("  On cynic-gpu: Windows schtask or services")
        log.info("  On cynic-core: systemctl start llama-server")
        return

    log.info(f"✓ Qwen connected. Test score: {test_score}\n")

    # Load and score all blocks
    log.info("Scoring text blocks for decision-ness...")
    scored_blocks = []

    with open(input_file) as f:
        blocks = [json.loads(line) for line in f]

    for i, block in enumerate(blocks):
        if i % 10 == 0:
            log.info(f"  {i}/{len(blocks)}...")

        score = score_block(block['text'])
        if score is not None:
            block['decision_score'] = score
            scored_blocks.append(block)

    # Filter to high-confidence decisions (>0.5)
    decisions = [b for b in scored_blocks if b['decision_score'] > 0.5]

    log.info(f"\n✓ Scored {len(scored_blocks)} blocks")
    log.info(f"  High-confidence decisions (>0.5): {len(decisions)}\n")

    # Save scored blocks
    output_file = Path('/home/user/Bureau/CYNIC/cynic-python/text_blocks_scored.jsonl')
    with open(output_file, 'w') as f:
        for block in scored_blocks:
            f.write(json.dumps(block) + '\n')

    log.info(f"✓ Saved scored blocks to {output_file}")

    # Save decisions only
    decisions_file = Path('/home/user/Bureau/CYNIC/cynic-python/decisions_filtered.jsonl')
    with open(decisions_file, 'w') as f:
        for block in decisions:
            f.write(json.dumps(block) + '\n')

    log.info(f"✓ Saved {len(decisions)} decisions to {decisions_file}")
    log.info(f"\nNext: Extract axiom vectors from decisions:")
    log.info(f"  python3 cynic-python/extract_axiom_vectors_from_decisions.py")

if __name__ == '__main__':
    main()
