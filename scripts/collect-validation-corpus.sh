#!/bin/bash
# Collect and validate wallet corpus (May 2-3 execution)

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_DIR="$PROJECT_ROOT/cynic-python"
ADDRESSES_FILE="$PYTHON_DIR/CORPUS_ADDRESSES.json"
OUTPUT_CORPUS="$PYTHON_DIR/validation_corpus_real.json"

echo "================================================================================"
echo "WALLET BEHAVIOR CORPUS COLLECTION — May 2-3 Execution"
echo "================================================================================"
echo ""

# Check API key
if [ -z "$HELIUS_API_KEY" ]; then
    echo "ERROR: HELIUS_API_KEY not set"
    echo "  Load from secure location (e.g., ~/.cynic-env) before running script"
    exit 1
fi

echo "✓ HELIUS_API_KEY configured"
echo ""

# Check addresses file
if [ ! -f "$ADDRESSES_FILE" ]; then
    echo "ERROR: $ADDRESSES_FILE not found"
    exit 1
fi

echo "✓ Corpus addresses loaded: $ADDRESSES_FILE"
echo ""

# Step 1: Generate Python script to collect profiles
echo "Step 1: Collecting wallet profiles from Helius..."
echo ""

python3 << 'PYTHON_SCRIPT'
import json
import sys
import os

sys.path.insert(0, os.environ.get('PYTHON_DIR', '.'))

from wallet_corpus_builder import KNOWN_HUMANS, KNOWN_SYBILS
from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_scorer import score_wallet

# Load addresses
addresses_file = os.environ.get('ADDRESSES_FILE')
with open(addresses_file, 'r') as f:
    corpus_config = json.load(f)

# Initialize collector
collector = HeliusWalletCollector()
output_file = os.environ.get('OUTPUT_CORPUS')

corpus = []
human_count = 0
sybil_count = 0

print("Fetching human wallets...")
for item in corpus_config['humans']:
    addr = item['address']
    label = item['label']
    print(f"  {label} ({addr[:8]}...)", end=" ", flush=True)

    try:
        profile = collector.collect_wallet_profile(addr)
        if profile:
            corpus_item = {
                "wallet_address": profile.wallet_address,
                "is_human": True,
                "source": item['source'],
                "label": label,
                "wallet_age_days": profile.wallet_age_days,
                "token_count": profile.token_count,
                "program_count": profile.program_count,
                "unique_swap_pairs": profile.unique_swap_pairs,
                "activity_span_days": profile.activity_span_days,
                "total_transactions": profile.total_transactions,
                "transaction_density": profile.transaction_density,
                "gap_max_days": profile.gap_max_days,
                "all_txs_same_hour": profile.all_txs_same_hour,
                "single_token_pct": profile.single_token_pct,
                "recent_whale_flag": profile.recent_whale_flag,
                "transaction_frequency_anomaly": profile.transaction_frequency_anomaly,
            }
            corpus.append(corpus_item)
            human_count += 1
            print(f"✓ score={profile.authenticity_score:.3f}")
        else:
            print("✗ (fetch failed)")
    except Exception as e:
        print(f"✗ ({str(e)[:40]}...)")

print("")
print("Fetching sybil wallets...")
for item in corpus_config['sybils']:
    addr = item['address']
    label = item['label']

    # Skip synthetic test wallets (would fail on live network)
    if addr.startswith('synthetic_') or addr.startswith('BadActor'):
        print(f"  {label} (SKIPPED: synthetic/test)", flush=True)
        continue

    print(f"  {label} ({addr[:8]}...)", end=" ", flush=True)

    try:
        profile = collector.collect_wallet_profile(addr)
        if profile:
            corpus_item = {
                "wallet_address": profile.wallet_address,
                "is_human": False,
                "source": item['source'],
                "label": label,
                "wallet_age_days": profile.wallet_age_days,
                "token_count": profile.token_count,
                "program_count": profile.program_count,
                "unique_swap_pairs": profile.unique_swap_pairs,
                "activity_span_days": profile.activity_span_days,
                "total_transactions": profile.total_transactions,
                "transaction_density": profile.transaction_density,
                "gap_max_days": profile.gap_max_days,
                "all_txs_same_hour": profile.all_txs_same_hour,
                "single_token_pct": profile.single_token_pct,
                "recent_whale_flag": profile.recent_whale_flag,
                "transaction_frequency_anomaly": profile.transaction_frequency_anomaly,
            }
            corpus.append(corpus_item)
            sybil_count += 1
            print(f"✓ score={profile.authenticity_score:.3f}")
        else:
            print("✗ (fetch failed)")
    except Exception as e:
        print(f"✗ ({str(e)[:40]}...)")

# Save corpus
print("")
print(f"Saving corpus: {output_file}")
with open(output_file, 'w') as f:
    json.dump(corpus, f, indent=2)

print(f"✓ Corpus complete: {len(corpus)} wallets ({human_count}H + {sybil_count}S)")
print("")

PYTHON_SCRIPT

export PYTHON_DIR="$PYTHON_DIR"
export ADDRESSES_FILE="$ADDRESSES_FILE"
export OUTPUT_CORPUS="$OUTPUT_CORPUS"

python3 << 'PYTHON_EXEC'
import json
import os
import sys

sys.path.insert(0, os.getenv('PYTHON_DIR', '.'))

from wallet_corpus_builder import KNOWN_HUMANS, KNOWN_SYBILS
from wallet_behavior_helius import HeliusWalletCollector

# Load addresses
addresses_file = os.getenv('ADDRESSES_FILE')
with open(addresses_file, 'r') as f:
    corpus_config = json.load(f)

# Initialize collector
collector = HeliusWalletCollector()
output_file = os.getenv('OUTPUT_CORPUS')

corpus = []
human_count = 0
sybil_count = 0

print("Fetching human wallets...")
for item in corpus_config['humans']:
    addr = item['address']
    label = item['label']
    print(f"  {label} ({addr[:8]}...)", end=" ", flush=True)

    try:
        profile = collector.collect_wallet_profile(addr)
        if profile:
            corpus_item = {
                "wallet_address": profile.wallet_address,
                "is_human": True,
                "source": item['source'],
                "label": label,
                "wallet_age_days": profile.wallet_age_days,
                "token_count": profile.token_count,
                "program_count": profile.program_count,
                "unique_swap_pairs": profile.unique_swap_pairs,
                "activity_span_days": profile.activity_span_days,
                "total_transactions": profile.total_transactions,
                "transaction_density": profile.transaction_density,
                "gap_max_days": profile.gap_max_days,
                "all_txs_same_hour": profile.all_txs_same_hour,
                "single_token_pct": profile.single_token_pct,
                "recent_whale_flag": profile.recent_whale_flag,
                "transaction_frequency_anomaly": profile.transaction_frequency_anomaly,
            }
            corpus.append(corpus_item)
            human_count += 1
            print(f"✓ score={profile.authenticity_score:.3f}")
        else:
            print("✗ (fetch failed)")
    except Exception as e:
        print(f"✗ ({str(e)[:40]}...)")

print("")
print("Fetching sybil wallets...")
for item in corpus_config['sybils']:
    addr = item['address']
    label = item['label']

    # Skip synthetic/test wallets
    if addr.startswith('synthetic_') or addr.startswith('BadActor'):
        print(f"  {label} (SKIPPED: synthetic)", flush=True)
        continue

    print(f"  {label} ({addr[:8]}...)", end=" ", flush=True)

    try:
        profile = collector.collect_wallet_profile(addr)
        if profile:
            corpus_item = {
                "wallet_address": profile.wallet_address,
                "is_human": False,
                "source": item['source'],
                "label": label,
                "wallet_age_days": profile.wallet_age_days,
                "token_count": profile.token_count,
                "program_count": profile.program_count,
                "unique_swap_pairs": profile.unique_swap_pairs,
                "activity_span_days": profile.activity_span_days,
                "total_transactions": profile.total_transactions,
                "transaction_density": profile.transaction_density,
                "gap_max_days": profile.gap_max_days,
                "all_txs_same_hour": profile.all_txs_same_hour,
                "single_token_pct": profile.single_token_pct,
                "recent_whale_flag": profile.recent_whale_flag,
                "transaction_frequency_anomaly": profile.transaction_frequency_anomaly,
            }
            corpus.append(corpus_item)
            sybil_count += 1
            print(f"✓ score={profile.authenticity_score:.3f}")
        else:
            print("✗ (fetch failed)")
    except Exception as e:
        print(f"✗ ({str(e)[:40]}...)")

# Save corpus
print("")
print(f"Saving corpus: {output_file}")
with open(output_file, 'w') as f:
    json.dump(corpus, f, indent=2)

print(f"✓ Corpus complete: {len(corpus)} wallets ({human_count}H + {sybil_count}S)")
PYTHON_EXEC

# Step 2: Validate corpus
echo ""
echo "Step 2: Validating corpus (ROC-AUC test)..."
echo ""

python3 "$PYTHON_DIR/wallet_behavior_validator.py" "$OUTPUT_CORPUS"

echo ""
echo "================================================================================"
echo "Corpus collection complete!"
echo "================================================================================"
