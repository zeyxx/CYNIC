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

/**
 * All domain factories for bulk registration
 */
export const allFactories = [
  // Dynamic imports to avoid circular dependency issues
];

/**
 * Get all factories (lazy load to avoid circular deps)
 */
export function getAllFactories() {
  const { judgmentFactory } = require('./judgment.js');
  const { ecosystemFactory } = require('./ecosystem.js');
  const { systemFactory } = require('./system.js');
  const { blockchainFactory } = require('./blockchain.js');
  const { consciousnessFactory } = require('./consciousness.js');
  const { sessionFactory } = require('./session.js');
  const { knowledgeFactory } = require('./knowledge.js');
  const { automationFactory } = require('./automation.js');
  const { codeFactory } = require('./code.js');

  return [
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
