/**
 * Persistence Module - ISP-Compliant Adapters
 *
 * "Interface Segregation Principle"
 * - Clients depend only on interfaces they use
 * - Each adapter handles one domain
 *
 * @module @cynic/mcp/persistence
 */

'use strict';

// Stores (fallback)
export { MemoryStore, FileStore } from './stores.js';

// Domain Adapters (ISP)
export { JudgmentAdapter } from './JudgmentAdapter.js';
export { PatternAdapter } from './PatternAdapter.js';
export { PoJChainAdapter } from './PoJChainAdapter.js';
export { TriggerAdapter } from './TriggerAdapter.js';
export { KnowledgeAdapter } from './KnowledgeAdapter.js';
export { FeedbackAdapter } from './FeedbackAdapter.js';
export { LibraryCacheAdapter } from './LibraryCacheAdapter.js';
export { PsychologyAdapter } from './PsychologyAdapter.js';

// Re-export main manager for backward compatibility
export { PersistenceManager } from '../persistence.js';
