/**
 * CYNIC Configuration Manager
 *
 * Environment-aware configuration with secure defaults.
 * NEVER logs actual secret values.
 *
 * @module @cynic/core/config
 */

'use strict';

/**
 * Detect current environment
 */
export function detectEnvironment() {
  const env = process.env.NODE_ENV || process.env.CYNIC_ENV;

  if (env === 'production' || env === 'prod') return 'production';
  if (env === 'staging' || env === 'stage') return 'staging';
  if (env === 'test' || env === 'testing') return 'test';
  return 'development';
}

/**
 * Validate required secrets are present
 */
export function validateSecrets(required = []) {
  const missing = required.filter(key => !process.env[key]);

  if (missing.length > 0) {
    const env = detectEnvironment();
    const isProduction = env === 'production';

    if (isProduction) {
      throw new Error(`[FATAL] Missing required secrets: ${missing.join(', ')}`);
    } else {
      console.error(`[CONFIG WARNING] Missing secrets for ${env}: ${missing.join(', ')}`);
      console.error('[CONFIG WARNING] Using fallback configuration');
    }
  }

  return missing.length === 0;
}

/**
 * Mask secret for safe logging
 */
export function maskSecret(value) {
  if (!value) return '<not set>';
  if (value.length < 8) return '***';
  return `${value.slice(0, 4)}...${value.slice(-4)}`;
}

/**
 * Get database configuration
 */
export function getDatabaseConfig() {
  const url = process.env.CYNIC_DATABASE_URL;
  const env = detectEnvironment();

  // Development fallback (local Docker PostgreSQL)
  const devFallback = 'postgresql://cynic:cynic@localhost:5432/cynic?sslmode=disable';

  return {
    url: url || (env === 'development' ? devFallback : null),
    isConfigured: !!url,
    environment: env,
  };
}

/**
 * Get Redis configuration
 */
export function getRedisConfig() {
  const url = process.env.CYNIC_REDIS_URL;
  const env = detectEnvironment();

  // Development fallback (local Docker Redis)
  const devFallback = 'redis://localhost:6379';

  return {
    url: url || (env === 'development' ? devFallback : null),
    isConfigured: !!url,
    environment: env,
  };
}

/**
 * Get MCP configuration
 */
export function getMcpConfig() {
  return {
    mode: process.env.MCP_MODE || 'stdio',
    port: parseInt(process.env.PORT || process.env.MCP_PORT || '3000', 10),
  };
}

/**
 * Log configuration status (NEVER log actual secrets)
 */
export function logConfigStatus(logger = console.error) {
  const env = detectEnvironment();
  const db = getDatabaseConfig();
  const redis = getRedisConfig();

  logger(`[CONFIG] Environment: ${env}`);
  logger(`[CONFIG] PostgreSQL: ${db.isConfigured ? 'configured' : 'using dev fallback'}`);
  logger(`[CONFIG] Redis: ${redis.isConfigured ? 'configured' : 'using dev fallback'}`);
}

/**
 * Validate production readiness
 */
export function validateProductionConfig() {
  const env = detectEnvironment();

  if (env !== 'production') {
    return { valid: true, warnings: ['Not in production mode'] };
  }

  const issues = [];

  if (!process.env.CYNIC_DATABASE_URL) {
    issues.push('CYNIC_DATABASE_URL not set');
  }

  if (!process.env.CYNIC_REDIS_URL) {
    issues.push('CYNIC_REDIS_URL not set');
  }

  // Check for development indicators in production
  const dbUrl = process.env.CYNIC_DATABASE_URL || '';
  if (dbUrl.includes('localhost') || dbUrl.includes('127.0.0.1')) {
    issues.push('Production using localhost database');
  }

  return {
    valid: issues.length === 0,
    issues,
  };
}

export default {
  detectEnvironment,
  validateSecrets,
  maskSecret,
  getDatabaseConfig,
  getRedisConfig,
  getMcpConfig,
  logConfigStatus,
  validateProductionConfig,
};
