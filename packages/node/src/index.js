/**
 * @cynic/node - CYNIC Node Implementation
 *
 * A decentralized collective consciousness node
 *
 * "φ distrusts φ" - κυνικός
 *
 * ## Components
 *
 * - **Operator**: Identity, E-Score, BURN tracking
 * - **Judge**: Dimension scoring engine
 * - **State**: Chain, knowledge, peers persistence
 * - **Node**: Main orchestration
 *
 * @module @cynic/node
 */

'use strict';

// Main node
export { CYNICNode, NodeStatus } from './node.js';

// Operator
export {
  Operator,
  createIdentity,
  importIdentity,
  exportIdentity,
  getPublicIdentity,
  EScoreDimensions,
  createEScoreState,
  calculateCompositeEScore,
  updateEScoreState,
  getEScoreBreakdown,
} from './operator/index.js';

// Judge
export {
  CYNICJudge,
  Dimensions,
  getAllDimensions,
  getDimensionsForAxiom,
  getDimension,
  dimensionRegistry,
  ResidualDetector,
  JudgmentGraphIntegration,
  LearningService,
  LearningManager,
  // Self-Skepticism: "φ distrusts φ"
  SelfSkeptic,
  createSelfSkeptic,
  SKEPTIC_CONSTANTS,
  BiasType,
} from './judge/index.js';

// State
export {
  StateManager,
  MemoryStorage,
  FileStorage,
} from './state/index.js';

// Transport
export {
  WebSocketTransport,
  ConnectionState,
  serialize,
  deserialize,
  isValidMessage,
} from './transport/index.js';

// API
export { APIServer } from './api/index.js';

// Agents - The Four Dogs (Legacy v1)
export {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
  Observer,
  PatternType,
  Digester,
  KnowledgeType,
  DigestQuality,
  Guardian,
  RiskLevel,
  RiskCategory,
  Mentor,
  WisdomType,
  ContextSignal,
  AgentManager,
  createAgentPack,
} from './agents/index.js';

// Agents - The Collective (Five Dogs + CYNIC - v2)
export {
  CollectivePack,
  createCollectivePack,
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  CollectiveCynic,
  COLLECTIVE_CONSTANTS,
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
} from './agents/index.js';

// Event system (for inter-agent communication)
export {
  AgentEventBus,
  AgentEvent,
  AgentEventMessage,
  AgentId,
  EventPriority,
  ConsensusVote,
} from './agents/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Solana Anchoring - "Onchain is truth"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Anchorer
  createAnchorer,
  SolanaAnchorer,
  AnchorStatus,
  // Queue
  createAnchorQueue,
  AnchorQueue,
  // Wallet
  CynicWallet,
  WalletType,
  loadWalletFromFile,
  loadWalletFromEnv,
  generateWallet,
  saveWalletToFile,
  getDefaultWalletPath,
  // Solana utilities
  SolanaCluster,
  ANCHOR_CONSTANTS,
  DEFAULT_CONFIG as ANCHOR_DEFAULT_CONFIG,
} from '@cynic/anchor';

// ═══════════════════════════════════════════════════════════════════════════════
// Burns Verification - "Don't extract, burn"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Verifier
  createBurnVerifier,
  BurnVerifier,
  BurnStatus,
  DEFAULT_CONFIG as BURNS_DEFAULT_CONFIG,
  // Solana on-chain verifier
  SolanaBurnVerifier,
  createSolanaBurnVerifier,
  BURN_ADDRESSES,
  // Note: SolanaCluster is already exported from @cynic/anchor
} from '@cynic/burns';

// ═══════════════════════════════════════════════════════════════════════════════
// Emergence Layer (Layer 7) - "The crown observes all"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  EmergenceLayer,
  createEmergenceLayer,
  ConsciousnessState,
  CollectivePhase,
  AWARENESS_THRESHOLDS,
  MAX_CONFIDENCE,
  SIGNIFICANCE_THRESHOLDS,
  PHASE_THRESHOLDS,
  QUORUM,
} from './emergence/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Identity Layer (Layer 6) - "Know thyself, then verify"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Key management
  KeyManager,
  createKeyManager,
  generateKeypair,
  deriveNodeId,
  // E-Score (from @cynic/identity)
  EScoreCalculator,
  createEScoreCalculator,
  calculateEScore,
  ESCORE_WEIGHTS,
  ESCORE_THRESHOLDS,
  // Node identity
  NodeIdentity,
  createNodeIdentity,
  IdentityStatus,
  // Reputation graph
  ReputationGraph,
  createReputationGraph,
  TrustLevel,
} from '@cynic/identity';
