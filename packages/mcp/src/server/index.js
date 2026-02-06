/**
 * Server Module - SRP-Compliant Components
 *
 * Extracted from MCPServer for single responsibility:
 * - HttpAdapter: HTTP/SSE/CORS
 * - ServiceInitializer: DIP-compliant service creation
 * - StdioTransport: stdin/stdout JSON-RPC transport
 * - JsonRpcHandler: Protocol routing and tool execution
 * - RouteHandlers: HTTP route domain logic
 * - InitializationPipeline: Post-ServiceInitializer setup
 * - ShutdownManager: Graceful multi-component teardown
 *
 * @module @cynic/mcp/server
 */

'use strict';

export { HttpAdapter } from './HttpAdapter.js';
export { ServiceInitializer, createServiceInitializer } from './ServiceInitializer.js';
export { StdioTransport } from './StdioTransport.js';
export { JsonRpcHandler } from './JsonRpcHandler.js';
export { RouteHandlers } from './RouteHandlers.js';
export { InitializationPipeline } from './InitializationPipeline.js';
export { ShutdownManager } from './ShutdownManager.js';
