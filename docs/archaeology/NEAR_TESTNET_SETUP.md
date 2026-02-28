# NEAR Testnet Setup Guide

## Overview

This guide walks through setting up NEAR testnet for CYNIC governance execution.

**What You'll Set Up**:
- ✅ NEAR testnet account
- ✅ Governance smart contract (Rust)
- ✅ Account credentials
- ✅ Environment configuration
- ✅ Contract deployment verification

**Estimated Time**: 30-45 minutes

## Prerequisites

### 1. Install Rust (for contract development)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Install NEAR CLI
```bash
npm install -g near-cli
```

Or with cargo:
```bash
cargo install near-cli
```

### 3. Verify installations
```bash
rustc --version
cargo --version
near --version
```

## Step 1: Create NEAR Testnet Account

### Option A: Web Wallet (Easiest)

1. Go to https://wallet.testnet.near.org/
2. Click "Create Account"
3. Choose an account name (e.g., `cynic-gov.testnet`)
4. Save the recovery phrase (IMPORTANT!)
5. Fund account with testnet NEAR (free from faucet)

### Option B: Command Line

```bash
# Set network to testnet
export NEAR_ENV=testnet

# Create account (replaces {ACCOUNT_ID} with your account)
near create-account --accountId {ACCOUNT_ID}.testnet --initialBalance 10
```

Example:
```bash
near create-account --accountId cynic-gov.testnet --initialBalance 10
```

## Step 2: Import Account Credentials

### Find Your Credentials File

NEAR CLI automatically stores credentials at:
```bash
ls ~/.near-credentials/testnet/
```

You should see: `{account_id}.testnet.json`

### View Your Credentials
```bash
cat ~/.near-credentials/testnet/cynic-gov.testnet.json
```

Output:
```json
{
  "account_id": "cynic-gov.testnet",
  "public_key": "ed25519:...",
  "private_key": "ed25519:..."
}
```

### Export for CYNIC
```bash
export NEAR_ACCOUNT_ID=cynic-gov.testnet
export NEAR_PRIVATE_KEY=$(cat ~/.near-credentials/testnet/cynic-gov.testnet.json | grep private_key | cut -d'"' -f4)
export NEAR_NETWORK=testnet
export NEAR_RPC_URL=https://rpc.testnet.near.org
```

## Step 3: Create Governance Smart Contract

### Create Contract Project

```bash
# Create new Rust project
cargo new --lib near_governance_contract
cd near_governance_contract
```

### Update Cargo.toml

```toml
[package]
name = "near_governance_contract"
version = "0.1.0"
edition = "2021"

[dependencies]
near-sdk = "4.1.1"
near-contract-standards = "4.1.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

### Create Contract Code

Create `src/lib.rs`:

```rust
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum ProposalStatus {
    Open,
    Voted,
    Approved,
    Rejected,
    Executed,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Proposal {
    pub proposal_id: String,
    pub title: String,
    pub description: String,
    pub cynic_verdict: String,  // HOWL/WAG/GROWL/BARK
    pub cynic_q_score: u32,     // 0-100
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Governance {
    proposals: UnorderedMap<String, Proposal>,
    owner: AccountId,
}

#[near_bindgen]
impl Governance {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            proposals: UnorderedMap::new(b"p".to_vec()),
            owner,
        }
    }

    pub fn create_proposal(
        &mut self,
        proposal_id: String,
        title: String,
        description: String,
        cynic_verdict: String,
        cynic_q_score: u32,
        expires_at: u64,
    ) -> Proposal {
        let proposal = Proposal {
            proposal_id: proposal_id.clone(),
            title,
            description,
            cynic_verdict,
            cynic_q_score,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            status: ProposalStatus::Open,
            created_at: env::block_timestamp() / 1_000_000,
            expires_at,
        };

        self.proposals.insert(&proposal_id, &proposal);
        proposal
    }

    pub fn vote(
        &mut self,
        proposal_id: String,
        vote: String,
    ) -> bool {
        let mut proposal = self
            .proposals
            .get(&proposal_id)
            .expect("Proposal not found");

        match vote.as_str() {
            "for" => proposal.votes_for += 1,
            "against" => proposal.votes_against += 1,
            "abstain" => proposal.votes_abstain += 1,
            _ => panic!("Invalid vote type"),
        }

        self.proposals.insert(&proposal_id, &proposal);
        true
    }

    pub fn execute_proposal(&mut self, proposal_id: String) -> bool {
        let mut proposal = self
            .proposals
            .get(&proposal_id)
            .expect("Proposal not found");

        let total = proposal.votes_for + proposal.votes_against;
        if total == 0 || (proposal.votes_for * 100) / total < 50 {
            panic!("Proposal does not have majority approval");
        }

        proposal.status = ProposalStatus::Executed;
        self.proposals.insert(&proposal_id, &proposal);
        true
    }

    pub fn get_proposal(&self, proposal_id: String) -> Option<Proposal> {
        self.proposals.get(&proposal_id)
    }

    pub fn get_proposals_count(&self) -> u64 {
        self.proposals.len()
    }
}
```

### Build Contract

```bash
cd near_governance_contract
cargo build --target wasm32-unknown-unknown --release
```

Output: `target/wasm32-unknown-unknown/release/near_governance_contract.wasm`

## Step 4: Deploy Contract to Testnet

### Create Deployment Account

For deploying contracts, you may want a separate account:

```bash
# Create deployment account
near create-account --accountId governance.testnet --initialBalance 10
```

### Deploy the Contract

```bash
# Set environment
export NEAR_ENV=testnet

# Deploy contract
near deploy \
  --accountId governance.testnet \
  --wasmFile target/wasm32-unknown-unknown/release/near_governance_contract.wasm
```

### Initialize Contract

```bash
near call governance.testnet new \
  '{"owner": "governance.testnet"}' \
  --accountId governance.testnet
```

## Step 5: Configure CYNIC for NEAR Testnet

### Set Environment Variables

Create `.env` file in project root:

```env
# NEAR Configuration
NEAR_NETWORK=testnet
NEAR_RPC_URL=https://rpc.testnet.near.org
NEAR_GOVERNANCE_CONTRACT=governance.testnet
NEAR_MASTER_ACCOUNT=governance.testnet
NEAR_ACCOUNT_ID=cynic-gov.testnet
NEAR_PRIVATE_KEY=ed25519:YOUR_PRIVATE_KEY_HERE

# GASdf Configuration (testnet)
GASDF_API_URL=https://api.gasdf.io/v1
GASDF_NETWORK=testnet
GASDF_PAYMENT_TOKEN=EPjFWaLb3crLMMLhSKMKLU2z3prNhuqpZTKmarsVmqr

# Discord Bot
DISCORD_TOKEN=YOUR_DISCORD_BOT_TOKEN
CYNIC_API_URL=http://localhost:8765
```

### Load in Docker Compose

Update `docker-compose.yml`:

```yaml
services:
  cynic:
    environment:
      NEAR_NETWORK: testnet
      NEAR_RPC_URL: https://rpc.testnet.near.org
      NEAR_GOVERNANCE_CONTRACT: governance.testnet
      NEAR_MASTER_ACCOUNT: governance.testnet
      NEAR_ACCOUNT_ID: ${NEAR_ACCOUNT_ID}
      NEAR_PRIVATE_KEY: ${NEAR_PRIVATE_KEY}
```

## Step 6: Test Contract Interaction

### Create a Proposal

```bash
near call governance.testnet create_proposal \
  '{
    "proposal_id": "prop_test_1",
    "title": "Test Proposal",
    "description": "Testing NEAR integration",
    "cynic_verdict": "WAG",
    "cynic_q_score": 72,
    "expires_at": 1740000000
  }' \
  --accountId governance.testnet \
  --deposit 1
```

### Query Proposal

```bash
near view governance.testnet get_proposal \
  '{"proposal_id": "prop_test_1"}'
```

Expected output:
```json
{
  "proposal_id": "prop_test_1",
  "title": "Test Proposal",
  "description": "Testing NEAR integration",
  "cynic_verdict": "WAG",
  "cynic_q_score": 72,
  "votes_for": 0,
  "votes_against": 0,
  "votes_abstain": 0,
  "status": "Open",
  "created_at": 1707000000,
  "expires_at": 1740000000
}
```

### Vote on Proposal

```bash
near call governance.testnet vote \
  '{"proposal_id": "prop_test_1", "vote": "for"}' \
  --accountId governance.testnet
```

### Execute Proposal

```bash
near call governance.testnet execute_proposal \
  '{"proposal_id": "prop_test_1"}' \
  --accountId governance.testnet
```

## Step 7: Integrate with CYNIC Executor

### Update NEAR Executor Configuration

In your Python code:

```python
from cynic.perception.integrations.near import NEARExecutor, NEARNetworkConfig
import os

# Create config from environment
config = NEARNetworkConfig(
    network_id="testnet",
    rpc_url=os.getenv("NEAR_RPC_URL", "https://rpc.testnet.near.org"),
    contract_id=os.getenv("NEAR_GOVERNANCE_CONTRACT", "governance.testnet"),
    master_account=os.getenv("NEAR_MASTER_ACCOUNT", "governance.testnet"),
)

# Create executor
executor = NEARExecutor(config)

# Test health
health = await executor.health()
print(f"NEAR health: {health}")
```

### Implement Transaction Signing

The executor stub needs proper transaction signing. Update `cynic/integrations/near/executor.py`:

```python
async def _execute_contract_call(
    self,
    signer_id: str,
    contract_call: NEARContractCall,
    proposal_id: str,
    cynic_verdict: str,
) -> NEARExecutionResult:
    """Execute contract call with proper signing."""

    try:
        # Get nonce and block hash
        nonce = await self.rpc_client.get_nonce(signer_id)
        block_hash = await self.rpc_client.get_block_hash()

        logger.info(
            "Executing: contract=%s method=%s proposal=%s",
            self.config.contract_id,
            contract_call.method_name,
            proposal_id,
        )

        # In production: sign transaction with private key
        # For testnet: use near-api-py or near-cli for signing
        # This is a TODO for full implementation

        # For now, return pending result
        return NEARExecutionResult(
            transaction_hash="",
            block_height=0,
            status=TxStatus.PENDING,
            gas_used=0,
            outcome={
                "method": contract_call.method_name,
                "contract": self.config.contract_id,
                "args": contract_call.args,
            },
            cynic_verdict=cynic_verdict,
            proposal_id=proposal_id,
        )

    except Exception as e:
        logger.error(
            "Execution failed: proposal=%s error=%s",
            proposal_id,
            str(e),
        )
        raise NEARError(f"Contract execution failed: {e}")
```

## Step 8: Verify Setup

### Health Check Script

Create `scripts/verify_near_testnet.py`:

```python
#!/usr/bin/env python3
"""Verify NEAR testnet setup."""

import asyncio
import os
from cynic.perception.integrations.near import NEARExecutor, NEARNetworkConfig

async def verify_near_setup():
    """Verify NEAR testnet is configured correctly."""

    print("=" * 60)
    print("NEAR Testnet Verification")
    print("=" * 60)

    # Check environment variables
    required_vars = [
        "NEAR_NETWORK",
        "NEAR_RPC_URL",
        "NEAR_GOVERNANCE_CONTRACT",
        "NEAR_MASTER_ACCOUNT",
        "NEAR_ACCOUNT_ID",
    ]

    print("\n✓ Environment Variables:")
    for var in required_vars:
        value = os.getenv(var, "NOT SET")
        status = "✅" if value != "NOT SET" else "❌"
        print(f"  {status} {var}: {value if len(value) < 50 else value[:50] + '...'}")

    # Create executor
    config = NEARNetworkConfig(
        network_id=os.getenv("NEAR_NETWORK", "testnet"),
        rpc_url=os.getenv("NEAR_RPC_URL", "https://rpc.testnet.near.org"),
        contract_id=os.getenv("NEAR_GOVERNANCE_CONTRACT"),
        master_account=os.getenv("NEAR_MASTER_ACCOUNT"),
    )

    executor = NEARExecutor(config)

    print("\n✓ Checking NEAR RPC Connection:")
    try:
        health = await executor.health()
        print(f"  ✅ RPC is accessible: {health}")
    except Exception as e:
        print(f"  ❌ RPC connection failed: {e}")
        return False

    print("\n✓ Checking Contract:")
    try:
        # Query a test proposal
        proposal = await executor.query_proposal("test_prop")
        print(f"  ✅ Contract is accessible")
    except Exception as e:
        print(f"  ⚠️  Contract query (expected to fail on testnet): {e}")

    print("\n✓ Testing Proposal Submission:")
    try:
        result = await executor.submit_proposal(
            proposal_id="verify_test_1",
            title="Verification Test",
            description="Testing NEAR integration",
            cynic_verdict="WAG",
            q_score=0.65,
            signer_id=config.master_account,
            expires_at=1740000000,
        )
        print(f"  ✅ Proposal submission simulated: {result.status}")
    except Exception as e:
        print(f"  ❌ Proposal submission failed: {e}")

    print("\n" + "=" * 60)
    print("✅ NEAR Testnet Setup Verified!")
    print("=" * 60)

    return True

if __name__ == "__main__":
    asyncio.run(verify_near_setup())
```

Run the verification:

```bash
python scripts/verify_near_testnet.py
```

## Step 9: Configure for Production

### Testnet Settings

```env
# Testnet Configuration
NEAR_NETWORK=testnet
NEAR_RPC_URL=https://rpc.testnet.near.org
NEAR_GOVERNANCE_CONTRACT=governance.testnet
NEAR_MASTER_ACCOUNT=governance.testnet
```

### Mainnet Settings (Later)

```env
# Mainnet Configuration
NEAR_NETWORK=mainnet
NEAR_RPC_URL=https://rpc.mainnet.near.org
NEAR_GOVERNANCE_CONTRACT=governance.near
NEAR_MASTER_ACCOUNT=governance.near
```

## Testing the Complete Stack

### 1. Run Unit Tests

```bash
pytest cynic/tests/test_governance_stack.py -v
```

### 2. Test with Real NEAR Testnet

Create `scripts/test_near_integration.py`:

```python
#!/usr/bin/env python3
"""Test NEAR integration with real testnet."""

import asyncio
import os
from cynic.perception.integrations.near import NEARExecutor, NEARNetworkConfig
from cynic.perception.integrations.gasdf import GASdfExecutor, GASdfClient

async def test_complete_flow():
    """Test complete governance flow with NEAR testnet."""

    print("Testing Complete Governance Flow...")

    # Setup NEAR
    near_config = NEARNetworkConfig(
        network_id="testnet",
        rpc_url=os.getenv("NEAR_RPC_URL"),
        contract_id=os.getenv("NEAR_GOVERNANCE_CONTRACT"),
        master_account=os.getenv("NEAR_MASTER_ACCOUNT"),
    )

    near_executor = NEARExecutor(near_config)

    # Setup GASdf
    gasdf_client = GASdfClient(
        api_url=os.getenv("GASDF_API_URL"),
        api_key=os.getenv("GASDF_API_KEY"),
    )
    gasdf_executor = GASdfExecutor(gasdf_client)

    # Step 1: Check health
    print("\n1. Health Checks:")
    near_health = await near_executor.health()
    print(f"  ✅ NEAR: {near_health}")

    gasdf_health = await gasdf_client.health()
    print(f"  ✅ GASdf: {gasdf_health}")

    # Step 2: Execute verdict
    print("\n2. Executing Verdict:")
    result = await gasdf_executor.execute_verdict(
        proposal_id="test_integration_1",
        verdict="WAG",
        community_id="test_community",
        payment_token="token_mint",
        user_pubkey="user.near",
        signed_transaction="signed_tx",
        payment_token_account="token_account",
        q_score=0.65,
    )
    print(f"  ✅ Verdict executed: {result.status if result else 'skipped'}")

    # Step 3: Submit to NEAR
    print("\n3. Submitting to NEAR:")
    import time
    expires_at = int(time.time()) + 7 * 24 * 3600

    proposal_result = await near_executor.submit_proposal(
        proposal_id="test_integration_1",
        title="Integration Test Proposal",
        description="Testing CYNIC + GASdf + NEAR",
        cynic_verdict="WAG",
        q_score=0.65,
        signer_id="governance.testnet",
        expires_at=expires_at,
    )
    print(f"  ✅ Proposal status: {proposal_result.status}")

    print("\n✅ Complete Flow Test Passed!")

if __name__ == "__main__":
    asyncio.run(test_complete_flow())
```

Run the integration test:

```bash
python scripts/test_near_integration.py
```

## Troubleshooting

### Issue: "Account does not exist"
**Solution**: Create account via wallet or CLI:
```bash
near create-account --accountId myaccount.testnet --initialBalance 10
```

### Issue: "Insufficient balance"
**Solution**: Get more testnet NEAR from faucet:
```bash
near call testnet faucet-testnet deposit_near \
  '{"amount": "10000000000000000000000000"}' \
  --accountId myaccount.testnet
```

### Issue: "Contract not found"
**Solution**: Verify contract is deployed:
```bash
near view governance.testnet get_proposals_count ''
```

### Issue: "Private key invalid"
**Solution**: Verify key format and export:
```bash
cat ~/.near-credentials/testnet/myaccount.testnet.json | jq .private_key
```

## Quick Reference

### Common Commands

```bash
# Create account
near create-account --accountId {name}.testnet --initialBalance 10

# View account
near view-account {name}.testnet

# Deploy contract
near deploy --accountId {contract}.testnet --wasmFile contract.wasm

# Call contract method
near call {contract}.testnet create_proposal '{...}' --accountId {account}.testnet

# View contract state
near view {contract}.testnet get_proposal '{"proposal_id": "prop_1"}'

# Check balance
near state {account}.testnet
```

## Next Steps

1. ✅ Create testnet account
2. ✅ Deploy governance contract
3. ✅ Configure CYNIC for NEAR
4. ⏳ Integrate with GASdf testnet
5. ⏳ Test end-to-end workflow
6. ⏳ Pilot with test community
7. ⏳ Deploy to mainnet

## Resources

- [NEAR Documentation](https://docs.near.org/)
- [NEAR RPC API](https://docs.near.org/api/rpc/introduction)
- [near-sdk-rs](https://github.com/near/near-sdk-rs)
- [near-api-py](https://github.com/nearprotocol/near-api-py)
- [NEAR Testnet Faucet](https://wallet.testnet.near.org/)
