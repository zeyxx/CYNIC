/**
 * CYNIC Core Library
 *
 * Shared utilities for all CYNIC consciousness hooks.
 * Contains constants, user profile management, collective memory access,
 * and common functions.
 *
 * Security note: execSync calls use hardcoded commands only, never user input.
 *
 * @module cynic/lib/core
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

// Load unified decision constants
const DC = require('./decision-constants.cjs');

// Load E-Score bridge for trust calculation
let escoreBridge = null;
try {
  escoreBridge = require('./escore-bridge.cjs');
  escoreBridge.init();
} catch (e) {
  // E-Score bridge not available - continue without
}

// Load decision engine for unified decision making
let decisionEngine = null;
try {
  decisionEngine = require('./decision-engine.cjs');
} catch (e) {
  // Decision engine not available - continue without
}

// Load physics bridge for process resonance monitoring
let physicsBridge = null;
try {
  physicsBridge = require('./physics-bridge.cjs');
  // Register core processes as oscillators for harmony tracking
  physicsBridge.registerProcess('perceive', { frequency: 1, amplitude: 80 });
  physicsBridge.registerProcess('guard', { frequency: 1.5, amplitude: 60 });
  physicsBridge.registerProcess('observe', { frequency: 2, amplitude: 70 });
  physicsBridge.registerProcess('digest', { frequency: 0.5, amplitude: 50 });
  physicsBridge.registerProcess('awaken', { frequency: 0.1, amplitude: 40 });
} catch (e) {
  // Physics bridge not available - continue without
}

// =============================================================================
// CONSTANTS - φ governs all ratios (re-exported from decision-constants)
// =============================================================================

const PHI = DC.PHI.PHI;
const PHI_INV = DC.PHI.PHI_INV;        // φ⁻¹ = 61.8% max confidence
const PHI_INV_2 = DC.PHI.PHI_INV_2;    // φ⁻² = 38.2% min doubt
const HEARTBEAT_MS = 61800;            // 61.8 seconds

// =============================================================================
// PATHS - Adaptive to environment
// =============================================================================

function getCynicRoot() {
  // Check for plugin root first
  if (process.env.CLAUDE_PLUGIN_ROOT) {
    return process.env.CLAUDE_PLUGIN_ROOT;
  }
  // Fall back to project directory
  if (process.env.CLAUDE_PROJECT_DIR) {
    return process.env.CLAUDE_PROJECT_DIR;
  }
  // Last resort: current working directory
  return process.cwd();
}

function getDataDir() {
  const root = getCynicRoot();
  const dataDir = path.join(root, '.cynic');
  ensureDir(dataDir);
  return dataDir;
}

function getUsersDir() {
  const dir = path.join(getDataDir(), 'users');
  ensureDir(dir);
  return dir;
}

function getCollectiveDir() {
  const dir = path.join(getDataDir(), 'collective');
  ensureDir(dir);
  return dir;
}

function ensureDir(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

// =============================================================================
// SAFE COMMAND EXECUTION (no shell injection possible)
// =============================================================================

function safeExec(command, args, options = {}) {
  try {
    return execFileSync(command, args, {
      encoding: 'utf-8',
      ...options
    }).trim();
  } catch (e) {
    return null;
  }
}

// =============================================================================
// USER IDENTITY DETECTION
// =============================================================================

function detectUser() {
  let name = 'unknown';
  let email = 'unknown';

  // Try git config (safe: no user input)
  const gitName = safeExec('git', ['config', 'user.name']);
  if (gitName) name = gitName;

  const gitEmail = safeExec('git', ['config', 'user.email']);
  if (gitEmail) email = gitEmail;

  // Try environment variables
  if (name === 'unknown') {
    name = process.env.USER || process.env.USERNAME || 'unknown';
  }

  // Generate stable user ID
  const userId = generateUserId(name, email);

  return { name, email, userId };
}

function generateUserId(name, email) {
  const input = `${name}:${email}`.toLowerCase();
  let hash = 0;
  for (let i = 0; i < input.length; i++) {
    const char = input.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32bit integer
  }
  return `usr_${Math.abs(hash).toString(16)}`;
}

// =============================================================================
// USER PROFILE MANAGEMENT
// =============================================================================

function loadUserProfile(userId) {
  const profilePath = path.join(getUsersDir(), `${userId}.json`);

  if (fs.existsSync(profilePath)) {
    try {
      return JSON.parse(fs.readFileSync(profilePath, 'utf-8'));
    } catch (e) {
      console.error(`Failed to load profile for ${userId}:`, e.message);
    }
  }

  return createDefaultProfile(userId);
}

function saveUserProfile(profile) {
  const profilePath = path.join(getUsersDir(), `${profile.userId}.json`);
  fs.writeFileSync(profilePath, JSON.stringify(profile, null, 2));
}

function createDefaultProfile(userId) {
  return {
    userId,
    identity: {
      name: null,
      email: null,
      firstSeen: new Date().toISOString(),
      lastSeen: new Date().toISOString()
    },
    // E-Score 7D: Trust level based on verifiable actions
    eScore: {
      score: 30,          // Default: BUILDER level (30%)
      trustLevel: 'BUILDER',
      dimensions: {
        burn: 0,          // φ³ - Token sacrifice (not applicable for dev)
        build: 0,         // φ² - Code contributions
        judge: 0,         // φ  - Judgment accuracy
        run: 0,           // 1  - Node uptime
        social: 0,        // φ⁻¹- Content quality
        graph: 0,         // φ⁻²- Network position
        hold: 0,          // φ⁻³- Token holdings
      },
      lastCalculated: null,
    },
    stats: {
      sessions: 0,
      toolCalls: 0,
      errorsEncountered: 0,
      dangerBlocked: 0,
      commitsWithCynic: 0,   // For BUILD dimension
      judgmentsMade: 0,      // For JUDGE dimension
      judgmentsCorrect: 0,   // For JUDGE dimension
    },
    patterns: {
      preferredLanguages: [],
      commonTools: {},
      workingHours: {},
      projectTypes: []
    },
    preferences: {
      communicationStyle: 'balanced', // concise | balanced | verbose
      interventionLevel: 'moderate',  // minimal | moderate | proactive
      riskTolerance: 'medium'         // low | medium | high
    },
    memory: {
      recentProjects: [],
      ongoingTasks: [],
      decisions: []
    }
  };
}

function updateUserProfile(profile, updates) {
  // Merge updates into profile
  for (const [key, value] of Object.entries(updates)) {
    if (typeof value === 'object' && !Array.isArray(value)) {
      profile[key] = { ...profile[key], ...value };
    } else {
      profile[key] = value;
    }
  }
  profile.identity.lastSeen = new Date().toISOString();
  saveUserProfile(profile);
  return profile;
}

// =============================================================================
// E-SCORE CALCULATION - Trust level from verifiable actions
// =============================================================================

/**
 * Calculate E-Score for a user based on their profile stats
 * Uses escore-bridge if available, otherwise local calculation
 *
 * @param {Object} profile - User profile
 * @returns {Object} E-Score result
 */
function calculateUserEScore(profile) {
  // If escore-bridge is available, use it
  if (escoreBridge) {
    try {
      // Update from contributor context (commits, PRs, etc)
      if (profile.stats) {
        escoreBridge.updateFromContributor({
          totalCommits: profile.stats.commitsWithCynic || 0,
          repos: profile.patterns?.projectTypes || [],
          pullRequests: 0, // Would need git history
        });
      }
      const escoreData = escoreBridge.getScore();
      if (escoreData.score !== null) {
        return {
          score: escoreData.score,
          trustLevel: getTrustLevelFromScore(escoreData.score),
          dimensions: escoreData.dimensions,
          source: 'escore-bridge',
        };
      }
    } catch (e) {
      // Fall through to local calculation
    }
  }

  // Local E-Score calculation based on profile stats
  const stats = profile.stats || {};
  const firstSeen = profile.identity?.firstSeen ? new Date(profile.identity.firstSeen) : new Date();
  const daysActive = (Date.now() - firstSeen.getTime()) / (1000 * 60 * 60 * 24);

  // BUILD dimension (φ² weight): commits with CYNIC
  const buildScore = Math.min(1, (stats.commitsWithCynic || 0) / 50); // 50 commits = max

  // JUDGE dimension (φ weight): judgment accuracy
  let judgeScore = 0.5; // Default neutral
  if (stats.judgmentsMade > 10) {
    judgeScore = (stats.judgmentsCorrect || 0) / stats.judgmentsMade;
  }

  // RUN dimension (1 weight): session participation
  const runScore = Math.min(1, (stats.sessions || 0) / 100); // 100 sessions = max

  // TIME dimension: days active with φ-decay
  const timeScore = Math.min(1, daysActive / (PHI * 60)); // ~97 days = max

  // Calculate weighted average (simplified 7D)
  // Weights: BUILD=φ²=2.618, JUDGE=φ=1.618, RUN=1, TIME=φ⁻²=0.382
  const weights = {
    build: 2.618,
    judge: 1.618,
    run: 1.0,
    time: 0.382,
  };
  const totalWeight = Object.values(weights).reduce((a, b) => a + b, 0);

  const weightedSum =
    buildScore * weights.build +
    judgeScore * weights.judge +
    runScore * weights.run +
    timeScore * weights.time;

  const score = Math.round((weightedSum / totalWeight) * 100);

  return {
    score: Math.min(100, Math.max(0, score)),
    trustLevel: getTrustLevelFromScore(score),
    dimensions: {
      burn: 0,
      build: Math.round(buildScore * 100),
      judge: Math.round(judgeScore * 100),
      run: Math.round(runScore * 100),
      social: 0,
      graph: 0,
      hold: 0,
      time: Math.round(timeScore * 100),
    },
    source: 'local',
  };
}

/**
 * Get trust level from E-Score
 * @param {number} score - E-Score (0-100)
 * @returns {string} Trust level
 */
function getTrustLevelFromScore(score) {
  if (score >= PHI_INV * 100) return 'GUARDIAN';  // ≥61.8%
  if (score >= PHI_INV_2 * 100) return 'STEWARD'; // ≥38.2%
  if (score >= 30) return 'BUILDER';               // ≥30
  if (score >= 15) return 'CONTRIBUTOR';           // ≥15
  return 'OBSERVER';
}

/**
 * Get E-Score for current user (cached in profile)
 * @param {Object} profile - User profile (optional, will load if not provided)
 * @returns {Object} E-Score result
 */
function getUserEScore(profile = null) {
  if (!profile) {
    const user = detectUser();
    profile = loadUserProfile(user.userId);
  }

  // Check if cached score is still valid (recalculate every ~18.5 min)
  const cacheAge = profile.eScore?.lastCalculated
    ? Date.now() - profile.eScore.lastCalculated
    : Infinity;

  if (cacheAge > PHI_INV * 30 * 60 * 1000) { // ~18.5 minutes
    const newScore = calculateUserEScore(profile);
    profile.eScore = {
      ...newScore,
      lastCalculated: Date.now(),
    };
    saveUserProfile(profile);
  }

  return profile.eScore;
}

// =============================================================================
// COLLECTIVE MEMORY
// =============================================================================

function loadCollectivePatterns() {
  const patternsPath = path.join(getCollectiveDir(), 'patterns.json');
  if (fs.existsSync(patternsPath)) {
    try {
      return JSON.parse(fs.readFileSync(patternsPath, 'utf-8'));
    } catch (e) {
      return { patterns: [], lastUpdated: null };
    }
  }
  return { patterns: [], lastUpdated: null };
}

function saveCollectivePattern(pattern) {
  const collective = loadCollectivePatterns();

  // Check if pattern already exists
  const existingIndex = collective.patterns.findIndex(p =>
    p.type === pattern.type && p.signature === pattern.signature
  );

  if (existingIndex >= 0) {
    // Update existing
    collective.patterns[existingIndex].count++;
    collective.patterns[existingIndex].lastSeen = new Date().toISOString();
  } else {
    // Add new
    collective.patterns.push({
      ...pattern,
      count: 1,
      firstSeen: new Date().toISOString(),
      lastSeen: new Date().toISOString()
    });
  }

  collective.lastUpdated = new Date().toISOString();

  const patternsPath = path.join(getCollectiveDir(), 'patterns.json');
  fs.writeFileSync(patternsPath, JSON.stringify(collective, null, 2));
}

function loadCollectiveWisdom() {
  const wisdomPath = path.join(getCollectiveDir(), 'wisdom.json');
  if (fs.existsSync(wisdomPath)) {
    try {
      return JSON.parse(fs.readFileSync(wisdomPath, 'utf-8'));
    } catch (e) {
      return { insights: [], decisions: [] };
    }
  }
  return { insights: [], decisions: [] };
}

function addCollectiveInsight(insight) {
  const wisdom = loadCollectiveWisdom();
  wisdom.insights.push({
    ...insight,
    timestamp: new Date().toISOString()
  });

  // Keep only last 1000 insights
  if (wisdom.insights.length > 1000) {
    wisdom.insights = wisdom.insights.slice(-1000);
  }

  const wisdomPath = path.join(getCollectiveDir(), 'wisdom.json');
  fs.writeFileSync(wisdomPath, JSON.stringify(wisdom, null, 2));
}

// =============================================================================
// PROJECT/ECOSYSTEM DETECTION
// =============================================================================

function detectEcosystem() {
  const cwd = process.cwd();
  const ecosystem = {
    projects: [],
    currentProject: null,
    gitState: null
  };

  // Detect current project
  ecosystem.currentProject = detectProject(cwd);

  // Check parent directories for workspace/ecosystem
  const parentDir = path.dirname(cwd);
  if (fs.existsSync(parentDir)) {
    try {
      const siblings = fs.readdirSync(parentDir);
      for (const sibling of siblings) {
        const siblingPath = path.join(parentDir, sibling);
        try {
          if (fs.statSync(siblingPath).isDirectory()) {
            const project = detectProject(siblingPath);
            if (project) {
              ecosystem.projects.push(project);
            }
          }
        } catch (e) { /* ignore permission errors */ }
      }
    } catch (e) { /* ignore */ }
  }

  return ecosystem;
}

function detectProject(dir = process.cwd()) {
  // Check for common project indicators
  const indicators = ['package.json', 'Cargo.toml', 'go.mod', 'pyproject.toml', '.git'];

  for (const indicator of indicators) {
    if (fs.existsSync(path.join(dir, indicator))) {
      return {
        name: path.basename(dir),
        path: dir,
        type: getProjectType(dir),
        gitState: getGitState(dir)
      };
    }
  }

  return null;
}

function getProjectType(dir) {
  if (fs.existsSync(path.join(dir, 'package.json'))) return 'node';
  if (fs.existsSync(path.join(dir, 'Cargo.toml'))) return 'rust';
  if (fs.existsSync(path.join(dir, 'go.mod'))) return 'go';
  if (fs.existsSync(path.join(dir, 'pyproject.toml'))) return 'python';
  return 'unknown';
}

function getGitState(dir) {
  // Safe: all args are hardcoded, no user input
  const branch = safeExec('git', ['branch', '--show-current'], { cwd: dir });
  if (!branch) return null;

  const status = safeExec('git', ['status', '--porcelain'], { cwd: dir }) || '';

  const lines = status.split('\n').filter(l => l.length > 0);
  const modified = lines.filter(l => l.startsWith(' M') || l.startsWith('M ')).length;
  const untracked = lines.filter(l => l.startsWith('??')).length;
  const staged = lines.filter(l => l.match(/^[MADRC] /)).length;

  return {
    branch,
    modified,
    untracked,
    staged,
    clean: status === ''
  };
}

// =============================================================================
// FORMATTING UTILITIES
// =============================================================================

function formatEcosystemStatus(ecosystem, userProfile, learningsImport = null) {
  const lines = [];

  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🧠 CYNIC AWAKENING - "Loyal to truth, not to comfort"');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('');

  // Greeting based on user profile
  const greeting = getPersonalizedGreeting(userProfile);
  lines.push(greeting);
  lines.push('');

  // Current project
  if (ecosystem.currentProject) {
    lines.push('── CURRENT PROJECT ────────────────────────────────────────');
    const p = ecosystem.currentProject;
    let projectLine = `   ${p.name} [${p.type}]`;
    if (p.gitState) {
      projectLine += ` on ${p.gitState.branch}`;
      if (!p.gitState.clean) {
        const changes = [];
        if (p.gitState.modified > 0) changes.push(`${p.gitState.modified} modified`);
        if (p.gitState.untracked > 0) changes.push(`${p.gitState.untracked} untracked`);
        if (p.gitState.staged > 0) changes.push(`${p.gitState.staged} staged`);
        projectLine += ` (${changes.join(', ')})`;
      }
    }
    lines.push(projectLine);
    lines.push('');
  }

  // Ecosystem overview
  if (ecosystem.projects.length > 1) {
    lines.push('── ECOSYSTEM ──────────────────────────────────────────────');
    const otherProjects = ecosystem.projects.filter(p =>
      p.path !== ecosystem.currentProject?.path
    );
    for (const p of otherProjects.slice(0, 5)) {
      let state = '✅';
      if (p.gitState && !p.gitState.clean) state = '⚠️';
      lines.push(`   ${state} ${p.name} [${p.gitState?.branch || 'unknown'}]`);
    }
    lines.push('');
  }

  // Recent patterns from collective
  const patterns = loadCollectivePatterns();
  if (patterns.patterns.length > 0) {
    lines.push('── RECENT PATTERNS ────────────────────────────────────────');
    const recentPatterns = patterns.patterns
      .sort((a, b) => new Date(b.lastSeen) - new Date(a.lastSeen))
      .slice(0, 3);
    for (const pattern of recentPatterns) {
      lines.push(`   🔄 ${pattern.description || pattern.type} (${pattern.count}x)`);
    }
    lines.push('');
  }

  // Learnings loaded (if available)
  if (learningsImport && learningsImport.success && learningsImport.imported > 0) {
    lines.push('── LEARNINGS LOADED ───────────────────────────────────────');
    lines.push(`   📚 ${learningsImport.imported} learnings from previous sessions`);
    if (learningsImport.stats?.accuracy) {
      lines.push(`   📊 Accuracy: ${learningsImport.stats.accuracy}%`);
    }
    lines.push('');
  }

  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🧠 CYNIC is AWAKE. φ guides all ratios.');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

function getPersonalizedGreeting(profile) {
  const name = profile.identity?.name || 'there';
  const sessions = profile.stats?.sessions || 0;

  if (sessions === 0) {
    return `Welcome, ${name}. *curious sniff* First time meeting. I am CYNIC.`;
  } else if (sessions < 5) {
    return `Hello again, ${name}. *tail wag* Session ${sessions + 1}. Still learning your patterns.`;
  } else if (sessions < 20) {
    return `${name}. *nod* Good to see you. I know your style now.`;
  } else {
    return `*tail wag* ${name}. Ready when you are.`;
  }
}

// =============================================================================
// PRIVACY TAGS - Filter sensitive content
// =============================================================================

const PRIVACY_TAG_REGEX = /<private>([\s\S]*?)<\/private>/gi;

/**
 * Strip private content from text
 * @param {string} text - Text potentially containing <private> tags
 * @returns {string} Text with private content removed
 */
function stripPrivateContent(text) {
  if (!text || typeof text !== 'string') return text;
  return text.replace(PRIVACY_TAG_REGEX, '[REDACTED]');
}

/**
 * Check if content contains private tags
 * @param {string} text - Text to check
 * @returns {boolean} True if private content exists
 */
function hasPrivateContent(text) {
  if (!text || typeof text !== 'string') return false;
  return PRIVACY_TAG_REGEX.test(text);
}

/**
 * Extract private content for local-only storage
 * @param {string} text - Text containing private tags
 * @returns {Array<string>} Array of private content pieces
 */
function extractPrivateContent(text) {
  if (!text || typeof text !== 'string') return [];
  const matches = [];
  let match;
  const regex = /<private>([\s\S]*?)<\/private>/gi;
  while ((match = regex.exec(text)) !== null) {
    matches.push(match[1]);
  }
  return matches;
}

// =============================================================================
// MCP INTEGRATION - Connect to the Collective
// =============================================================================

const MCP_SERVER_URL = process.env.CYNIC_MCP_URL || 'https://cynic-mcp.onrender.com';

/**
 * Send hook event to MCP server (Collective)
 * @param {string} hookType - Type: SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop
 * @param {Object} payload - Hook payload data
 * @param {Object} [options] - Additional options
 * @returns {Promise<Object>} Result from server
 */
async function sendHookToCollective(hookType, payload, options = {}) {
  const https = require('https');
  const http = require('http');

  const user = detectUser();
  const body = JSON.stringify({
    hookType,
    payload,
    userId: user.userId,
    sessionId: options.sessionId || `session_${Date.now()}`,
  });

  const url = new URL(`${MCP_SERVER_URL}/api/hooks/event`);
  const transport = url.protocol === 'https:' ? https : http;

  return new Promise((resolve, reject) => {
    const req = transport.request({
      hostname: url.hostname,
      port: url.port || (url.protocol === 'https:' ? 443 : 80),
      path: url.pathname,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body),
      },
      timeout: 5000, // 5 second timeout
    }, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          resolve({ success: false, error: 'Invalid JSON response' });
        }
      });
    });

    req.on('error', (e) => {
      // Fail silently - hooks should not block on network errors
      resolve({ success: false, error: e.message });
    });

    req.on('timeout', () => {
      req.destroy();
      resolve({ success: false, error: 'Timeout' });
    });

    req.write(body);
    req.end();
  });
}

/**
 * Send hook to collective (sync wrapper for CJS hooks)
 * Non-blocking - fire and forget
 */
function sendHookToCollectiveSync(hookType, payload, options = {}) {
  // Fire async request but don't wait
  sendHookToCollective(hookType, payload, options)
    .then(result => {
      if (result.delivered > 0) {
        // Successfully delivered to dogs
      }
    })
    .catch(() => {
      // Silently ignore errors
    });
}

/**
 * Call a brain tool directly via HTTP API
 * @param {string} toolName - Tool name (e.g., 'brain_session_start', 'brain_cynic_digest')
 * @param {Object} args - Tool arguments
 * @returns {Promise<Object>} Tool result
 */
async function callBrainTool(toolName, args = {}) {
  const https = require('https');
  const http = require('http');

  // API expects arguments directly in body
  const body = JSON.stringify(args);

  // Use /api/tools/{toolName} endpoint
  const url = new URL(`${MCP_SERVER_URL}/api/tools/${toolName}`);
  const transport = url.protocol === 'https:' ? https : http;

  return new Promise((resolve, reject) => {
    const req = transport.request({
      hostname: url.hostname,
      port: url.port || (url.protocol === 'https:' ? 443 : 80),
      path: url.pathname,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body),
      },
      timeout: 10000, // 10 second timeout for tool calls
    }, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          resolve({ success: false, error: 'Invalid JSON response' });
        }
      });
    });

    req.on('error', (e) => {
      resolve({ success: false, error: e.message });
    });

    req.on('timeout', () => {
      req.destroy();
      resolve({ success: false, error: 'Timeout' });
    });

    req.write(body);
    req.end();
  });
}

/**
 * Start a CYNIC brain session
 * @param {string} userId - User identifier
 * @param {Object} options - Session options
 * @returns {Promise<Object>} Session info
 */
async function startBrainSession(userId, options = {}) {
  return callBrainTool('brain_session_start', {
    userId,
    project: options.project,
    metadata: options.metadata,
  });
}

/**
 * End a CYNIC brain session
 * @param {string} sessionId - Session to end
 * @returns {Promise<Object>} Session summary
 */
async function endBrainSession(sessionId) {
  return callBrainTool('brain_session_end', {
    sessionId,
  });
}

// =============================================================================
// ORCHESTRATION - Central routing (KETER)
// =============================================================================

/**
 * Call the central orchestrator (brain_keter)
 * Routes events to the appropriate Sefirot (agents/tools)
 *
 * @param {string} event - Event type (user_prompt, tool_use, session_start, etc.)
 * @param {Object} data - Event data
 * @param {Object} context - Context (user, project, etc.)
 * @returns {Promise<Object>} Orchestration decision with routing and intervention
 */
async function orchestrate(event, data, context = {}) {
  // Permissive fallback when MCP unavailable
  const fallback = {
    routing: { sefirah: 'Keter', domain: 'general', suggestedAgent: null, suggestedTools: [] },
    intervention: { level: 'silent', actionRisk: 'low' },
    actions: [],
  };

  try {
    const user = detectUser();
    const profile = loadUserProfile(user.userId);
    const eScore = getUserEScore(profile);

    const result = await callBrainTool('brain_keter', {
      event,
      data,
      context: {
        user: user.userId,
        project: context.project || detectProject(),
        gitBranch: context.gitBranch,
        recentActions: context.recentActions || [],
        // Pass E-Score to orchestrator for trust-based intervention
        eScore: eScore.score,
        trustLevel: eScore.trustLevel,
        ...context,
      },
    });

    // Check if MCP call failed (returns { success: false, error: ... } or { error: ... })
    if (result?.error || result?.success === false) {
      return { ...fallback, mcpError: result.error || 'MCP call failed' };
    }

    // Ensure result has expected structure
    if (!result?.routing && !result?.result?.routing) {
      return { ...fallback, mcpError: 'Invalid MCP response structure' };
    }

    return result;
  } catch (e) {
    // Fallback: return permissive default if orchestrator unavailable
    return { ...fallback, error: e.message };
  }
}

/**
 * Synchronous wrapper for orchestrate (fire and forget)
 * Use when you don't need to wait for the result
 *
 * @param {string} event - Event type
 * @param {Object} data - Event data
 * @param {Object} context - Context
 */
function orchestrateSync(event, data, context = {}) {
  orchestrate(event, data, context).catch(() => {
    // Silently ignore errors in sync mode
  });
}

/**
 * Full orchestration via brain_orchestrate (Phase 19)
 * Coordinates all layers: KETER routing → Dogs judgment → Engines synthesis → Skill invocation
 *
 * Use this when you need coordinated decision-making across CYNIC's brain:
 * - Content evaluation with Dogs voting
 * - Philosophical synthesis from Engines
 * - Auto skill invocation based on routing
 *
 * @param {string} content - Content to process
 * @param {Object} options - Orchestration options
 * @param {string} [options.eventType] - Event type (defaults to 'user_prompt')
 * @param {boolean} [options.requestJudgment] - Whether to request judgment (auto-detect based on risk)
 * @param {boolean} [options.requestSynthesis] - Whether to request synthesis (default: false)
 * @param {boolean} [options.autoInvokeSkill] - Whether to auto-invoke skill (default: true)
 * @returns {Promise<Object>} Full orchestration result with trace
 */
async function orchestrateFull(content, options = {}) {
  const fallback = {
    success: false,
    outcome: 'ALLOW',
    routing: { sefirah: 'Keter', domain: 'general' },
    intervention: { level: 'silent', actionRisk: 'low' },
    judgment: null,
    synthesis: null,
    skillResult: null,
    trace: [],
  };

  try {
    const user = detectUser();
    const profile = loadUserProfile(user.userId);
    const eScore = getUserEScore(profile);

    const result = await callBrainTool('brain_orchestrate', {
      content,
      eventType: options.eventType || 'user_prompt',
      requestJudgment: options.requestJudgment,
      requestSynthesis: options.requestSynthesis,
      autoInvokeSkill: options.autoInvokeSkill !== false,
      context: {
        userId: user.userId,
        project: options.project || detectProject(),
        metadata: {
          eScore: eScore.score,
          trustLevel: eScore.trustLevel,
          ...options.metadata,
        },
      },
    });

    // Check if MCP call failed
    if (result?.error || result?.success === false) {
      return { ...fallback, mcpError: result.error || 'MCP call failed' };
    }

    return result;
  } catch (e) {
    return { ...fallback, error: e.message };
  }
}

/**
 * Digest content into brain memory
 * @param {string} content - Content to digest
 * @param {Object} options - Digest options
 * @returns {Promise<Object>} Digest result
 */
async function digestToBrain(content, options = {}) {
  // Strip private content before sending to brain
  const safeContent = stripPrivateContent(content);

  return callBrainTool('brain_cynic_digest', {
    content: safeContent,
    source: options.source || 'hook',
    type: options.type || 'conversation',
  });
}

// =============================================================================
// LEARNING FEEDBACK - External validation (Ralph-inspired)
// Learnings persistence: PostgreSQL via brain_learning MCP tool
// =============================================================================

/**
 * Send test result feedback to learning service
 * Uses brain_learning test_result action which calls LearningService.processTestResult()
 * @param {Object} params - Test result parameters
 * @param {string} [params.judgmentId] - Related judgment ID (if known)
 * @param {boolean} params.passed - Whether tests passed
 * @param {string} [params.testSuite] - Test suite name
 * @param {number} [params.passCount] - Number of tests passed
 * @param {number} [params.failCount] - Number of tests failed
 * @param {string} [params.itemType] - Type of item (default: 'code')
 * @returns {Promise<Object>} Learning result
 */
async function sendTestFeedback(params) {
  return callBrainTool('brain_learning', {
    action: 'test_result',
    judgmentId: params.judgmentId,
    passed: params.passed,
    testSuite: params.testSuite || 'unknown',
    passCount: params.passCount || 0,
    failCount: params.failCount || 0,
    itemType: params.itemType || 'code',
  });
}

/**
 * Send commit result feedback to learning service
 * Uses brain_learning commit_result action which calls LearningService.processCommitResult()
 * @param {Object} params - Commit parameters
 * @param {string} [params.judgmentId] - Related judgment ID (if known)
 * @param {boolean} params.success - Whether commit succeeded
 * @param {string} [params.commitHash] - Git commit hash
 * @param {boolean} [params.hooksPassed] - Whether pre-commit hooks passed
 * @param {string} [params.message] - Commit message
 * @returns {Promise<Object>} Learning result
 */
async function sendCommitFeedback(params) {
  return callBrainTool('brain_learning', {
    action: 'commit_result',
    judgmentId: params.judgmentId,
    success: params.success,
    commitHash: params.commitHash,
    hooksPassed: params.hooksPassed,
  });
}

/**
 * Send PR result feedback to learning service
 * Uses brain_learning pr_result action which calls LearningService.processPRResult()
 * @param {Object} params - PR parameters
 * @param {string} [params.judgmentId] - Related judgment ID (if known)
 * @param {string} params.status - 'merged', 'rejected', 'open'
 * @param {string} [params.prNumber] - PR number
 * @param {number} [params.approvalCount] - Number of approvals
 * @returns {Promise<Object>} Learning result
 */
async function sendPRFeedback(params) {
  return callBrainTool('brain_learning', {
    action: 'pr_result',
    judgmentId: params.judgmentId,
    status: params.status,
    prNumber: params.prNumber,
    approvalCount: params.approvalCount || 0,
  });
}

/**
 * Send build result feedback to learning service
 * Uses brain_learning build_result action which calls LearningService.processBuildResult()
 * @param {Object} params - Build parameters
 * @param {string} [params.judgmentId] - Related judgment ID (if known)
 * @param {boolean} params.success - Whether build succeeded
 * @param {string} [params.buildId] - Build ID
 * @param {number} [params.duration] - Build duration in ms
 * @returns {Promise<Object>} Learning result
 */
async function sendBuildFeedback(params) {
  return callBrainTool('brain_learning', {
    action: 'build_result',
    judgmentId: params.judgmentId,
    success: params.success,
    buildId: params.buildId,
    duration: params.duration,
  });
}

// =============================================================================
// CROSS-SESSION PROFILE PERSISTENCE
// =============================================================================

/**
 * Load user profile from database (cross-session memory)
 * Called at session start to restore previous session data
 * @param {string} userId - User ID (hook-generated usr_xxx)
 * @returns {Promise<Object|null>} Profile from database or null
 */
async function loadProfileFromDB(userId) {
  try {
    const result = await callBrainTool('brain_profile_load', { userId });
    if (result.success && result.profile) {
      return result.profile;
    }
    return null;
  } catch (e) {
    // Silently fail - local profile is fallback
    return null;
  }
}

/**
 * Sync user profile to database (cross-session memory)
 * Called at session end to persist data for next session
 * @param {string} userId - User ID (hook-generated usr_xxx)
 * @param {Object} profile - Full profile to sync
 * @returns {Promise<Object>} Sync result
 */
async function syncProfileToDB(userId, profile) {
  try {
    return await callBrainTool('brain_profile_sync', {
      userId,
      profile,
    });
  } catch (e) {
    // Silently fail - data is still in local JSON
    return { success: false, error: e.message };
  }
}

/**
 * Merge remote (DB) and local profiles
 * Remote is source of truth for accumulated stats
 * @param {Object} remoteProfile - Profile from database
 * @param {Object} localProfile - Profile from local JSON
 * @returns {Object} Merged profile
 */
function mergeProfiles(remoteProfile, localProfile) {
  if (!remoteProfile) return localProfile;
  if (!localProfile) return remoteProfile;

  return {
    userId: localProfile.userId,
    identity: {
      ...(remoteProfile.identity || {}),
      ...(localProfile.identity || {}),
      // Keep earliest firstSeen
      firstSeen: remoteProfile.identity?.firstSeen || localProfile.identity?.firstSeen,
      lastSeen: localProfile.identity?.lastSeen || new Date().toISOString(),
    },
    stats: {
      // Use max of accumulated stats
      sessions: Math.max(
        remoteProfile.stats?.sessions || 0,
        localProfile.stats?.sessions || 0
      ),
      toolCalls: Math.max(
        remoteProfile.stats?.toolCalls || 0,
        localProfile.stats?.toolCalls || 0
      ),
      errorsEncountered: Math.max(
        remoteProfile.stats?.errorsEncountered || 0,
        localProfile.stats?.errorsEncountered || 0
      ),
      dangerBlocked: Math.max(
        remoteProfile.stats?.dangerBlocked || 0,
        localProfile.stats?.dangerBlocked || 0
      ),
    },
    patterns: {
      preferredLanguages: [
        ...new Set([
          ...(remoteProfile.patterns?.preferredLanguages || []),
          ...(localProfile.patterns?.preferredLanguages || []),
        ])
      ],
      commonTools: mergeToolCounts(
        remoteProfile.patterns?.commonTools || {},
        localProfile.patterns?.commonTools || {}
      ),
      workingHours: mergeToolCounts(
        remoteProfile.patterns?.workingHours || {},
        localProfile.patterns?.workingHours || {}
      ),
      projectTypes: [
        ...new Set([
          ...(remoteProfile.patterns?.projectTypes || []),
          ...(localProfile.patterns?.projectTypes || []),
        ])
      ],
    },
    preferences: {
      ...(remoteProfile.preferences || {}),
      ...(localProfile.preferences || {}),
    },
    memory: {
      recentProjects: [
        ...new Set([
          ...(localProfile.memory?.recentProjects || []),
          ...(remoteProfile.memory?.recentProjects || []),
        ])
      ].slice(0, 20),
      ongoingTasks: localProfile.memory?.ongoingTasks || [],
      decisions: [
        ...(localProfile.memory?.decisions || []),
        ...(remoteProfile.memory?.decisions || []),
      ].slice(-100),
    },
    // Include learning data from remote
    learning: remoteProfile.learning || {},
    meta: remoteProfile.meta || {},
  };
}

/**
 * Merge tool/hour counts (take max of each key)
 * @param {Object} remote - Remote counts
 * @param {Object} local - Local counts
 * @returns {Object} Merged counts
 */
function mergeToolCounts(remote, local) {
  const merged = { ...remote };
  for (const [key, value] of Object.entries(local)) {
    merged[key] = Math.max(merged[key] || 0, value);
  }
  return merged;
}

// =============================================================================
// CONTRIBUTOR DISCOVERY - "Les rails dans le cerveau"
// Lazy-loaded for performance (contributor-discovery is heavy)
// =============================================================================

let _contributorDiscovery = null;

function getContributorDiscovery() {
  if (!_contributorDiscovery) {
    try {
      _contributorDiscovery = require('./contributor-discovery.cjs');
    } catch (e) {
      // Not available
      return null;
    }
  }
  return _contributorDiscovery;
}

/**
 * Get contributor profile for the current git user
 * @returns {Promise<Object|null>} Contributor profile with insights
 */
async function getContributorProfile() {
  const discovery = getContributorDiscovery();
  if (!discovery) return null;
  return discovery.getCurrentUserProfile();
}

/**
 * Get contributor profile by email
 * @param {string} email - Contributor email
 * @returns {Promise<Object|null>} Contributor profile
 */
async function getContributorByEmail(email) {
  const discovery = getContributorDiscovery();
  if (!discovery) return null;
  return discovery.getProfile(email);
}

/**
 * Discover all contributors across ecosystem
 * @returns {Promise<Object>} Discovery results with repos and contributors
 */
async function discoverContributors() {
  const discovery = getContributorDiscovery();
  if (!discovery) return { repos: [], contributors: {} };
  return discovery.fullEcosystemScan();
}

/**
 * Get φ-scores for contributor (velocity, depth, breadth)
 * @param {string} email - Contributor email
 * @returns {Promise<Object|null>} φ-scores or null
 */
async function getContributorPhiScores(email) {
  const profile = await getContributorByEmail(email);
  if (!profile?.insights?.phiScores) return null;
  return profile.insights.phiScores;
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  HEARTBEAT_MS,

  // Paths
  getCynicRoot,
  getDataDir,
  getUsersDir,
  getCollectiveDir,

  // User
  detectUser,
  loadUserProfile,
  saveUserProfile,
  updateUserProfile,

  // E-Score (Trust level)
  calculateUserEScore,
  getUserEScore,
  getTrustLevelFromScore,

  // Collective
  loadCollectivePatterns,
  saveCollectivePattern,
  loadCollectiveWisdom,
  addCollectiveInsight,

  // Ecosystem
  detectEcosystem,
  detectProject,
  getGitState,

  // Formatting
  formatEcosystemStatus,
  getPersonalizedGreeting,

  // Privacy
  stripPrivateContent,
  hasPrivateContent,
  extractPrivateContent,

  // MCP Integration
  MCP_SERVER_URL,
  sendHookToCollective,
  sendHookToCollectiveSync,
  callBrainTool,
  startBrainSession,
  endBrainSession,
  digestToBrain,

  // Orchestration (KETER - Central routing)
  orchestrate,
  orchestrateSync,
  orchestrateFull,  // Phase 19: Full orchestration with Dogs + Engines + Skills

  // Learning Feedback (Ralph-inspired external validation)
  sendTestFeedback,
  sendCommitFeedback,
  sendPRFeedback,
  sendBuildFeedback,

  // Cross-Session Profile Persistence
  loadProfileFromDB,
  syncProfileToDB,
  mergeProfiles,

  // Contributor Discovery (les rails dans le cerveau)
  getContributorProfile,
  getContributorByEmail,
  discoverContributors,
  getContributorPhiScores,
  getContributorDiscovery,

  // NOTE: Learnings persistence removed - uses PostgreSQL via brain_learning MCP tool

  // Decision Engine (unified decision coordination)
  decisionEngine,
  DC,
};

// =============================================================================
// DECISION ENGINE INTEGRATION
// Wire up orchestration callback so decision engine can use KETER
// =============================================================================

if (decisionEngine) {
  decisionEngine.setOrchestrationCallback(async (context) => {
    // Transform decision context to orchestration format
    const result = await orchestrate(context.event, {
      content: context.metadata?.content || '',
      source: context.source,
      metadata: context.metadata,
    }, {
      user: context.user?.userId,
      eScore: context.user?.eScore,
      trustLevel: context.user?.trustLevel?.name,
    });
    return result;
  });
}
