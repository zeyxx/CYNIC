# IGNIS: Burn-as-a-Service Protocol

Ignis is an autonomous, permissionless protocol executing continuous buyback & burn operations on Solana. It is designed following the CYNIC architecture laws.

## 1. Architectural Triad

To respect the "Organic Always Wins" and "Permissionless" mandates, Ignis is divided into three distinct biological layers:

### A. The On-Chain Program (Pinocchio / eBPF)
- **Role:** The unbreakable physical membrane. It holds funds in a Vault PDA and exposes a strictly permissionless `crank_buyback_and_burn` instruction. Written in pure **Pinocchio** (Zero-Dependency Rust) to interact directly with eBPF syscalls, dividing Compute Units and binary size by a factor of 10. This is the absolute peak of "Organic Always Wins".
- **Mechanics:** 
  - Anyone can call the instruction.
  - It receives a CPI payload containing routing instructions (e.g., Jupiter swap data).
  - It verifies the outcome: if the purchased tokens are not immediately sent to the `Burn` address, the transaction reverts.

### B. The Off-Chain Keeper (TypeScript / Node)
- **Role:** The automated nervous reflex. 
- **Mechanics:** 
  - Runs a rapid loop observing the Liquidity Pool and internal signals.
  - Fetches the optimal swap route via the Jupiter API.
  - Submits the `crank_buyback_and_burn` transaction. 
  - Written in TypeScript to leverage `@solana/web3.js` and Jupiter's SDK at maximum velocity.

### C. The Intelligence / ML Layer (Python / UV Organ)
- **Role:** The analytical brain (Homeostasis).
- **Mechanics:** 
  - Analyzes market conditions (volume, volatility, sentiment) using deterministic code and ML models.
  - Dynamically adjusts the "Burn Pressure" parameters by pushing configurations to the Keeper via the CYNIC internal bus.

## 2. Directory Structure

```text
services/ignis/
├── program/        (Pinocchio: The Solana eBPF Smart Contract)
├── client/         (TypeScript: The Keeper Bot)
└── intelligence/   (Python: CYNIC Organ / ML heuristics)
```

## 3. Data Flow (Constitution Compliant)
1. `intelligence` processes market states into `.jsonl` (Append-Only) and computes a burn strategy.
2. `intelligence` sends a JSON command to `client` via the Mempool Bus.
3. `client` builds the CPI payload via Jupiter and calls the `program`.
4. `program` verifies the cryptographic constraints and burns the token.

## 4. Scalability (The Factory Pattern)
To scale this across thousands of Pump.fun tokens without deploying a new contract each time, the Rust Program operates as a **Factory**:
- A new project initializes an Ignis Vault by calling `initialize_vault(mint)`.
- The program derives a unique PDA (Program Derived Address) using the token's mint address as a seed: `PDA = hash("vault", mint_address)`.
- The project configures Pump.fun to send the Creator Fees to this specific PDA.
- The Keeper Bot iterates over all active PDAs and cranks them autonomously.

## 5. Verification & Gamification (The Holdex Vector)
The hardest part of tokenomics is visibility. A trustless burn is useless if the market doesn't know about it. 
- **The API / Indexer:** The CYNIC `intelligence` layer will expose a deterministic `/ignis/verified` REST endpoint. It reads the blockchain state and mathematically proves that a token's Pump.fun Creator Fees are locked to an Ignis PDA.
- **The Integration:** Platforms like **Holdex** (developed by @gcrtrd) can consume this API. If Holdex accepts the integration, they can display an "Ignis Verified" or "Auto-Burn Enabled" badge natively on their UI.
- **Value Loop:** This gamifies the burn. Projects will compete to have the Ignis badge to prove they aren't rugging, driving mass adoption to our protocol and massive transaction volume to our Keeper.
