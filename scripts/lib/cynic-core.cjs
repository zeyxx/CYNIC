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

// =============================================================================
// CONSTANTS - φ governs all ratios
// =============================================================================

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;     // φ⁻¹ = 61.8% max confidence
const PHI_INV_2 = 0.381966011250105;   // φ⁻² = 38.2% min doubt
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
    stats: {
      sessions: 0,
      toolCalls: 0,
      errorsEncountered: 0,
      dangerBlocked: 0
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

function detectProject(dir) {
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
// LEARNINGS PERSISTENCE - cynic-learnings.md
// =============================================================================

const LEARNINGS_FILE = 'cynic-learnings.md';

/**
 * Get path to learnings file
 * @returns {string} Path to cynic-learnings.md
 */
function getLearningsPath() {
  const root = getCynicRoot();
  // Store in .claude directory at project root
  const claudeDir = path.join(root, '.claude');
  ensureDir(claudeDir);
  return path.join(claudeDir, LEARNINGS_FILE);
}

/**
 * Load learnings from cynic-learnings.md
 * @returns {Object} Learnings data
 */
function loadLearnings() {
  const filePath = getLearningsPath();
  if (!fs.existsSync(filePath)) {
    return { learnings: [], stats: {}, patterns: {} };
  }

  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    return parseLearningsMarkdown(content);
  } catch (e) {
    return { learnings: [], stats: {}, patterns: {} };
  }
}

/**
 * Save learnings to cynic-learnings.md
 * @param {Object} data - Learnings data
 */
function saveLearnings(data) {
  const filePath = getLearningsPath();
  const markdown = formatLearningsMarkdown(data);
  fs.writeFileSync(filePath, markdown, 'utf-8');
}

/**
 * Parse learnings markdown to structured data
 * @param {string} markdown - Markdown content
 * @returns {Object} Parsed learnings
 */
function parseLearningsMarkdown(markdown) {
  const result = {
    learnings: [],
    stats: {},
    patterns: {},
    lastUpdated: null,
  };

  // Extract last updated
  const dateMatch = markdown.match(/Last updated: ([\d\-T:.Z]+)/);
  if (dateMatch) {
    result.lastUpdated = dateMatch[1];
  }

  // Extract statistics
  const totalFeedbackMatch = markdown.match(/Total feedback: (\d+)/);
  const accuracyMatch = markdown.match(/Accuracy: ([\d.]+)%/);
  const iterationsMatch = markdown.match(/Learning iterations: (\d+)/);
  const avgErrorMatch = markdown.match(/Avg score error: ([\d.]+)/);

  if (totalFeedbackMatch) result.stats.totalFeedback = parseInt(totalFeedbackMatch[1], 10);
  if (accuracyMatch) result.stats.accuracy = parseFloat(accuracyMatch[1]);
  if (iterationsMatch) result.stats.learningIterations = parseInt(iterationsMatch[1], 10);
  if (avgErrorMatch) result.stats.avgScoreError = parseFloat(avgErrorMatch[1]);

  // Extract discovered learnings
  const learningsMatch = markdown.match(/## Discovered Learnings\n\n([\s\S]*?)(?=\n## |$)/);
  if (learningsMatch) {
    const blocks = learningsMatch[1].split(/\n### /).filter(Boolean);
    for (const block of blocks) {
      const lines = block.split('\n');
      const pattern = lines[0]?.trim();
      if (!pattern) continue;

      const learning = { pattern };
      for (const line of lines.slice(1)) {
        const match = line.match(/^- (\w+): (.+)$/);
        if (match) {
          const [, key, value] = match;
          if (key === 'Insight') learning.insight = value;
          if (key === 'Source') learning.source = value;
          if (key === 'Confidence') learning.confidence = parseFloat(value) / 100;
        }
      }
      if (learning.insight) {
        result.learnings.push(learning);
      }
    }
  }

  return result;
}

/**
 * Format learnings data as markdown
 * @param {Object} data - Learnings data
 * @returns {string} Markdown content
 */
function formatLearningsMarkdown(data) {
  const lines = [];
  const { stats = {}, learnings = [], patterns = {}, feedbackSources = {} } = data;

  lines.push('# CYNIC Learnings');
  lines.push('');
  lines.push('> Auto-generated by CYNIC Learning Service');
  lines.push(`> Last updated: ${new Date().toISOString()}`);
  lines.push('');

  // Statistics
  lines.push('## Statistics');
  lines.push('');
  lines.push(`- Total feedback: ${stats.totalFeedback || 0}`);
  lines.push(`- Accuracy: ${stats.accuracy || 0}%`);
  lines.push(`- Learning iterations: ${stats.learningIterations || 0}`);
  lines.push(`- Avg score error: ${stats.avgScoreError || 0}`);
  lines.push('');

  // Feedback sources
  if (Object.keys(feedbackSources).length > 0) {
    lines.push('## Feedback Sources');
    lines.push('');
    lines.push('| Source | Count | Correct Rate | Avg Delta |');
    lines.push('|--------|-------|--------------|-----------|');

    for (const [source, srcData] of Object.entries(feedbackSources)) {
      const correctRate = srcData.count > 0
        ? Math.round((srcData.correctCount || 0) / srcData.count * 100)
        : 0;
      lines.push(`| ${source} | ${srcData.count || 0} | ${correctRate}% | ${(srcData.avgDelta || 0).toFixed(1)} |`);
    }
    lines.push('');
  }

  // Item type patterns
  if (Object.keys(patterns.byItemType || {}).length > 0) {
    lines.push('## Patterns by Item Type');
    lines.push('');
    for (const [itemType, typeData] of Object.entries(patterns.byItemType)) {
      const trend = (typeData.avgDelta || 0) > 5 ? 'underscoring' :
                    (typeData.avgDelta || 0) < -5 ? 'overscoring' : 'neutral';
      lines.push(`### ${itemType}`);
      lines.push(`- Feedback count: ${typeData.feedbackCount || 0}`);
      lines.push(`- Avg delta: ${(typeData.avgDelta || 0).toFixed(1)}`);
      lines.push(`- Trend: ${trend}`);
      lines.push('');
    }
  }

  // Discovered learnings
  if (learnings.length > 0) {
    lines.push('## Discovered Learnings');
    lines.push('');
    for (const learning of learnings) {
      lines.push(`### ${learning.pattern}`);
      lines.push(`- Insight: ${learning.insight}`);
      lines.push(`- Source: ${learning.source || 'manual'}`);
      lines.push(`- Confidence: ${((learning.confidence || 0.5) * 100).toFixed(0)}%`);
      if (learning.createdAt) {
        lines.push(`- Discovered: ${new Date(learning.createdAt).toISOString()}`);
      }
      lines.push('');
    }
  }

  // Weight adjustments
  if (Object.keys(patterns.weightModifiers || {}).length > 0) {
    const changedModifiers = Object.entries(patterns.weightModifiers)
      .filter(([, v]) => Math.abs(v - 1.0) > 0.01);

    if (changedModifiers.length > 0) {
      lines.push('## Weight Adjustments');
      lines.push('');
      lines.push('| Dimension | Modifier | Interpretation |');
      lines.push('|-----------|----------|----------------|');
      for (const [dim, mod] of changedModifiers) {
        const interp = mod > 1 ? 'Increased importance' : 'Decreased importance';
        lines.push(`| ${dim} | ${mod.toFixed(3)} | ${interp} |`);
      }
      lines.push('');
    }
  }

  return lines.join('\n');
}

/**
 * Export learnings from brain to file
 * Calls brain_learning state action and saves to cynic-learnings.md
 * @returns {Promise<Object>} Export result
 */
async function exportLearningsToFile() {
  try {
    // Get learning state from brain
    const state = await callBrainTool('brain_learning', { action: 'state' });

    if (!state || state.error) {
      return { success: false, error: state?.error || 'Failed to get learning state' };
    }

    // Format and save
    const data = {
      stats: state.stats || {},
      learnings: state.learnings || [],
      patterns: {
        byItemType: state.patterns?.byItemType || {},
        byDimension: state.patterns?.byDimension || {},
        weightModifiers: state.modifiers || {},
      },
      feedbackSources: state.patterns?.bySource || {},
    };

    saveLearnings(data);

    return {
      success: true,
      path: getLearningsPath(),
      stats: data.stats,
      learningsCount: data.learnings.length,
    };
  } catch (e) {
    return { success: false, error: e.message };
  }
}

/**
 * Import learnings from file to brain
 * Loads cynic-learnings.md and sends to learning service via brain_learning import action
 * @returns {Promise<Object>} Import result
 */
async function importLearningsFromFile() {
  try {
    const data = loadLearnings();

    if (data.learnings.length === 0 && Object.keys(data.stats).length === 0) {
      return { success: true, imported: 0, message: 'No learnings to import' };
    }

    // Send learnings to brain_learning import action
    // The learning service will restore state from this data
    const result = await callBrainTool('brain_learning', {
      action: 'import',
      data: {
        patterns: {
          overall: data.stats || {},
          byItemType: data.patterns?.byItemType || new Map(),
          byDimension: data.patterns?.byDimension || new Map(),
          bySource: data.feedbackSources || new Map(),
        },
        modifiers: {
          weights: data.patterns?.weightModifiers || {},
          thresholds: {},
        },
      },
    });

    return {
      success: result && !result.error,
      imported: data.learnings?.length || 0,
      stats: data.stats,
      message: `Imported learnings from ${getLearningsPath()}`,
      result,
    };
  } catch (e) {
    // Non-blocking - if MCP server isn't available, just return success with local data
    const data = loadLearnings();
    return {
      success: true,
      imported: data.learnings?.length || 0,
      stats: data.stats,
      message: `Loaded ${data.learnings?.length || 0} learnings (MCP unavailable: ${e.message})`,
      local: true,
    };
  }
}

// =============================================================================
// LEARNING FEEDBACK - External validation (Ralph-inspired)
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

  // Learning Feedback (Ralph-inspired external validation)
  sendTestFeedback,
  sendCommitFeedback,
  sendPRFeedback,
  sendBuildFeedback,

  // Learnings Persistence
  getLearningsPath,
  loadLearnings,
  saveLearnings,
  exportLearningsToFile,
  importLearningsFromFile,
};
