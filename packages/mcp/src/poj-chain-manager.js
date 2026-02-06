/**
 * PoJ Chain Manager - Re-export for backwards compatibility
 *
 * The implementation has been extracted to poj-chain/ directory:
 *   - poj-chain/crypto-utils.js   - SHA-256, Merkle tree
 *   - poj-chain/anchor-manager.js - Solana anchoring
 *   - poj-chain/p2p-consensus.js  - Distributed finality
 *   - poj-chain/block-validator.js - Block verification
 *   - poj-chain/chain-exporter.js - Export/import
 *   - poj-chain/index.js          - Thin orchestrator
 *
 * "The chain remembers, the dog forgets" - κυνικός
 *
 * @module @cynic/mcp/poj-chain-manager
 * @deprecated Use '@cynic/mcp/poj-chain' instead
 */

'use strict';

// Re-export everything from new location
export {
  PoJChainManager,
  AnchorStatus,
  BlockchainEvent,
  sha256,
  merkleRoot,
  OperatorRegistry,
} from './poj-chain/index.js';

// Default export for backwards compatibility
export { default } from './poj-chain/index.js';
