/**
 * Redis Client - Session & Cache Manager
 *
 * Uses ioredis for Redis connections.
 * Handles sessions, library cache, and ephemeral state.
 *
 * @module @cynic/persistence/redis
 */

'use strict';

import Redis from 'ioredis';
import { secureToken, createLogger } from '@cynic/core';

const log = createLogger('RedisClient');

// Singleton instance
let redis = null;

/**
 * Circuit breaker states
 */
const CircuitState = {
  CLOSED: 'CLOSED',     // Normal operation
  OPEN: 'OPEN',         // Failing, reject requests
  HALF_OPEN: 'HALF_OPEN', // Testing if recovered
};

/**
 * Circuit breaker configuration (φ-derived)
 */
const CIRCUIT_BREAKER_CONFIG = {
  failureThreshold: 5,           // Open after 5 consecutive failures
  resetTimeoutMs: 61800,         // φ⁻¹ × 100000 - time before trying again
  halfOpenMaxRequests: 1,        // Allow 1 test request in half-open
};

/**
 * Default TTLs (φ-derived, in seconds)
 */
export const TTL = {
  SESSION: 61800,        // ~17h - φ⁻¹ × 100000
  LIBRARY_CACHE: 86400,  // 24h - library docs cache
  PATTERN_CACHE: 3600,   // 1h - pattern cache
  RATE_LIMIT: 60,        // 1min - rate limit window
  LOCK: 30,              // 30s - distributed lock
};

/**
 * Key prefixes for namespacing
 */
export const PREFIX = {
  SESSION: 'cynic:session:',
  USER: 'cynic:user:',
  LIBRARY: 'cynic:lib:',
  PATTERN: 'cynic:pattern:',
  RATE: 'cynic:rate:',
  LOCK: 'cynic:lock:',
  JUDGMENT: 'cynic:jdg:',
};

/**
 * Lua script for atomic lock release (Redis EVAL, not JS eval)
 * This is safe - it runs server-side in Redis, not in Node.js
 */
const RELEASE_LOCK_SCRIPT = `
  if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
  else
    return 0
  end
`;

/**
 * Lua script for atomic JSON field increment
 * Increments a numeric field in a JSON object stored at key
 * Returns the updated JSON string
 */
const INCREMENT_JSON_FIELD_SCRIPT = `
  local key = KEYS[1]
  local field = ARGV[1]
  local ttl = tonumber(ARGV[2])

  local data = redis.call("get", key)
  if not data then
    return nil
  end

  local obj = cjson.decode(data)
  if type(obj[field]) == "number" then
    obj[field] = obj[field] + 1
  end
  obj["lastActiveAt"] = ARGV[3]

  local updated = cjson.encode(obj)
  if ttl > 0 then
    redis.call("setex", key, ttl, updated)
  else
    redis.call("set", key, updated)
  end

  return updated
`;

/**
 * Redis Client wrapper
 */
export class RedisClient {
  constructor(url, config = {}) {
    this.url = url || process.env.CYNIC_REDIS_URL;
    this.client = null;

    // Circuit breaker state
    this._circuitState = CircuitState.CLOSED;
    this._failureCount = 0;
    this._lastFailureTime = 0;
    this._circuitConfig = { ...CIRCUIT_BREAKER_CONFIG, ...config.circuitBreaker };
  }

  /**
   * Get current circuit breaker state
   */
  getCircuitState() {
    return {
      state: this._circuitState,
      failureCount: this._failureCount,
      lastFailureTime: this._lastFailureTime,
    };
  }

  /**
   * Check if circuit allows request
   * @private
   */
  _canExecute() {
    if (this._circuitState === CircuitState.CLOSED) {
      return true;
    }

    if (this._circuitState === CircuitState.OPEN) {
      // Check if enough time has passed to try again
      const now = Date.now();
      if (now - this._lastFailureTime >= this._circuitConfig.resetTimeoutMs) {
        this._circuitState = CircuitState.HALF_OPEN;
        return true;
      }
      return false;
    }

    // HALF_OPEN: allow limited requests to test recovery
    return true;
  }

  /**
   * Record successful operation
   * @private
   */
  _recordSuccess() {
    if (this._circuitState === CircuitState.HALF_OPEN) {
      // Successfully recovered - close circuit
      this._circuitState = CircuitState.CLOSED;
      this._failureCount = 0;
      log.info('Circuit breaker: CLOSED (recovered)');
    } else {
      // Reset failure count on success
      this._failureCount = 0;
    }
  }

  /**
   * Record failed operation
   * @private
   */
  _recordFailure() {
    this._failureCount++;
    this._lastFailureTime = Date.now();

    if (this._circuitState === CircuitState.HALF_OPEN) {
      // Failed during recovery test - reopen circuit
      this._circuitState = CircuitState.OPEN;
      log.warn('Circuit breaker: OPEN (recovery failed)');
    } else if (this._failureCount >= this._circuitConfig.failureThreshold) {
      // Too many failures - open circuit
      this._circuitState = CircuitState.OPEN;
      log.warn('Circuit breaker: OPEN', { failures: this._failureCount });
    }
  }

  /**
   * Reset circuit breaker (for manual recovery)
   */
  resetCircuitBreaker() {
    this._circuitState = CircuitState.CLOSED;
    this._failureCount = 0;
    this._lastFailureTime = 0;
    log.info('Circuit breaker: manually reset to CLOSED');
  }

  /**
   * Connect to Redis
   */
  async connect() {
    if (this.client) return this;

    if (!this.url) {
      throw new Error('CYNIC_REDIS_URL not set');
    }

    this.client = new Redis(this.url, {
      maxRetriesPerRequest: 3,
      retryDelayOnFailover: 100,
      lazyConnect: true,
    });

    await this.client.connect();
    log.info('Redis connected');

    return this;
  }

  /**
   * Disconnect from Redis
   */
  async close() {
    if (this.client) {
      await this.client.quit();
      this.client = null;
      log.info('Redis disconnected');
    }
  }

  // ==========================================================================
  // BASIC OPERATIONS (circuit breaker protected)
  // ==========================================================================

  /**
   * Execute operation with circuit breaker protection
   * @private
   */
  async _executeProtected(operation) {
    // Check circuit breaker
    if (!this._canExecute()) {
      const err = new Error('Circuit breaker is OPEN - Redis unavailable');
      err.code = 'CIRCUIT_OPEN';
      err.circuitState = this.getCircuitState();
      throw err;
    }

    try {
      const result = await operation();
      this._recordSuccess();
      return result;
    } catch (error) {
      this._recordFailure();
      throw error;
    }
  }

  async get(key) {
    return this._executeProtected(async () => {
      const value = await this.client.get(key);
      try {
        return value ? JSON.parse(value) : null;
      } catch {
        return value;
      }
    });
  }

  async set(key, value, ttl = null) {
    return this._executeProtected(async () => {
      const serialized = typeof value === 'string' ? value : JSON.stringify(value);
      if (ttl) {
        await this.client.setex(key, ttl, serialized);
      } else {
        await this.client.set(key, serialized);
      }
    });
  }

  async del(key) {
    return this._executeProtected(() => this.client.del(key));
  }

  async exists(key) {
    return this._executeProtected(() => this.client.exists(key));
  }

  async expire(key, ttl) {
    return this._executeProtected(() => this.client.expire(key, ttl));
  }

  async ttl(key) {
    return this._executeProtected(() => this.client.ttl(key));
  }

  // ==========================================================================
  // BATCH OPERATIONS
  // ==========================================================================

  /**
   * Get multiple keys in one round-trip
   * @param {string[]} keys - Keys to fetch
   * @returns {Promise<Map<string, any>>} Map of key → value (null for missing)
   */
  async mget(keys) {
    if (!keys || keys.length === 0) return new Map();

    return this._executeProtected(async () => {
      const values = await this.client.mget(...keys);
      const result = new Map();

      keys.forEach((key, i) => {
        const value = values[i];
        if (value) {
          try {
            result.set(key, JSON.parse(value));
          } catch {
            result.set(key, value);
          }
        } else {
          result.set(key, null);
        }
      });

      return result;
    });
  }

  /**
   * Set multiple keys in one round-trip
   * @param {Map<string, any>|Object} entries - Key-value pairs
   * @param {number} [ttl] - Optional TTL for all keys
   */
  async mset(entries, ttl = null) {
    const map = entries instanceof Map ? entries : new Map(Object.entries(entries));
    if (map.size === 0) return;

    return this._executeProtected(async () => {
      if (ttl) {
        // With TTL: use pipeline for atomic SETEX commands
        const pipeline = this.client.pipeline();
        for (const [key, value] of map) {
          const serialized = typeof value === 'string' ? value : JSON.stringify(value);
          pipeline.setex(key, ttl, serialized);
        }
        await pipeline.exec();
      } else {
        // Without TTL: use native MSET
        const args = [];
        for (const [key, value] of map) {
          args.push(key, typeof value === 'string' ? value : JSON.stringify(value));
        }
        await this.client.mset(...args);
      }
    });
  }

  /**
   * Delete multiple keys in one round-trip
   * @param {string[]} keys - Keys to delete
   * @returns {Promise<number>} Number of keys deleted
   */
  async mdel(keys) {
    if (!keys || keys.length === 0) return 0;
    return this._executeProtected(() => this.client.del(...keys));
  }

  /**
   * Execute multiple commands in a pipeline (batched round-trip)
   * @param {Function} callback - Receives pipeline, call commands on it
   * @returns {Promise<Array>} Results array
   * @example
   * const results = await redis.pipeline(p => {
   *   p.get('key1');
   *   p.set('key2', 'value');
   *   p.incr('counter');
   * });
   */
  async pipeline(callback) {
    return this._executeProtected(async () => {
      const p = this.client.pipeline();
      callback(p);
      const results = await p.exec();
      // ioredis returns [[err, result], ...] - extract results
      return results.map(([err, result]) => {
        if (err) throw err;
        return result;
      });
    });
  }

  /**
   * Scan keys matching pattern (cursor-based, memory-safe)
   * @param {string} pattern - Glob pattern (e.g., 'cynic:session:*')
   * @param {number} [count=100] - Hint for batch size
   * @returns {AsyncGenerator<string>} Yields keys
   */
  async *scan(pattern, count = 100) {
    let cursor = '0';
    do {
      const [nextCursor, keys] = await this._executeProtected(() =>
        this.client.scan(cursor, 'MATCH', pattern, 'COUNT', count)
      );
      cursor = nextCursor;
      for (const key of keys) {
        yield key;
      }
    } while (cursor !== '0');
  }

  /**
   * Get all keys matching pattern (use scan for large datasets)
   * @param {string} pattern - Glob pattern
   * @returns {Promise<string[]>} Matching keys
   */
  async keys(pattern) {
    return this._executeProtected(() => this.client.keys(pattern));
  }

  // ==========================================================================
  // SESSION OPERATIONS
  // ==========================================================================

  async getSession(sessionId) {
    return this.get(`${PREFIX.SESSION}${sessionId}`);
  }

  async setSession(sessionId, data, ttl = TTL.SESSION) {
    await this.set(`${PREFIX.SESSION}${sessionId}`, data, ttl);
  }

  async deleteSession(sessionId) {
    return this.del(`${PREFIX.SESSION}${sessionId}`);
  }

  async touchSession(sessionId, ttl = TTL.SESSION) {
    return this.expire(`${PREFIX.SESSION}${sessionId}`, ttl);
  }

  /**
   * Get multiple sessions in one round-trip
   * @param {string[]} sessionIds - Session IDs
   * @returns {Promise<Map<string, Object>>} Map of sessionId → session
   */
  async getSessions(sessionIds) {
    if (!sessionIds || sessionIds.length === 0) return new Map();
    const keys = sessionIds.map(id => `${PREFIX.SESSION}${id}`);
    const results = await this.mget(keys);

    // Remap keys back to session IDs
    const sessions = new Map();
    sessionIds.forEach((id, i) => {
      sessions.set(id, results.get(keys[i]));
    });
    return sessions;
  }

  /**
   * Set multiple sessions in one round-trip
   * @param {Map<string, Object>|Object} sessions - sessionId → data
   * @param {number} [ttl] - TTL (default: SESSION TTL)
   */
  async setSessions(sessions, ttl = TTL.SESSION) {
    const map = sessions instanceof Map ? sessions : new Map(Object.entries(sessions));
    if (map.size === 0) return;

    const entries = new Map();
    for (const [id, data] of map) {
      entries.set(`${PREFIX.SESSION}${id}`, data);
    }
    await this.mset(entries, ttl);
  }

  /**
   * Atomic increment of a field in a session JSON object
   * Uses Lua script to avoid race conditions
   */
  async incrementSessionField(sessionId, field, ttl = TTL.SESSION) {
    return this._executeProtected(async () => {
      const key = `${PREFIX.SESSION}${sessionId}`;
      const timestamp = new Date().toISOString();

      const result = await this.client.call(
        'EVAL',
        INCREMENT_JSON_FIELD_SCRIPT,
        1,
        key,
        field,
        ttl.toString(),
        timestamp
      );

      if (!result) return null;

      try {
        return JSON.parse(result);
      } catch {
        return result;
      }
    });
  }

  /**
   * Atomic session creation using SETNX
   * Returns { created: true, session } if new, { created: false } if exists
   */
  async createSessionIfNotExists(sessionId, data, ttl = TTL.SESSION) {
    return this._executeProtected(async () => {
      const key = `${PREFIX.SESSION}${sessionId}`;
      const serialized = JSON.stringify(data);

      // SET ... NX returns OK if set, null if key exists
      const result = await this.client.set(key, serialized, 'EX', ttl, 'NX');

      if (result === 'OK') {
        return { created: true, session: data };
      }

      // Key already exists - get the existing session
      const existing = await this.get(key);
      return { created: false, session: existing };
    });
  }

  // ==========================================================================
  // LIBRARY CACHE
  // ==========================================================================

  async getLibraryDoc(libraryId, query) {
    const key = `${PREFIX.LIBRARY}${libraryId}:${this._hash(query)}`;
    return this.get(key);
  }

  async setLibraryDoc(libraryId, query, content, ttl = TTL.LIBRARY_CACHE) {
    const key = `${PREFIX.LIBRARY}${libraryId}:${this._hash(query)}`;
    await this.set(key, content, ttl);
  }

  // ==========================================================================
  // RATE LIMITING (circuit breaker protected)
  // ==========================================================================

  async checkRateLimit(identifier, limit = 100, window = TTL.RATE_LIMIT) {
    return this._executeProtected(async () => {
      const key = `${PREFIX.RATE}${identifier}`;
      const current = await this.client.incr(key);

      if (current === 1) {
        await this.client.expire(key, window);
      }

      return {
        allowed: current <= limit,
        current,
        limit,
        remaining: Math.max(0, limit - current),
      };
    });
  }

  // ==========================================================================
  // DISTRIBUTED LOCK (circuit breaker protected, using Redis Lua scripting)
  // ==========================================================================

  async acquireLock(resource, ttl = TTL.LOCK) {
    return this._executeProtected(async () => {
      const key = `${PREFIX.LOCK}${resource}`;
      const token = secureToken();

      const acquired = await this.client.set(key, token, 'EX', ttl, 'NX');
      return acquired ? token : null;
    });
  }

  /**
   * Release a distributed lock atomically
   * Uses Redis EVAL (Lua scripting) - NOT JavaScript eval
   * This is safe: Lua runs server-side in Redis
   */
  async releaseLock(resource, token) {
    return this._executeProtected(async () => {
      const key = `${PREFIX.LOCK}${resource}`;
      // ioredis evalsha/eval runs Lua on Redis server, not JS
      return this.client.call('EVAL', RELEASE_LOCK_SCRIPT, 1, key, token);
    });
  }

  // ==========================================================================
  // HELPERS
  // ==========================================================================

  _hash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) - hash) + str.charCodeAt(i);
      hash = hash & hash;
    }
    return Math.abs(hash).toString(36);
  }

  /**
   * Health check (includes circuit breaker state)
   */
  async health() {
    const circuitState = this.getCircuitState();

    // If circuit is open, report unhealthy without hitting Redis
    if (this._circuitState === CircuitState.OPEN) {
      return {
        status: 'unhealthy',
        error: 'Circuit breaker is OPEN',
        circuitBreaker: circuitState,
      };
    }

    try {
      const start = Date.now();
      await this.client.ping();
      const latency = Date.now() - start;

      const info = await this.client.info('memory');
      const memMatch = info.match(/used_memory_human:(\S+)/);

      // If we got here from HALF_OPEN, record success
      if (this._circuitState === CircuitState.HALF_OPEN) {
        this._recordSuccess();
      }

      return {
        status: 'healthy',
        latency,
        memory: memMatch ? memMatch[1] : 'unknown',
        circuitBreaker: circuitState,
      };
    } catch (error) {
      this._recordFailure();
      return {
        status: 'unhealthy',
        error: error.message,
        circuitBreaker: this.getCircuitState(),
      };
    }
  }
}

/**
 * Get shared Redis instance (singleton)
 */
export function getRedis(url) {
  if (!redis) {
    redis = new RedisClient(url);
  }
  return redis;
}

export default RedisClient;
