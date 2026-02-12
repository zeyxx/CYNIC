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
  // C1.5: DPO Training Pipeline (CODE × LEARN)
  DPOProcessor,
  DPOOptimizer,
  // C6.7: Residual Governance (CYNIC × EMERGE)
  ResidualGovernance,
  CalibrationTracker,
  // Learning Scheduler (runs DPO + Governance daily)
  LearningScheduler,
  getLearningScheduler,
  resetLearningScheduler,
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
// Collective Singleton - "One pack, one truth"
// Use these instead of createCollectivePack() directly
// ═══════════════════════════════════════════════════════════════════════════════
export {
  getCollectivePack,
  getCollectivePackAsync,
  getSharedMemory,
  getReasoningBank, // FIX P5: Export ReasoningBank singleton
  getQLearningServiceSingleton,
  initializeQLearning,
  awakenCynic,
  saveState as saveCollectiveState,
  getSingletonStatus,
  isReady as isCollectiveReady,
  _resetForTesting as _resetCollectiveForTesting,
  // C6.1: Dog State Emitter (CYNIC × PERCEIVE)
  getDogStateEmitterSingleton,
  // C1.5 + C6.7: Learning Scheduler (DPO + Governance)
  getLearningSchedulerSingleton,
  // Unified Learning Bridge (Judge → UnifiedSignal)
  getUnifiedBridgeSingleton,
  // C5.*: Symbiosis Layer (Human × CYNIC)
  getHumanAdvisorSingleton,
  getHumanLearningSingleton,
  getHumanAccountantSingleton,
  getHumanEmergenceSingleton,
  // C7.*: Cosmos Pipeline (C7.2-C7.5) singletons
  getCosmosJudgeSingleton,
  getCosmosDeciderSingleton,
  getCosmosActorSingleton,
  getCosmosLearnerSingleton,
  // AXE 2 (PERSIST): Event Listeners - Close data loops
  startEventListeners,
  stopEventListeners,
  getListenerStats,
  cleanupOldEventData,
  wireSolanaEventListeners,
} from './collective-singleton.js';

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
  // Skeptic Service - Kabbalistic verification
  SkepticService,
  createSkepticService,
  ClaimType,
  VerificationStatus,
  SkepticVerdict,
  // Orchestration Visibility
  OrchestrationVisibility,
  VisibilityLevel,
  createOrchestrationVisibility,
  getOrchestrationVisibility,
  // HeartbeatService - Continuous health monitoring (AXE 5: OBSERVE)
  HeartbeatService,
  HealthStatus,
  createHeartbeatService,
  createDefaultChecks,
  getHeartbeatService,
  // SLATracker - 99.9% uptime compliance (AXE 5: OBSERVE)
  SLATracker,
  SLAStatus,
  SLA_TARGETS,
  createSLATracker,
  getSLATracker,
  // ConsciousnessBridge - System health to awareness (AXE 5: OBSERVE)
  ConsciousnessBridge,
  ObservationType,
  createConsciousnessBridge,
  getConsciousnessBridge,
  wireConsciousness,
  // EmergenceDetector - Cross-session pattern analysis (AXE 6: EMERGE)
  EmergenceDetector,
  PatternCategory,
  SignificanceLevel,
  createEmergenceDetector,
  getEmergenceDetector,
  // BrainService - Fully configured Brain with orchestrators (CONSCIOUSNESS)
  BrainService,
  BRAIN_CONFIG,
  getBrainService,
  getConfiguredBrain,
  thinkWithBrain,
  _resetBrainServiceForTesting,
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
  // Q-Learning Service (AXE 2: PERSIST)
  QLearningService,
  QLearningQTable,
  QLearningStateFeatures,
  LEARNING_CONFIG,
  createQLearningService,
  getQLearningService,
  getQLearningServiceAsync,
  _resetQLearningServiceForTesting,
} from './orchestration/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Metrics - Data-Driven Organism Measurement
// ═══════════════════════════════════════════════════════════════════════════════
export { MetricsDashboard } from './metrics/dashboard.js';

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
  // Cost Optimizer
  CostOptimizer,
  createCostOptimizer,
  getCostOptimizer,
} from './routing/index.js';

// LLM Router - Now in @cynic/llm (Task #21)
export {
  LLMRouter,
  createLLMRouter,
  getLLMRouter,
  _resetLLMRouterForTesting,
} from '@cynic/llm';

// ═══════════════════════════════════════════════════════════════════════════════
// Learning - Self-Optimizing Neural Adaptation (SONA) + Unified Pipeline
// ═══════════════════════════════════════════════════════════════════════════════
export {
  SONA,
  createSONA,
  SONA_CONFIG,
  // Behavior Modifier (feedback → behavior changes)
  BehaviorModifier,
  createBehaviorModifier,
  // Meta-Cognition (self-monitoring + strategy switching)
  MetaCognition,
  createMetaCognition,
  getMetaCognition,
  // Unified Learning Pipeline
  UnifiedSignal,
  UnifiedSignalStore,
  SignalSource,
  SignalOutcome,
  getUnifiedSignalStore,
  resetUnifiedSignalStore,
  UnifiedBridge,
  getUnifiedBridge,
  resetUnifiedBridge,
  // Model Intelligence (Thompson Sampling over LLM models)
  ModelIntelligence,
  ModelTier,
  TaskCategory,
  getModelIntelligence,
  resetModelIntelligence,
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

// ═══════════════════════════════════════════════════════════════════════════════
// Workers - Background Task Processing (O4.2)
// ═══════════════════════════════════════════════════════════════════════════════
export {
  WorkerPool,
  WorkerTask,
  TaskStatus,
  WORKER_CONFIG,
  createWorkerPool,
  getWorkerPool,
  resetWorkerPool,
  registerBuiltinHandlers,
  BUILTIN_TASK_TYPES,
} from './workers/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Perception Layer - Multi-Dimensional Awareness (BATCH 4)
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Filesystem Watcher
  FilesystemWatcher,
  createFilesystemWatcher,
  FilesystemEventType,
  // Solana Watcher - C2.1 (SOLANA × PERCEIVE)
  SolanaWatcher,
  createSolanaWatcher,
  SolanaEventType,
  getSolanaWatcher,
  resetSolanaWatcher,
  // Unified Perception
  createPerceptionLayer,
  // C6.1: Dog State Emitter (CYNIC × PERCEIVE)
  DogStateEmitter,
  DogStateType,
  getDogStateEmitter,
  resetDogStateEmitter,
} from './perception/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Workflow Tracker - C1.3 (CODE × DECIDE) Multi-step dangerous workflow detection
// "Le chien voit les séquences, pas juste les commandes"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  WorkflowTracker,
  WorkflowStatus,
  getWorkflowTracker,
  resetWorkflowTracker,
} from './agents/collective/workflow-tracker.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Accounting Layer - 7×7 Fractal Matrix ACCOUNT Column (C*.6)
// "Le chien compte ce qui compte" - Economic tracking for CODE and CYNIC
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // C1.6: CODE × ACCOUNT
  CodeAccountant,
  RiskLevel as CodeRiskLevel, // Aliased - RiskLevel already exported from agents
  getCodeAccountant,
  resetCodeAccountant,
  // C6.6: CYNIC × ACCOUNT
  CynicAccountant,
  OperationType,
  getCynicAccountant,
  resetCynicAccountant,
  // C4.6: SOCIAL × ACCOUNT
  SocialAccountant,
  InteractionType,
  getSocialAccountant,
  resetSocialAccountant,
  // C7.6: COSMOS × ACCOUNT
  CosmosAccountant,
  ValueFlowType,
  getCosmosAccountant,
  resetCosmosAccountant,
  // Cross-cutting cost accounting
  CostLedger,
  ModelId,
  BudgetStatus,
  getCostLedger,
  resetCostLedger,
} from './accounting/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Code Emergence - 7×7 Fractal Matrix EMERGE Column (C1.7)
// "Le code révèle ses patterns cachés" - Cross-session pattern detection
// ═══════════════════════════════════════════════════════════════════════════════
export {
  CodeEmergence,
  CodePatternType,
  getCodeEmergence,
  resetCodeEmergence,
} from './emergence/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Symbiosis Layer - Human-CYNIC Interface (C5.*)
// "Le chien amplifie l'humain, l'humain guide le chien"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // C5.3: HUMAN × DECIDE
  HumanAdvisor,
  InterventionType,
  UrgencyLevel,
  getHumanAdvisor,
  resetHumanAdvisor,
  // C5.5: HUMAN × LEARN
  HumanLearning,
  LearningCategory,
  getHumanLearning,
  resetHumanLearning,
  // C5.6: HUMAN × ACCOUNT
  HumanAccountant,
  ActivityType as HumanActivityType, // Aliased - ActivityType may conflict
  getHumanAccountant,
  resetHumanAccountant,
  // C5.7: HUMAN × EMERGE
  HumanEmergence,
  HumanPatternType,
  HumanSignificanceLevel,
  getHumanEmergence,
  resetHumanEmergence,
} from './symbiosis/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Solana Module - SOLANA Row of 7×7 Matrix (C2.*)
// "On-chain is truth"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // C2.2: SOLANA × JUDGE
  SolanaJudge,
  SolanaJudgmentType,
  getSolanaJudge,
  resetSolanaJudge,
  // C2.3: SOLANA × DECIDE
  SolanaDecider,
  SolanaDecisionType,
  PriorityLevel,
  getSolanaDecider,
  resetSolanaDecider,
  // C2.4: SOLANA × ACT
  SolanaActor,
  SolanaActionType,
  ActionStatus as SolanaActionStatus, // Aliased to avoid conflict
  getSolanaActor,
  resetSolanaActor,
  // C2.5: SOLANA × LEARN
  SolanaLearner,
  SolanaLearningCategory,
  getSolanaLearner,
  resetSolanaLearner,
  // C2.6: SOLANA × ACCOUNT
  SolanaAccountant,
  SolanaTransactionType,
  getSolanaAccountant,
  resetSolanaAccountant,
  // C2.7: SOLANA × EMERGE
  SolanaEmergence,
  SolanaEmergencePattern,
  SolanaSignificance,
  getSolanaEmergence,
  resetSolanaEmergence,
} from './solana/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Cosmos Module - COSMOS Row of 7×7 Matrix (C7.2-C7.5)
// "Le chien voit les étoiles" - Ecosystem-level awareness
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // C7.2: COSMOS × JUDGE
  CosmosJudge,
  CosmosJudgmentType,
  getCosmosJudge,
  resetCosmosJudge,
  // C7.3: COSMOS × DECIDE
  CosmosDecider,
  CosmosDecisionType,
  getCosmosDecider,
  resetCosmosDecider,
  // C7.4: COSMOS × ACT
  CosmosActor,
  CosmosActionType,
  CosmosActionStatus,
  getCosmosActor,
  resetCosmosActor,
  // C7.5: COSMOS × LEARN
  CosmosLearner,
  CosmosLearningCategory,
  getCosmosLearner,
  resetCosmosLearner,
} from './cosmos/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Inference Module - Bayesian reasoning
// "Mettre à jour les croyances avec l'évidence"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Core Bayes
  bayesTheorem,
  computeMarginal,
  updateBelief,
  batchUpdateBelief,
  // Multi-hypothesis
  Hypothesis,
  HypothesisSet,
  createHypothesisSet,
  // Beta-Binomial
  BetaDistribution,
  createBetaTracker,
  // Naive Bayes
  NaiveBayesClassifier,
  createClassifier,
  // Belief Networks
  BeliefNode,
  BeliefNetwork,
  createBeliefNetwork,
  // Utilities
  likelihoodRatio,
  probabilityToOdds,
  oddsToProbability,
  logOdds,
  sigmoid,
  updateOdds,
  // Config
  BAYES_CONFIG,
  // Poisson (rare events)
  factorial,
  logFactorial,
  poissonPMF,
  poissonCDF,
  poissonSurvival,
  poissonQuantile,
  poissonMean,
  poissonVariance,
  poissonStdDev,
  estimateRate,
  rateConfidenceInterval,
  detectAnomaly,
  anomalyScore,
  PoissonProcess,
  EventRateTracker,
  poissonGoodnessOfFit,
  waitingTimeCDF,
  timeToNEvents,
  createPoissonProcess,
  createEventTracker,
  POISSON_CONFIG,
  // Gaussian (noise, distributions)
  gaussianPDF,
  gaussianLogPDF,
  gaussianCDF,
  gaussianSurvival,
  gaussianQuantile,
  erf,
  erfc,
  zScore,
  fromZScore,
  standardize,
  zScoreToPValue,
  pValueToZScore,
  confidenceInterval,
  phiConfidenceInterval,
  confidenceToZScore,
  randomStandardNormal,
  randomNormal,
  randomNormalArray,
  randomCorrelatedNormal,
  addNoise,
  addNoiseArray,
  GaussianNoiseGenerator,
  createNoiseGenerator,
  gaussianKernel,
  gaussianKernelScaled,
  silvermanBandwidth,
  kernelDensityEstimate,
  kde,
  GaussianDistribution,
  DiagonalGaussian,
  createGaussian,
  standardNormal,
  computeStats,
  skewness,
  kurtosis,
  jarqueBeraTest,
  GAUSSIAN_CONSTANTS,
  GAUSSIAN_CONFIG,
} from './inference/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Organism Module - 7 Dimensions of CYNIC's Life Force
// "Le chien est vivant, pas juste actif"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Metabolism
  MetabolicSample,
  MetabolismTracker,
  createMetabolismTracker,
  getMetabolismTracker,
  resetMetabolismTracker,
  METABOLISM_CONFIG,
  // Thermodynamics
  ThermoEvent,
  ThermoEventType,
  ThermodynamicState,
  createThermodynamicState,
  getThermodynamicState,
  resetThermodynamicState,
  THERMO_CONFIG,
  // Homeostasis
  MetricBaseline,
  HomeostasisTracker,
  createHomeostasisTracker,
  getHomeostasisTracker,
  resetHomeostasisTracker,
  HOMEOSTASIS_CONFIG,
  // Growth
  GrowthEvent,
  GrowthTracker,
  createGrowthTracker,
  getGrowthTracker,
  resetGrowthTracker,
  GROWTH_CONFIG,
  // Resilience
  Incident,
  ResilienceTracker,
  createResilienceTracker,
  getResilienceTracker,
  resetResilienceTracker,
  RESILIENCE_CONFIG,
  // Vitality
  VitalityMonitor,
  calculateVitality,
  getVitalityStatus,
  getDimensionScores,
  getDimensionStatuses,
  getVitalitySummary,
  createVitalityMonitor,
  getVitalityMonitor,
  resetVitalityMonitor,
  VITALITY_CONFIG,
  // Unified interface
  getOrganismTrackers,
  getOrganismState,
  getOrganismHealth,
  resetOrganismState,
  startOrganismMonitoring,
  stopOrganismMonitoring,
  recordMetabolic,
  recordThermo,
  recordSuccess,
  recordError,
  recordGrowth,
  updateHomeostasis,
  markRecovered,
} from './organism/index.js';

// Registry - Auto-discovery and auto-wiring
export {
  ComponentRegistry,
  ComponentCategory,
  ComponentMetadata,
  getRegistry,
  createComponentMarker,
  AutoWirer,
  initAutoWiring,
} from './registry/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Network Module - PHASE 2: DECENTRALIZE
// "The pack hunts together"
// ═══════════════════════════════════════════════════════════════════════════════
export {
  // Multi-Node Orchestrator
  CYNICNetworkNode,
  NetworkState,
  // Extracted SRP components
  ForkDetector,
  ValidatorManager,
  SolanaAnchoringManager,
  StateSyncManager,
  BlockProducer,
  // Component re-exports for convenience
  PeerDiscovery,
  DiscoveryState,
  TransportComponent,
  ConsensusComponent,
  ConsensusState,
  BlockStatus,
} from './network/index.js';

// PHASE 2: Network singleton
export {
  getNetworkNode,
  startNetworkNode,
  stopNetworkNode,
  getNetworkStatus,
  isP2PEnabled,
} from './collective-singleton.js';
