#!/bin/bash
# Deploy NEAR Governance Contract to Testnet

set -e

# Configuration
CONTRACT_NAME=${1:-governance}
ACCOUNT_ID="${CONTRACT_NAME}.testnet"
WASM_PATH="contracts/governance/target/wasm32-unknown-unknown/release/governance_contract.wasm"

echo "=================================================="
echo "NEAR Governance Contract Deployment"
echo "=================================================="
echo ""
echo "Configuration:"
echo "  Contract Name: $CONTRACT_NAME"
echo "  Account ID: $ACCOUNT_ID"
echo "  WASM Path: $WASM_PATH"
echo ""

# Check if WASM file exists
if [ ! -f "$WASM_PATH" ]; then
    echo "❌ WASM file not found at $WASM_PATH"
    echo ""
    echo "Building contract..."
    cd contracts/governance
    cargo build --target wasm32-unknown-unknown --release
    cd ../..
fi

# Set network to testnet
export NEAR_ENV=testnet

echo "Step 1: Creating account (if needed)..."
near create-account --accountId "$ACCOUNT_ID" --initialBalance 10 2>/dev/null || echo "  (Account may already exist)"

echo ""
echo "Step 2: Deploying contract..."
near deploy \
    --accountId "$ACCOUNT_ID" \
    --wasmFile "$WASM_PATH"

echo ""
echo "Step 3: Initializing contract..."
near call "$ACCOUNT_ID" new \
    "{\"owner\": \"$ACCOUNT_ID\"}" \
    --accountId "$ACCOUNT_ID"

echo ""
echo "Step 4: Verifying contract..."
near view "$ACCOUNT_ID" get_owner ''

echo ""
echo "=================================================="
echo "✅ Contract deployed successfully!"
echo "=================================================="
echo ""
echo "Contract Details:"
echo "  Account ID: $ACCOUNT_ID"
echo "  Network: testnet"
echo "  RPC URL: https://rpc.testnet.near.org"
echo ""
echo "Next steps:"
echo "  1. Set NEAR_GOVERNANCE_CONTRACT=$ACCOUNT_ID in .env"
echo "  2. Test with: near view $ACCOUNT_ID get_proposals_count ''"
echo "  3. Create proposal: near call $ACCOUNT_ID create_proposal '{...}' --accountId $ACCOUNT_ID"
echo ""
