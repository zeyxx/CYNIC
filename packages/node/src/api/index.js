/**
 * CYNIC API Module
 *
 * HTTP API layer exports
 *
 * @module @cynic/node/api
 */

'use strict';

export { APIServer } from './server.js';
export { default } from './server.js';

// Burns verification API
export { BurnsAPI, setupBurnsRoutes } from './burns-api.js';

// Emergence API (Layer 7)
export { setupEmergenceRoutes } from './emergence-api.js';
