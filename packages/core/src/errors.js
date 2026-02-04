/**
 * CYNIC Custom Error Types
 *
 * Hierarchical error system for programmatic error handling.
 * All errors include code, message, and optional metadata.
 *
 * "Errors are judgments too" - CYNIC
 *
 * @module @cynic/core/errors
 */

'use strict';

import { PHI_INV } from './axioms/constants.js';

/**
 * Error codes by category
 */
export const ErrorCode = {
  // Validation errors (1xxx)
  VALIDATION_FAILED: 'E1000',
  INVALID_INPUT: 'E1001',
  MISSING_REQUIRED: 'E1002',
  OUT_OF_RANGE: 'E1003',
  INVALID_FORMAT: 'E1004',
  SCHEMA_VIOLATION: 'E1005',

  // Configuration errors (2xxx)
  CONFIG_MISSING: 'E2000',
  CONFIG_INVALID: 'E2001',
  ENV_MISSING: 'E2002',
  INIT_FAILED: 'E2003',
  INITIALIZATION_FAILED: 'E2004',

  // Lifecycle errors (25xx)
  INVALID_STATE: 'E2500',
  CIRCULAR_DEPENDENCY: 'E2501',
  DEPENDENCY_MISSING: 'E2502',
  LAYER_VIOLATION: 'E2503',

  // Transport errors (3xxx)
  CONNECTION_FAILED: 'E3000',
  TIMEOUT: 'E3001',
  DISCONNECTED: 'E3002',
  MESSAGE_FAILED: 'E3003',
  SERIALIZATION_FAILED: 'E3004',

  // Consensus errors (4xxx)
  CONSENSUS_FAILED: 'E4000',
  QUORUM_NOT_REACHED: 'E4001',
  INVALID_VOTE: 'E4002',
  BLOCK_REJECTED: 'E4003',
  CHAIN_DIVERGED: 'E4004',
  INVALID_JUDGMENT: 'E4005',

  // Crypto errors (5xxx)
  CRYPTO_FAILED: 'E5000',
  SIGNATURE_INVALID: 'E5001',
  KEY_GENERATION_FAILED: 'E5002',
  DECRYPTION_FAILED: 'E5003',
  HASH_MISMATCH: 'E5004',

  // Storage errors (6xxx)
  STORAGE_FAILED: 'E6000',
  NOT_FOUND: 'E6001',
  DUPLICATE: 'E6002',
  QUERY_FAILED: 'E6003',
  CONNECTION_LOST: 'E6004',

  // Judgment errors (7xxx)
  JUDGMENT_FAILED: 'E7000',
  DIMENSION_INVALID: 'E7001',
  SCORE_OUT_OF_BOUNDS: 'E7002',
  CONFIDENCE_EXCEEDED: 'E7003',
  AXIOM_VIOLATION: 'E7004',

  // Identity errors (8xxx)
  IDENTITY_FAILED: 'E8000',
  NOT_INITIALIZED: 'E8001',
  UNAUTHORIZED: 'E8002',
  INVALID_CREDENTIAL: 'E8003',

  // Anchor/Solana errors (9xxx)
  ANCHOR_FAILED: 'E9000',
  TRANSACTION_FAILED: 'E9001',
  INSUFFICIENT_FUNDS: 'E9002',
  PROGRAM_ERROR: 'E9003',

  // Network/API errors (10xxx) - Pattern from GASdf
  NETWORK_ERROR: 'E10000',
  RATE_LIMIT: 'E10001',
  API_ERROR: 'E10002',
  TIMEOUT_ERROR: 'E10003',
  SERVER_ERROR: 'E10004',
};

/**
 * Base CYNIC Error
 *
 * All custom errors extend this class.
 * Includes error code, message, metadata, and optional cause.
 */
export class CYNICError extends Error {
  /**
   * @param {string} code - Error code from ErrorCode enum
   * @param {string} message - Human-readable error message
   * @param {Object} [metadata={}] - Additional context
   * @param {Error} [cause] - Original error if wrapping
   * @param {number} [statusCode] - HTTP status code (pattern from GASdf)
   */
  constructor(code, message, metadata = {}, cause = null, statusCode = null) {
    super(message);
    this.name = 'CYNICError';
    this.code = code;
    this.metadata = metadata;
    this.cause = cause;
    this.statusCode = statusCode;
    this.timestamp = Date.now();
    this.confidence = PHI_INV; // Even errors have max 61.8% certainty

    // Capture stack trace
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Convert to JSON for logging/serialization
   */
  toJSON() {
    return {
      name: this.name,
      code: this.code,
      message: this.message,
      statusCode: this.statusCode,
      metadata: this.metadata,
      timestamp: this.timestamp,
      confidence: this.confidence,
      stack: this.stack,
      cause: this.cause?.message,
    };
  }

  /**
   * Check if error is of specific type
   * @param {string} code - Error code to check
   */
  isCode(code) {
    return this.code === code;
  }

  /**
   * Check if error is in category (by prefix)
   * @param {string} prefix - Category prefix (e.g., 'E1' for validation)
   */
  isCategory(prefix) {
    return this.code.startsWith(prefix);
  }
}

/**
 * Validation Error
 *
 * Thrown when input validation fails.
 */
export class ValidationError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.VALIDATION_FAILED, message, metadata, cause);
    this.name = 'ValidationError';
  }

  /**
   * Create for missing required field
   */
  static missingRequired(field) {
    return new ValidationError(
      `Missing required field: ${field}`,
      { field },
    );
  }

  /**
   * Create for invalid format
   */
  static invalidFormat(field, expected) {
    return new ValidationError(
      `Invalid format for ${field}: expected ${expected}`,
      { field, expected },
    );
  }

  /**
   * Create for out of range value
   */
  static outOfRange(field, min, max, actual) {
    return new ValidationError(
      `Value for ${field} out of range: ${actual} (expected ${min}-${max})`,
      { field, min, max, actual },
    );
  }
}

/**
 * Configuration Error
 *
 * Thrown when configuration is missing or invalid.
 */
export class ConfigurationError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.CONFIG_INVALID, message, metadata, cause);
    this.name = 'ConfigurationError';
  }

  /**
   * Create for missing config
   */
  static missing(key) {
    const error = new ConfigurationError(
      `Missing configuration: ${key}`,
      { key },
    );
    error.code = ErrorCode.CONFIG_MISSING;
    return error;
  }

  /**
   * Create for missing env var
   */
  static envMissing(envVar) {
    const error = new ConfigurationError(
      `Missing environment variable: ${envVar}`,
      { envVar },
    );
    error.code = ErrorCode.ENV_MISSING;
    return error;
  }
}

/**
 * Transport Error
 *
 * Thrown for network/communication failures.
 */
export class TransportError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.CONNECTION_FAILED, message, metadata, cause);
    this.name = 'TransportError';
  }

  /**
   * Create for timeout
   */
  static timeout(operation, timeoutMs) {
    const error = new TransportError(
      `Operation timed out: ${operation} (${timeoutMs}ms)`,
      { operation, timeoutMs },
    );
    error.code = ErrorCode.TIMEOUT;
    return error;
  }

  /**
   * Create for disconnection
   */
  static disconnected(endpoint) {
    const error = new TransportError(
      `Disconnected from: ${endpoint}`,
      { endpoint },
    );
    error.code = ErrorCode.DISCONNECTED;
    return error;
  }
}

/**
 * Consensus Error
 *
 * Thrown for PoJ consensus failures.
 */
export class ConsensusError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.CONSENSUS_FAILED, message, metadata, cause);
    this.name = 'ConsensusError';
  }

  /**
   * Create for quorum not reached
   */
  static quorumNotReached(required, received) {
    const error = new ConsensusError(
      `Quorum not reached: ${received}/${required} votes`,
      { required, received },
    );
    error.code = ErrorCode.QUORUM_NOT_REACHED;
    return error;
  }

  /**
   * Create for invalid judgment
   */
  static invalidJudgment(reason) {
    const error = new ConsensusError(
      `Invalid judgment: ${reason}`,
      { reason },
    );
    error.code = ErrorCode.INVALID_JUDGMENT;
    return error;
  }
}

/**
 * Crypto Error
 *
 * Thrown for cryptographic operation failures.
 */
export class CryptoError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.CRYPTO_FAILED, message, metadata, cause);
    this.name = 'CryptoError';
  }

  /**
   * Create for invalid signature
   */
  static signatureInvalid(publicKey) {
    const error = new CryptoError(
      'Signature verification failed',
      { publicKey: publicKey?.slice(0, 16) + '...' },
    );
    error.code = ErrorCode.SIGNATURE_INVALID;
    return error;
  }

  /**
   * Create for hash mismatch
   */
  static hashMismatch(expected, actual) {
    const error = new CryptoError(
      'Hash mismatch detected',
      {
        expected: expected?.slice(0, 16) + '...',
        actual: actual?.slice(0, 16) + '...',
      },
    );
    error.code = ErrorCode.HASH_MISMATCH;
    return error;
  }
}

/**
 * Storage Error
 *
 * Thrown for database/persistence failures.
 */
export class StorageError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.STORAGE_FAILED, message, metadata, cause);
    this.name = 'StorageError';
  }

  /**
   * Create for not found
   */
  static notFound(entity, id) {
    const error = new StorageError(
      `${entity} not found: ${id}`,
      { entity, id },
    );
    error.code = ErrorCode.NOT_FOUND;
    return error;
  }

  /**
   * Create for duplicate
   */
  static duplicate(entity, id) {
    const error = new StorageError(
      `${entity} already exists: ${id}`,
      { entity, id },
    );
    error.code = ErrorCode.DUPLICATE;
    return error;
  }
}

/**
 * Judgment Error
 *
 * Thrown for CYNIC judgment system failures.
 */
export class JudgmentError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.JUDGMENT_FAILED, message, metadata, cause);
    this.name = 'JudgmentError';
  }

  /**
   * Create for confidence exceeded
   */
  static confidenceExceeded(confidence) {
    const error = new JudgmentError(
      `Confidence ${confidence} exceeds φ⁻¹ (${PHI_INV})`,
      { confidence, max: PHI_INV },
    );
    error.code = ErrorCode.CONFIDENCE_EXCEEDED;
    return error;
  }

  /**
   * Create for axiom violation
   */
  static axiomViolation(axiom, reason) {
    const error = new JudgmentError(
      `Axiom ${axiom} violated: ${reason}`,
      { axiom, reason },
    );
    error.code = ErrorCode.AXIOM_VIOLATION;
    return error;
  }
}

/**
 * Identity Error
 *
 * Thrown for identity/auth failures.
 */
export class IdentityError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.IDENTITY_FAILED, message, metadata, cause);
    this.name = 'IdentityError';
  }

  /**
   * Create for not initialized
   */
  static notInitialized(component) {
    const error = new IdentityError(
      `${component} not initialized. Call initialize() first.`,
      { component },
    );
    error.code = ErrorCode.NOT_INITIALIZED;
    return error;
  }

  /**
   * Create for unauthorized
   */
  static unauthorized(action) {
    const error = new IdentityError(
      `Unauthorized: ${action}`,
      { action },
    );
    error.code = ErrorCode.UNAUTHORIZED;
    return error;
  }
}

/**
 * Anchor Error
 *
 * Thrown for Solana/Anchor failures.
 */
export class AnchorError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.ANCHOR_FAILED, message, metadata, cause);
    this.name = 'AnchorError';
  }

  /**
   * Create for transaction failure
   */
  static transactionFailed(signature, reason) {
    const error = new AnchorError(
      `Transaction failed: ${reason}`,
      { signature: signature?.slice(0, 16) + '...', reason },
    );
    error.code = ErrorCode.TRANSACTION_FAILED;
    return error;
  }

  /**
   * Create for insufficient funds
   */
  static insufficientFunds(required, available) {
    const error = new AnchorError(
      `Insufficient funds: need ${required} SOL, have ${available} SOL`,
      { required, available },
    );
    error.code = ErrorCode.INSUFFICIENT_FUNDS;
    return error;
  }
}

/**
 * Wrap an error in a CYNIC error type
 * @param {Error} error - Original error
 * @param {typeof CYNICError} ErrorClass - Error class to wrap with
 * @returns {CYNICError}
 */
export function wrapError(error, ErrorClass = CYNICError) {
  if (error instanceof CYNICError) {
    return error;
  }
  return new ErrorClass(error.message, {}, error);
}

/**
 * Check if error is a CYNIC error
 * @param {Error} error
 * @returns {boolean}
 */
export function isCYNICError(error) {
  return error instanceof CYNICError;
}

/**
 * Check if error is of specific category
 * @param {Error} error
 * @param {string} category - Category prefix
 * @returns {boolean}
 */
export function isErrorCategory(error, category) {
  return error instanceof CYNICError && error.isCategory(category);
}

// ============================================================================
// Network/API Errors (Pattern adopted from GASdf)
// ============================================================================

/**
 * Network Error
 *
 * Thrown for network/connection failures (fetch errors, DNS, etc.)
 * Pattern: GASdf error hierarchy
 */
export class NetworkError extends CYNICError {
  constructor(message, metadata = {}, cause = null) {
    super(ErrorCode.NETWORK_ERROR, message, metadata, cause);
    this.name = 'NetworkError';
  }

  /**
   * Create for connection refused
   */
  static connectionRefused(endpoint) {
    return new NetworkError(
      `Connection refused: ${endpoint}`,
      { endpoint },
    );
  }

  /**
   * Create for DNS failure
   */
  static dnsFailure(hostname) {
    return new NetworkError(
      `DNS lookup failed: ${hostname}`,
      { hostname },
    );
  }

  /**
   * Create for fetch error
   */
  static fetchFailed(url, reason) {
    return new NetworkError(
      `Fetch failed for ${url}: ${reason}`,
      { url, reason },
    );
  }
}

/**
 * Rate Limit Error
 *
 * Thrown when API rate limit is exceeded (HTTP 429).
 * Pattern: GASdf error hierarchy
 */
export class RateLimitError extends CYNICError {
  /**
   * @param {number} [retryAfter] - Seconds until retry is allowed
   */
  constructor(retryAfter = null) {
    super(
      ErrorCode.RATE_LIMIT,
      'Rate limit exceeded',
      { retryAfter },
      null,
      429,
    );
    this.name = 'RateLimitError';
    this.retryAfter = retryAfter;
  }
}

/**
 * API Error
 *
 * Generic API error with status code.
 * Pattern: GASdf error hierarchy
 */
export class ApiError extends CYNICError {
  constructor(message, statusCode, metadata = {}) {
    super(ErrorCode.API_ERROR, message, metadata, null, statusCode);
    this.name = 'ApiError';
  }

  /**
   * Create for server error (5xx)
   */
  static serverError(statusCode, message = 'Internal server error') {
    const error = new ApiError(message, statusCode);
    error.code = ErrorCode.SERVER_ERROR;
    return error;
  }
}

/**
 * Parse API error response into appropriate error class
 *
 * Pattern adopted from GASdf - converts HTTP responses to typed errors.
 *
 * @param {number} status - HTTP status code
 * @param {Object|string} body - Response body
 * @returns {CYNICError}
 */
export function parseApiError(status, body) {
  const data = typeof body === 'string' ? { error: body } : body || {};
  const message = data.error || data.message || 'Unknown error';
  const errors = data.errors || [];

  switch (status) {
    case 400:
      return new ValidationError(message, { errors });

    case 401:
      return IdentityError.unauthorized(message);

    case 403:
      return IdentityError.unauthorized(`Forbidden: ${message}`);

    case 404:
      return StorageError.notFound(data.entity || 'Resource', data.id || 'unknown');

    case 429: {
      const retryAfter = data.retryAfter || data.retry_after;
      return new RateLimitError(retryAfter);
    }

    case 500:
    case 502:
    case 503:
    case 504:
      return ApiError.serverError(status, message);

    default:
      return new ApiError(message, status, { errors });
  }
}

/**
 * Check if error is retryable
 *
 * @param {Error} error
 * @returns {boolean}
 */
export function isRetryableError(error) {
  if (error instanceof RateLimitError) return true;
  if (error instanceof NetworkError) return true;
  if (error instanceof ApiError && error.statusCode >= 500) return true;
  if (error instanceof TransportError) return true;
  return false;
}

/**
 * Get retry delay for error (in ms)
 *
 * @param {Error} error
 * @param {number} attempt - Current attempt number (0-indexed)
 * @returns {number} Delay in milliseconds
 */
export function getRetryDelay(error, attempt = 0) {
  // Rate limit: use retryAfter if available
  if (error instanceof RateLimitError && error.retryAfter) {
    return error.retryAfter * 1000;
  }

  // Exponential backoff with φ-aligned jitter
  // Base: 1s, multiplier: φ, max: 30s
  const baseDelay = 1000;
  const maxDelay = 30000;
  const delay = Math.min(baseDelay * Math.pow(1.618, attempt), maxDelay);

  // Add jitter: ±φ⁻² (38.2%)
  const jitter = delay * (Math.random() - 0.5) * 2 * 0.382;
  return Math.round(delay + jitter);
}
