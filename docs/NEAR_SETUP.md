# NEAR Integration Setup

## Overview

The CYNIC Governance Bot integrates with NEAR blockchain to:
1. Create proposals on-chain
2. Record votes on NEAR
3. Execute governance decisions with smart contracts

## Prerequisites

1. **NEAR Account on Testnet**
   - Visit: https://wallet.testnet.near.org
   - Create account (e.g., `governance.testnet`)
   - Get testnet tokens: https://faucet.testnet.near.org

2. **Generate ed25519 Keys**
   ```bash
   near login  # Creates keys in ~/.near-credentials/testnet/
   ```

3. **Deploy Governance Contract**
   - See: `docs/NEAR_CONTRACT_SETUP.md`
   - Deploy contract to your account

## Configuration

Set in `.env` or environment:

```bash
NEAR_ACCOUNT_ID=governance.testnet
NEAR_PRIVATE_KEY=<base64-encoded-ed25519-key>
NEAR_CONTRACT_ID=governance.testnet
NEAR_RPC_URL=https://rpc.testnet.near.org
NEAR_EXECUTION_ENABLED=true
```

## Finding Your Private Key

1. After `near login`, find your credentials:
   ```bash
   cat ~/.near-credentials/testnet/governance.testnet.json
   ```

2. The JSON will contain:
   ```json
   {
     "account_id": "governance.testnet",
     "public_key": "ed25519:...",
     "private_key": "ed25519:..."
   }
   ```

3. Extract the private key (the part after `ed25519:`) and base64 encode it:
   ```bash
   # Extract just the key part
   KEY_PART="your_key_from_json"
   # Encode to base64
   echo -n "$KEY_PART" | base64
   ```

## Testing Integration

Run integration tests:

```bash
# Set credentials first
export NEAR_ACCOUNT_ID="your_account.testnet"
export NEAR_PRIVATE_KEY="your_base64_key"
export NEAR_CONTRACT_ID="governance.testnet"

# Run tests
pytest cynic/tests/test_near_integration_live.py -v
```

## Deployment to Mainnet

Before mainnet deployment:
1. Audit smart contract for security
2. Test thoroughly on testnet
3. Deploy contract to NEAR mainnet
4. Update NEAR_* configuration to mainnet values
5. Monitor transaction gas costs and adjust limits
6. Set NEAR_EXECUTION_ENABLED=true only after validation

## Troubleshooting

### Tests are skipped
- Make sure NEAR_ACCOUNT_ID environment variable is set
- Tests skip automatically if credentials are not configured

### Transaction failures
- Check account has sufficient testnet tokens (call faucet)
- Verify contract ID matches deployed contract
- Check RPC URL is accessible

### Key encoding issues
- Ensure private key is properly base64 encoded
- Private key should be 64 bytes when decoded
- Check for whitespace in key strings

## RPC Methods Used

The integration uses the following NEAR RPC methods:

1. **query** - View account state and contract code
2. **broadcast_tx_commit** - Submit transactions
3. **tx** - Poll transaction status
4. **block** - Get latest block information

See NEAR RPC documentation: https://docs.near.org/api/rpc/introduction
