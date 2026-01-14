/**
 * @cynic/protocol - CYNIC 4-Layer Protocol
 *
 * A decentralized collective consciousness protocol
 *
 * "φ distrusts φ" - κυνικός
 *
 * ## 4 Layers
 *
 * 1. **Proof of Judgment (PoJ)** - SHA-256 chain, Ed25519 signatures
 * 2. **Merkle Knowledge Tree** - Patterns partitioned by axiom
 * 3. **Gossip Propagation** - Fanout=13, O(log₁₃ n) scalability
 * 4. **φ-BFT Consensus** - 61.8% threshold, BURN-weighted votes
 *
 * @module @cynic/protocol
 */

'use strict';

// Layer 1: Proof of Judgment
export {
  // Block management
  BlockType,
  calculateSlot,
  slotToTimestamp,
  createGenesisBlock,
  createJudgmentBlock,
  createKnowledgeBlock,
  createGovernanceBlock,
  hashBlock,
  validateBlockStructure,
  validateBlockChain,
  // Chain management
  PoJChain,
  // Judgments
  Verdict,
  scoreToVerdict,
  createJudgment,
  validateJudgment,
  calculateResidual,
  isAnomalous,
  mergeJudgments,
} from './poj/index.js';

// Layer 2: Merkle Knowledge Tree
export {
  MerkleTree,
  KnowledgeTree,
  // Patterns
  createPattern,
  createLearning,
  validatePatternEmergence,
  checkPatternFormation,
  mergePatterns,
  calculatePatternDecay,
} from './merkle/index.js';

// Layer 3: Gossip Propagation
export {
  MessageType,
  generateMessageId,
  createMessage,
  verifyMessage,
  shouldRelay,
  prepareRelay,
  createBlockMessage,
  createSyncRequest,
  createSyncResponse,
  createHeartbeat,
  createPeerAnnounce,
  // Peers
  PeerStatus,
  createPeerInfo,
  PeerManager,
  // Protocol
  GossipProtocol,
} from './gossip/index.js';

// Layer 4: φ-BFT Consensus
export {
  // Voting
  VoteType,
  ConsensusType,
  calculateVoteWeight,
  generateVoteId,
  createVote,
  verifyVote,
  calculateConsensus,
  checkSoftConsensus,
  aggregateVoteRounds,
  // Lockout
  VoterLockout,
  LockoutManager,
  calculateTotalLockout,
  confirmationsForLockout,
  // Proposals
  ProposalAction,
  ProposalStatus,
  generateProposalId,
  createProposal,
  verifyProposal,
  validateProposal,
  addVoteToProposal,
  finalizeProposal,
  createAddDimensionProposal,
  createParameterChangeProposal,
} from './consensus/index.js';

// Crypto utilities
export {
  sha256,
  sha256Prefixed,
  chainHash,
  phiSaltedHash,
  randomHex,
  merkleRoot,
  hashObject,
  // Signatures
  generateKeypair,
  signData,
  verifySignature,
  signBlock,
  verifyBlock,
  formatPublicKey,
} from './crypto/index.js';
