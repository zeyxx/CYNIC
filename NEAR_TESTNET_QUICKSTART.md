# NEAR Testnet Quick Start

## TL;DR — 5 Minutes to Testnet

```bash
# 1. Install NEAR CLI
npm install -g near-cli

# 2. Create account (web wallet)
# Go to https://wallet.testnet.near.org/
# Create account: cynic-gov.testnet
# Fund it (free testnet NEAR)

# 3. Export credentials
export NEAR_ACCOUNT_ID=cynic-gov.testnet
export NEAR_NETWORK=testnet
export NEAR_RPC_URL=https://rpc.testnet.near.org

# 4. Verify setup
python scripts/verify_near_setup.py

# 5. Deploy contract
./scripts/deploy_near_contract.sh governance
```

## Step-by-Step

### 1️⃣ Install NEAR CLI

**macOS/Linux**:
```bash
npm install -g near-cli
```

**Windows (PowerShell)**:
```powershell
npm install -g near-cli
```

Verify:
```bash
near --version
```

### 2️⃣ Create Testnet Account

**Option A: Web Wallet (Easiest)**

1. Go to https://wallet.testnet.near.org/
2. Click "Create Account"
3. Enter account name: `cynic-gov`
4. Save recovery phrase (store safely!)
5. Choose network: TESTNET
6. Account created: `cynic-gov.testnet`
7. Get free testnet NEAR from faucet

**Option B: Command Line**

```bash
near create-account --accountId cynic-gov.testnet --initialBalance 10
```

### 3️⃣ Set Environment Variables

Create `.env` file:

```bash
# NEAR Configuration
export NEAR_ACCOUNT_ID=cynic-gov.testnet
export NEAR_NETWORK=testnet
export NEAR_RPC_URL=https://rpc.testnet.near.org
export NEAR_GOVERNANCE_CONTRACT=governance.testnet
export NEAR_MASTER_ACCOUNT=governance.testnet
```

Or set inline:
```bash
export NEAR_ACCOUNT_ID=cynic-gov.testnet
export NEAR_NETWORK=testnet
```

### 4️⃣ Verify Connection

```bash
# Check if CLI can connect
near state cynic-gov.testnet
```

Expected output:
```
Account cynic-gov.testnet:
  Amount: 10,000,000,000,000,000,000,000,000 yoctoNEAR
  Locked: 0 yoctoNEAR
  Code hash: 11111111111111111111111111111111
  Storage used: 182 bytes
  ...
```

### 5️⃣ Build Governance Contract

```bash
# Build the contract
cd contracts/governance
cargo build --target wasm32-unknown-unknown --release

# Output: target/wasm32-unknown-unknown/release/governance_contract.wasm
```

### 6️⃣ Deploy Contract

```bash
# Deploy to testnet
./scripts/deploy_near_contract.sh governance

# Or manually:
near deploy \
  --accountId governance.testnet \
  --wasmFile contracts/governance/target/wasm32-unknown-unknown/release/governance_contract.wasm
```

### 7️⃣ Verify Contract

```bash
# Check contract is deployed
near view governance.testnet get_proposals_count ''

# Output: 0 (no proposals yet)
```

### 8️⃣ Update Configuration

Update `.env` with deployed contract:

```env
NEAR_GOVERNANCE_CONTRACT=governance.testnet
NEAR_MASTER_ACCOUNT=governance.testnet
```

### 9️⃣ Run Verification Script

```bash
python scripts/verify_near_setup.py
```

Expected output:
```
======================================================================
NEAR TESTNET VERIFICATION
======================================================================

📋 Environment Variables:
  ✅ NEAR_NETWORK: testnet
  ✅ NEAR_RPC_URL: https://rpc.testnet.near.org
  ✅ NEAR_GOVERNANCE_CONTRACT: governance.testnet
  ✅ NEAR_MASTER_ACCOUNT: governance.testnet
  ✅ NEAR_ACCOUNT_ID: cynic-gov.testnet

📡 NEAR RPC Connection:
  ✅ RPC is accessible and healthy

📦 Contract Status:
  ✅ Contract is deployed and accessible

📝 Testing Proposal Submission:
  ✅ Proposal submission successful

🗳️  Testing Vote Recording:
  ✅ Vote recording successful

⚡ Testing Proposal Execution:
  ✅ Proposal execution successful

======================================================================
✅ NEAR TESTNET SETUP VERIFIED
======================================================================
```

## Common Commands

### Create Proposal

```bash
near call governance.testnet create_proposal \
  '{
    "proposal_id": "prop_1",
    "title": "Test Proposal",
    "description": "Testing governance",
    "cynic_verdict": "WAG",
    "cynic_q_score": 72,
    "expires_at": 1740000000
  }' \
  --accountId governance.testnet \
  --deposit 1
```

### Vote on Proposal

```bash
near call governance.testnet vote \
  '{
    "proposal_id": "prop_1",
    "vote": "for"
  }' \
  --accountId cynic-gov.testnet
```

### Check Proposal

```bash
near view governance.testnet get_proposal \
  '{"proposal_id": "prop_1"}'
```

Output:
```json
{
  "proposal_id": "prop_1",
  "title": "Test Proposal",
  "description": "Testing governance",
  "cynic_verdict": "WAG",
  "cynic_q_score": 72,
  "votes_for": 1,
  "votes_against": 0,
  "votes_abstain": 0,
  "status": "Voted",
  "created_at": 1707000000,
  "expires_at": 1740000000
}
```

### Execute Proposal

```bash
near call governance.testnet execute_proposal \
  '{"proposal_id": "prop_1"}' \
  --accountId governance.testnet
```

## Troubleshooting

### "Account does not exist"
```bash
# Create the account
near create-account --accountId cynic-gov.testnet --initialBalance 10
```

### "Insufficient balance"
```bash
# Get more testnet NEAR from faucet
# https://wallet.testnet.near.org/
# Click "Send" button, choose "Get More NEAR"
```

### "Contract not found"
```bash
# Make sure contract is deployed
near view governance.testnet get_proposals_count ''

# If not deployed, run:
./scripts/deploy_near_contract.sh governance
```

### "Invalid private key"
```bash
# Check credentials file exists
ls ~/.near-credentials/testnet/cynic-gov.testnet.json

# Export correct key
export NEAR_PRIVATE_KEY=$(cat ~/.near-credentials/testnet/cynic-gov.testnet.json | jq -r .private_key)
```

## Monitoring

### View Transactions

```bash
# View account on testnet explorer
https://testnet.nearblocks.io/accounts/governance.testnet

# View transactions
https://testnet.nearblocks.io/addresses/governance.testnet
```

### Check Balance

```bash
near state cynic-gov.testnet
```

### View Contract State

```bash
near view governance.testnet get_proposals_count ''
```

## Next Steps

1. ✅ Testnet account created
2. ✅ Contract deployed
3. ⏳ Wire into CYNIC API
4. ⏳ Connect Discord bot
5. ⏳ Test end-to-end
6. ⏳ Deploy to mainnet

## Useful Links

- **Testnet Faucet**: https://wallet.testnet.near.org/
- **Testnet Explorer**: https://testnet.nearblocks.io/
- **NEAR Docs**: https://docs.near.org/
- **RPC API**: https://docs.near.org/api/rpc/introduction
- **Rust SDK**: https://github.com/near/near-sdk-rs

## Costs

✅ **Testnet**: FREE
- Account creation: Free
- Contract deployment: Free (testnet NEAR)
- Transactions: Negligible testnet NEAR

⚠️ **Mainnet**: Requires NEAR tokens
- Account creation: ~0.0425 NEAR
- Contract deployment: ~2 NEAR
- Transactions: ~0.00025 NEAR per call

## What's Next?

After testnet setup:

1. **Integrate with Discord Bot**
   ```bash
   python -m cynic.discord.bot
   ```

2. **Wire GASdf Integration**
   - Get GASdf testnet API keys
   - Update `.env` with GASdf config
   - Test fee flow

3. **Run Complete Tests**
   ```bash
   pytest cynic/tests/test_governance_stack.py -v
   ```

4. **Deploy with Docker**
   ```bash
   docker-compose up --profile with-discord
   ```

5. **Monitor Live Governance**
   - Create proposals via Discord
   - Vote using reactions
   - Watch transactions on explorer
   - Verify treasury grows

---

**You're now ready to test CYNIC governance on NEAR testnet!** 🚀
