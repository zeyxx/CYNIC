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

// Consciousness domain (patterns, milestone_history, self_mod, emergence)
// Import for local use + re-export
import {
  createPatternsTool,
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  consciousnessFactory,
} from './domains/consciousness.js';

export {
  createPatternsTool,
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
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

// System domain (health, metrics, collective_status, agents_status, agent_diagnostic)
import {
  createHealthTool,
  createAgentsStatusTool,
  createMetricsTool,
  createCollectiveStatusTool,
  createAgentDiagnosticTool,
  systemFactory,
} from './domains/system.js';

export {
  createHealthTool,
  createAgentsStatusTool,
  createMetricsTool,
  createCollectiveStatusTool,
  createAgentDiagnosticTool,
  systemFactory,
};

// NOTE: System tools moved to domains/system.js

// Automation domain (orchestration, triggers)
import {
  createOrchestrationTool,
  createTriggersTool,
  automationFactory,
} from './domains/automation.js';

export {
  createOrchestrationTool,
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
  orchestrationFactory,
} from './domains/orchestration.js';

export {
  createOrchestrateTool,
  orchestrationFactory,
};

// NOTE: KETER orchestrator moved to domains/orchestration.js

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
 * @param {Function} [options.onJudgment] - Callback when judgment is completed (for SSE broadcast)
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
    onJudgment = null, // SSE broadcast callback
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
    createJudgeTool(judge, persistence, sessionManager, pojChainManager, graphIntegration, onJudgment),
    createRefineTool(judge, persistence), // Self-refinement: critique → refine → learn
    createOrchestrationTool({ judge, agents, persistence }), // Multi-agent parallel execution
    createOrchestrateTool({ judge, persistence }), // KETER: Central consciousness routing
    createVectorSearchTool({ persistence }), // Semantic search with embeddings
    createLearningTool({ learningService, persistence }), // Learning service: feedback → weight modifiers → improvement
    createTriggersTool({ judge, persistence }), // Auto-judgment triggers
    createDigestTool(persistence, sessionManager),
    createHealthTool(node, judge, persistence),
    createPsychologyTool(persistence), // Human psychology dashboard
    createSearchTool(persistence),
    // Progressive Search Tools (3-layer retrieval for 10x token savings)
    createSearchIndexTool(persistence),
    createTimelineTool(persistence),
    createGetObservationsTool(persistence),
    createPatternsTool(judge, persistence),
    createFeedbackTool(persistence, sessionManager),
    createAgentsStatusTool(collective), // DEPRECATED: redirects to Collective
    createCollectiveStatusTool(collective), // The Eleven Dogs + CYNIC (Keter)
    createAgentDiagnosticTool(collective),
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
    // LSP Tools (code intelligence: symbols, references, call graphs, refactoring)
    ...createLSPTools(lsp),
    // JSON Render (streaming UI components)
    createJSONRenderTool(jsonRenderer),
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
  // Progressive Search (3-layer retrieval)
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
  createPatternsTool,
  createFeedbackTool,
  createAgentsStatusTool,
  createCollectiveStatusTool, // NEW: The Five Dogs + CYNIC
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
  // LSP Tools (code intelligence)
  LSPService,
  createLSPTools,
  // JSON Render
  JSONRenderService,
  createJSONRenderTool,
  createAllTools,
};

