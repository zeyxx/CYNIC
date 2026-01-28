/**
 * @cynic/core - Config Tests
 *
 * Tests configuration management:
 * - Environment detection
 * - Secret validation
 * - Database/Redis config
 * - Production validation
 *
 * "Trust no config" - κυνικός
 *
 * @module @cynic/core/test/config
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  detectEnvironment,
  validateSecrets,
  maskSecret,
  getDatabaseConfig,
  getRedisConfig,
  getMcpConfig,
  logConfigStatus,
  validateProductionConfig,
  validateStartupConfig,
} from '../src/config/index.js';

// =============================================================================
// DETECT ENVIRONMENT TESTS
// =============================================================================

describe('detectEnvironment', () => {
  let originalNodeEnv;
  let originalCynicEnv;

  beforeEach(() => {
    originalNodeEnv = process.env.NODE_ENV;
    originalCynicEnv = process.env.CYNIC_ENV;
    delete process.env.NODE_ENV;
    delete process.env.CYNIC_ENV;
  });

  afterEach(() => {
    if (originalNodeEnv !== undefined) {
      process.env.NODE_ENV = originalNodeEnv;
    } else {
      delete process.env.NODE_ENV;
    }
    if (originalCynicEnv !== undefined) {
      process.env.CYNIC_ENV = originalCynicEnv;
    } else {
      delete process.env.CYNIC_ENV;
    }
  });

  it('should detect production environment', () => {
    process.env.NODE_ENV = 'production';
    assert.strictEqual(detectEnvironment(), 'production');

    process.env.NODE_ENV = 'prod';
    assert.strictEqual(detectEnvironment(), 'production');
  });

  it('should detect staging environment', () => {
    process.env.NODE_ENV = 'staging';
    assert.strictEqual(detectEnvironment(), 'staging');

    process.env.NODE_ENV = 'stage';
    assert.strictEqual(detectEnvironment(), 'staging');
  });

  it('should detect test environment', () => {
    process.env.NODE_ENV = 'test';
    assert.strictEqual(detectEnvironment(), 'test');

    process.env.NODE_ENV = 'testing';
    assert.strictEqual(detectEnvironment(), 'test');
  });

  it('should default to development', () => {
    delete process.env.NODE_ENV;
    delete process.env.CYNIC_ENV;
    assert.strictEqual(detectEnvironment(), 'development');

    process.env.NODE_ENV = 'dev';
    assert.strictEqual(detectEnvironment(), 'development');
  });

  it('should use CYNIC_ENV if NODE_ENV not set', () => {
    delete process.env.NODE_ENV;
    process.env.CYNIC_ENV = 'production';
    assert.strictEqual(detectEnvironment(), 'production');
  });

  it('should prefer NODE_ENV over CYNIC_ENV', () => {
    process.env.NODE_ENV = 'test';
    process.env.CYNIC_ENV = 'production';
    assert.strictEqual(detectEnvironment(), 'test');
  });
});

// =============================================================================
// VALIDATE SECRETS TESTS
// =============================================================================

describe('validateSecrets', () => {
  let originalEnv;
  let capturedErrors;
  let originalConsoleError;

  beforeEach(() => {
    originalEnv = { ...process.env };
    capturedErrors = [];
    originalConsoleError = console.error;
    console.error = (...args) => capturedErrors.push(args);
  });

  afterEach(() => {
    // Restore environment
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
    console.error = originalConsoleError;
  });

  it('should return true when all secrets present', () => {
    process.env.TEST_SECRET_1 = 'value1';
    process.env.TEST_SECRET_2 = 'value2';

    const result = validateSecrets(['TEST_SECRET_1', 'TEST_SECRET_2']);
    assert.strictEqual(result, true);
  });

  it('should return true for empty required list', () => {
    const result = validateSecrets([]);
    assert.strictEqual(result, true);
  });

  it('should return false when secrets missing in non-production', () => {
    process.env.NODE_ENV = 'development';
    delete process.env.MISSING_SECRET;

    const result = validateSecrets(['MISSING_SECRET']);
    assert.strictEqual(result, false);
    assert.ok(capturedErrors.length > 0, 'Should log warning');
  });

  it('should throw in production when secrets missing', () => {
    process.env.NODE_ENV = 'production';
    delete process.env.MISSING_PROD_SECRET;

    assert.throws(
      () => validateSecrets(['MISSING_PROD_SECRET']),
      /Missing required secrets/
    );
  });

  it('should list all missing secrets in error', () => {
    process.env.NODE_ENV = 'production';
    delete process.env.SECRET_A;
    delete process.env.SECRET_B;

    assert.throws(
      () => validateSecrets(['SECRET_A', 'SECRET_B']),
      (err) => err.message.includes('SECRET_A') && err.message.includes('SECRET_B')
    );
  });
});

// =============================================================================
// MASK SECRET TESTS
// =============================================================================

describe('maskSecret', () => {
  it('should return placeholder for empty value', () => {
    assert.strictEqual(maskSecret(''), '<not set>');
    assert.strictEqual(maskSecret(null), '<not set>');
    assert.strictEqual(maskSecret(undefined), '<not set>');
  });

  it('should mask short secrets completely', () => {
    assert.strictEqual(maskSecret('abc'), '***');
    assert.strictEqual(maskSecret('1234567'), '***');
  });

  it('should show first and last 4 chars for longer secrets', () => {
    assert.strictEqual(maskSecret('12345678'), '1234...5678');
    assert.strictEqual(maskSecret('mysupersecretkey'), 'mysu...tkey');
  });

  it('should handle exactly 8 characters', () => {
    const result = maskSecret('abcdefgh');
    assert.strictEqual(result, 'abcd...efgh');
  });
});

// =============================================================================
// GET DATABASE CONFIG TESTS
// =============================================================================

describe('getDatabaseConfig', () => {
  let originalEnv;

  beforeEach(() => {
    originalEnv = { ...process.env };
    // Clear database-related env vars
    delete process.env.CYNIC_DATABASE_URL;
    delete process.env.CYNIC_DB_HOST;
    delete process.env.CYNIC_DB_PORT;
    delete process.env.CYNIC_DB_USER;
    delete process.env.CYNIC_DB_PASSWORD;
    delete process.env.CYNIC_DB_NAME;
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should use CYNIC_DATABASE_URL if set', () => {
    process.env.CYNIC_DATABASE_URL = 'postgresql://user:pass@host:5432/db';

    const config = getDatabaseConfig();
    assert.strictEqual(config.url, 'postgresql://user:pass@host:5432/db');
    assert.strictEqual(config.isConfigured, true);
  });

  it('should build URL from components', () => {
    process.env.CYNIC_DB_HOST = 'localhost';
    process.env.CYNIC_DB_PASSWORD = 'secret';
    process.env.CYNIC_DB_USER = 'testuser';
    process.env.CYNIC_DB_NAME = 'testdb';
    process.env.CYNIC_DB_PORT = '5433';

    const config = getDatabaseConfig();
    assert.strictEqual(config.isConfigured, true);
    assert.ok(config.url.includes('testuser'));
    assert.ok(config.url.includes('localhost'));
    assert.ok(config.url.includes('5433'));
    assert.ok(config.url.includes('testdb'));
  });

  it('should use defaults for optional components', () => {
    process.env.CYNIC_DB_HOST = 'myhost';
    process.env.CYNIC_DB_PASSWORD = 'mypass';
    // No user, port, or name set

    const config = getDatabaseConfig();
    assert.strictEqual(config.isConfigured, true);
    assert.ok(config.url.includes('cynic'), 'Should use default user');
    assert.ok(config.url.includes('5432'), 'Should use default port');
  });

  it('should return null URL when not configured', () => {
    const config = getDatabaseConfig();
    assert.strictEqual(config.url, null);
    assert.strictEqual(config.isConfigured, false);
  });

  it('should include environment in config', () => {
    process.env.NODE_ENV = 'test';
    const config = getDatabaseConfig();
    assert.strictEqual(config.environment, 'test');
  });
});

// =============================================================================
// GET REDIS CONFIG TESTS
// =============================================================================

describe('getRedisConfig', () => {
  let originalEnv;

  beforeEach(() => {
    originalEnv = { ...process.env };
    delete process.env.CYNIC_REDIS_URL;
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should use CYNIC_REDIS_URL if set', () => {
    process.env.CYNIC_REDIS_URL = 'redis://localhost:6379';

    const config = getRedisConfig();
    assert.strictEqual(config.url, 'redis://localhost:6379');
    assert.strictEqual(config.isConfigured, true);
  });

  it('should return null URL when not configured', () => {
    const config = getRedisConfig();
    assert.strictEqual(config.url, undefined);
    assert.strictEqual(config.isConfigured, false);
  });

  it('should include environment in config', () => {
    process.env.NODE_ENV = 'staging';
    const config = getRedisConfig();
    assert.strictEqual(config.environment, 'staging');
  });
});

// =============================================================================
// GET MCP CONFIG TESTS
// =============================================================================

describe('getMcpConfig', () => {
  let originalEnv;

  beforeEach(() => {
    originalEnv = { ...process.env };
    delete process.env.MCP_MODE;
    delete process.env.PORT;
    delete process.env.MCP_PORT;
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should return default values', () => {
    const config = getMcpConfig();
    assert.strictEqual(config.mode, 'stdio');
    assert.strictEqual(config.port, 3000);
  });

  it('should use MCP_MODE env var', () => {
    process.env.MCP_MODE = 'http';

    const config = getMcpConfig();
    assert.strictEqual(config.mode, 'http');
  });

  it('should use PORT env var', () => {
    process.env.PORT = '8080';

    const config = getMcpConfig();
    assert.strictEqual(config.port, 8080);
  });

  it('should use MCP_PORT env var', () => {
    process.env.MCP_PORT = '9000';

    const config = getMcpConfig();
    assert.strictEqual(config.port, 9000);
  });

  it('should prefer PORT over MCP_PORT', () => {
    process.env.PORT = '8080';
    process.env.MCP_PORT = '9000';

    const config = getMcpConfig();
    assert.strictEqual(config.port, 8080);
  });
});

// =============================================================================
// LOG CONFIG STATUS TESTS
// =============================================================================

describe('logConfigStatus', () => {
  let originalEnv;
  let logOutput;

  beforeEach(() => {
    originalEnv = { ...process.env };
    logOutput = [];
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should log configuration status', () => {
    process.env.NODE_ENV = 'development';

    logConfigStatus((msg) => logOutput.push(msg));

    assert.ok(logOutput.some(msg => msg.includes('Environment')));
    assert.ok(logOutput.some(msg => msg.includes('PostgreSQL')));
    assert.ok(logOutput.some(msg => msg.includes('Redis')));
  });

  it('should use custom logger', () => {
    let customLoggerCalled = false;
    const customLogger = () => { customLoggerCalled = true; };

    logConfigStatus(customLogger);

    assert.strictEqual(customLoggerCalled, true);
  });
});

// =============================================================================
// VALIDATE PRODUCTION CONFIG TESTS
// =============================================================================

describe('validateProductionConfig', () => {
  let originalEnv;
  let logOutput;

  beforeEach(() => {
    originalEnv = { ...process.env };
    logOutput = [];
    delete process.env.CYNIC_DATABASE_URL;
    delete process.env.CYNIC_REDIS_URL;
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should return valid for non-production', () => {
    process.env.NODE_ENV = 'development';

    const result = validateProductionConfig();
    assert.strictEqual(result.valid, true);
  });

  it('should throw in production without database', () => {
    process.env.NODE_ENV = 'production';

    assert.throws(
      () => validateProductionConfig({ strict: true, logger: () => {} }),
      /Production config invalid/
    );
  });

  it('should detect localhost in production', () => {
    process.env.NODE_ENV = 'production';
    process.env.CYNIC_DATABASE_URL = 'postgresql://user:pass@localhost:5432/db';

    assert.throws(
      () => validateProductionConfig({ strict: true, logger: () => {} }),
      /localhost/i
    );
  });

  it('should warn about weak passwords', () => {
    process.env.NODE_ENV = 'production';
    process.env.CYNIC_DATABASE_URL = 'postgresql://user:cynic@prod.host:5432/db';

    const result = validateProductionConfig({ strict: false, logger: (msg) => logOutput.push(msg) });
    assert.ok(result.warnings.some(w => w.includes('password')));
  });

  it('should warn about missing Redis', () => {
    process.env.NODE_ENV = 'production';
    process.env.CYNIC_DATABASE_URL = 'postgresql://user:strongpass@prod.host:5432/db';

    const result = validateProductionConfig({ strict: false, logger: (msg) => logOutput.push(msg) });
    assert.ok(result.warnings.some(w => w.includes('Redis')));
  });

  it('should not throw when strict is false', () => {
    process.env.NODE_ENV = 'production';

    assert.doesNotThrow(() => {
      validateProductionConfig({ strict: false, logger: () => {} });
    });
  });
});

// =============================================================================
// VALIDATE STARTUP CONFIG TESTS
// =============================================================================

describe('validateStartupConfig', () => {
  let originalEnv;
  let logOutput;

  beforeEach(() => {
    originalEnv = { ...process.env };
    logOutput = [];
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should be valid in test mode', () => {
    process.env.NODE_ENV = 'test';

    const result = validateStartupConfig({ logger: (msg) => logOutput.push(msg) });
    assert.strictEqual(result.valid, true);
  });

  it('should warn in development without services', () => {
    process.env.NODE_ENV = 'development';
    delete process.env.CYNIC_DATABASE_URL;
    delete process.env.CYNIC_REDIS_URL;

    const result = validateStartupConfig({ logger: (msg) => logOutput.push(msg) });
    assert.strictEqual(result.valid, true);
    assert.ok(result.warnings.length > 0);
  });

  it('should validate in production', () => {
    process.env.NODE_ENV = 'production';
    process.env.CYNIC_DATABASE_URL = 'postgresql://user:strongpass@prod.host:5432/db';
    process.env.CYNIC_REDIS_URL = 'redis://prod.redis:6379';

    const result = validateStartupConfig({ logger: (msg) => logOutput.push(msg) });
    assert.strictEqual(result.valid, true);
  });

  it('should log startup message', () => {
    process.env.NODE_ENV = 'development';

    validateStartupConfig({ logger: (msg) => logOutput.push(msg) });

    assert.ok(logOutput.some(msg => msg.includes('Starting')));
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('Config Edge Cases', () => {
  let originalEnv;

  beforeEach(() => {
    originalEnv = { ...process.env };
  });

  afterEach(() => {
    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it('should handle empty string env values', () => {
    process.env.NODE_ENV = '';
    assert.strictEqual(detectEnvironment(), 'development');
  });

  it('should handle whitespace in env values', () => {
    process.env.NODE_ENV = '  production  ';
    // detectEnvironment does exact match, so this won't match 'production'
    assert.strictEqual(detectEnvironment(), 'development');
  });

  it('should handle special characters in database URL', () => {
    process.env.CYNIC_DATABASE_URL = 'postgresql://user:p%40ss%23word@host:5432/db';

    const config = getDatabaseConfig();
    assert.strictEqual(config.isConfigured, true);
  });
});
