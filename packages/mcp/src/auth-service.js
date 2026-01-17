/**
 * CYNIC Authentication Service
 *
 * Provides authentication and rate limiting for API endpoints.
 * "φ distrusts φ" - verify all requests
 *
 * Supports:
 * - API key authentication (X-API-Key header)
 * - Bearer token authentication (session tokens)
 * - Rate limiting per IP/key
 *
 * @module @cynic/mcp/auth-service
 */

'use strict';

import crypto from 'crypto';
import { PHI_INV } from '@cynic/core';

// Default configuration
const DEFAULT_RATE_LIMIT = 100;          // Requests per window
const DEFAULT_RATE_WINDOW = 60 * 1000;   // 1 minute window
const DEFAULT_TOKEN_TTL = 24 * 60 * 60;  // 24 hours in seconds

/**
 * Generate a secure random API key
 * @returns {string} API key in format: cynic_sk_<random>
 */
export function generateApiKey() {
  const random = crypto.randomBytes(24).toString('base64url');
  return `cynic_sk_${random}`;
}

/**
 * Generate a session token
 * @param {string} sessionId - Session identifier
 * @param {string} secret - Secret key for signing
 * @returns {string} Signed session token
 */
export function generateSessionToken(sessionId, secret) {
  const payload = {
    sid: sessionId,
    iat: Math.floor(Date.now() / 1000),
    exp: Math.floor(Date.now() / 1000) + DEFAULT_TOKEN_TTL,
  };
  const data = Buffer.from(JSON.stringify(payload)).toString('base64url');
  const signature = crypto
    .createHmac('sha256', secret)
    .update(data)
    .digest('base64url');
  return `${data}.${signature}`;
}

/**
 * Verify a session token
 * @param {string} token - Token to verify
 * @param {string} secret - Secret key for verification
 * @returns {{valid: boolean, payload?: Object, error?: string}}
 */
export function verifySessionToken(token, secret) {
  try {
    const [data, signature] = token.split('.');
    if (!data || !signature) {
      return { valid: false, error: 'Invalid token format' };
    }

    // Verify signature
    const expectedSig = crypto
      .createHmac('sha256', secret)
      .update(data)
      .digest('base64url');

    if (signature !== expectedSig) {
      return { valid: false, error: 'Invalid signature' };
    }

    // Decode payload
    const payload = JSON.parse(Buffer.from(data, 'base64url').toString());

    // Check expiration
    if (payload.exp && payload.exp < Math.floor(Date.now() / 1000)) {
      return { valid: false, error: 'Token expired' };
    }

    return { valid: true, payload };
  } catch (err) {
    return { valid: false, error: err.message };
  }
}

/**
 * Authentication Service
 *
 * Handles API key validation, session tokens, and rate limiting.
 */
export class AuthService {
  /**
   * @param {Object} [options] - Configuration options
   * @param {string[]} [options.apiKeys] - Valid API keys
   * @param {string} [options.secret] - Secret for session tokens
   * @param {boolean} [options.required] - Whether auth is required (default: based on NODE_ENV)
   * @param {string[]} [options.publicPaths] - Paths that don't require auth
   * @param {number} [options.rateLimit] - Requests per window
   * @param {number} [options.rateWindow] - Rate limit window in ms
   */
  constructor(options = {}) {
    // API keys from options or environment
    this.apiKeys = new Set(options.apiKeys || []);

    // Add keys from environment variable (comma-separated)
    const envKeys = process.env.CYNIC_API_KEYS || process.env.CYNIC_API_KEY;
    if (envKeys) {
      envKeys.split(',').map(k => k.trim()).filter(Boolean).forEach(k => this.apiKeys.add(k));
    }

    // Secret for session tokens
    this.secret = options.secret || process.env.CYNIC_AUTH_SECRET || crypto.randomBytes(32).toString('hex');

    // Auth requirement based on environment
    const isProduction = process.env.NODE_ENV === 'production';
    this.required = options.required ?? isProduction;

    // Public paths that don't require auth
    this.publicPaths = new Set(options.publicPaths || [
      '/',
      '/health',
      '/metrics',
    ]);

    // Rate limiting configuration
    this.rateLimit = options.rateLimit || DEFAULT_RATE_LIMIT;
    this.rateWindow = options.rateWindow || DEFAULT_RATE_WINDOW;

    // Rate limit tracking: Map<identifier, {count, windowStart}>
    this._rateLimits = new Map();

    // Active sessions: Map<sessionId, {userId, createdAt, lastAccess}>
    this._sessions = new Map();

    // Stats
    this._stats = {
      totalRequests: 0,
      authenticatedRequests: 0,
      rejectedRequests: 0,
      rateLimitedRequests: 0,
    };

    // Cleanup interval for expired rate limits
    this._cleanupInterval = setInterval(() => this._cleanup(), this.rateWindow);
  }

  /**
   * Check if a path is public (no auth required)
   * @param {string} path - Request path
   * @returns {boolean}
   */
  isPublicPath(path) {
    return this.publicPaths.has(path);
  }

  /**
   * Validate API key
   * @param {string} apiKey - API key to validate
   * @returns {boolean}
   */
  validateApiKey(apiKey) {
    if (!apiKey) return false;
    return this.apiKeys.has(apiKey);
  }

  /**
   * Validate session token
   * @param {string} token - Bearer token
   * @returns {{valid: boolean, sessionId?: string, error?: string}}
   */
  validateToken(token) {
    const result = verifySessionToken(token, this.secret);
    if (result.valid && result.payload?.sid) {
      // Update session last access
      const session = this._sessions.get(result.payload.sid);
      if (session) {
        session.lastAccess = Date.now();
      }
      return { valid: true, sessionId: result.payload.sid };
    }
    return { valid: false, error: result.error };
  }

  /**
   * Create a new session
   * @param {string} userId - User identifier
   * @param {Object} [metadata] - Additional session metadata
   * @returns {{sessionId: string, token: string}}
   */
  createSession(userId, metadata = {}) {
    const sessionId = `ses_${Date.now().toString(36)}_${crypto.randomBytes(8).toString('hex')}`;
    const token = generateSessionToken(sessionId, this.secret);

    this._sessions.set(sessionId, {
      userId,
      metadata,
      createdAt: Date.now(),
      lastAccess: Date.now(),
    });

    return { sessionId, token };
  }

  /**
   * End a session
   * @param {string} sessionId - Session to end
   * @returns {boolean} True if session was ended
   */
  endSession(sessionId) {
    return this._sessions.delete(sessionId);
  }

  /**
   * Get session info
   * @param {string} sessionId - Session identifier
   * @returns {Object|null} Session info or null
   */
  getSession(sessionId) {
    return this._sessions.get(sessionId) || null;
  }

  /**
   * Check rate limit for an identifier
   * @param {string} identifier - IP address or API key
   * @returns {{allowed: boolean, remaining: number, resetIn: number}}
   */
  checkRateLimit(identifier) {
    const now = Date.now();
    let record = this._rateLimits.get(identifier);

    // Create new record or reset if window expired
    if (!record || now - record.windowStart >= this.rateWindow) {
      record = { count: 0, windowStart: now };
      this._rateLimits.set(identifier, record);
    }

    // Increment and check
    record.count++;
    const remaining = Math.max(0, this.rateLimit - record.count);
    const resetIn = Math.max(0, this.rateWindow - (now - record.windowStart));

    if (record.count > this.rateLimit) {
      this._stats.rateLimitedRequests++;
      return { allowed: false, remaining: 0, resetIn };
    }

    return { allowed: true, remaining, resetIn };
  }

  /**
   * Authenticate a request
   * @param {Object} req - Request object with headers
   * @param {string} [clientIp] - Client IP for rate limiting
   * @returns {{authenticated: boolean, method?: string, identifier?: string, error?: string, statusCode?: number}}
   */
  authenticate(req, clientIp = 'unknown') {
    this._stats.totalRequests++;

    // Check if path is public
    const path = req.url?.split('?')[0] || req.path || '/';
    if (this.isPublicPath(path)) {
      return { authenticated: true, method: 'public', identifier: clientIp };
    }

    // Check rate limit first
    const apiKey = req.headers?.['x-api-key'];
    const rateLimitId = apiKey || clientIp;
    const rateCheck = this.checkRateLimit(rateLimitId);

    if (!rateCheck.allowed) {
      return {
        authenticated: false,
        error: 'Rate limit exceeded',
        statusCode: 429,
        retryAfter: Math.ceil(rateCheck.resetIn / 1000),
      };
    }

    // If auth not required, allow with warning
    if (!this.required) {
      return { authenticated: true, method: 'none', identifier: clientIp };
    }

    // Try API key authentication
    if (apiKey) {
      if (this.validateApiKey(apiKey)) {
        this._stats.authenticatedRequests++;
        return { authenticated: true, method: 'api_key', identifier: apiKey.slice(0, 16) + '...' };
      }
      this._stats.rejectedRequests++;
      return { authenticated: false, error: 'Invalid API key', statusCode: 401 };
    }

    // Try Bearer token authentication
    const authHeader = req.headers?.authorization;
    if (authHeader?.startsWith('Bearer ')) {
      const token = authHeader.slice(7);
      const result = this.validateToken(token);
      if (result.valid) {
        this._stats.authenticatedRequests++;
        return { authenticated: true, method: 'bearer', identifier: result.sessionId };
      }
      this._stats.rejectedRequests++;
      return { authenticated: false, error: result.error || 'Invalid token', statusCode: 401 };
    }

    // No authentication provided
    this._stats.rejectedRequests++;
    return {
      authenticated: false,
      error: 'Authentication required. Provide X-API-Key header or Bearer token.',
      statusCode: 401,
    };
  }

  /**
   * Create HTTP middleware function
   * @returns {Function} Express/Connect-compatible middleware
   */
  middleware() {
    return (req, res, next) => {
      // Get client IP
      const clientIp = req.headers['x-forwarded-for']?.split(',')[0]?.trim()
        || req.socket?.remoteAddress
        || 'unknown';

      const result = this.authenticate(req, clientIp);

      if (!result.authenticated) {
        res.statusCode = result.statusCode || 401;

        // Add rate limit headers if applicable
        if (result.statusCode === 429) {
          res.setHeader('Retry-After', result.retryAfter || 60);
        }

        res.setHeader('Content-Type', 'application/json');
        res.end(JSON.stringify({
          error: result.error,
          statusCode: result.statusCode,
        }));
        return;
      }

      // Attach auth info to request
      req.auth = {
        method: result.method,
        identifier: result.identifier,
      };

      // Add rate limit headers
      const rateCheck = this.checkRateLimit(result.identifier || clientIp);
      res.setHeader('X-RateLimit-Limit', this.rateLimit);
      res.setHeader('X-RateLimit-Remaining', rateCheck.remaining);
      res.setHeader('X-RateLimit-Reset', Math.ceil(rateCheck.resetIn / 1000));

      next();
    };
  }

  /**
   * Get service statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      activeSessions: this._sessions.size,
      trackedRateLimits: this._rateLimits.size,
      apiKeysConfigured: this.apiKeys.size,
      authRequired: this.required,
      rateLimit: this.rateLimit,
      rateWindow: this.rateWindow,
      maxConfidence: PHI_INV,
    };
  }

  /**
   * Add an API key
   * @param {string} key - API key to add
   */
  addApiKey(key) {
    this.apiKeys.add(key);
  }

  /**
   * Remove an API key
   * @param {string} key - API key to remove
   * @returns {boolean} True if key was removed
   */
  removeApiKey(key) {
    return this.apiKeys.delete(key);
  }

  /**
   * Cleanup expired rate limit records
   * @private
   */
  _cleanup() {
    const now = Date.now();
    for (const [id, record] of this._rateLimits) {
      if (now - record.windowStart >= this.rateWindow * 2) {
        this._rateLimits.delete(id);
      }
    }
  }

  /**
   * Close the service
   */
  close() {
    if (this._cleanupInterval) {
      clearInterval(this._cleanupInterval);
      this._cleanupInterval = null;
    }
    this._rateLimits.clear();
    this._sessions.clear();
  }
}

export default AuthService;
