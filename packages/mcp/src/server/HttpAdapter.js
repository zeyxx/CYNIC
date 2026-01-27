/**
 * HTTP Adapter for MCP Server
 *
 * SRP: Only HTTP-related concerns
 * - HTTP server lifecycle
 * - SSE broadcasting
 * - CORS handling
 * - Request routing
 *
 * @module @cynic/mcp/server/HttpAdapter
 */

'use strict';

import { createServer } from 'http';
import { readFile } from 'fs/promises';
import { join, extname } from 'path';
import { createLogger } from '@cynic/core';

const log = createLogger('HttpAdapter');

// MIME types for static files
const MIME_TYPES = {
  '.html': 'text/html',
  '.css': 'text/css',
  '.js': 'application/javascript',
  '.json': 'application/json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
};

// HTTP mode constants
const MAX_BODY_SIZE = 1024 * 1024; // 1MB max request body
const REQUEST_TIMEOUT_MS = 30000; // 30 second request timeout
const MAX_RESPONSE_SIZE = 100 * 1024; // 100KB max response

// Rate limiting constants (φ-aligned)
const RATE_LIMIT_WINDOW_MS = 61800; // ~61.8 seconds (φ⁻¹ × 100000ms)
const RATE_LIMIT_MAX_REQUESTS = 100; // Max requests per window
const RATE_LIMIT_EXEMPT_PATHS = ['/health', '/', '/metrics']; // Public paths

/**
 * HTTP Adapter - Handles all HTTP concerns
 */
export class HttpAdapter {
  /**
   * @param {Object} options
   * @param {number} [options.port=3000]
   * @param {string} [options.dashboardPath] - Path to dashboard static files
   * @param {Object} [options.auth] - AuthService for authentication
   * @param {string[]} [options.corsOrigins] - Allowed CORS origins (null = allow all)
   */
  constructor(options = {}) {
    this.port = options.port || 3000;
    this.dashboardPath = options.dashboardPath || null;
    this.auth = options.auth || null;

    // CORS configuration - whitelist in production
    const corsEnv = process.env.CYNIC_CORS_ORIGINS;
    this.allowedOrigins = options.corsOrigins || (corsEnv ? corsEnv.split(',').map(o => o.trim()) : null);

    this._server = null;
    this._sseClients = new Set();
    this._activeRequests = new Set();

    // Rate limiting (sliding window per IP)
    this._rateLimits = new Map(); // IP -> { count, windowStart }
    this._rateLimitConfig = {
      windowMs: options.rateLimitWindow || RATE_LIMIT_WINDOW_MS,
      maxRequests: options.rateLimitMax || RATE_LIMIT_MAX_REQUESTS,
      enabled: options.rateLimit !== false, // Enabled by default
    };

    // Route handlers (injected)
    this._routes = {
      mcp: null,      // POST /mcp - JSON-RPC
      sse: null,      // GET /mcp - SSE stream
      api: null,      // GET /api/* - REST API
      hooks: null,    // POST /hooks/* - Hook events
      psychology: null, // Psychology endpoints
      metrics: null,  // GET /metrics - Prometheus format
      health: null,   // GET /health or / - Health check (custom)
    };
  }

  /**
   * Set route handler
   * @param {string} route - Route name
   * @param {Function} handler - Async handler function
   */
  setRoute(route, handler) {
    this._routes[route] = handler;
  }

  /**
   * Start HTTP server
   * @returns {Promise<void>}
   */
  async start() {
    return new Promise((resolve, reject) => {
      this._server = createServer((req, res) => this._handleRequest(req, res));

      this._server.on('error', (err) => {
        if (err.code === 'EADDRINUSE') {
          reject(new Error(`Port ${this.port} already in use`));
        } else {
          reject(err);
        }
      });

      this._server.listen(this.port, () => {
        log.info('HTTP server listening', { port: this.port });
        resolve();
      });
    });
  }

  /**
   * Stop HTTP server
   * @param {number} [timeout=10000] - Shutdown timeout in ms
   * @returns {Promise<void>}
   */
  async stop(timeout = 10000) {
    if (!this._server) return;

    // Close SSE clients
    for (const client of this._sseClients) {
      try {
        client.end();
      } catch {
        // Ignore
      }
    }
    this._sseClients.clear();

    // Wait for active requests with timeout
    if (this._activeRequests.size > 0) {
      log.debug('Waiting for active requests', { count: this._activeRequests.size });
      const deadline = Date.now() + timeout;

      while (this._activeRequests.size > 0 && Date.now() < deadline) {
        await new Promise(r => setTimeout(r, 100));
      }

      if (this._activeRequests.size > 0) {
        log.warn('Force closing requests', { count: this._activeRequests.size });
      }
    }

    // Close server
    return new Promise((resolve) => {
      this._server.close(() => {
        log.info('HTTP server closed');
        resolve();
      });
    });
  }

  /**
   * Broadcast SSE event to all clients
   * @param {string} event - Event name
   * @param {Object} data - Event data
   */
  broadcast(event, data) {
    const message = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;

    for (const client of this._sseClients) {
      try {
        client.write(message);
      } catch {
        this._sseClients.delete(client);
      }
    }
  }

  /**
   * Get SSE client count
   * @returns {number}
   */
  get sseClientCount() {
    return this._sseClients.size;
  }

  /**
   * Handle incoming HTTP request
   * @private
   */
  async _handleRequest(req, res) {
    const requestId = `req_${Date.now().toString(36)}`;
    this._activeRequests.add(requestId);

    // Request timeout
    const timeoutId = setTimeout(() => {
      if (!res.headersSent) {
        res.writeHead(408, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Request timeout' }));
      }
      this._activeRequests.delete(requestId);
    }, REQUEST_TIMEOUT_MS);

    try {
      // CORS headers
      this._setCorsHeaders(req, res);

      // Handle preflight
      if (req.method === 'OPTIONS') {
        res.writeHead(204);
        res.end();
        return;
      }

      // Parse URL once for rate limiting and routing
      const url = new URL(req.url, `http://localhost:${this.port}`);

      // Rate limiting check (skip for exempt paths)
      if (!this._checkRateLimit(req, res, url.pathname)) {
        return; // Rate limited response already sent
      }

      // Route request
      await this._routeRequest(req, res, url);

    } catch (err) {
      log.error('HTTP error', { error: err.message });
      if (!res.headersSent) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Internal server error' }));
      }
    } finally {
      clearTimeout(timeoutId);
      this._activeRequests.delete(requestId);
    }
  }

  /**
   * Route request to appropriate handler
   * @private
   */
  async _routeRequest(req, res, url) {
    const pathname = url.pathname;

    // Health check (supports custom health handler)
    if (pathname === '/health' || pathname === '/') {
      if (this._routes.health) {
        await this._routes.health(req, res, url);
      } else {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'ok' }));
      }
      return;
    }

    // Metrics endpoint (Prometheus format)
    if (pathname === '/metrics' || pathname === '/metrics/html') {
      if (this._routes.metrics) {
        await this._routes.metrics(req, res, url);
      } else {
        res.writeHead(501, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Metrics handler not configured' }));
      }
      return;
    }

    // SSE endpoint (dedicated)
    if (pathname === '/sse') {
      await this._handleSSE(req, res);
      return;
    }

    // MCP endpoint
    if (pathname === '/mcp' || pathname === '/message') {
      if (req.method === 'GET') {
        // SSE stream
        await this._handleSSE(req, res);
      } else if (req.method === 'POST') {
        // JSON-RPC
        if (this._routes.mcp) {
          await this._routes.mcp(req, res);
        } else {
          res.writeHead(501, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'MCP handler not configured' }));
        }
      }
      return;
    }

    // API endpoints
    if (pathname.startsWith('/api/')) {
      if (this._routes.api) {
        await this._routes.api(req, res, url);
      } else {
        res.writeHead(501, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'API handler not configured' }));
      }
      return;
    }

    // Hook endpoints
    if (pathname.startsWith('/hooks/')) {
      if (this._routes.hooks) {
        await this._routes.hooks(req, res, url);
      } else {
        res.writeHead(501, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Hooks handler not configured' }));
      }
      return;
    }

    // Psychology endpoints
    if (pathname.startsWith('/psychology/')) {
      if (this._routes.psychology) {
        await this._routes.psychology(req, res, url);
      } else {
        res.writeHead(501, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Psychology handler not configured' }));
      }
      return;
    }

    // Dashboard (static files)
    if (this.dashboardPath) {
      await this._serveDashboard(req, res, url);
    } else {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Not found' }));
    }
  }

  /**
   * Handle SSE connection
   * @private
   */
  async _handleSSE(req, res) {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
      'X-Accel-Buffering': 'no',
    });

    // Send initial endpoint event
    res.write('event: endpoint\ndata: /message\n\n');

    this._sseClients.add(res);

    // Cleanup on close
    req.on('close', () => {
      this._sseClients.delete(res);
    });
  }

  /**
   * Serve dashboard static files
   * @private
   */
  async _serveDashboard(req, res, url) {
    let filePath = url.pathname;

    // Strip /dashboard prefix if present (dashboard uses <base href="/dashboard/">)
    if (filePath.startsWith('/dashboard')) {
      filePath = filePath.slice('/dashboard'.length) || '/';
    }

    filePath = filePath === '/' ? '/index.html' : filePath;
    filePath = join(this.dashboardPath, filePath);

    try {
      const content = await readFile(filePath);
      const ext = extname(filePath);
      const contentType = MIME_TYPES[ext] || 'application/octet-stream';

      res.writeHead(200, { 'Content-Type': contentType });
      res.end(content);
    } catch (err) {
      if (err.code === 'ENOENT') {
        // Try index.html for SPA routing
        try {
          const indexPath = join(this.dashboardPath, 'index.html');
          const content = await readFile(indexPath);
          res.writeHead(200, { 'Content-Type': 'text/html' });
          res.end(content);
        } catch {
          res.writeHead(404, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Not found' }));
        }
      } else {
        throw err;
      }
    }
  }

  /**
   * Set CORS headers (configurable whitelist in production)
   * @param {http.IncomingMessage} req
   * @param {http.ServerResponse} res
   * @private
   */
  _setCorsHeaders(req, res) {
    const origin = req.headers.origin;
    let allowOrigin = '*';

    if (this.allowedOrigins && this.allowedOrigins.length > 0) {
      // Production: only allow whitelisted origins
      if (origin && this.allowedOrigins.includes(origin)) {
        allowOrigin = origin;
      } else if (origin) {
        // Origin not in whitelist - don't set permissive CORS
        // Return without setting Allow-Origin header (browser will block)
        return;
      }
    }

    res.setHeader('Access-Control-Allow-Origin', allowOrigin);
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Session-Id, X-User-Id');
    res.setHeader('Access-Control-Expose-Headers', 'X-Request-Id');

    // Content Security Policy
    res.setHeader('Content-Security-Policy', [
      "default-src 'self'",
      "script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net", // Dashboard + CDN libs
      "style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net",  // Dashboard + CDN styles
      "img-src 'self' data: blob:",
      "font-src 'self' https://cdn.jsdelivr.net",
      "connect-src 'self' wss: ws:",       // WebSocket connections
      "frame-ancestors 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join('; '));

    // Additional security headers
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.setHeader('X-Frame-Options', 'DENY');
    res.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin');
  }

  /**
   * Check rate limit for request
   * @param {http.IncomingMessage} req
   * @param {http.ServerResponse} res
   * @param {string} pathname
   * @returns {boolean} True if request is allowed
   * @private
   */
  _checkRateLimit(req, res, pathname) {
    // Skip if disabled or exempt path
    if (!this._rateLimitConfig.enabled) return true;
    if (RATE_LIMIT_EXEMPT_PATHS.some(p => pathname === p || pathname.startsWith(p + '/'))) {
      return true;
    }

    // Get client IP
    const ip = req.headers['x-forwarded-for']?.split(',')[0]?.trim() ||
               req.socket.remoteAddress ||
               'unknown';

    const now = Date.now();
    const { windowMs, maxRequests } = this._rateLimitConfig;

    // Get or create rate limit entry
    let entry = this._rateLimits.get(ip);
    if (!entry || now - entry.windowStart > windowMs) {
      // New window
      entry = { count: 0, windowStart: now };
      this._rateLimits.set(ip, entry);
    }

    entry.count++;

    // Add rate limit headers
    const remaining = Math.max(0, maxRequests - entry.count);
    const reset = Math.ceil((entry.windowStart + windowMs - now) / 1000);
    res.setHeader('X-RateLimit-Limit', maxRequests);
    res.setHeader('X-RateLimit-Remaining', remaining);
    res.setHeader('X-RateLimit-Reset', reset);

    // Check if over limit
    if (entry.count > maxRequests) {
      res.writeHead(429, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        error: 'Too many requests',
        retryAfter: reset,
      }));
      return false;
    }

    return true;
  }

  /**
   * Clean up old rate limit entries (call periodically)
   */
  cleanupRateLimits() {
    const now = Date.now();
    const { windowMs } = this._rateLimitConfig;
    for (const [ip, entry] of this._rateLimits) {
      if (now - entry.windowStart > windowMs * 2) {
        this._rateLimits.delete(ip);
      }
    }
  }

  /**
   * Read request body
   * @param {http.IncomingMessage} req
   * @returns {Promise<string>}
   */
  static readBody(req) {
    return new Promise((resolve, reject) => {
      let body = '';
      let size = 0;

      req.on('data', (chunk) => {
        size += chunk.length;
        if (size > MAX_BODY_SIZE) {
          reject(new Error('Request body too large'));
          req.destroy();
          return;
        }
        body += chunk;
      });

      req.on('end', () => resolve(body));
      req.on('error', reject);
    });
  }

  /**
   * Send JSON response
   * @param {http.ServerResponse} res
   * @param {number} status
   * @param {Object} data
   */
  static sendJson(res, status, data) {
    const json = JSON.stringify(data);

    // Truncate large responses
    if (json.length > MAX_RESPONSE_SIZE) {
      const truncated = {
        ...data,
        _truncated: true,
        _originalSize: json.length,
      };
      res.writeHead(status, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(truncated));
    } else {
      res.writeHead(status, { 'Content-Type': 'application/json' });
      res.end(json);
    }
  }
}
