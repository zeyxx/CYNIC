#!/bin/bash
# Calibrate token heuristics on real ground-truth data (CultScreener)
# Workflow: ingest → measure → analyze → (optional) tune → deploy
#
# Prerequisites:
#   - CultScreener API live (public, no auth required)
#   - Helius API key set (HELIUS_API_KEY or CYNIC_REST_ADDR)
#   - Hermes available (optional, for twitter enrichment)
#
# Usage:
#   ./scripts/calibrate-token-heuristics.sh [measure|tune|deploy] [--dry-run]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel)"
HEURISTICS_DIR="${REPO_ROOT}/cynic-python/heuristics"
DATASET_DIR="${HOME}/.cynic/datasets/tokens"

# Target accuracy thresholds
TARGET_FUSED_ACCURACY=0.75
TARGET_BARK_ACCURACY=0.90
TARGET_HOWL_ACCURACY=0.90
TARGET_GROWL_ACCURACY=0.70

# Phase: ingest | measure | analyze | tune | deploy | all
PHASE="${1:-all}"
DRY_RUN="${2:-}"

if [ "$DRY_RUN" == "--dry-run" ]; then
    DRY_RUN=true
else
    DRY_RUN=false
fi

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║ Token Heuristics Calibration Pipeline (2026-04-29)                        ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""
echo "Phase: $PHASE"
echo "Dry-run: $DRY_RUN"
echo ""

# ──────────────────────────────────────────────────────────────────────────────
# Phase 1: Ingest ground truth from CultScreener
# ──────────────────────────────────────────────────────────────────────────────
do_ingest() {
    echo "▶ PHASE 1: Ingest ground truth from CultScreener"
    echo ""

    if [ ! -d "$DATASET_DIR" ]; then
        mkdir -p "$DATASET_DIR"
        echo "  Created $DATASET_DIR"
    fi

    cd "$HEURISTICS_DIR"

    if [ "$DRY_RUN" == "true" ]; then
        echo "  [DRY-RUN] Would run: python token_dataset_ingester.py"
        echo "  [DRY-RUN] Expected output: 60 tokens (20 per risk level)"
        return 0
    fi

    echo "  Fetching 20 high-risk + 20 medium-risk + 20 low-risk tokens..."
    python3 token_dataset_ingester.py

    INGESTED_COUNT=$(python3 -c "import json; data=json.load(open('${DATASET_DIR}/ground_truth.json')); print(len(data))")
    echo ""
    echo "  ✓ Ingested $INGESTED_COUNT tokens"
    echo "    - Full signals: ${DATASET_DIR}/ground_truth.json"
    echo "    - Summary: ${DATASET_DIR}/ground_truth.csv"
    echo ""
}

# ──────────────────────────────────────────────────────────────────────────────
# Phase 2: Measure calibration on real data
# ──────────────────────────────────────────────────────────────────────────────
do_measure() {
    echo "▶ PHASE 2: Measure calibration"
    echo ""

    if [ ! -f "$DATASET_DIR/ground_truth.json" ]; then
        echo "  ✗ Dataset not found: ${DATASET_DIR}/ground_truth.json"
        echo "    Run phase 1 (ingest) first"
        return 1
    fi

    cd "$HEURISTICS_DIR"

    if [ "$DRY_RUN" == "true" ]; then
        echo "  [DRY-RUN] Would run: python measure_against_ground_truth.py"
        echo "  [DRY-RUN] Expected output: accuracy report + confusion matrix"
        return 0
    fi

    echo "  Measuring calibration on $(jq 'length' ${DATASET_DIR}/ground_truth.json) tokens..."
    python3 measure_against_ground_truth.py | tee /tmp/measurement_report.txt

    echo ""
    echo "  ✓ Measurement complete. Report: /tmp/measurement_report.txt"
    echo ""
}

# ──────────────────────────────────────────────────────────────────────────────
# Phase 3: Analyze results and identify tuning targets
# ──────────────────────────────────────────────────────────────────────────────
do_analyze() {
    echo "▶ PHASE 3: Analyze results"
    echo ""

    if [ ! -f "/tmp/measurement_report.txt" ]; then
        if [ "$DRY_RUN" == "true" ]; then
            echo "  [DRY-RUN] Would analyze results from /tmp/measurement_report.txt"
            return 0
        fi
        echo "  ✗ Measurement report not found. Run phase 2 (measure) first."
        return 1
    fi

    echo "  Extracting accuracy metrics..."

    OVERALL=$(grep "Overall Accuracy:" /tmp/measurement_report.txt | grep -oE '[0-9.]+%' | head -1)
    BARK=$(grep "Bark.*:" /tmp/measurement_report.txt | grep -oE '[0-9.]+%' | head -1)
    GROWL=$(grep "Growl.*:" /tmp/measurement_report.txt | grep -oE '[0-9.]+%' | head -1)
    HOWL=$(grep "Howl.*:" /tmp/measurement_report.txt | grep -oE '[0-9.]+%' | head -1)

    echo ""
    echo "  Results:"
    echo "    Overall: $OVERALL (target: ${TARGET_FUSED_ACCURACY%.*}%+)"
    echo "    BARK:    $BARK (target: ${TARGET_BARK_ACCURACY%.*}%+)"
    echo "    GROWL:   $GROWL (target: ${TARGET_GROWL_ACCURACY%.*}%+)"
    echo "    HOWL:    $HOWL (target: ${TARGET_HOWL_ACCURACY%.*}%+)"
    echo ""

    # Identify weak categories
    echo "  Tuning targets:"
    if [ "${GROWL%.* }" -lt 70 ]; then
        echo "    ⚠ GROWL accuracy < 70% — twitter/wallet domains miscalibrating ambiguous tokens"
        echo "      Recommendation: Implement age-stratified thresholds (tokens < 336h discount engagement signals)"
    fi
    if [ "${BARK%.* }" -lt 90 ]; then
        echo "    ⚠ BARK accuracy < 90% — missing rug detection"
        echo "      Recommendation: Strengthen authority + concentration checks"
    fi
    if [ "${HOWL%.* }" -lt 90 ]; then
        echo "    ⚠ HOWL accuracy < 90% — false positives on legitimate tokens"
        echo "      Recommendation: Soften concentration penalties for old, trading-active tokens"
    fi
    echo ""
}

# ──────────────────────────────────────────────────────────────────────────────
# Phase 4: Tune heuristics (if accuracy < target)
# ──────────────────────────────────────────────────────────────────────────────
do_tune() {
    echo "▶ PHASE 4: Tune heuristics (if needed)"
    echo ""
    echo "  Tuning is domain-dependent. Edit the appropriate file based on analysis:"
    echo ""
    echo "    For GROWL accuracy < 70%:"
    echo "      1. Edit: $HEURISTICS_DIR/twitter_heuristics.py"
    echo "      2. Adjust: engagement_rate thresholds for young tokens (age_hours < 336)"
    echo "      3. Measure: python measure_against_ground_truth.py"
    echo "      4. Iterate until GROWL >= ${TARGET_GROWL_ACCURACY%.*}%"
    echo ""
    echo "    For BARK/HOWL accuracy < 90%:"
    echo "      1. Edit: $HEURISTICS_DIR/token_heuristics.py or wallet_heuristics.py"
    echo "      2. Adjust thresholds based on failing tokens"
    echo "      3. Measure: python measure_against_ground_truth.py"
    echo ""
    echo "  Manual tuning starts here. See CALIBRATION-ANALYSIS.md for guidance."
    echo ""
}

# ──────────────────────────────────────────────────────────────────────────────
# Phase 5: Deploy calibrated thresholds to kernel
# ──────────────────────────────────────────────────────────────────────────────
do_deploy() {
    echo "▶ PHASE 5: Deploy to kernel"
    echo ""
    echo "  Once calibration accuracy >= ${TARGET_FUSED_ACCURACY%.*}%:"
    echo ""
    echo "    1. Review the threshold changes in token_heuristics.py / twitter_heuristics.py"
    echo "    2. Translate thresholds to Rust: $REPO_ROOT/cynic-kernel/src/dogs/deterministic/token.rs"
    echo "    3. Build and test: cd $REPO_ROOT && make check"
    echo "    4. Measure agreement with Dogs: scripts/test-chess.sh (after deploy)"
    echo "    5. Deploy to kernel: cp target/release/cynic-kernel ~/bin/cynic-kernel"
    echo ""
    echo "  Deployment checklist:"
    echo "    [ ] Token thresholds translated to Rust"
    echo "    [ ] Twitter/wallet heuristics added to deterministic-dog (if needed)"
    echo "    [ ] make check passes"
    echo "    [ ] Local measurement shows ✓ same accuracy as Python"
    echo "    [ ] Kernel restart: systemctl restart cynic-kernel.service"
    echo ""
}

# ──────────────────────────────────────────────────────────────────────────────
# Main execution
# ──────────────────────────────────────────────────────────────────────────────

case "$PHASE" in
    ingest)
        do_ingest
        ;;
    measure)
        do_measure
        ;;
    analyze)
        do_measure
        do_analyze
        ;;
    tune)
        do_analyze
        do_tune
        ;;
    deploy)
        do_deploy
        ;;
    all)
        do_ingest
        do_measure
        do_analyze
        do_deploy
        ;;
    *)
        echo "Unknown phase: $PHASE"
        echo "Usage: $0 [ingest|measure|analyze|tune|deploy|all] [--dry-run]"
        exit 1
        ;;
esac

echo "────────────────────────────────────────────────────────────────────────────"
echo "Calibration workflow complete. Next steps in CALIBRATION-ANALYSIS.md."
echo "────────────────────────────────────────────────────────────────────────────"
