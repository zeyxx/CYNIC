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
 * Build database URL from component env vars
 * @private
 */
function buildDatabaseUrlFromComponents() {
  const { CYNIC_DB_HOST, CYNIC_DB_PORT, CYNIC_DB_USER, CYNIC_DB_PASSWORD, CYNIC_DB_NAME } = process.env;

  if (!CYNIC_DB_HOST || !CYNIC_DB_PASSWORD) {
    return null;
  }

  const host = CYNIC_DB_HOST;
  const port = CYNIC_DB_PORT || '5432';
  const user = CYNIC_DB_USER || 'cynic';
  const name = CYNIC_DB_NAME || 'cynic';

  // Build URL from components (no hardcoded credentials)
  const url = new URL('postgresql://localhost');
  url.username = user;
  url.password = CYNIC_DB_PASSWORD;
  url.hostname = host;
  url.port = port;
  url.pathname = `/${name}`;

  // Disable SSL for local development (component-based config)
  if (host === 'localhost' || host === '127.0.0.1' || host === 'postgres') {
    url.searchParams.set('sslmode', 'disable');
  }

  return url.toString();
}

/**
 * Get database configuration
 *
 * Priority: CYNIC_DATABASE_URL > component env vars > null
 * NO hardcoded fallbacks - credentials must come from environment
 */
export function getDatabaseConfig() {
  const url = process.env.CYNIC_DATABASE_URL || buildDatabaseUrlFromComponents();
  const env = detectEnvironment();

  return {
    url,
    isConfigured: !!url,
    environment: env,
  };
}

/**
 * Get Redis configuration
 *
 * NO hardcoded fallbacks - credentials must come from environment
 */
export function getRedisConfig() {
  const url = process.env.CYNIC_REDIS_URL;
  const env = detectEnvironment();

  return {
    url,
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
  logger(`[CONFIG] PostgreSQL: ${db.isConfigured ? 'configured' : 'not configured'}`);
  logger(`[CONFIG] Redis: ${redis.isConfigured ? 'configured' : 'not configured'}`);
}

/**
 * Validate production readiness
 * In production, this throws if critical config is missing
 */
export function validateProductionConfig(options = {}) {
  const { strict = true, logger = console.error } = options;
  const env = detectEnvironment();

  if (env !== 'production') {
    return { valid: true, warnings: ['Not in production mode'], issues: [] };
  }

  const issues = [];
  const warnings = [];

  // Check database configuration
  const dbConfig = getDatabaseConfig();
  if (!dbConfig.isConfigured) {
    issues.push('Database not configured (set CYNIC_DATABASE_URL or CYNIC_DB_HOST + CYNIC_DB_PASSWORD)');
  } else if (dbConfig.url) {
    // Check for development indicators in production
    if (dbConfig.url.includes('localhost') || dbConfig.url.includes('127.0.0.1')) {
      issues.push('Production using localhost database - this is likely misconfigured');
    }
    // Check for weak passwords
    if (dbConfig.url.includes(':cynic@') || dbConfig.url.includes(':password@') || dbConfig.url.includes(':test@')) {
      warnings.push('Database password appears to be a default/weak value');
    }
  }

  // Check Redis configuration
  const redisConfig = getRedisConfig();
  if (!redisConfig.isConfigured) {
    warnings.push('Redis not configured - sessions will use in-memory storage');
  }

  // Check for NODE_ENV
  if (process.env.NODE_ENV !== 'production') {
    warnings.push(`NODE_ENV is "${process.env.NODE_ENV}" instead of "production"`);
  }

  const valid = issues.length === 0;

  // In strict mode, throw on critical issues
  if (strict && !valid) {
    logger('[FATAL] Production configuration validation failed:');
    issues.forEach(issue => logger(`  - ${issue}`));
    throw new Error(`Production config invalid: ${issues.join('; ')}`);
  }

  // Log warnings
  if (warnings.length > 0) {
    logger('[CONFIG WARNINGS]');
    warnings.forEach(w => logger(`  - ${w}`));
  }

  return { valid, issues, warnings };
}

/**
 * Validate configuration at startup
 * Call this early in your application's entry point
 */
export function validateStartupConfig(options = {}) {
  const { logger = console.error } = options;
  const env = detectEnvironment();

  logger(`[CONFIG] Starting in ${env} mode`);

  if (env === 'production') {
    return validateProductionConfig({ strict: true, logger });
  }

  if (env === 'test') {
    // Tests can run without external services
    logger('[CONFIG] Test mode - external services optional');
    return { valid: true, issues: [], warnings: [] };
  }

  // Development mode - warn if services not configured
  const dbConfig = getDatabaseConfig();
  const redisConfig = getRedisConfig();
  const warnings = [];

  if (!dbConfig.isConfigured) {
    warnings.push('PostgreSQL not configured - using in-memory storage');
  }
  if (!redisConfig.isConfigured) {
    warnings.push('Redis not configured - using in-memory cache');
  }

  if (warnings.length > 0) {
    logger('[CONFIG] Development mode warnings:');
    warnings.forEach(w => logger(`  - ${w}`));
    logger('[CONFIG] Set environment variables or use docker-compose for full functionality');
  }

  return { valid: true, issues: [], warnings };
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
  validateStartupConfig,
};
