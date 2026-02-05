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
| `ComplexityTier` | ComplexityTier implementation |
| `TransformIntent` | TransformIntent implementation |
| `TransformStatus` | TransformStatus implementation |
| `LLMRouter` | LLMRouter implementation |
| `SONA` | SONA implementation |
| `TokenOptimizer` | TokenOptimizer implementation |
| `CompressionStrategy` | CompressionStrategy implementation |
| `ABBREVIATIONS` | ABBREVIATIONS implementation |
| `HyperbolicSpace` | HyperbolicSpace implementation |
| `PoincareOperations` | PoincareOperations implementation |
| `WorkerPool` | WorkerPool implementation |
| `WorkerTask` | WorkerTask implementation |
| `TaskStatus` | TaskStatus implementation |

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
| `createTokenOptimizer()` | Create TokenOptimizer instance |
| `createHyperbolicSpace()` | Create HyperbolicSpace instance |
| `createWorkerPool()` | Create WorkerPool instance |

### Singletons

| Function | Description |
|----------|-------------|
| `getPublicIdentity()` | Get PublicIdentity singleton |
| `getEScoreBreakdown()` | Get EScoreBreakdown singleton |
| `getAllDimensions()` | Get AllDimensions singleton |
| `getDimensionsForAxiom()` | Get DimensionsForAxiom singleton |
| `getDimension()` | Get Dimension singleton |
| `getCollectivePack()` | Get CollectivePack singleton |
| `getCollectivePackAsync()` | Get CollectivePackAsync singleton |
| `getSharedMemory()` | Get SharedMemory singleton |
| `getQLearningServiceSingleton()` | Get QLearningServiceSingleton singleton |
| `getSingletonStatus()` | Get SingletonStatus singleton |
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
| `getWorkerPool()` | Get WorkerPool singleton |

### Constants

`SKEPTIC_CONSTANTS`, `CONTEXT_DOMAIN_MAP`, `COLLECTIVE_CONSTANTS`, `CYNIC_CONSTANTS`, `ANCHOR_CONSTANTS`, `DEFAULT_CONFIG`, `BURN_ADDRESSES`, `AWARENESS_THRESHOLDS`, `MAX_CONFIDENCE`, `SIGNIFICANCE_THRESHOLDS` + 18 more

### Functions

`importIdentity`, `exportIdentity`, `calculateCompositeEScore`, `updateEScoreState`, `dimensionRegistry`, `serialize`, `deserialize`, `isValidMessage`, `initializeQLearning`, `awakenCynic` + 20 more

## Dependencies

**CYNIC**: @cynic/anchor, @cynic/burns, @cynic/core, @cynic/emergence, @cynic/identity, @cynic/protocol
**External**: blessed, chalk, commander, express, ws

## Stats

- **Source files**: 176
- **Test files**: 50
- **Test ratio**: 28%
- **Exports**: 212 named

## Fractal Structure

- **Depth**: 5 (root)
- **Children**: anchor → burns → core → emergence → identity → protocol

## Dimensions (4 Axioms)

```
[███░░░░░░░] 28% φ (Confidence)
[█████░░░░░] 50% Verify
[█████░░░░░] 50% Culture
[██████░░░░] 60% Burn (Simplicity)
[██████░░░░] 62% Emergence
```

*Max 62% (φ⁻¹) - certainty is hubris*

---

*Auto-generated by CYNIC meta-cognition. "φ distrusts φ" - κυνικός*
