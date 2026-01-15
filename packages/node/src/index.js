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

// Agents - The Four Dogs
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
