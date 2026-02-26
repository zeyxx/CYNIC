# NEAR Governance Contract Setup

## Prerequisites

- Rust toolchain: `rustup install stable` and `rustup target add wasm32-unknown-unknown`
- NEAR CLI: `npm install -g near-cli`
- NEAR testnet account with funds

## Building Contract

```bash
cd contracts
cargo build --release --target wasm32-unknown-unknown
```

Output: `target/wasm32-unknown-unknown/release/governance_contract.wasm`

## Deploying to Testnet

1. Set account:
```bash
export NEAR_ENV=testnet
near login
```

2. Deploy:
```bash
near deploy --accountId governance.testnet \
  --wasmFile contracts/target/wasm32-unknown-unknown/release/governance_contract.wasm \
  --initFunction new --initArgs '{"owner":"governance.testnet"}'
```

3. Verify:
```bash
near view governance.testnet get_proposal '{"id":"test"}'
```

## Contract Methods

- `create_proposal(id, title, description)` - Create proposal
- `vote(proposal_id, choice)` - Vote on proposal (true=for, false=against)
- `get_proposal(id)` - Get proposal details
- `execute_proposal(id)` - Execute approved proposal
