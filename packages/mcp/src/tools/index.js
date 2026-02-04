/**
 * CYNIC MCP Tools
 *
 * Tool definitions for MCP server
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/mcp/tools
 */

'use strict';

import {
  PHI_INV,
  PHI_INV_2,
  THRESHOLDS,
  EMERGENCE,
  IDENTITY,
  getVerdictFromScore,
  EcosystemMonitor,
  summarizeUpdates,
} from '@cynic/core';
import { createMetaTool } from '../meta-dashboard.js';
import { createCodeAnalyzer } from '../code-analyzer.js';
import { LSPService, createLSPTools } from '../lsp-service.js';
import { JSONRenderService, createJSONRenderTool } from '../json-render.js';
import {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
} from './search-progressive.js';

// ═══════════════════════════════════════════════════════════════════════════
// OCP: Re-export from domain modules
// ═══════════════════════════════════════════════════════════════════════════

// Judgment domain (judge, refine, feedback, learning)
// Import for local use + re-export
import {
  createJudgeTool,
  createRefineTool,
  createFeedbackTool,
  createLearningTool,
  judgmentFactory,
} from './domains/judgment.js';

export {
  createJudgeTool,
  createRefineTool,
  createFeedbackTool,
  createLearningTool,
  judgmentFactory,
};

// NOTE: createJudgeTool and createRefineTool implementations moved to domains/judgment.js

// Knowledge domain (search, digest, docs)
// Import for local use + re-export
import {
  createSearchTool,
  createDigestTool,
  createDocsTool,
  knowledgeFactory,
} from './domains/knowledge.js';

export {
  createSearchTool,
  createDigestTool,
  createDocsTool,
  knowledgeFactory,
};

// NOTE: createDigestTool, createSearchTool, createDocsTool implementations moved to domains/knowledge.js

// Blockchain domain (poj_chain, trace)
// Import for local use + re-export
import {
  createPoJChainTool,
  createTraceTool,
  blockchainFactory,
} from './domains/blockchain.js';

export {
  createPoJChainTool,
  createTraceTool,
  blockchainFactory,
};

// NOTE: createPoJChainTool, createTraceTool implementations moved to domains/blockchain.js

// Consciousness domain (patterns, milestone_history, self_mod, emergence, consciousness)
// Import for local use + re-export
import {
  createPatternsTool,
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  createConsciousnessTool,
  consciousnessFactory,
} from './domains/consciousness.js';

export {
  createPatternsTool,
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  createConsciousnessTool,
  consciousnessFactory,
};

// NOTE: createPatternsTool, createMilestoneHistoryTool, createSelfModTool, createEmergenceTool moved to domains/consciousness.js

// Session domain (psychology, session_start, session_end, profile_sync, profile_load)
// Import for local use + re-export
import {
  createPsychologyTool,
  createSessionStartTool,
  createSessionEndTool,
  createProfileSyncTool,
  createProfileLoadTool,
  sessionFactory,
} from './domains/session.js';

export {
  createPsychologyTool,
  createSessionStartTool,
  createSessionEndTool,
  createProfileSyncTool,
  createProfileLoadTool,
  sessionFactory,
};

// NOTE: Session tools moved to domains/session.js

// Ecosystem domain (ecosystem, ecosystem_monitor, integrator, discovery)
import {
  createEcosystemTool,
  createEcosystemMonitorTool,
  createIntegratorTool,
  createDiscoveryTool,
  ecosystemFactory,
} from './domains/ecosystem.js';

export {
  createEcosystemTool,
  createEcosystemMonitorTool,
  createIntegratorTool,
  createDiscoveryTool,
  ecosystemFactory,
};

// NOTE: Ecosystem tools moved to domains/ecosystem.js

// System domain (health, metrics, collective_status, agent_diagnostic, consensus)
import {
  createHealthTool,
  createMetricsTool,
  createCollectiveStatusTool,
  createAgentDiagnosticTool,
  createConsensusTool,
  systemFactory,
} from './domains/system.js';

export {
  createHealthTool,
  createMetricsTool,
  createCollectiveStatusTool,
  createAgentDiagnosticTool,
  createConsensusTool,
  systemFactory,
};

// NOTE: System tools moved to domains/system.js

// Automation domain (triggers)
import {
  createTriggersTool,
  automationFactory,
} from './domains/automation.js';

export {
  createTriggersTool,
  automationFactory,
};

// NOTE: Automation tools moved to domains/automation.js

// Code domain (vector_search, codebase)
import {
  createVectorSearchTool,
  createCodebaseTool,
  codeFactory,
} from './domains/code.js';

export {
  createVectorSearchTool,
  createCodebaseTool,
  codeFactory,
};

// NOTE: Code tools moved to domains/code.js

// Orchestration domain (KETER - central consciousness)
import {
  createOrchestrateTool,
  createFullOrchestrateTool,
  createCircuitBreakerTool,
  createDecisionsTool,
  orchestrationFactory,
} from './domains/orchestration.js';

export {
  createOrchestrateTool,
  createFullOrchestrateTool,
  createCircuitBreakerTool,
  createDecisionsTool,
  orchestrationFactory,
};

// NOTE: KETER orchestrator moved to domains/orchestration.js

// Claude Flow domain (complexity, boost, optimize, route, hyperbolic, sona)
import {
  createComplexityTool,
  createBoosterTool,
  createOptimizerTool,
  createRouterTool,
  createHyperbolicTool,
  createSONATool,
  claudeFlowFactory,
} from './domains/claude-flow.js';

export {
  createComplexityTool,
  createBoosterTool,
  createOptimizerTool,
  createRouterTool,
  createHyperbolicTool,
  createSONATool,
  claudeFlowFactory,
};

// Memory domain (memory_search, memory_store, memory_stats, self_correction, goals, notifications, tasks)
import {
  createMemorySearchTool,
  createMemoryStoreTool,
  createMemoryStatsTool,
  createSelfCorrectionTool,
  createGoalsTool,
  createNotificationsTool,
  createTasksTool,
  memoryFactory,
} from './domains/memory.js';

export {
  createMemorySearchTool,
  createMemoryStoreTool,
  createMemoryStatsTool,
  createSelfCorrectionTool,
  createGoalsTool,
  createNotificationsTool,
  createTasksTool,
  memoryFactory,
};

// Social domain (X/Twitter vision: x_feed, x_search, x_analyze, x_trends)
import {
  createXFeedTool,
  createXSearchTool,
  createXAnalyzeTool,
  createXTrendsTool,
  createXSyncTool,
  socialFactory,
} from './domains/social.js';

export {
  createXFeedTool,
  createXSearchTool,
  createXAnalyzeTool,
  createXTrendsTool,
  createXSyncTool,
  socialFactory,
};

// X Coach domain (communication coaching - autonomize the human)
import {
  createXCoachTool,
  createXLearnTool,
  createXStyleTool,
  xCoachFactory,
} from './domains/x-coach.js';

export {
  createXCoachTool,
  createXLearnTool,
  createXStyleTool,
  xCoachFactory,
};

// Oracle domain (token scoring - 17-dim φ-governed judgment)
import {
  createOracleScoreTool,
  createOracleWatchlistTool,
  createOracleStatsTool,
  oracleFactory,
} from './domains/oracle.js';

export {
  createOracleScoreTool,
  createOracleWatchlistTool,
  createOracleStatsTool,
  oracleFactory,
};

// NOTE: Memory tools moved to domains/memory.js

/**
 * Create all tools
 * @param {Object} options - Tool options
 * @param {Object} options.judge - CYNICJudge instance
 * @param {Object} [options.node] - CYNICNode instance
 * @param {Object} [options.persistence] - PersistenceManager instance (handles fallback)
 * @param {Object} [options.agents] - DEPRECATED: Legacy AgentManager (use collective instead)
 * @param {Object} [options.sessionManager] - SessionManager instance for multi-user sessions
 * @param {Object} [options.pojChainManager] - PoJChainManager instance for blockchain
 * @param {Object} [options.librarian] - LibrarianService instance for documentation caching
 * @param {Object} [options.ecosystem] - EcosystemService instance for pre-loaded docs
 * @param {Object} [options.integrator] - IntegratorService instance for cross-project sync
 * @param {Object} [options.metrics] - MetricsService instance for monitoring
 * @param {Object} [options.graphIntegration] - JudgmentGraphIntegration instance for graph edges
 * @param {Object} [options.codebaseOptions] - Options for code analyzer (rootPath, etc)
 * @param {Object} [options.discovery] - DiscoveryService instance for MCP/plugin/node discovery
 * @param {Object} [options.learningService] - LearningService instance for RLHF-style learning
 * @param {Object} [options.eScoreCalculator] - EScoreCalculator instance for vote weight
 * @param {Object} [options.emergenceLayer] - EmergenceLayer instance (Layer 7 - Keter) for consciousness tracking
 * @param {Object} [options.patternDetector] - PatternDetector instance for statistical pattern recognition
 * @param {Function} [options.onJudgment] - Callback when judgment is completed (for SSE broadcast)
 * @param {Object} [options.complexityClassifier] - ComplexityClassifier for tiered routing
 * @param {Object} [options.tieredRouter] - TieredRouter for request routing
 * @param {Object} [options.agentBooster] - AgentBooster for fast code transforms
 * @param {Object} [options.tokenOptimizer] - TokenOptimizer for compression
 * @param {Object} [options.hyperbolicSpace] - HyperbolicSpace for hierarchical embeddings
 * @param {Object} [options.sona] - SONA for adaptive learning
 * @returns {Object} All tools keyed by name
 */
export function createAllTools(options = {}) {
  const {
    judge,
    node = null,
    persistence = null,
    agents = null,
    collective = null, // CollectivePack (The Five Dogs + CYNIC)
    sessionManager = null,
    pojChainManager = null,
    librarian = null,
    ecosystem = null,
    integrator = null,
    metrics = null,
    graphIntegration = null, // JudgmentGraphIntegration for graph edges
    codebaseOptions = {},
    lspOptions = {}, // LSP service options (rootPath, extensions, cacheTTL)
    discovery = null, // DiscoveryService for MCP/plugin/node discovery
    learningService = null, // LearningService for RLHF feedback
    eScoreCalculator = null, // EScoreCalculator for vote weight
    emergenceLayer = null, // EmergenceLayer (Layer 7) - consciousness, patterns, dimensions, collective
    patternDetector = null, // PatternDetector for statistical pattern recognition
    thermodynamics = null, // ThermodynamicsTracker (Phase 2) - heat/work/efficiency
    onJudgment = null, // SSE broadcast callback
    // Memory/Autonomy dependencies
    memoryRetriever = null, // MemoryRetriever for memory search/store
    goalsRepo = null, // GoalsRepo for autonomous goals
    notificationsRepo = null, // NotificationsRepo for proactive notifications
    tasksRepo = null, // TasksRepo for durable task queue
    automationExecutor = null, // AutomationExecutor for daemon stats
    // AXE 5: OBSERVE - Uptime awareness
    heartbeat = null, // HeartbeatService for continuous health monitoring
    slaTracker = null, // SLATracker for 99.9% uptime compliance
    // Claude Flow dependencies (Phase 21)
    complexityClassifier = null, // ComplexityClassifier for tiered routing
    tieredRouter = null, // TieredRouter for request routing
    agentBooster = null, // AgentBooster for fast code transforms
    tokenOptimizer = null, // TokenOptimizer for compression
    hyperbolicSpace = null, // HyperbolicSpace for hierarchical embeddings
    sona = null, // SONA for adaptive learning
    // Phase 22: Orchestrators for brain_orchestrate
    dogOrchestrator = null, // DogOrchestrator for Dogs voting
    engineOrchestrator = null, // EngineOrchestrator for Engines synthesis
    // X/Twitter vision
    xRepository = null, // XDataRepository for cloud social data (optional)
    // Local Privacy Stores (SQLite - privacy by design)
    localXStore = null, // LocalXStore for local X data (primary)
    localPrivacyStore = null, // LocalPrivacyStore for E-Score, Learning, Psychology
    // Oracle (token scoring)
    oracle = null, // { fetcher, scorer, memory, watchlist }
  } = options;

  // Initialize LSP service for code intelligence
  const lsp = new LSPService({
    rootPath: lspOptions.rootPath || codebaseOptions.rootPath || process.cwd(),
    ...lspOptions,
  });

  // Initialize JSON render service for streaming UI
  const jsonRenderer = new JSONRenderService();

  if (!judge) throw new Error('judge is required');

  const tools = {};
  const toolDefs = [
    createJudgeTool(judge, persistence, sessionManager, pojChainManager, graphIntegration, onJudgment, null /* burnEnforcer */, emergenceLayer, thermodynamics),
    createRefineTool(judge, persistence), // Self-refinement: critique → refine → learn
    createOrchestrateTool({ judge, persistence }), // KETER: Central consciousness routing
    createFullOrchestrateTool({ judge, persistence, dogOrchestrator, engineOrchestrator }), // Full orchestration: routing + judgment + synthesis
    createCircuitBreakerTool({ persistence }), // Circuit breaker health/stats (Phase 21)
    createDecisionsTool({ persistence }), // Decision history/tracing (Phase 21)
    createVectorSearchTool({ persistence }), // Semantic search with embeddings
    createLearningTool({ learningService, persistence }), // Learning service: feedback → weight modifiers → improvement
    createTriggersTool({ judge, persistence }), // Auto-judgment triggers
    createDigestTool(persistence, sessionManager),
    createHealthTool(node, judge, persistence, automationExecutor, thermodynamics, heartbeat, slaTracker),
    createPsychologyTool(persistence), // Human psychology dashboard
    createSearchTool(persistence),
    // Progressive Search Tools (3-layer retrieval for 10x token savings)
    createSearchIndexTool(persistence),
    createTimelineTool(persistence),
    createGetObservationsTool(persistence),
    createPatternsTool(judge, persistence, patternDetector),
    createFeedbackTool(persistence, sessionManager),
    createCollectiveStatusTool(collective), // The Eleven Dogs + CYNIC (Keter)
    createAgentDiagnosticTool(collective),
    createConsensusTool(collective), // Inter-agent voting (φ⁻¹ threshold)
    createSessionStartTool(sessionManager),
    createSessionEndTool(sessionManager),
    createProfileSyncTool(persistence),  // Cross-session memory: sync profile to DB
    createProfileLoadTool(persistence),  // Cross-session memory: load profile from DB
    createDocsTool(librarian, persistence),
    createEcosystemTool(ecosystem),
    createEcosystemMonitorTool({ judge, persistence }), // External sources: GitHub, Twitter, Web + auto-analysis
    createDiscoveryTool(discovery), // MCP servers, plugins, CYNIC nodes
    createPoJChainTool(pojChainManager, persistence),
    createTraceTool(persistence, pojChainManager),
    createIntegratorTool(integrator),
    createMetricsTool(metrics),
    createMetaTool(), // CYNIC self-analysis dashboard
    createCodebaseTool(codebaseOptions), // Code structure analyzer
    // Dashboard real-data tools (Singularity Index components)
    createMilestoneHistoryTool(persistence), // Historical singularity scores
    createSelfModTool(), // Git history analysis
    createEmergenceTool(judge, persistence), // Consciousness signals
    createConsciousnessTool(emergenceLayer), // Layer 7 (Keter) consciousness monitor direct access
    // Memory/Autonomy Tools (total memory + full autonomy)
    createMemorySearchTool(memoryRetriever), // Hybrid FTS + vector search
    createMemoryStoreTool(memoryRetriever), // Store memories with embeddings
    createMemoryStatsTool(memoryRetriever), // Memory statistics
    createSelfCorrectionTool(memoryRetriever), // Lessons learned analysis
    createGoalsTool(goalsRepo), // Autonomous goals CRUD
    createNotificationsTool(notificationsRepo), // Proactive notifications
    createTasksTool(tasksRepo), // Durable task queue
    // LSP Tools (code intelligence: symbols, references, call graphs, refactoring)
    ...createLSPTools(lsp),
    // JSON Render (streaming UI components)
    createJSONRenderTool(jsonRenderer),
    // Claude Flow Tools (Phase 21 - intelligent routing & optimization)
    createComplexityTool(complexityClassifier),
    createBoosterTool(agentBooster),
    createOptimizerTool(tokenOptimizer),
    createRouterTool(tieredRouter),
    createHyperbolicTool(hyperbolicSpace),
    createSONATool(sona),
    // X/Twitter Vision Tools (privacy-first: local SQLite, optional cloud sync)
    ...(localXStore ? [
      createXFeedTool(localXStore, xRepository),
      createXSearchTool(localXStore, xRepository, judge),
      createXAnalyzeTool(localXStore, judge),
      createXTrendsTool(localXStore, xRepository),
      createXSyncTool(localXStore, xRepository),
    ] : []),
    // Oracle Tools (token scoring - 17-dim φ-governed judgment, lazy-init in server.js)
    ...(oracle ? [
      createOracleScoreTool(oracle),
      ...(oracle.watchlist ? [createOracleWatchlistTool(oracle)] : []),
      ...(oracle.memory ? [createOracleStatsTool(oracle)] : []),
    ] : []),
    // X Coach Tools (communication coaching - autonomize the human, requires localXStore)
    ...(localXStore ? [
      createXCoachTool(judge, localXStore),
      createXLearnTool(localXStore),
      createXStyleTool(localXStore),
    ] : []),
  ];

  for (const tool of toolDefs) {
    tools[tool.name] = tool;
  }

  return tools;
}

// Re-export meta tool
export { createMetaTool } from '../meta-dashboard.js';

// Re-export progressive search tools
export {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
} from './search-progressive.js';

// Re-export LSP tools
export { LSPService, createLSPTools } from '../lsp-service.js';

// Re-export JSON render
export { JSONRenderService, createJSONRenderTool } from '../json-render.js';

// ═══════════════════════════════════════════════════════════════════════════
// OCP: TOOL REGISTRY (Open for Extension, Closed for Modification)
// Add new tools by registering factories, not modifying this file
// ═══════════════════════════════════════════════════════════════════════════

export { ToolRegistry, defaultRegistry, registerTool, registerTools } from './registry.js';

export default {
  createJudgeTool,
  createDigestTool,
  createHealthTool,
  createSearchTool,
  // Orchestration tools (Phase 21)
  createOrchestrateTool,
  createFullOrchestrateTool,
  createCircuitBreakerTool,
  createDecisionsTool,
  // Progressive Search (3-layer retrieval)
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
  createPatternsTool,
  createFeedbackTool,
  createCollectiveStatusTool, // The Eleven Dogs + CYNIC
  createConsensusTool, // Inter-agent voting
  createSessionStartTool,
  createSessionEndTool,
  createProfileSyncTool,
  createProfileLoadTool,
  createDocsTool,
  createEcosystemTool,
  createDiscoveryTool,
  createPoJChainTool,
  createTraceTool,
  createIntegratorTool,
  createMetricsTool,
  createMetaTool,
  createCodebaseTool,
  // Dashboard real-data tools
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  // Memory/Autonomy tools
  createMemorySearchTool,
  createMemoryStoreTool,
  createMemoryStatsTool,
  createSelfCorrectionTool,
  createGoalsTool,
  createNotificationsTool,
  createTasksTool,
  memoryFactory,
  // Social/X Twitter tools (privacy-first)
  createXFeedTool,
  createXSearchTool,
  createXAnalyzeTool,
  createXTrendsTool,
  createXSyncTool,
  socialFactory,
  // LSP Tools (code intelligence)
  LSPService,
  createLSPTools,
  // JSON Render
  JSONRenderService,
  createJSONRenderTool,
  createAllTools,
};

