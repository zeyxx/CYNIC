# @cynic/node

> CYNIC Node - Decentralized Collective Consciousness Node

**Category**: runtime | **Version**: 0.1.0 | **Quality**: 🟠 needs tests

## Installation

```bash
npm install @cynic/node
```

## Quick Start

```javascript
import { createIdentity } from '@cynic/node';

const instance = createIdentity();
```

## API Reference

### Classes

| Class | Description |
|-------|-------------|
| `CYNICNode` | CYNICNode implementation |
| `NodeStatus` | NodeStatus implementation |
| `Operator` | Operator implementation |
| `EScoreDimensions` | EScoreDimensions implementation |
| `CYNICJudge` | CYNICJudge implementation |
| `Dimensions` | Dimensions implementation |
| `ResidualDetector` | ResidualDetector implementation |
| `JudgmentGraphIntegration` | JudgmentGraphIntegration implementation |
| `LearningService` | LearningService implementation |
| `LearningManager` | LearningManager implementation |
| `BiasType` | BiasType implementation |
| `EngineConsultation` | EngineConsultation implementation |
| `DPOOptimizer` | DPOOptimizer implementation |
| `CalibrationTracker` | CalibrationTracker implementation |
| `StateManager` | StateManager implementation |
| `MemoryStorage` | MemoryStorage implementation |
| `FileStorage` | FileStorage implementation |
| `WebSocketTransport` | WebSocketTransport implementation |
| `ConnectionState` | ConnectionState implementation |
| `APIServer` | APIServer implementation |
| `BaseAgent` | BaseAgent implementation |
| `AgentTrigger` | AgentTrigger implementation |
| `AgentBehavior` | AgentBehavior implementation |
| `AgentResponse` | AgentResponse implementation |
| `Observer` | Observer implementation |
| `PatternType` | PatternType implementation |
| `Digester` | Digester implementation |
| `KnowledgeType` | KnowledgeType implementation |
| `DigestQuality` | DigestQuality implementation |
| `Guardian` | Guardian implementation |
| `RiskLevel` | RiskLevel implementation |
| `RiskCategory` | RiskCategory implementation |
| `Mentor` | Mentor implementation |
| `WisdomType` | WisdomType implementation |
| `ContextSignal` | ContextSignal implementation |
| `AgentManager` | AgentManager implementation |
| `CollectivePack` | CollectivePack implementation |
| `CollectiveGuardian` | CollectiveGuardian implementation |
| `CollectiveAnalyst` | CollectiveAnalyst implementation |
| `CollectiveScholar` | CollectiveScholar implementation |
| `CollectiveArchitect` | CollectiveArchitect implementation |
| `CollectiveSage` | CollectiveSage implementation |
| `CollectiveCynic` | CollectiveCynic implementation |
| `CynicDecisionType` | CynicDecisionType implementation |
| `CynicGuidanceType` | CynicGuidanceType implementation |
| `MetaState` | MetaState implementation |
| `AgentEventBus` | AgentEventBus implementation |
| `AgentEvent` | AgentEvent implementation |
| `AgentEventMessage` | AgentEventMessage implementation |
| `AgentId` | AgentId implementation |
| `EventPriority` | EventPriority implementation |
| `ConsensusVote` | ConsensusVote implementation |
| `SolanaAnchorer` | SolanaAnchorer implementation |
| `AnchorStatus` | AnchorStatus implementation |
| `AnchorQueue` | AnchorQueue implementation |
| `WalletType` | WalletType implementation |
| `BurnVerifier` | BurnVerifier implementation |
| `BurnStatus` | BurnStatus implementation |
| `EmergenceLayer` | EmergenceLayer implementation |
| `ConsciousnessState` | ConsciousnessState implementation |
| `CollectivePhase` | CollectivePhase implementation |
| `QUORUM` | QUORUM implementation |
| `IdentityStatus` | IdentityStatus implementation |
| `TrustLevel` | TrustLevel implementation |
| `LabManager` | LabManager implementation |
| `DogOrchestrator` | DogOrchestrator implementation |
| `DogMode` | DogMode implementation |
| `DogModel` | DogModel implementation |
| `EventType` | EventType implementation |
| `ClaimType` | ClaimType implementation |
| `VerificationStatus` | VerificationStatus implementation |
| `SkepticVerdict` | SkepticVerdict implementation |
| `VisibilityLevel` | VisibilityLevel implementation |
| `HealthStatus` | HealthStatus implementation |
| `SLAStatus` | SLAStatus implementation |
| `ObservationType` | ObservationType implementation |
| `PatternCategory` | PatternCategory implementation |
| `SignificanceLevel` | SignificanceLevel implementation |
| `DecisionStage` | DecisionStage implementation |
| `DecisionOutcome` | DecisionOutcome implementation |
| `EventSource` | EventSource implementation |
| `StorageMode` | StorageMode implementation |
| `QLearningQTable` | QLearningQTable implementation |
| `QLearningStateFeatures` | QLearningStateFeatures implementation |
| `MetricsDashboard` | MetricsDashboard implementation |
| `ComplexityTier` | ComplexityTier implementation |
| `TransformIntent` | TransformIntent implementation |
| `TransformStatus` | TransformStatus implementation |
| `LLMRouter` | LLMRouter implementation |
| `SONA` | SONA implementation |
| `UnifiedSignalStore` | UnifiedSignalStore implementation |
| `SignalSource` | SignalSource implementation |
| `SignalOutcome` | SignalOutcome implementation |
| `UnifiedBridge` | UnifiedBridge implementation |
| `ModelTier` | ModelTier implementation |
| `TaskCategory` | TaskCategory implementation |
| `TokenOptimizer` | TokenOptimizer implementation |
| `CompressionStrategy` | CompressionStrategy implementation |
| `ABBREVIATIONS` | ABBREVIATIONS implementation |
| `HyperbolicSpace` | HyperbolicSpace implementation |
| `PoincareOperations` | PoincareOperations implementation |
| `WorkerPool` | WorkerPool implementation |
| `WorkerTask` | WorkerTask implementation |
| `TaskStatus` | TaskStatus implementation |
| `FilesystemEventType` | FilesystemEventType implementation |
| `SolanaEventType` | SolanaEventType implementation |
| `DogStateType` | DogStateType implementation |
| `WorkflowTracker` | WorkflowTracker implementation |
| `WorkflowStatus` | WorkflowStatus implementation |
| `OperationType` | OperationType implementation |
| `InteractionType` | InteractionType implementation |
| `ValueFlowType` | ValueFlowType implementation |
| `ModelId` | ModelId implementation |
| `BudgetStatus` | BudgetStatus implementation |
| `CodeEmergence` | CodeEmergence implementation |
| `CodePatternType` | CodePatternType implementation |
| `InterventionType` | InterventionType implementation |
| `UrgencyLevel` | UrgencyLevel implementation |
| `LearningCategory` | LearningCategory implementation |
| `ActivityType` | ActivityType implementation |
| `HumanPatternType` | HumanPatternType implementation |
| `HumanSignificanceLevel` | HumanSignificanceLevel implementation |
| `SolanaJudgmentType` | SolanaJudgmentType implementation |
| `SolanaDecisionType` | SolanaDecisionType implementation |
| `PriorityLevel` | PriorityLevel implementation |
| `SolanaActionType` | SolanaActionType implementation |
| `ActionStatus` | ActionStatus implementation |
| `SolanaLearningCategory` | SolanaLearningCategory implementation |
| `SolanaTransactionType` | SolanaTransactionType implementation |
| `SolanaEmergencePattern` | SolanaEmergencePattern implementation |
| `SolanaSignificance` | SolanaSignificance implementation |
| `CosmosJudgmentType` | CosmosJudgmentType implementation |
| `CosmosDecisionType` | CosmosDecisionType implementation |
| `CosmosActionType` | CosmosActionType implementation |
| `CosmosActionStatus` | CosmosActionStatus implementation |
| `CosmosLearningCategory` | CosmosLearningCategory implementation |
| `HypothesisSet` | HypothesisSet implementation |
| `BeliefNetwork` | BeliefNetwork implementation |
| `PoissonProcess` | PoissonProcess implementation |
| `EventRateTracker` | EventRateTracker implementation |
| `GaussianNoiseGenerator` | GaussianNoiseGenerator implementation |
| `GaussianDistribution` | GaussianDistribution implementation |
| `DiagonalGaussian` | DiagonalGaussian implementation |
| `MetabolismTracker` | MetabolismTracker implementation |
| `ThermoEventType` | ThermoEventType implementation |
| `ThermodynamicState` | ThermodynamicState implementation |
| `HomeostasisTracker` | HomeostasisTracker implementation |
| `GrowthTracker` | GrowthTracker implementation |
| `ResilienceTracker` | ResilienceTracker implementation |
| `ComponentRegistry` | ComponentRegistry implementation |
| `ComponentCategory` | ComponentCategory implementation |
| `ComponentMetadata` | ComponentMetadata implementation |
| `AutoWirer` | AutoWirer implementation |
| `NetworkState` | NetworkState implementation |
| `ValidatorManager` | ValidatorManager implementation |
| `SolanaAnchoringManager` | SolanaAnchoringManager implementation |
| `StateSyncManager` | StateSyncManager implementation |
| `BlockProducer` | BlockProducer implementation |
| `DiscoveryState` | DiscoveryState implementation |
| `TransportComponent` | TransportComponent implementation |
| `ConsensusComponent` | ConsensusComponent implementation |
| `ConsensusState` | ConsensusState implementation |
| `BlockStatus` | BlockStatus implementation |

### Factory Functions

| Function | Description |
|----------|-------------|
| `createIdentity()` | Create Identity instance |
| `createEScoreState()` | Create EScoreState instance |
| `createSelfSkeptic()` | Create SelfSkeptic instance |
| `createEngineIntegration()` | Create EngineIntegration instance |
| `createAgentPack()` | Create AgentPack instance |
| `createCollectivePack()` | Create CollectivePack instance |
| `createSolanaBurnVerifier()` | Create SolanaBurnVerifier instance |
| `createEmergenceLayer()` | Create EmergenceLayer instance |
| `createKeyManager()` | Create KeyManager instance |
| `createEScoreCalculator()` | Create EScoreCalculator instance |
| `createNodeIdentity()` | Create NodeIdentity instance |
| `createReputationGraph()` | Create ReputationGraph instance |
| `createAutonomousDaemon()` | Create AutonomousDaemon instance |
| `createEventBus()` | Create EventBus instance |
| `createAutomationExecutor()` | Create AutomationExecutor instance |
| `createSkepticService()` | Create SkepticService instance |
| `createOrchestrationVisibility()` | Create OrchestrationVisibility instance |
| `createHeartbeatService()` | Create HeartbeatService instance |
| `createDefaultChecks()` | Create DefaultChecks instance |
| `createSLATracker()` | Create SLATracker instance |
| `createConsciousnessBridge()` | Create ConsciousnessBridge instance |
| `createEmergenceDetector()` | Create EmergenceDetector instance |
| `createFromHook()` | Create FromHook instance |
| `createFromTool()` | Create FromTool instance |
| `createUnifiedOrchestrator()` | Create UnifiedOrchestrator instance |
| `createSkillRegistry()` | Create SkillRegistry instance |
| `createDecisionTracer()` | Create DecisionTracer instance |
| `createQLearningService()` | Create QLearningService instance |
| `createComplexityClassifier()` | Create ComplexityClassifier instance |
| `createTieredRouter()` | Create TieredRouter instance |
| `createAgentBooster()` | Create AgentBooster instance |
| `createCostOptimizer()` | Create CostOptimizer instance |
| `createLLMRouter()` | Create LLMRouter instance |
| `createSONA()` | Create SONA instance |
| `createBehaviorModifier()` | Create BehaviorModifier instance |
| `createMetaCognition()` | Create MetaCognition instance |
| `createTokenOptimizer()` | Create TokenOptimizer instance |
| `createHyperbolicSpace()` | Create HyperbolicSpace instance |
| `createWorkerPool()` | Create WorkerPool instance |
| `createFilesystemWatcher()` | Create FilesystemWatcher instance |
| `createSolanaWatcher()` | Create SolanaWatcher instance |
| `createHypothesisSet()` | Create HypothesisSet instance |
| `createBetaTracker()` | Create BetaTracker instance |
| `createClassifier()` | Create Classifier instance |
| `createBeliefNetwork()` | Create BeliefNetwork instance |
| `createPoissonProcess()` | Create PoissonProcess instance |
| `createEventTracker()` | Create EventTracker instance |
| `createNoiseGenerator()` | Create NoiseGenerator instance |
| `createGaussian()` | Create Gaussian instance |
| `createMetabolismTracker()` | Create MetabolismTracker instance |
| `createThermodynamicState()` | Create ThermodynamicState instance |
| `createHomeostasisTracker()` | Create HomeostasisTracker instance |
| `createGrowthTracker()` | Create GrowthTracker instance |
| `createResilienceTracker()` | Create ResilienceTracker instance |
| `createVitalityMonitor()` | Create VitalityMonitor instance |
| `createComponentMarker()` | Create ComponentMarker instance |

### Singletons

| Function | Description |
|----------|-------------|
| `getPublicIdentity()` | Get PublicIdentity singleton |
| `getEScoreBreakdown()` | Get EScoreBreakdown singleton |
| `getAllDimensions()` | Get AllDimensions singleton |
| `getDimensionsForAxiom()` | Get DimensionsForAxiom singleton |
| `getDimension()` | Get Dimension singleton |
| `getLearningScheduler()` | Get LearningScheduler singleton |
| `getCollectivePack()` | Get CollectivePack singleton |
| `getCollectivePackAsync()` | Get CollectivePackAsync singleton |
| `getSharedMemory()` | Get SharedMemory singleton |
| `getReasoningBank()` | Get ReasoningBank singleton |
| `getSingletonStatus()` | Get SingletonStatus singleton |
| `getHumanLearningSingleton()` | Get HumanLearningSingleton singleton |
| `getHumanAccountantSingleton()` | Get HumanAccountantSingleton singleton |
| `getHumanEmergenceSingleton()` | Get HumanEmergenceSingleton singleton |
| `getCosmosDeciderSingleton()` | Get CosmosDeciderSingleton singleton |
| `getCosmosActorSingleton()` | Get CosmosActorSingleton singleton |
| `getCosmosLearnerSingleton()` | Get CosmosLearnerSingleton singleton |
| `getListenerStats()` | Get ListenerStats singleton |
| `getDefaultWalletPath()` | Get DefaultWalletPath singleton |
| `getEventBus()` | Get EventBus singleton |
| `getOrchestrationVisibility()` | Get OrchestrationVisibility singleton |
| `getHeartbeatService()` | Get HeartbeatService singleton |
| `getSLATracker()` | Get SLATracker singleton |
| `getConsciousnessBridge()` | Get ConsciousnessBridge singleton |
| `getEmergenceDetector()` | Get EmergenceDetector singleton |
| `getBrainService()` | Get BrainService singleton |
| `getConfiguredBrain()` | Get ConfiguredBrain singleton |
| `getOrchestrator()` | Get Orchestrator singleton |
| `getQLearningService()` | Get QLearningService singleton |
| `getQLearningServiceAsync()` | Get QLearningServiceAsync singleton |
| `getCostOptimizer()` | Get CostOptimizer singleton |
| `getLLMRouter()` | Get LLMRouter singleton |
| `getMetaCognition()` | Get MetaCognition singleton |
| `getUnifiedSignalStore()` | Get UnifiedSignalStore singleton |
| `getUnifiedBridge()` | Get UnifiedBridge singleton |
| `getModelIntelligence()` | Get ModelIntelligence singleton |
| `getWorkerPool()` | Get WorkerPool singleton |
| `getSolanaWatcher()` | Get SolanaWatcher singleton |
| `getDogStateEmitter()` | Get DogStateEmitter singleton |
| `getWorkflowTracker()` | Get WorkflowTracker singleton |
| `getCynicAccountant()` | Get CynicAccountant singleton |
| `getSocialAccountant()` | Get SocialAccountant singleton |
| `getCosmosAccountant()` | Get CosmosAccountant singleton |
| `getCostLedger()` | Get CostLedger singleton |
| `getCodeEmergence()` | Get CodeEmergence singleton |
| `getHumanAdvisor()` | Get HumanAdvisor singleton |
| `getHumanLearning()` | Get HumanLearning singleton |
| `getHumanEmergence()` | Get HumanEmergence singleton |
| `getSolanaJudge()` | Get SolanaJudge singleton |
| `getSolanaDecider()` | Get SolanaDecider singleton |
| `getSolanaLearner()` | Get SolanaLearner singleton |
| `getSolanaAccountant()` | Get SolanaAccountant singleton |
| `getSolanaEmergence()` | Get SolanaEmergence singleton |
| `getCosmosJudge()` | Get CosmosJudge singleton |
| `getCosmosDecider()` | Get CosmosDecider singleton |
| `getCosmosActor()` | Get CosmosActor singleton |
| `getCosmosLearner()` | Get CosmosLearner singleton |
| `getMetabolismTracker()` | Get MetabolismTracker singleton |
| `getThermodynamicState()` | Get ThermodynamicState singleton |
| `getHomeostasisTracker()` | Get HomeostasisTracker singleton |
| `getGrowthTracker()` | Get GrowthTracker singleton |
| `getResilienceTracker()` | Get ResilienceTracker singleton |
| `getVitalityStatus()` | Get VitalityStatus singleton |
| `getDimensionScores()` | Get DimensionScores singleton |
| `getDimensionStatuses()` | Get DimensionStatuses singleton |
| `getVitalitySummary()` | Get VitalitySummary singleton |
| `getVitalityMonitor()` | Get VitalityMonitor singleton |
| `getOrganismState()` | Get OrganismState singleton |
| `getOrganismHealth()` | Get OrganismHealth singleton |
| `getRegistry()` | Get Registry singleton |
| `getNetworkNode()` | Get NetworkNode singleton |
| `getNetworkStatus()` | Get NetworkStatus singleton |

### Constants

`SKEPTIC_CONSTANTS`, `CONTEXT_DOMAIN_MAP`, `COLLECTIVE_CONSTANTS`, `CYNIC_CONSTANTS`, `ANCHOR_CONSTANTS`, `DEFAULT_CONFIG`, `BURN_ADDRESSES`, `AWARENESS_THRESHOLDS`, `MAX_CONFIDENCE`, `SIGNIFICANCE_THRESHOLDS` + 27 more

### Functions

`importIdentity`, `exportIdentity`, `calculateCompositeEScore`, `updateEScoreState`, `dimensionRegistry`, `resetLearningScheduler`, `serialize`, `deserialize`, `isValidMessage`, `initializeQLearning` + 124 more

## Dependencies

**CYNIC**: @cynic/anchor, @cynic/burns, @cynic/core, @cynic/emergence, @cynic/identity, @cynic/protocol
**External**: blessed, chalk, chokidar, commander, express + 1 more

## Stats

- **Source files**: 318
- **Test files**: 111
- **Test ratio**: 35%
- **Exports**: 459 named

## Fractal Structure

- **Depth**: 5 (root)
- **Children**: anchor → burns → core → emergence → identity → protocol

## Dimensions (4 Axioms)

```
[███░░░░░░░] 35% φ (Confidence)
[█████░░░░░] 50% Verify
[█████░░░░░] 50% Culture
[█████░░░░░] 52% Burn (Simplicity)
[██████░░░░] 62% Emergence
```

*Max 62% (φ⁻¹) - certainty is hubris*

---

*Auto-generated by CYNIC meta-cognition. "φ distrusts φ" - κυνικός*
