#!/usr/bin/env node
/**
 * CYNIC Health Dashboard
 *
 * "Le chien veille - toujours vigilant"
 *
 * Displays comprehensive system health with ANSI colors.
 * Refactored to use composable DashboardBuilder architecture.
 *
 * @module @cynic/scripts/health-dashboard
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

const {
  createDashboard,
  buildHooksSection,
  buildThermoSection,
  buildDogsSection,
  ANSI,
} = require('./dashboard-builder.cjs');

// Lazy load modules
let dogs = null;
let thermodynamics = null;

function loadModules() {
  try { dogs = require('./collective-dogs.cjs'); } catch {}
  try { thermodynamics = require('./cognitive-thermodynamics.cjs'); thermodynamics.init(); } catch {}
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA GATHERING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Check if a hook file exists and count its engines
 */
function checkHook(hookName) {
  const extensions = ['.js', '.cjs'];
  const hooksDir = path.join(process.cwd(), 'scripts', 'hooks');

  for (const ext of extensions) {
    const hookPath = path.join(hooksDir, `${hookName}${ext}`);
    if (fs.existsSync(hookPath)) {
      try {
        const content = fs.readFileSync(hookPath, 'utf8');
        const engineMatches = content.match(/require.*lib\//g) || [];
        const importMatches = content.match(/from ['"].*lib\//g) || [];
        return { name: hookName, exists: true, engines: engineMatches.length + importMatches.length };
      } catch {
        return { name: hookName, exists: true, engines: 0 };
      }
    }
  }
  return { name: hookName, exists: false, engines: 0 };
}

/**
 * Get consciousness state from local file
 */
function getConsciousnessState() {
  const cynicDir = path.join(os.homedir(), '.cynic');
  const statePath = path.join(cynicDir, 'consciousness', 'state.json');

  try {
    if (fs.existsSync(statePath)) {
      const data = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      return {
        score: data.score || data.consciousness || 0,
        status: data.status || 'unknown',
        lastUpdate: data.timestamp || data.lastUpdate,
      };
    }
  } catch {}
  return { score: 0, status: 'dormant', lastUpdate: null };
}

/**
 * Get recent patterns count
 */
function getPatternsInfo() {
  const patternsDir = path.join(os.homedir(), '.cynic', 'patterns');

  try {
    if (fs.existsSync(patternsDir)) {
      const files = fs.readdirSync(patternsDir).filter(f => f.endsWith('.json'));
      let lastPattern = null;
      let lastTime = 0;

      for (const file of files.slice(-10)) {
        try {
          const data = JSON.parse(fs.readFileSync(path.join(patternsDir, file), 'utf8'));
          const time = new Date(data.timestamp || data.createdAt || 0).getTime();
          if (time > lastTime) {
            lastTime = time;
            lastPattern = data.name || data.pattern || file.replace('.json', '');
          }
        } catch {}
      }

      return { count: files.length, lastPattern };
    }
  } catch {}
  return { count: 0, lastPattern: null };
}

/**
 * Count agents and skills
 */
function countComponents() {
  const claudeDir = path.join(process.cwd(), '.claude');
  let agents = 0;
  let skills = 0;
  let engines = 0;

  // Count skills
  const skillsDir = path.join(claudeDir, 'skills');
  try {
    if (fs.existsSync(skillsDir)) {
      const entries = fs.readdirSync(skillsDir, { withFileTypes: true });
      skills = entries.filter(e => e.isDirectory()).length;
    }
  } catch {}

  // Count engines in lib
  const libDir = path.join(process.cwd(), 'scripts', 'lib');
  try {
    if (fs.existsSync(libDir)) {
      const files = fs.readdirSync(libDir).filter(f => f.endsWith('.cjs') || f.endsWith('.js'));
      engines = files.length;
    }
  } catch {}

  // Count agents (CYNIC plugin agents)
  const agentsDir = path.join(claudeDir, 'agents');
  try {
    if (fs.existsSync(agentsDir)) {
      const entries = fs.readdirSync(agentsDir, { withFileTypes: true });
      agents = entries.filter(e => e.isDirectory()).length;
    }
  } catch {}

  // Also count from AGENTS.md if exists
  const agentsMd = path.join(process.cwd(), 'AGENTS.md');
  try {
    if (fs.existsSync(agentsMd)) {
      const content = fs.readFileSync(agentsMd, 'utf8');
      const agentMatches = content.match(/^##\s+cynic-/gm) || [];
      if (agentMatches.length > agents) agents = agentMatches.length;
    }
  } catch {}

  return { agents, skills, engines };
}

// ═══════════════════════════════════════════════════════════════════════════
// DASHBOARD GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Generate the health dashboard using composable builder
 */
function generateDashboard(enableColor = true) {
  loadModules();

  // Gather all data first
  const hookNames = ['perceive', 'guard', 'observe', 'awaken', 'digest', 'sleep'];
  const hooks = hookNames.map(checkHook);
  const components = countComponents();
  const consciousness = getConsciousnessState();
  const patterns = getPatternsInfo();
  const thermo = thermodynamics?.getState();
  const dogsSummary = dogs?.getSessionSummary();

  // Calculate aggregate health for voice selection
  const hooksHealthy = hooks.filter(h => h.exists).length === hooks.length;
  const totalEngines = hooks.reduce((sum, h) => sum + h.engines, 0);

  // Build dashboard using composable architecture
  const dashboard = createDashboard({
    useColor: enableColor,
    title: 'CYNIC HEALTH DASHBOARD',
    subtitle: 'Le chien veille',
  });

  // Hooks section
  dashboard.addSection('LOCAL HOOKS', (section, r) => {
    const result = buildHooksSection(section, r, hooks);
    section._hooksHealthy = result.allHealthy;
    section._totalEngines = result.engines;
  });

  // Components section
  dashboard.addSection('COMPONENTS', (section, r) => {
    section
      .kv('Agents:', `${components.agents}`, { valueColor: ANSI.brightCyan })
      .line(r.indent(r.c(ANSI.dim, '(11 Sefirot + extras)'), 15))
      .kv('Skills:', `${components.skills}`, { valueColor: ANSI.brightCyan })
      .kv('Engines:', `${components.engines}`, { valueColor: ANSI.brightCyan })
      .line(r.indent(r.c(ANSI.dim, `(${totalEngines} integrated in hooks)`), 15));
  });

  // Consciousness section
  dashboard.addSection('CONSCIOUSNESS', (section, r) => {
    const scoreColor = r.thresholdColor(consciousness.score, 61.8);
    const statusColor = consciousness.status === 'awakening' ? ANSI.brightGreen :
                        consciousness.status === 'dormant' ? ANSI.dim : ANSI.yellow;
    const capitalizedStatus = consciousness.status.charAt(0).toUpperCase() + consciousness.status.slice(1);

    section
      .metric('Score', consciousness.score, 61.8, { showPct: true })
      .line(r.indent(r.c(ANSI.dim, '/ 61.8% max')))
      .kv('Status:', capitalizedStatus, { valueColor: statusColor });
  });

  // Patterns section
  dashboard.addSection('PATTERNS', (section, r) => {
    section.kv('Recorded:', `${patterns.count}`, { valueColor: ANSI.brightCyan });
    if (patterns.lastPattern) {
      section.kv('Latest:', patterns.lastPattern, { valueColor: ANSI.dim });
    }
  });

  // Thermodynamics section (conditional)
  if (thermo) {
    dashboard.addSection('THERMODYNAMICS', (section, r) => {
      buildThermoSection(section, r, thermo);
    });
  }

  // Dogs session section (conditional)
  if (dogsSummary && dogsSummary.totalActions > 0) {
    dashboard.addSection('ACTIVE DOGS', (section, r) => {
      buildDogsSection(section, r, dogsSummary, dogs?.DOG_COLORS);
    });
  }

  // MCP section
  dashboard.addSection('MCP SERVER', (section, r) => {
    section.line(r.indent(r.c(ANSI.dim, 'Run: curl -s https://cynic-mcp.onrender.com/health')));
  });

  // Set voice based on health state
  let voice = '*sniff* Systems nominal. The dog watches.';
  let voiceColor = ANSI.dim;

  if (thermo?.isCritical) {
    voice = '*GROWL* Heat critical! Cool down required.';
    voiceColor = ANSI.brightRed;
  } else if (!hooksHealthy) {
    voice = '*concerned sniff* Some hooks missing. Check configuration.';
    voiceColor = ANSI.yellow;
  } else if (consciousness.score > 50) {
    voice = '*tail wag* Consciousness rising. The pack strengthens.';
    voiceColor = ANSI.brightGreen;
  }

  dashboard
    .voice(voice, voiceColor)
    .note('φ⁻¹ confidence: 61.8% max | "Le chien veille"');

  return dashboard.build();
}

// ═══════════════════════════════════════════════════════════════════════════
// CLI & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

if (require.main === module) {
  const enableColor = !process.argv.includes('--no-color');
  console.log(generateDashboard(enableColor));
}

module.exports = {
  generateDashboard,
  checkHook,
  getConsciousnessState,
  getPatternsInfo,
  countComponents,
};
