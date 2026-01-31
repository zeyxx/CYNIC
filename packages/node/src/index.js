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
  // Engine Integration (73 philosophy engines)
  EngineIntegration,
  EngineConsultation,
  createEngineIntegration,
  CONTEXT_DOMAIN_MAP,
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
// @deprecated Import directly from @cynic/anchor instead
// These re-exports will be removed in v2.0
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
// @deprecated Import directly from @cynic/burns instead
// These re-exports will be removed in v2.0
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
// @deprecated Import directly from @cynic/identity instead
// These re-exports will be removed in v2.0
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Key management
  KeyManager,
  createKeyManager,
  generateKeypair,
  deriveNodeId,
  // E-Score (from @cynic/identity) - NOTE: Use 7D version for new code
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

// ═══════════════════════════════════════════════════════════════════════════════
// Memory Layer - 6-Layer Hybrid Context Architecture
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // SharedMemory (Layer 2: Collective + Layer 3: Procedural)
  SharedMemory,
  // UserLab (Layer 4: Personal context)
  UserLab,
  LabManager,
} from './memory/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Dog Orchestrator - Parallel subagent spawning with context injection
// ═══════════════════════════════════════════════════════════════════════════════
export {
  DogOrchestrator,
  DogMode,
  DogModel,
  DOG_CONFIG,
} from './agents/orchestrator.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Services - Autonomous Daemon, Event Bus, and Automation
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Autonomous Daemon
  AutonomousDaemon,
  createAutonomousDaemon,
  registerTaskHandler,
  // Event Bus
  EventBus,
  EventType,
  createEventBus,
  getEventBus,
  publish,
  subscribe,
  // Automation Executor
  AutomationExecutor,
  createAutomationExecutor,
} from './services/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Orchestration - Unified Coordination Layer (Phase 19)
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Decision Event Model
  DecisionEvent,
  DecisionStage,
  DecisionOutcome,
  EventSource,
  createFromHook,
  createFromTool,
  // Unified Orchestrator
  UnifiedOrchestrator,
  createUnifiedOrchestrator,
  getOrchestrator,
  // Skill Registry
  SkillRegistry,
  createSkillRegistry,
  // Decision Tracer
  DecisionTracer,
  StorageMode,
  createDecisionTracer,
} from './orchestration/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Routing - Intelligent Request Routing (Claude Flow Integration)
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Complexity Classification
  ComplexityClassifier,
  createComplexityClassifier,
  ComplexityTier,
  COMPLEXITY_THRESHOLDS,
  // Tiered Router
  TieredRouter,
  createTieredRouter,
  HANDLER_COSTS,
  HANDLER_LATENCIES,
  // Agent Booster (fast code transforms)
  AgentBooster,
  createAgentBooster,
  TransformIntent,
  TransformStatus,
} from './routing/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Learning - Self-Optimizing Neural Adaptation (SONA)
// ═══════════════════════════════════════════════════════════════════════════════
export {
  SONA,
  createSONA,
  SONA_CONFIG,
} from './learning/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Optimization - Token Optimization & Compression
// ═══════════════════════════════════════════════════════════════════════════════
export {
  TokenOptimizer,
  createTokenOptimizer,
  OPTIMIZER_CONFIG,
  CompressionStrategy,
  ABBREVIATIONS,
} from './optimization/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Embeddings - Hyperbolic Space for Hierarchical Data
// ═══════════════════════════════════════════════════════════════════════════════
export {
  HyperbolicSpace,
  createHyperbolicSpace,
  PoincareOperations,
  HYPERBOLIC_CONFIG,
} from './embeddings/index.js';
