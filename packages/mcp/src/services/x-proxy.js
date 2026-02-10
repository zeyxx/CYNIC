/**
 * X/Twitter Proxy Service
 *
 * HTTPS MITM proxy that intercepts X/Twitter traffic,
 * captures tweets/users/trends, and forwards to real X.com.
 *
 * Uses @bjowes/http-mitm-proxy for reliable HTTPS interception.
 *
 * PRIVACY BY DESIGN:
 * - All data goes to LOCAL SQLite first (LocalXStore)
 * - User explicitly chooses what to sync to cloud
 * - Cloud repository (xRepository) is optional
 *
 * "Your data, your device, your choice" - κυνικός
 *
 * @module @cynic/mcp/services/x-proxy
 */

'use strict';

import Proxy from '@bjowes/http-mitm-proxy';
import zlib from 'zlib';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { parseXResponse, parseMutation } from './x-parser.js';

// Diagnostic log file for proxy debugging
const DIAG_FILE = path.join(os.homedir(), '.cynic', 'x-proxy-diag.log');

const log = createLogger('XProxy');

// X/Twitter domains to intercept
const X_DOMAINS = new Set([
  'x.com',
  'twitter.com',
  'api.x.com',
  'api.twitter.com',
  'mobile.twitter.com',
  'mobile.x.com',
]);

// API paths that contain data we want to capture
const CAPTURE_PATHS = [
  '/i/api/graphql/',  // GraphQL API (main data source)
  '/i/api/2/',        // REST API v2
  '/1.1/',            // Legacy REST API
];

/**
 * X/Twitter Proxy Service
 *
 * Intercepts HTTPS traffic to X domains, parses responses,
 * and stores captured data LOCALLY via LocalXStore.
 *
 * Privacy-first: all data stays local unless user syncs to cloud.
 */
export class XProxyService {
  /**
   * @param {Object} options - Proxy options
   * @param {number} [options.port=8888] - Proxy port
   * @param {Object} options.localStore - LocalXStore instance (PRIMARY - local SQLite)
   * @param {Object} [options.xRepository] - XDataRepository instance (OPTIONAL - cloud PostgreSQL)
   * @param {string} [options.sslCaDir] - Directory for CA certificates
   * @param {boolean} [options.verbose=false] - Enable verbose logging
   */
  constructor(options = {}) {
    this.port = options.port || 8888;
    // PRIMARY: Local SQLite store (privacy-first)
    this.localStore = options.localStore;
    // OPTIONAL: Cloud PostgreSQL (only for explicit sync)
    this.xRepository = options.xRepository;
    this.sslCaDir = options.sslCaDir || './.cynic-proxy-certs';
    this.verbose = options.verbose || false;

    this.proxy = null;

    this._stats = {
      requestsTotal: 0,
      requestsIntercepted: 0,
      tweetsCaptured: 0,
      usersCaptured: 0,
      trendsCaptured: 0,
      actionsCaptured: 0,
      errors: 0,
      startedAt: null,
    };
  }

  /**
   * Start the proxy server
   */
  async start() {
    this.proxy = Proxy();

    // Handle errors
    this.proxy.onError((ctx, err) => {
      this._stats.errors++;
      if (this.verbose) {
        log.debug('Proxy error', {
          error: err.message,
          host: ctx?.clientToProxyRequest?.headers?.host,
        });
      }
    });

    // Intercept requests
    this.proxy.onRequest((ctx, callback) => {
      this._stats.requestsTotal++;

      const host = ctx.clientToProxyRequest.headers.host || '';
      const hostname = host.split(':')[0];

      // Check if this is an X domain
      if (this._isXDomain(hostname)) {
        this._stats.requestsIntercepted++;
        this._handleXRequest(ctx, callback);
      } else {
        // Not an X domain - pass through
        callback();
      }
    });

    // Start listening
    await new Promise((resolve, reject) => {
      this.proxy.listen({
        port: this.port,
        host: '0.0.0.0',
        sslCaDir: this.sslCaDir,
        keepAlive: true,
      }, (err) => {
        if (err) {
          reject(err);
        } else {
          this._stats.startedAt = Date.now();
          resolve();
        }
      });
    });

    log.info('X Proxy started', {
      port: this.port,
      sslCaDir: this.sslCaDir,
    });

    // Log CA certificate location for user
    console.error(`   CA Certificate: ${this.sslCaDir}/certs/ca.pem`);
    console.error(`   Install this certificate in your browser/system to enable HTTPS interception`);
  }

  /**
   * Stop the proxy server
   */
  async stop() {
    if (this.proxy) {
      await new Promise(resolve => {
        this.proxy.close();
        resolve();
      });
      this.proxy = null;
      log.info('X Proxy stopped', { stats: this._stats });
    }
  }

  /**
   * Get proxy statistics
   */
  getStats() {
    return {
      ...this._stats,
      uptime: this._stats.startedAt ? Date.now() - this._stats.startedAt : 0,
    };
  }

  /**
   * Handle request to X domain
   * @private
   */
  _handleXRequest(ctx, callback) {
    const path = ctx.clientToProxyRequest.url;
    const shouldCapture = CAPTURE_PATHS.some(p => path.includes(p));

    if (!shouldCapture) {
      // Not an API path we care about - pass through
      fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] PASS-THROUGH: ${path.slice(0, 100)}\n`);
      return callback();
    }

    fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] INTERCEPTING: ${path.slice(0, 100)}\n`);
    if (this.verbose) {
      log.debug('Intercepting X API', { path: path.slice(0, 80) });
    }

    // Capture POST request body (for mutations: likes, RTs, bookmarks)
    const method = ctx.clientToProxyRequest.method;
    const reqChunks = [];

    if (method === 'POST') {
      ctx.onRequestData((ctx, chunk, callback) => {
        reqChunks.push(chunk);
        callback(null, chunk); // Pass through unchanged
      });

      ctx.onRequestEnd((ctx, callback) => {
        // Parse mutation from request body
        this._processMutation(path, reqChunks).catch(() => {});
        callback();
      });
    }

    // Collect response chunks
    const chunks = [];

    // Modify response handling to capture data
    ctx.onResponse((ctx, callback) => {
      // Let response headers pass through
      callback();
    });

    ctx.onResponseData((ctx, chunk, callback) => {
      chunks.push(chunk);
      callback(null, chunk); // Pass chunk to client unchanged
    });

    ctx.onResponseEnd((ctx, callback) => {
      // Process captured response
      this._processResponse(ctx, chunks, path).catch(err => {
        if (this.verbose) {
          log.debug('Response processing error', { error: err.message });
        }
      });
      callback();
    });

    callback();
  }

  /**
   * Process captured response
   * @private
   */
  async _processResponse(ctx, chunks, path) {
    // Require at least localStore for privacy-first storage
    if (!this.localStore || chunks.length === 0) {
      fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] SKIP: no localStore=${!this.localStore} chunks=${chunks.length}\n`);
      return;
    }

    try {
      let body = Buffer.concat(chunks);
      const bodySize = body.length;

      // Decompress if needed
      const encoding = ctx.serverToProxyResponse?.headers?.['content-encoding'];
      fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] PROCESS: path=${path.slice(0, 80)} encoding=${encoding} size=${bodySize}\n`);
      if (encoding === 'gzip') {
        body = await this._gunzip(body);
      } else if (encoding === 'br') {
        body = await this._brotliDecompress(body);
      } else if (encoding === 'deflate') {
        body = await this._inflate(body);
      }

      // Check content type
      const contentType = ctx.serverToProxyResponse?.headers?.['content-type'] || '';
      if (!contentType.includes('json')) {
        fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] SKIP-TYPE: ${contentType.slice(0, 60)}\n`);
        return;
      }

      // Parse JSON
      const data = JSON.parse(body.toString('utf8'));

      // Extract tweets, users, trends
      const parsed = parseXResponse(path, data);
      fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] PARSED: tweets=${parsed.tweets.length} users=${parsed.users.length} trends=${parsed.trends.length}\n`);

      // Store LOCALLY first (privacy-first)
      await this._storeDataLocal(parsed, path);

    } catch (err) {
      fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] ERROR: ${err.message.slice(0, 200)}\n`);
      // Silently ignore parse errors (many responses won't be valid JSON)
      if (this.verbose && !err.message.includes('Unexpected')) {
        log.debug('Parse error', { error: err.message, path: path.slice(0, 50) });
      }
    }
  }

  /**
   * Store parsed data in LOCAL SQLite store (privacy-first)
   * @private
   */
  async _storeDataLocal(parsed, path) {
    // Store users first (tweets reference them)
    if (parsed.users.length > 0) {
      for (const user of parsed.users) {
        try {
          this.localStore.upsertUser(user);
          this._stats.usersCaptured++;
        } catch (err) {
          if (!err.message.includes('UNIQUE constraint')) {
            log.debug('User store error', { error: err.message });
          }
        }
      }
    }

    // Store tweets
    if (parsed.tweets.length > 0) {
      for (const tweet of parsed.tweets) {
        try {
          this.localStore.createTweet(tweet);
          this._stats.tweetsCaptured++;
        } catch (err) {
          if (!err.message.includes('UNIQUE constraint')) {
            log.debug('Tweet store error', { error: err.message });
          }
        }
      }

      fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] STORED: users=${parsed.users.length} tweets=${parsed.tweets.length}\n`);

      // Emit SOCIAL_CAPTURE → event bus (feeds EmergenceDetector + event-listeners)
      try {
        globalEventBus.publish(EventType.SOCIAL_CAPTURE, {
          source: 'proxy',
          tweets: parsed.tweets.length,
          users: parsed.users.length,
          path: path.slice(0, 60),
        }, { source: 'XProxy' });
      } catch { /* non-blocking */ }

      if (this.verbose) {
        log.debug('Captured locally', {
          path: path.slice(0, 60),
          tweets: parsed.tweets.length,
          users: parsed.users.length,
        });
      }
    }

    // Store trends
    if (parsed.trends.length > 0) {
      for (const trend of parsed.trends) {
        try {
          this.localStore.upsertTrend(trend);
          this._stats.trendsCaptured++;
        } catch (err) {
          log.debug('Trend store error', { error: err.message });
        }
      }
    }
  }

  /**
   * Process a GraphQL mutation (like, RT, bookmark, etc.)
   * @private
   */
  async _processMutation(path, reqChunks) {
    if (!this.localStore || reqChunks.length === 0) return;

    try {
      const body = Buffer.concat(reqChunks).toString('utf8');
      const data = JSON.parse(body);
      const action = parseMutation(path, data);

      if (action) {
        this.localStore.recordAction(action);
        this._stats.actionsCaptured++;

        fs.appendFileSync(DIAG_FILE, `[${new Date().toISOString()}] ACTION: ${action.actionType} tweet=${action.tweetId}\n`);

        if (this.verbose) {
          log.debug('Action captured', {
            type: action.actionType,
            tweet: action.tweetId,
          });
        }
      }
    } catch {
      // Request body may not be JSON (e.g., multipart) — ignore silently
    }
  }

  /**
   * Check if hostname is an X domain
   * @private
   */
  _isXDomain(hostname) {
    if (X_DOMAINS.has(hostname)) return true;

    // Check subdomains
    for (const domain of X_DOMAINS) {
      if (hostname.endsWith('.' + domain)) return true;
    }

    return false;
  }

  /**
   * Decompress gzip
   * @private
   */
  _gunzip(buffer) {
    return new Promise((resolve, reject) => {
      zlib.gunzip(buffer, (err, result) => {
        if (err) reject(err);
        else resolve(result);
      });
    });
  }

  /**
   * Decompress brotli
   * @private
   */
  _brotliDecompress(buffer) {
    return new Promise((resolve, reject) => {
      zlib.brotliDecompress(buffer, (err, result) => {
        if (err) reject(err);
        else resolve(result);
      });
    });
  }

  /**
   * Decompress deflate
   * @private
   */
  _inflate(buffer) {
    return new Promise((resolve, reject) => {
      zlib.inflate(buffer, (err, result) => {
        if (err) reject(err);
        else resolve(result);
      });
    });
  }
}

export default XProxyService;
