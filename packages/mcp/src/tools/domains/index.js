/**
 * Tool Domain Factories
 *
 * OCP-Compliant factory organization by domain.
 * Each domain exports a factory that can be registered with ToolRegistry.
 *
 * @module @cynic/mcp/tools/domains
 */

'use strict';

export { judgmentFactory } from './judgment.js';
export { ecosystemFactory } from './ecosystem.js';
export { systemFactory } from './system.js';
export { blockchainFactory } from './blockchain.js';
export { consciousnessFactory } from './consciousness.js';
export { sessionFactory } from './session.js';
export { knowledgeFactory } from './knowledge.js';
export { automationFactory } from './automation.js';
export { codeFactory } from './code.js';
export { orchestrationFactory } from './orchestration.js';

/**
 * All domain factories for bulk registration
 */
export const allFactories = [
  // Dynamic imports to avoid circular dependency issues
];

/**
 * Get all factories (lazy load to avoid circular deps)
 * @returns {Promise<Array>} Array of factory functions
 */
export async function getAllFactories() {
  // Use dynamic import() instead of require() for ESM compatibility
  const [
    { judgmentFactory },
    { ecosystemFactory },
    { systemFactory },
    { blockchainFactory },
    { consciousnessFactory },
    { sessionFactory },
    { knowledgeFactory },
    { automationFactory },
    { codeFactory },
    { orchestrationFactory },
  ] = await Promise.all([
    import('./judgment.js'),
    import('./ecosystem.js'),
    import('./system.js'),
    import('./blockchain.js'),
    import('./consciousness.js'),
    import('./session.js'),
    import('./knowledge.js'),
    import('./automation.js'),
    import('./code.js'),
    import('./orchestration.js'),
  ]);

  return [
    orchestrationFactory,  // KETER - First, central orchestrator
    judgmentFactory,
    ecosystemFactory,
    systemFactory,
    blockchainFactory,
    consciousnessFactory,
    sessionFactory,
    knowledgeFactory,
    automationFactory,
    codeFactory,
  ];
}
