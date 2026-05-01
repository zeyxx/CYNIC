#!/usr/bin/env python3
"""
Semantic axiom extraction via Qwen 9B.
Replaces keyword heuristics with LLM-based reasoning detection.
"""

import json
import requests
import re
import os
from pathlib import Path
import logging

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

# Configured via env; see ~/.cynic-env or fleet.toml
QWEN_ENDPOINT = os.getenv('QWEN_ENDPOINT', 'http://<TAILSCALE_GPU>:8080/v1/chat/completions')

def extract_axiom_scores_semantic(text: str) -> dict:
    """
    Extract axiom scores using semantic LLM analysis.
    Detects axiom-in-action (doing) not just axiom-talk (mentioning).
    """
    scores = {
        'fidelity': 0.5,
        'sovereignty': 0.5,
        'burn': 0.5,
        'phi': 0.5,
        'verify': 0.5,
        'culture': 0.5,
    }

    axiom_prompts = {
        'fidelity': """Does this text show truth-seeking, epistemic rigor, or honest assessment?
Even if "honest/truth" aren't mentioned, truth-in-action counts (stating facts, acknowledging uncertainty, etc).
Text: {text}
Rate 0-1 only:""",

        'sovereignty': """Does this text discuss autonomy, freedom, agency, or respecting others' agency?
Truth-in-action: giving space, enabling choice, respecting independence.
Text: {text}
Rate 0-1 only:""",

        'burn': """Does this text value efficiency, minimal waste, or avoiding unnecessary overhead?
Text: {text}
Rate 0-1 only:""",

        'phi': """Does this text seek harmony, balance, proportion, or structural coherence?
Text: {text}
Rate 0-1 only:""",

        'verify': """Does this text involve testing, measuring, or empirical grounding?
Text: {text}
Rate 0-1 only:""",

        'culture': """Does this text honor traditions, patterns, precedent, or inherited knowledge?
Text: {text}
Rate 0-1 only:""",
    }

    for axiom, prompt_template in axiom_prompts.items():
        prompt = prompt_template.format(text=text[:300])

        try:
            response = requests.post(
                QWEN_ENDPOINT,
                json={
                    "model": "qwen",
                    "messages": [{"role": "user", "content": prompt}],
                    "temperature": 0.1,
                    "max_tokens": 5,
                },
                timeout=30,
            )

            if response.status_code == 200:
                msg = response.json()['choices'][0]['message']
                text_response = (msg.get('content', '') + msg.get('reasoning_content', '')).strip()

                # Extract number
                match = re.search(r'0\.[0-9]+|1\.?0*|0', text_response)
                if match:
                    score = float(match.group())
                    scores[axiom] = max(0, min(1, score))
        except Exception as e:
            log.warning(f"Semantic extraction failed for {axiom}: {e}")

    return scores

def main():
    input_file = Path('/home/user/Bureau/CYNIC/cynic-python/decisions_filtered.jsonl')

    if not input_file.exists():
        log.error(f"Input file not found: {input_file}")
        return

    log.info("Extracting axiom vectors via semantic analysis...\n")

    decisions = []
    with open(input_file) as f:
        for line in f:
            decisions.append(json.loads(line))

    axiom_data = []
    for i, decision in enumerate(decisions):
        if i % 10 == 0:
            log.info(f"  {i}/{len(decisions)}...")

        axioms = extract_axiom_scores_semantic(decision['text'])
        axiom_data.append({
            'session_id': decision['session_id'],
            'timestamp': decision['timestamp'],
            'cortex_type': decision['cortex_type'],
            'domain': decision['domain'],
            'decision_score': decision.get('decision_score', 0),
            **axioms,
            'text_preview': decision['text'][:100],
        })

    log.info(f"\n✓ Extracted axiom vectors for {len(axiom_data)} decisions\n")

    # Save to CSV
    output_csv = Path('/home/user/Bureau/CYNIC/cynic-python/axiom_vectors_semantic.csv')
    import csv
    with open(output_csv, 'w') as f:
        writer = csv.DictWriter(
            f,
            fieldnames=['session_id', 'timestamp', 'cortex_type', 'domain', 'decision_score',
                       'fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture', 'text_preview']
        )
        writer.writeheader()
        writer.writerows(axiom_data)

    log.info(f"✓ Saved semantic axiom vectors to {output_csv}\n")

    # Compare vs heuristic
    log.info("=== SEMANTIC vs HEURISTIC Comparison ===\n")
    axiom_means_semantic = {ax: 0 for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']}
    for d in axiom_data:
        for ax in axiom_means_semantic:
            axiom_means_semantic[ax] += d[ax]

    log.info("SEMANTIC means:")
    for ax, total in axiom_means_semantic.items():
        mean = total / len(axiom_data) if axiom_data else 0
        log.info(f"  {ax.upper()}: {mean:.2f}")

    # Load heuristic for comparison
    import csv
    heuristic_means = {ax: 0 for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']}
    with open('cynic-python/axiom_vectors_final.csv') as f:
        reader = csv.DictReader(f)
        count = 0
        for row in reader:
            for ax in heuristic_means:
                heuristic_means[ax] += float(row[ax])
            count += 1

    log.info(f"\nHEURISTIC means (for comparison):")
    for ax, total in heuristic_means.items():
        mean = total / count if count else 0
        log.info(f"  {ax.upper()}: {mean:.2f}")

if __name__ == '__main__':
    main()
