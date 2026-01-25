/**
 * CYNIC Boot Providers
 *
 * Standard component providers for the boot system.
 * Each provider creates a lifecycle-managed component.
 *
 * @module @cynic/core/boot/providers
 */

'use strict';

// Engine system providers
export {
  createEngineRegistryProvider,
  createEngineOrchestratorProvider,
  createEnginesProvider,
} from './engines.js';

// MCP server providers
export {
  registerMCPProviders,
  createMCPProvider,
  createConfigProvider,
  createMigrationsProvider,
} from './mcp.js';
