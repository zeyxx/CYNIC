/**
 * CYNIC Security Patterns
 *
 * "Don't trust, verify" - φ⁻¹ guides all ratios
 *
 * Pattern database for detecting sensitive data in code.
 * Severity weighted by φ (golden ratio) for mathematical harmony.
 *
 * @module cynic/lib/security-patterns
 */

'use strict';

// =============================================================================
// φ-WEIGHTED SEVERITY LEVELS
// =============================================================================

const PHI = 1.618033988749895;
const SEVERITY_WEIGHTS = {
  critical: PHI * PHI,      // φ² = 2.618 - immediate block
  high: PHI,                // φ¹ = 1.618 - strong warning
  medium: 1,                // φ⁰ = 1.000 - warning
  low: 1 / PHI              // φ⁻¹ = 0.618 - notice
};

// =============================================================================
// SECRET PATTERNS
// =============================================================================

const SECRET_PATTERNS = [
  // ═══════════════════════════════════════════════════════════════════════════
  // CRITICAL - Private Keys & Root Secrets (φ² severity)
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'private-key-rsa',
    pattern: /-----BEGIN (?:RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----/,
    severity: 'critical',
    description: 'Private key detected',
    recommendation: 'Never commit private keys. Use secrets manager.'
  },
  {
    id: 'private-key-pgp',
    pattern: /-----BEGIN PGP PRIVATE KEY BLOCK-----/,
    severity: 'critical',
    description: 'PGP private key detected',
    recommendation: 'Never commit PGP private keys.'
  },
  {
    id: 'aws-secret-key',
    pattern: /(?:aws_secret_access_key|AWS_SECRET_ACCESS_KEY)\s*[=:]\s*['"]?[A-Za-z0-9/+=]{40}['"]?/,
    severity: 'critical',
    description: 'AWS Secret Access Key detected',
    recommendation: 'Use AWS Secrets Manager or environment variables.'
  },
  {
    id: 'aws-access-key-id',
    pattern: /(?:AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}/,
    severity: 'critical',
    description: 'AWS Access Key ID detected',
    recommendation: 'Use IAM roles instead of hardcoded keys.'
  },
  {
    id: 'gcp-service-account',
    pattern: /"type"\s*:\s*"service_account"[\s\S]*"private_key"/,
    severity: 'critical',
    description: 'GCP service account key detected',
    recommendation: 'Use Workload Identity or Secret Manager.'
  },
  {
    id: 'github-token',
    pattern: /gh[pousr]_[A-Za-z0-9_]{36,}/,
    severity: 'critical',
    description: 'GitHub token detected',
    recommendation: 'Use GitHub Actions secrets or environment variables.'
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // HIGH - API Keys & Tokens (φ¹ severity)
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'stripe-secret',
    pattern: /sk_live_[0-9a-zA-Z]{24,}/,
    severity: 'high',
    description: 'Stripe live secret key detected',
    recommendation: 'Use environment variables for Stripe keys.'
  },
  {
    id: 'stripe-restricted',
    pattern: /rk_live_[0-9a-zA-Z]{24,}/,
    severity: 'high',
    description: 'Stripe restricted key detected',
    recommendation: 'Use environment variables for Stripe keys.'
  },
  {
    id: 'openai-key',
    pattern: /sk-[A-Za-z0-9]{48,}/,
    severity: 'high',
    description: 'OpenAI API key detected',
    recommendation: 'Use environment variables for API keys.'
  },
  {
    id: 'anthropic-key',
    pattern: /sk-ant-[A-Za-z0-9-]{90,}/,
    severity: 'high',
    description: 'Anthropic API key detected',
    recommendation: 'Use environment variables for API keys.'
  },
  {
    id: 'slack-token',
    pattern: /xox[baprs]-[0-9a-zA-Z-]{10,}/,
    severity: 'high',
    description: 'Slack token detected',
    recommendation: 'Use Slack app configuration for tokens.'
  },
  {
    id: 'discord-token',
    pattern: /[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}/,
    severity: 'high',
    description: 'Discord bot token detected',
    recommendation: 'Use environment variables for Discord tokens.'
  },
  {
    id: 'twilio-key',
    pattern: /SK[0-9a-fA-F]{32}/,
    severity: 'high',
    description: 'Twilio API key detected',
    recommendation: 'Use environment variables for Twilio credentials.'
  },
  {
    id: 'sendgrid-key',
    pattern: /SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}/,
    severity: 'high',
    description: 'SendGrid API key detected',
    recommendation: 'Use environment variables for SendGrid keys.'
  },
  {
    id: 'jwt-token',
    pattern: /eyJ[A-Za-z0-9-_]+\.eyJ[A-Za-z0-9-_]+\.[A-Za-z0-9-_.+/=]*/,
    severity: 'high',
    description: 'JWT token detected',
    recommendation: 'Never hardcode JWT tokens in source code.'
  },
  {
    id: 'bearer-token',
    pattern: /(?:Bearer|bearer)\s+[A-Za-z0-9\-_.~+\/]+=*/,
    severity: 'high',
    description: 'Bearer token detected',
    recommendation: 'Use secure token storage.'
  },
  {
    id: 'basic-auth',
    pattern: /(?:Basic|basic)\s+[A-Za-z0-9+\/]+=*/,
    severity: 'high',
    description: 'Basic authentication credentials detected',
    recommendation: 'Never hardcode authentication credentials.'
  },
  {
    id: 'password-assignment',
    pattern: /(?:password|passwd|pwd|secret)\s*[=:]\s*['"][^'"]{8,}['"]/i,
    severity: 'high',
    description: 'Hardcoded password detected',
    recommendation: 'Use environment variables or secrets manager.'
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // HIGH - Database & Connection Strings (φ¹ severity)
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'postgres-url',
    pattern: /postgres(?:ql)?:\/\/[^:]+:[^@]+@[^/]+/,
    severity: 'high',
    description: 'PostgreSQL connection string with credentials',
    recommendation: 'Use environment variables for database URLs.'
  },
  {
    id: 'mysql-url',
    pattern: /mysql:\/\/[^:]+:[^@]+@[^/]+/,
    severity: 'high',
    description: 'MySQL connection string with credentials',
    recommendation: 'Use environment variables for database URLs.'
  },
  {
    id: 'mongodb-url',
    pattern: /mongodb(?:\+srv)?:\/\/[^:]+:[^@]+@[^/]+/,
    severity: 'high',
    description: 'MongoDB connection string with credentials',
    recommendation: 'Use environment variables for database URLs.'
  },
  {
    id: 'redis-url',
    pattern: /redis:\/\/[^:]*:[^@]+@[^/]+/,
    severity: 'high',
    description: 'Redis connection string with credentials',
    recommendation: 'Use environment variables for Redis URLs.'
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // MEDIUM - Potentially Sensitive (φ⁰ severity)
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'generic-api-key',
    pattern: /(?:api[_-]?key|apikey)\s*[=:]\s*['"][A-Za-z0-9_\-]{20,}['"]/i,
    severity: 'medium',
    description: 'Generic API key pattern detected',
    recommendation: 'Verify this is not a sensitive API key.'
  },
  {
    id: 'generic-secret',
    pattern: /(?:client[_-]?secret|app[_-]?secret)\s*[=:]\s*['"][^'"]{10,}['"]/i,
    severity: 'medium',
    description: 'Generic secret pattern detected',
    recommendation: 'Use environment variables for secrets.'
  },
  {
    id: 'private-ip',
    pattern: /(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3})/,
    severity: 'medium',
    description: 'Private IP address detected',
    recommendation: 'Avoid hardcoding IP addresses.'
  },
  {
    id: 'helius-key',
    pattern: /[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/i,
    severity: 'medium',
    description: 'UUID pattern (possibly Helius/RPC API key)',
    recommendation: 'If this is an API key, use environment variables.'
  },
  {
    id: 'solana-private-key',
    pattern: /\[[0-9,\s]{100,}\]/,
    severity: 'critical',
    description: 'Possible Solana private key array',
    recommendation: 'Never commit wallet private keys!'
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // LOW - Suspicious Patterns (φ⁻¹ severity)
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'todo-secret',
    pattern: /TODO:?\s*(?:add|remove|hide)\s*(?:secret|key|password|token)/i,
    severity: 'low',
    description: 'TODO comment about secrets',
    recommendation: 'Address security TODOs before committing.'
  },
  {
    id: 'temp-credential',
    pattern: /(?:temp|temporary|test)[_-]?(?:password|key|token|secret)/i,
    severity: 'low',
    description: 'Temporary credential pattern',
    recommendation: 'Remove temporary credentials before committing.'
  }
];

// =============================================================================
// SENSITIVE FILES
// =============================================================================

const SENSITIVE_FILES = [
  { pattern: /\.env(?:\.local|\.prod|\.production)?$/, severity: 'high', description: 'Environment file' },
  { pattern: /\.env\.[^.]+$/, severity: 'high', description: 'Environment file variant' },
  { pattern: /credentials\.json$/, severity: 'critical', description: 'Credentials file' },
  { pattern: /service[_-]?account.*\.json$/, severity: 'critical', description: 'Service account key' },
  { pattern: /id_rsa|id_ed25519|id_ecdsa/, severity: 'critical', description: 'SSH private key' },
  { pattern: /\.pem$/, severity: 'high', description: 'PEM certificate/key' },
  { pattern: /\.p12$|\.pfx$/, severity: 'high', description: 'PKCS12 certificate' },
  { pattern: /\.keystore$/, severity: 'high', description: 'Java keystore' },
  { pattern: /secrets?\.(ya?ml|json)$/, severity: 'high', description: 'Secrets configuration' },
  { pattern: /\.npmrc$/, severity: 'medium', description: 'NPM config (may contain tokens)' },
  { pattern: /\.pypirc$/, severity: 'medium', description: 'PyPI config (may contain tokens)' },
  { pattern: /\.netrc$/, severity: 'high', description: 'Netrc credentials file' },
  { pattern: /\.htpasswd$/, severity: 'high', description: 'HTTP password file' },
  { pattern: /\.pgpass$/, severity: 'high', description: 'PostgreSQL password file' },
  { pattern: /\.my\.cnf$/, severity: 'high', description: 'MySQL config with credentials' }
];

// =============================================================================
// SAFE PATTERNS (false positive reduction)
// =============================================================================

const SAFE_PATTERNS = [
  /\.example$/,              // .env.example is safe
  /\.sample$/,               // sample files
  /\.template$/,             // template files
  /test[_-]?fixtures?/i,     // test fixtures
  /mock[_-]?data/i,          // mock data
  /placeholder/i,            // placeholder values
  /your[_-]?api[_-]?key/i,   // documentation placeholders
  /xxx+/i,                   // masked values
  /\*{4,}/,                  // starred out values
  /CHANGE[_-]?ME/i,          // change me placeholders
  /<[^>]+>/                  // template variables
];

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  PHI,
  SEVERITY_WEIGHTS,
  SECRET_PATTERNS,
  SENSITIVE_FILES,
  SAFE_PATTERNS,

  /**
   * Get φ-weighted severity score
   * @param {string} severity - critical|high|medium|low
   * @returns {number} φ-weighted score
   */
  getSeverityWeight(severity) {
    return SEVERITY_WEIGHTS[severity] || 1;
  },

  /**
   * Check if content matches safe patterns (false positive)
   * @param {string} content - content to check
   * @returns {boolean} true if likely false positive
   */
  isSafePattern(content) {
    return SAFE_PATTERNS.some(pattern => pattern.test(content));
  }
};
