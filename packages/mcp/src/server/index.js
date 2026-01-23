/**
 * Server Module - SRP-Compliant Components
 *
 * Extracted from MCPServer for single responsibility:
 * - HttpAdapter: HTTP/SSE/CORS
 * - ServiceInitializer: DIP-compliant service creation
 *
 * @module @cynic/mcp/server
 */

'use strict';

export { HttpAdapter } from './HttpAdapter.js';
export { ServiceInitializer, createServiceInitializer } from './ServiceInitializer.js';
