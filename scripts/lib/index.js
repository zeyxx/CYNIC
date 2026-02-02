/**
 * ESM Bridge for scripts/lib/
 *
 * Allows ESM hooks to import CJS libraries.
 * Uses createRequire() to load CommonJS modules.
 *
 * "The bridge between worlds" - κυνικός
 *
 * @module scripts/lib
 */

'use strict';

import { createRequire } from 'module';
import { fileURLToPath } from 'url';
import path from 'path';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// ═══════════════════════════════════════════════════════════════════════════
// CORE LIBRARY (cynic-core.cjs)
// ═══════════════════════════════════════════════════════════════════════════

const cynicCore = require('./cynic-core.cjs');

export const {
  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  HEARTBEAT_MS,

  // Paths
  getCynicRoot,
  getDataDir,
  getUsersDir,
  getCollectiveDir,

  // User
  detectUser,
  loadUserProfile,
  saveUserProfile,
  updateUserProfile,

  // E-Score (Trust level)
  calculateUserEScore,
  getUserEScore,
  getTrustLevelFromScore,

  // Collective
  loadCollectivePatterns,
  saveCollectivePattern,
  loadCollectiveWisdom,
  addCollectiveInsight,

  // Ecosystem
  detectEcosystem,
  detectProject,
  getGitState,

  // Formatting
  formatEcosystemStatus,
  getPersonalizedGreeting,

  // Privacy
  stripPrivateContent,
  hasPrivateContent,
  extractPrivateContent,

  // MCP Integration
  MCP_SERVER_URL,
  sendHookToCollective,
  sendHookToCollectiveSync,
  callBrainTool,
  startBrainSession,
  endBrainSession,
  digestToBrain,

  // Orchestration (KETER - Central routing)
  orchestrate,
  orchestrateSync,
  orchestrateFull,  // Phase 19: Full orchestration with Dogs + Engines + Skills

  // Learning Feedback (Ralph-inspired external validation)
  sendTestFeedback,
  sendCommitFeedback,
  sendPRFeedback,
  sendBuildFeedback,

  // Cross-Session Profile Persistence
  loadProfileFromDB,
  syncProfileToDB,
  mergeProfiles,

  // Cross-Session Pattern Persistence
  loadSessionPatterns,
  saveSessionPatterns,

  // Contributor Discovery (functions from cynic-core)
  getContributorProfile,
  getContributorByEmail,
  discoverContributors,
  getContributorPhiScores,
  // Note: getContributorDiscovery is provided as lazy-loading getter below

  // Decision Engine
  decisionEngine,
} = cynicCore;

// Export the whole module for backward compatibility
export default cynicCore;

// ═══════════════════════════════════════════════════════════════════════════
// DECISION CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

export const DC = require('./decision-constants.cjs');

// ═══════════════════════════════════════════════════════════════════════════
// OPTIONAL MODULES (loaded with error handling)
// ═══════════════════════════════════════════════════════════════════════════

let _cockpit = null;
let _consciousness = null;
let _proactiveAdvisor = null;
let _signalCollector = null;
let _psychology = null;
let _contributorDiscovery = null;
let _securityPatterns = null;
let _monitoring = null;
let _watchdog = null;
let _autoJudge = null;

// Lazy load optional modules
export function getCockpit() {
  if (_cockpit === null) {
    try {
      _cockpit = require('./cockpit.cjs');
    } catch (e) {
      _cockpit = false;
    }
  }
  return _cockpit || null;
}

export function getConsciousness() {
  if (_consciousness === null) {
    try {
      _consciousness = require('./consciousness.cjs');
      _consciousness.init?.();
    } catch (e) {
      _consciousness = false;
    }
  }
  return _consciousness || null;
}

export function getProactiveAdvisor() {
  if (_proactiveAdvisor === null) {
    try {
      _proactiveAdvisor = require('./proactive-advisor.cjs');
      _proactiveAdvisor.init?.();
    } catch (e) {
      _proactiveAdvisor = false;
    }
  }
  return _proactiveAdvisor || null;
}

export function getSignalCollector() {
  if (_signalCollector === null) {
    try {
      _signalCollector = require('./signal-collector.cjs');
      _signalCollector.init?.();
    } catch (e) {
      _signalCollector = false;
    }
  }
  return _signalCollector || null;
}

export function getPsychology() {
  if (_psychology === null) {
    try {
      _psychology = require('./human-psychology.cjs');
      _psychology.init?.();
    } catch (e) {
      _psychology = false;
    }
  }
  return _psychology || null;
}

export function getContributorDiscovery() {
  if (_contributorDiscovery === null) {
    try {
      _contributorDiscovery = require('./contributor-discovery.cjs');
    } catch (e) {
      _contributorDiscovery = false;
    }
  }
  return _contributorDiscovery || null;
}

export function getSecurityPatterns() {
  if (_securityPatterns === null) {
    try {
      _securityPatterns = require('./security-patterns.cjs');
    } catch (e) {
      _securityPatterns = false;
    }
  }
  return _securityPatterns || null;
}

export function getMonitoring() {
  if (_monitoring === null) {
    try {
      _monitoring = require('./monitoring.cjs');
      _monitoring.init?.();
    } catch (e) {
      _monitoring = false;
    }
  }
  return _monitoring || null;
}

export function getWatchdog() {
  if (_watchdog === null) {
    try {
      _watchdog = require('./watchdog.cjs');
    } catch (e) {
      _watchdog = false;
    }
  }
  return _watchdog || null;
}

export function getAutoJudge() {
  if (_autoJudge === null) {
    try {
      _autoJudge = require('./auto-judge.cjs');
      _autoJudge.init?.();
    } catch (e) {
      _autoJudge = false;
    }
  }
  return _autoJudge || null;
}

// LLM Judgment Bridge (autonomous improvement with open source LLMs)
let _llmBridge = null;

export function getLLMJudgmentBridge() {
  if (_llmBridge === null) {
    try {
      _llmBridge = require('./llm-judgment-bridge.cjs');
    } catch (e) {
      _llmBridge = false;
    }
  }
  return _llmBridge || null;
}

// Additional module getters for hooks
let _circuitBreaker = null;
let _physisDetector = null;
let _voluntaryPoverty = null;
let _heisenberg = null;
let _elenchus = null;
let _chriaDB = null;
let _tiEsti = null;
let _definitionTracker = null;
let _fallacyDetector = null;
let _roleReversal = null;
let _hypothesisTesting = null;
let _physicsBridge = null;

export function getCircuitBreaker() {
  if (_circuitBreaker === null) {
    try {
      _circuitBreaker = require('./circuit-breaker.cjs');
    } catch (e) {
      _circuitBreaker = false;
    }
  }
  return _circuitBreaker || null;
}

export function getPhysisDetector() {
  if (_physisDetector === null) {
    try {
      _physisDetector = require('./physis-detector.cjs');
      _physisDetector.init?.();
    } catch (e) {
      _physisDetector = false;
    }
  }
  return _physisDetector || null;
}

export function getVoluntaryPoverty() {
  if (_voluntaryPoverty === null) {
    try {
      _voluntaryPoverty = require('./voluntary-poverty.cjs');
      _voluntaryPoverty.init?.();
    } catch (e) {
      _voluntaryPoverty = false;
    }
  }
  return _voluntaryPoverty || null;
}

export function getHeisenberg() {
  if (_heisenberg === null) {
    try {
      _heisenberg = require('./heisenberg-confidence.cjs');
    } catch (e) {
      _heisenberg = false;
    }
  }
  return _heisenberg || null;
}

export function getElenchus() {
  if (_elenchus === null) {
    try {
      _elenchus = require('./elenchus-engine.cjs');
      _elenchus.init?.();
    } catch (e) {
      _elenchus = false;
    }
  }
  return _elenchus || null;
}

export function getChriaDB() {
  if (_chriaDB === null) {
    try {
      _chriaDB = require('./chria-database.cjs');
      _chriaDB.init?.();
    } catch (e) {
      _chriaDB = false;
    }
  }
  return _chriaDB || null;
}

export function getTiEsti() {
  if (_tiEsti === null) {
    try {
      _tiEsti = require('./ti-esti-engine.cjs');
      _tiEsti.init?.();
    } catch (e) {
      _tiEsti = false;
    }
  }
  return _tiEsti || null;
}

export function getDefinitionTracker() {
  if (_definitionTracker === null) {
    try {
      _definitionTracker = require('./definition-tracker.cjs');
      _definitionTracker.init?.();
    } catch (e) {
      _definitionTracker = false;
    }
  }
  return _definitionTracker || null;
}

export function getFallacyDetector() {
  if (_fallacyDetector === null) {
    try {
      _fallacyDetector = require('./fallacy-detector.cjs');
      _fallacyDetector.init?.();
    } catch (e) {
      _fallacyDetector = false;
    }
  }
  return _fallacyDetector || null;
}

export function getRoleReversal() {
  if (_roleReversal === null) {
    try {
      _roleReversal = require('./role-reversal.cjs');
    } catch (e) {
      _roleReversal = false;
    }
  }
  return _roleReversal || null;
}

export function getHypothesisTesting() {
  if (_hypothesisTesting === null) {
    try {
      _hypothesisTesting = require('./hypothesis-testing.cjs');
    } catch (e) {
      _hypothesisTesting = false;
    }
  }
  return _hypothesisTesting || null;
}

export function getPhysicsBridge() {
  if (_physicsBridge === null) {
    try {
      _physicsBridge = require('./physics-bridge.cjs');
    } catch (e) {
      _physicsBridge = false;
    }
  }
  return _physicsBridge || null;
}

// More module getters for digest and other hooks
let _taskEnforcer = null;
let _thermodynamics = null;
let _emergence = null;

export function getTaskEnforcer() {
  if (_taskEnforcer === null) {
    try {
      _taskEnforcer = require('./task-enforcer.cjs');
    } catch (e) {
      _taskEnforcer = false;
    }
  }
  return _taskEnforcer || null;
}

export function getThermodynamics() {
  if (_thermodynamics === null) {
    try {
      _thermodynamics = require('./cognitive-thermodynamics.cjs');
      _thermodynamics.init?.();
    } catch (e) {
      _thermodynamics = false;
    }
  }
  return _thermodynamics || null;
}

export function getEmergence() {
  if (_emergence === null) {
    try {
      _emergence = require('./emergence-detector.cjs');
    } catch (e) {
      _emergence = false;
    }
  }
  return _emergence || null;
}

// Total Memory module (Phase 16)
let _totalMemory = null;

export function getTotalMemory() {
  if (_totalMemory === null) {
    try {
      _totalMemory = require('./total-memory.cjs');
    } catch (e) {
      _totalMemory = false;
    }
  }
  return _totalMemory || null;
}

// Collective Dogs (Sefirot) module
let _collectiveDogs = null;

export function getCollectiveDogs() {
  if (_collectiveDogs === null) {
    try {
      _collectiveDogs = require('./collective-dogs.cjs');
    } catch (e) {
      _collectiveDogs = false;
    }
  }
  return _collectiveDogs || null;
}

// More module getters for observe hook
let _selfRefinement = null;
let _harmonyAnalyzer = null;
let _cognitiveBiases = null;
let _topologyTracker = null;
let _interventionEngine = null;
let _cosmopolitan = null;
let _dialectic = null;
let _inferenceEngine = null;

export function getSelfRefinement() {
  if (_selfRefinement === null) {
    try {
      _selfRefinement = require('./self-refinement.cjs');
    } catch (e) {
      _selfRefinement = false;
    }
  }
  return _selfRefinement || null;
}

export function getHarmonyAnalyzer() {
  if (_harmonyAnalyzer === null) {
    try {
      _harmonyAnalyzer = require('./harmony-analyzer.cjs');
    } catch (e) {
      _harmonyAnalyzer = false;
    }
  }
  return _harmonyAnalyzer || null;
}

export function getCognitiveBiases() {
  if (_cognitiveBiases === null) {
    try {
      _cognitiveBiases = require('./cognitive-biases.cjs');
      _cognitiveBiases.init?.();
    } catch (e) {
      _cognitiveBiases = false;
    }
  }
  return _cognitiveBiases || null;
}

export function getTopologyTracker() {
  if (_topologyTracker === null) {
    try {
      _topologyTracker = require('./topology-tracker.cjs');
      _topologyTracker.init?.();
    } catch (e) {
      _topologyTracker = false;
    }
  }
  return _topologyTracker || null;
}

export function getInterventionEngine() {
  if (_interventionEngine === null) {
    try {
      _interventionEngine = require('./intervention-engine.cjs');
      _interventionEngine.init?.();
    } catch (e) {
      _interventionEngine = false;
    }
  }
  return _interventionEngine || null;
}

export function getCosmopolitan() {
  if (_cosmopolitan === null) {
    try {
      _cosmopolitan = require('./cosmopolitan-learning.cjs');
      _cosmopolitan.init?.();
    } catch (e) {
      _cosmopolitan = false;
    }
  }
  return _cosmopolitan || null;
}

export function getDialectic() {
  if (_dialectic === null) {
    try {
      _dialectic = require('./dialectic-synthesizer.cjs');
      _dialectic.init?.();
    } catch (e) {
      _dialectic = false;
    }
  }
  return _dialectic || null;
}

export function getInferenceEngine() {
  if (_inferenceEngine === null) {
    try {
      _inferenceEngine = require('./inference-engine.cjs');
      _inferenceEngine.init?.();
    } catch (e) {
      _inferenceEngine = false;
    }
  }
  return _inferenceEngine || null;
}

// ═══════════════════════════════════════════════════════════════════════════
// DIRECT REQUIRE (for modules that need full access)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Get a CJS module by name
 * @param {string} name - Module name (without .cjs extension)
 * @returns {Object|null}
 */
export function requireLib(name) {
  try {
    return require(path.join(__dirname, `${name}.cjs`));
  } catch (e) {
    console.error(`Failed to load ${name}: ${e.message}`);
    return null;
  }
}
