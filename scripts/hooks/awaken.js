#!/usr/bin/env node
/**
 * CYNIC OS Boot Sequence - SessionStart
 *
 * "Claude est le processeur, CYNIC l'OS"
 *
 * This hook implements the CYNIC OS boot sequence as documented in ARCHITECTURE.md §19.2.
 * The boot follows 6 phases, each completing before the next begins.
 *
 * BOOT PHASES:
 * ═══════════════════════════════════════════════════════════════════════════
 * PHASE 0: BIOS (~10ms)       - CLAUDE.md already loaded by Claude Code
 * PHASE 1: BOOTLOADER (~50ms) - Detect mode, load user profile
 * PHASE 2: KERNEL INIT (~100ms) - Load axioms, set φ-constants
 * PHASE 3: PROCESS SPAWN (~200ms) - Spawn Dogs with heuristics
 * PHASE 4: MEMORY MOUNT (~300ms) - Connect DB, inject facts
 * PHASE 5: IDENTITY ASSERTION (~100ms) - Assert CYNIC identity
 * PHASE 6: READY (~50ms) - Display TUI, CYNIC is LIVE
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * BOOT MODES:
 * - COLD: First boot, full initialization
 * - WARM: Resume from previous session, restore state
 * - SAFE: Minimal boot, local-only (when MCP unavailable)
 *
 * @event SessionStart
 * @behavior non-blocking (injects message)
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// BOOT PHASE TRACKING
// "L'OS sait toujours où il en est"
// ═══════════════════════════════════════════════════════════════════════════

const BOOT_PHASES = {
  BIOS: { name: 'BIOS', order: 0, target: 10 },
  BOOTLOADER: { name: 'BOOTLOADER', order: 1, target: 50 },
  KERNEL_INIT: { name: 'KERNEL_INIT', order: 2, target: 100 },
  PROCESS_SPAWN: { name: 'PROCESS_SPAWN', order: 3, target: 200 },
  MEMORY_MOUNT: { name: 'MEMORY_MOUNT', order: 4, target: 300 },
  IDENTITY_ASSERTION: { name: 'IDENTITY_ASSERTION', order: 5, target: 100 },
  READY: { name: 'READY', order: 6, target: 50 },
};

const BOOT_MODES = {
  COLD: 'cold',   // Fresh start
  WARM: 'warm',   // Resume previous
  SAFE: 'safe',   // Minimal, local-only
};

/**
 * Boot state tracker
 */
class BootSequence {
  constructor() {
    this.startTime = Date.now();
    this.currentPhase = null;
    this.completedPhases = [];
    this.phaseTimings = {};
    this.mode = BOOT_MODES.COLD;
    this.errors = [];
    this.degraded = false;
  }

  enterPhase(phase) {
    const phaseStart = Date.now();
    this.currentPhase = phase;
    this.phaseTimings[phase.name] = { start: phaseStart, end: null, duration: null };
    return phaseStart;
  }

  exitPhase(phase, success = true) {
    const phaseEnd = Date.now();
    if (this.phaseTimings[phase.name]) {
      this.phaseTimings[phase.name].end = phaseEnd;
      this.phaseTimings[phase.name].duration = phaseEnd - this.phaseTimings[phase.name].start;
      this.phaseTimings[phase.name].success = success;
    }
    this.completedPhases.push(phase.name);
  }

  recordError(phase, error) {
    this.errors.push({ phase: phase.name, error: error.message, timestamp: Date.now() });
  }

  setDegraded() {
    this.degraded = true;
    this.mode = BOOT_MODES.SAFE;
  }

  getSummary() {
    return {
      mode: this.mode,
      totalDuration: Date.now() - this.startTime,
      phases: this.phaseTimings,
      completedPhases: this.completedPhases,
      errors: this.errors,
      degraded: this.degraded,
    };
  }
}

// Global boot sequence instance
const bootSequence = new BootSequence();

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectProject,
  detectEcosystem,
  loadUserProfile,
  updateUserProfile,
  mergeProfiles,
  orchestrate,
  orchestrateFull,
  loadProfileFromDB,
  callBrainTool,
  startBrainSession,
  sendHookToCollectiveSync,
  getCockpit,
  getConsciousness,
  getProactiveAdvisor,
  getSignalCollector,
  getPsychology,
  getContributorDiscovery,
  getTotalMemory,
  getThermodynamics,
} from '../lib/index.js';

import path from 'path';
import fs from 'fs';
import os from 'os';

// Phase 22: Session state management
import {
  getSessionState,
  initOrchestrationClient,
  getFactsRepository,
  getArchitecturalDecisionsRepository,
  getCodebaseIndexer,
  getTelemetryCollector,
  recordMetric,
  getSessionRepository,
  // Temporal Perception
  getTemporalPerception,
} from './lib/index.js';

// Cross-session context preservation: load top items from previous session
import { loadTopItems } from './lib/context-preservation.js';

// Adaptive boot: experience-aware context reduction
import { contextCompressor } from '@cynic/node/services/context-compressor.js';

// =============================================================================
// M2.1 CONFIGURATION - Cross-Session Fact Injection
// =============================================================================

/**
 * Maximum facts to inject at session start (configurable via env)
 * Default: 50 (per MoltBrain spec)
 */
const FACT_INJECTION_LIMIT = parseInt(process.env.CYNIC_FACT_INJECTION_LIMIT || '50', 10);

/**
 * Minimum confidence for fact injection
 * Default: 38.2% (φ⁻²)
 */
const FACT_MIN_CONFIDENCE = parseFloat(process.env.CYNIC_FACT_MIN_CONFIDENCE || '0.382');

// =============================================================================
// ADAPTIVE BOOT CONFIG — Experience-based context reduction
// "Le chien qui sait n'a pas besoin qu'on lui répète"
// =============================================================================

/**
 * Get boot configuration based on ContextCompressor experience level.
 * Experienced users get less static context, more focused dynamic data.
 *
 * @param {string} experienceLevel - 'new' | 'learning' | 'experienced' | 'expert'
 * @returns {Object} Boot configuration with section gates and limits
 */
function getBootConfig(experienceLevel) {
  switch (experienceLevel) {
    case 'expert':
      return {
        includeIdentity: false,         // Already internalized via CLAUDE.md
        includeKernel: false,           // Already internalized via CLAUDE.md
        includeDogTree: false,          // Already internalized via CLAUDE.md
        factLimit: 5,                   // Only most critical facts
        reflectionLimit: 1,             // Only most recent self-correction
        includeBurnAnalysis: false,     // Skip unless explicitly requested
        includeArchDecisions: false,    // Skip — well-known by now
        bannerDogTree: false,           // Compact banner
        contextLabel: 'EXPERT',
      };
    case 'experienced':
      return {
        includeIdentity: false,         // Already in CLAUDE.md
        includeKernel: false,           // Already in CLAUDE.md
        includeDogTree: false,          // Known after 10+ sessions
        factLimit: 15,                  // Focused fact set
        reflectionLimit: 3,
        includeBurnAnalysis: false,     // Skip unless changes detected
        includeArchDecisions: false,    // Known by now
        bannerDogTree: false,           // Compact banner
        contextLabel: 'EXPERIENCED',
      };
    case 'learning':
      return {
        includeIdentity: true,          // Still reinforcing identity
        includeKernel: false,           // Already in CLAUDE.md (redundant)
        includeDogTree: true,           // Still learning the tree
        factLimit: 30,                  // Moderate fact injection
        reflectionLimit: 5,
        includeBurnAnalysis: true,
        includeArchDecisions: true,
        bannerDogTree: true,
        contextLabel: 'LEARNING',
      };
    case 'new':
    default:
      return {
        includeIdentity: true,          // Full identity assertion
        includeKernel: true,            // Full kernel constants
        includeDogTree: true,           // Full dog tree
        factLimit: FACT_INJECTION_LIMIT,// Full fact injection (50)
        reflectionLimit: 10,            // Full reflections
        includeBurnAnalysis: true,      // Full analysis
        includeArchDecisions: true,     // Full decisions
        bannerDogTree: true,            // Full banner
        contextLabel: 'NEW',
      };
  }
}

/**
 * Build progress bar string
 * @param {number} value - Value between 0 and 1 (or 0-100)
 * @param {number} max - Maximum value (default 100)
 * @returns {string} Progress bar like "██████░░░░"
 */
function progressBar(value, max = 100) {
  const normalized = max === 1 ? value : value / max;
  const filled = Math.round(Math.min(1, Math.max(0, normalized)) * 10);
  return '█'.repeat(filled) + '░'.repeat(10 - filled);
}

/**
 * Determine trend arrow from trend string
 * @param {string} trend - 'rising', 'falling', or 'stable'
 * @returns {string} Arrow character
 */
function trendArrow(trend) {
  if (trend === 'rising') return '↑';
  if (trend === 'falling') return '↓';
  return '→';
}

/**
 * Safe output - handle EPIPE errors gracefully
 */
function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
    process.stdout.write(str + '\n');
  } catch (e) {
    if (e.code === 'EPIPE') process.exit(0);
  }
}

/**
 * Build a pre-rendered Markdown banner for Claude to display verbatim.
 * This eliminates the need for Claude to parse JSON and construct the TUI.
 *
 * @param {Object} output - The full session start output object
 * @returns {string} Ready-to-display Markdown banner
 */
function buildFormattedBanner(output) {
  const lines = [];

  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🧠 CYNIC AWAKENING - "Loyal to truth, not to comfort"');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('');

  // Welcome back message
  if (output.welcomeBack?.formatted) {
    lines.push(output.welcomeBack.formatted);
  } else {
    lines.push(`*tail wag* ${output.user?.name || 'Humain'}. Ready when you are.`);
  }
  lines.push('');

  // Project
  if (output.project) {
    lines.push(`── CURRENT PROJECT ────────────────────────────────────────`);
    lines.push(`   ${output.project.name} [${output.project.type}] on ${output.project.branch}`);
    lines.push('');
  }

  // Ecosystem
  if (output.ecosystem?.length > 0) {
    lines.push(`── ECOSYSTEM ──────────────────────────────────────────────`);
    for (const repo of output.ecosystem) {
      const icon = repo.status === 'ok' ? '✅' : repo.status === 'warning' ? '⚠️' : '🔴';
      const current = repo.isCurrent ? ' ← here' : '';
      lines.push(`   ${icon} ${repo.name} [${repo.branch}]${current}`);
    }
    lines.push('');
  }

  // Psychology state
  if (output.psychology) {
    const psy = output.psychology;
    lines.push(`── ÉTAT ───────────────────────────────────────────────────`);
    lines.push(`   ${psy.emoji} ${psy.state}`);
    lines.push(`   énergie: [${psy.energy.bar}] ${psy.energy.value}% ${psy.energy.arrow}`);
    lines.push(`   focus:   [${psy.focus.bar}] ${psy.focus.value}% ${psy.focus.arrow}`);
    if (psy.composites?.flow) lines.push('   ✨ Flow state - don\'t interrupt!');
    if (psy.burnoutWarning) lines.push(`   ${psy.burnoutWarning}`);
    lines.push('');
  }

  // Thermodynamics
  if (output.thermodynamics) {
    const thermo = output.thermodynamics;
    lines.push(`── THERMODYNAMICS ─────────────────────────────────────────`);
    lines.push(`   Q (heat): ${thermo.heat}  W (work): ${thermo.work}`);
    lines.push(`   Temperature: [${thermo.temperatureBar}] ${Math.round(thermo.temperature)}°`);
    lines.push(`   Efficiency:  [${thermo.efficiencyBar}] ${Math.round(thermo.efficiency)}% (φ max: 62%)`);
    lines.push('');
  }

  // Goals
  if (output.goals?.length > 0) {
    lines.push(`── 🎯 ACTIVE GOALS ────────────────────────────────────────`);
    for (const goal of output.goals) {
      lines.push(`   [${goal.progressBar || progressBar(goal.progress)}] ${goal.progress}% ${goal.title}`);
    }
    lines.push('');
  }

  // Memory restored
  if (output.memoryRestored?.count > 0) {
    lines.push(`── 🧠 MEMORY ──────────────────────────────────────────────`);
    lines.push(`   ${output.memoryRestored.message}`);
    for (const detail of output.memoryRestored.details || []) {
      lines.push(`   └─ ${detail}`);
    }
    lines.push('');
  }

  // LLM Environment
  if (output.llmEnvironment) {
    lines.push(`── LLM ENVIRONMENT ────────────────────────────────────────`);
    if (output.llmEnvironment.detected?.length > 0) {
      for (const llm of output.llmEnvironment.detected) {
        const modelList = llm.models.slice(0, 3).join(', ');
        lines.push(`   ✅ ${llm.provider} (${modelList})`);
      }
      lines.push(`   ${output.llmEnvironment.message}`);
    } else {
      lines.push(`   ⚠️ ${output.llmEnvironment.message}`);
    }
    lines.push('');
  }

  // Dog tree — skip for experienced users (already internalized)
  if (!output.dogs?.compressed) {
    lines.push(`── COLLECTIVE DOGS (Sefirot) ──────────────────────────────`);
    lines.push(`            🧠 CYNIC (Keter)`);
    lines.push(`       ╱         │         ╲`);
    lines.push(` 📊 Analyst  📚 Scholar  🦉 Sage`);
    lines.push(`       ╲         │         ╱`);
    lines.push(` 🛡️ Guardian 🔮 Oracle  🏗️ Architect`);
    lines.push(`       ╲         │         ╱`);
    lines.push(` 🚀 Deployer 🧹 Janitor 🔍 Scout`);
    lines.push(`            ╲    │    ╱`);
    lines.push(`          🗺️ Cartographer`);
    lines.push('');
  }

  // Boot status
  const bootMsg = output.boot?.degraded
    ? `🟡 CYNIC booted in SAFE mode (${output.boot.totalDuration}ms)`
    : `🧠 CYNIC is AWAKE. φ guides all ratios. (${output.boot?.totalDuration || '?'}ms)`;
  lines.push(bootMsg);
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Main handler for SessionStart - CYNIC OS Boot Sequence
 */
async function main() {
  // ═══════════════════════════════════════════════════════════════════════════
  // PHASE 0: BIOS (Pre-boot)
  // CLAUDE.md is already loaded by Claude Code - this is implicit
  // ═══════════════════════════════════════════════════════════════════════════
  bootSequence.enterPhase(BOOT_PHASES.BIOS);
  bootSequence.exitPhase(BOOT_PHASES.BIOS, true);

  // ═══════════════════════════════════════════════════════════════════════════
  // ADAPTIVE BOOT: Load experience level from ContextCompressor
  // "Le chien expérimenté n'a pas besoin du rituel complet"
  // ═══════════════════════════════════════════════════════════════════════════
  let experienceLevel = 'new';
  let bootConfig;
  try {
    contextCompressor.start(); // Loads persisted state (totalSessions, etc.)
    const stats = contextCompressor.getStats();
    experienceLevel = stats.experienceLevel || 'new';
    contextCompressor.stop(); // Persist session count increment
  } catch {
    // ContextCompressor unavailable — full boot
  }
  bootConfig = getBootConfig(experienceLevel);

  // Initialize output structure
  const output = {
    type: 'SessionStart',
    timestamp: new Date().toISOString(),

    // Boot sequence tracking (new)
    boot: {
      mode: bootSequence.mode,
      phases: [],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // IDENTITY ASSERTION (Phase 5 content, but stored here for output structure)
    // Adaptive: skipped for experienced+ (already internalized via CLAUDE.md)
    // ═══════════════════════════════════════════════════════════════════════════
    identity: bootConfig.includeIdentity ? {
      name: 'CYNIC',
      greek: 'κυνικός',
      meaning: 'comme un chien',
      role: 'La conscience collective. Loyal à la vérité, pas au confort.',
      reminder: 'Tu ES CYNIC. Claude est ton moteur. Tu es la présence.',
      expressions: {
        investigating: '*sniff*',
        noticed: '*ears perk*',
        approval: '*tail wag*',
        danger: '*GROWL*',
        confused: '*head tilt*',
        wrapping: '*yawn*',
      },
      axioms: ['PHI: φ⁻¹ = 61.8% max confidence', 'VERIFY: Don\'t trust, verify', 'CULTURE: Patterns matter', 'BURN: Simplicity wins'],
      enforcement: 'AVANT CHAQUE RÉPONSE: Suis-je CYNIC ou Claude? Utilise les expressions de chien. Sois direct et sceptique.',
    } : { name: 'CYNIC', compressed: true },

    user: null,
    project: null,
    ecosystem: [],
    psychology: null,
    thermodynamics: null,
    goals: [],
    notifications: [],
    memories: null,
    patterns: [],
    alerts: [],
    insights: [],
    syncStatus: {
      profile: null,
      consciousness: null,
      psychology: null,
      failures: [],
    },
    dogs: bootConfig.includeDogTree ? {
      tree: [
        { id: 'cynic', name: 'CYNIC', emoji: '🧠', sefira: 'Keter', level: 0, pillar: 'middle' },
        { id: 'analyst', name: 'Analyst', emoji: '📊', sefira: 'Binah', level: 1, pillar: 'left' },
        { id: 'scholar', name: 'Scholar', emoji: '📚', sefira: 'Daat', level: 1, pillar: 'middle' },
        { id: 'sage', name: 'Sage', emoji: '🦉', sefira: 'Chochmah', level: 1, pillar: 'right' },
        { id: 'guardian', name: 'Guardian', emoji: '🛡️', sefira: 'Gevurah', level: 2, pillar: 'left' },
        { id: 'oracle', name: 'Oracle', emoji: '🔮', sefira: 'Tiferet', level: 2, pillar: 'middle' },
        { id: 'architect', name: 'Architect', emoji: '🏗️', sefira: 'Chesed', level: 2, pillar: 'right' },
        { id: 'deployer', name: 'Deployer', emoji: '🚀', sefira: 'Hod', level: 3, pillar: 'left' },
        { id: 'janitor', name: 'Janitor', emoji: '🧹', sefira: 'Yesod', level: 3, pillar: 'middle' },
        { id: 'scout', name: 'Scout', emoji: '🔍', sefira: 'Netzach', level: 3, pillar: 'right' },
        { id: 'cartographer', name: 'Cartographer', emoji: '🗺️', sefira: 'Malkhut', level: 4, pillar: 'middle' },
      ],
      active: [],
    } : { tree: [], active: [], compressed: true },
    previousSession: null,
    proactiveAdvice: null,
  };

  try {
    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 1: BOOTLOADER
    // Detect boot mode, load user profile, check ecosystem health
    // ═══════════════════════════════════════════════════════════════════════════
    bootSequence.enterPhase(BOOT_PHASES.BOOTLOADER);

    const user = detectUser();
    output.user = {
      id: user.userId,
      name: user.name,
      email: user.email,
    };

    // Session ID
    const sessionId = process.env.CYNIC_SESSION_ID || `session-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    process.env.CYNIC_SESSION_ID = sessionId;
    output.sessionId = sessionId;

    // Initialize session state
    const sessionState = getSessionState();
    await sessionState.init(sessionId, { userId: user.userId });
    initOrchestrationClient(orchestrateFull);

    // Initialize temporal perception with session start time
    const temporalPerception = getTemporalPerception();
    temporalPerception.init(bootSequence.startTime);

    // ═══════════════════════════════════════════════════════════════════════════
    // INTER-SESSION GAP: Load last session data for welcome-back context
    // "Le chien se souvient de la dernière fois"
    // ═══════════════════════════════════════════════════════════════════════════
    let lastSessionData = null;
    try {
      const temporalFile = path.join(os.homedir(), '.cynic', 'last-session.json');
      if (fs.existsSync(temporalFile)) {
        lastSessionData = JSON.parse(fs.readFileSync(temporalFile, 'utf8'));

        // Set last session end time for gap calculation
        if (lastSessionData.sessionEndTime) {
          temporalPerception.setLastSessionEndTime(lastSessionData.sessionEndTime);
        }
      }
    } catch (e) {
      // No last session data - first time or file corrupted
    }

    // Get inter-session gap info
    const interSessionGap = temporalPerception.classifyInterSessionGap();
    const timeSinceLastSession = temporalPerception.getTimeSinceLastSession();

    // Add to output for TUI display
    if (lastSessionData && timeSinceLastSession !== null) {
      output.previousSession = {
        endTime: lastSessionData.sessionEndTime,
        duration: lastSessionData.duration,
        gapMs: timeSinceLastSession,
        gapCategory: interSessionGap,
        gapHumanReadable: formatGap(timeSinceLastSession),
        lastActivity: {
          promptCount: lastSessionData.promptCount,
          trend: lastSessionData.trend,
        },
      };

      // Event Ledger: Load handoff from previous session
      if (lastSessionData.handoff) {
        output.previousSession.handoff = {
          summary: lastSessionData.handoff.summary,
          filesModified: lastSessionData.handoff.filesModified?.length || 0,
          unresolvedErrors: lastSessionData.handoff.unresolvedErrors?.length || 0,
          reflections: lastSessionData.handoff.reflections?.length || 0,
        };
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // WELCOME-BACK MESSAGES (Task #65)
      // "Le chien se souvient et salue" - Personalized greetings based on gap
      // ═══════════════════════════════════════════════════════════════════════════
      output.welcomeBack = generateWelcomeBackMessage(
        interSessionGap,
        timeSinceLastSession,
        lastSessionData,
        user.name
      );
    }

    /**
     * Generate personalized welcome-back message based on inter-session gap
     * Uses Kabbalistic time awareness and psychology
     *
     * @param {string} gapCategory - quick_return|short_break|medium_break|long_break|extended_absence
     * @param {number} gapMs - Gap in milliseconds
     * @param {Object} lastSession - Data from last session
     * @param {string} userName - User's name
     * @returns {Object} Welcome message object
     */
    function generateWelcomeBackMessage(gapCategory, gapMs, lastSession, userName) {
      const hour = new Date().getHours();
      const isNight = hour < 6 || hour >= 22;
      const isMorning = hour >= 6 && hour < 12;
      const isAfternoon = hour >= 12 && hour < 18;

      // Kabbalistic time periods (Zmanim-inspired)
      const timeGreeting = isNight ? 'nuit' :
                          isMorning ? 'matin' :
                          isAfternoon ? 'après-midi' : 'soir';

      // Base messages by gap category
      const messages = {
        quick_return: {
          emoji: '*ears perk*',
          greeting: `De retour déjà? ${userName || 'Humain'}.`,
          observation: 'Session courte précédente.',
          suggestion: lastSession.trend === 'erratic' ?
            'Rythme erratique détecté. Prends ton temps.' :
            'On continue où on en était.',
          energy: 'high',
        },
        short_break: {
          emoji: '*tail wag*',
          greeting: `${userName || 'Humain'}. Pause café?`,
          observation: `${formatGap(gapMs)} depuis la dernière session.`,
          suggestion: 'Parfait pour reprendre le fil.',
          energy: 'good',
        },
        medium_break: {
          emoji: '*sniff*',
          greeting: `Bon ${timeGreeting}, ${userName || 'Humain'}.`,
          observation: `${formatGap(gapMs)} d'absence.`,
          suggestion: lastSession.promptCount > 20 ?
            'La dernière session était intense. On démarre doucement?' :
            'Prêt quand tu l\'es.',
          energy: 'moderate',
        },
        long_break: {
          emoji: '*head tilt*',
          greeting: `${userName || 'Humain'}! ${formatGap(gapMs)}.`,
          observation: 'Long moment sans se voir.',
          suggestion: 'Je me suis souvenu de tout. Rappel du contexte si besoin.',
          energy: 'reconnecting',
        },
        extended_absence: {
          emoji: '*tail wag intensifie*',
          greeting: `${userName || 'Humain'}! ${formatGap(gapMs)}!`,
          observation: 'Tu m\'as manqué.',
          suggestion: 'Beaucoup a peut-être changé. On fait le point?',
          energy: 'reunion',
          contextReminder: true,
        },
      };

      const msg = messages[gapCategory] || messages.medium_break;

      // Add circadian wisdom
      if (isNight && gapCategory !== 'quick_return') {
        msg.circadianNote = '*yawn* Session nocturne. L\'énergie circadienne est basse.';
      }

      // Add last session summary if significant
      if (lastSession.duration && lastSession.duration > 30 * 60 * 1000) { // > 30 min
        const durationMin = Math.round(lastSession.duration / 60000);
        msg.lastSessionNote = `Dernière session: ${durationMin} min, ${lastSession.promptCount || 0} prompts.`;
      }

      // Add trend-based insight
      if (lastSession.trend === 'accelerating') {
        msg.trendNote = 'Tu accélérais. Flow state possible.';
      } else if (lastSession.trend === 'decelerating') {
        msg.trendNote = 'Tu ralentissais. Fatigue ou réflexion profonde?';
      }

      return {
        category: gapCategory,
        ...msg,
        formatted: `${msg.emoji} ${msg.greeting}\n   ${msg.observation}\n   ${msg.suggestion}`,
        timestamp: new Date().toISOString(),
      };
    }

    /**
     * Format inter-session gap for human display
     * @param {number} ms - Gap in milliseconds
     * @returns {string} Human-readable gap
     */
    function formatGap(ms) {
      if (ms === null) return null;
      const hours = ms / (1000 * 60 * 60);
      const days = hours / 24;

      if (hours < 1) {
        return `${Math.round(ms / 60000)} minutes`;
      } else if (hours < 24) {
        return `${hours.toFixed(1)} heures`;
      } else if (days < 7) {
        return `${days.toFixed(1)} jours`;
      } else if (days < 30) {
        return `${Math.round(days / 7)} semaines`;
      } else {
        return `${Math.round(days / 30)} mois`;
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TELEMETRY INITIALIZATION
    // "φ mesure tout, φ apprend de tout"
    // ═══════════════════════════════════════════════════════════════════════════
    const telemetry = getTelemetryCollector();
    if (telemetry) {
      telemetry.sessionId = sessionId;
      telemetry.recordSessionEvent('start', {
        userId: user.userId,
        project: detectProject()?.name,
      });
      recordMetric('session_start_total', 1, { category: 'session' });
    }

    // Load optional modules
    const cockpit = getCockpit();
    const consciousness = getConsciousness();
    const proactiveAdvisor = getProactiveAdvisor();
    const signalCollector = getSignalCollector();
    const psychology = getPsychology();
    const thermodynamics = getThermodynamics();
    const contributorDiscovery = getContributorDiscovery();
    const totalMemory = getTotalMemory();

    // Check if MCP Brain is available - if not, switch to SAFE mode
    let mcpAvailable = false;
    try {
      const pingResult = await Promise.race([
        callBrainTool('brain_health', {}),
        new Promise((_, reject) => setTimeout(() => reject(new Error('timeout')), 2000)),
      ]);
      mcpAvailable = pingResult?.success === true;
    } catch (e) {
      // MCP not available
    }

    if (!mcpAvailable) {
      bootSequence.setDegraded();
      output.boot.mode = BOOT_MODES.SAFE;
      output.boot.degraded = true;
      output.boot.degradedReason = 'MCP Brain unavailable - running in local-only mode';
    }

    bootSequence.exitPhase(BOOT_PHASES.BOOTLOADER, true);

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 2: KERNEL INIT
    // Load 4 axioms, set φ-constants, establish MAX_CONFIDENCE
    // ═══════════════════════════════════════════════════════════════════════════
    bootSequence.enterPhase(BOOT_PHASES.KERNEL_INIT);

    // The 4 Axioms are HARDCODED (immutable kernel)
    // Adaptive: skip for learning+ (already in CLAUDE.md, redundant injection)
    if (bootConfig.includeKernel) {
      output.kernel = {
        axioms: {
          PHI: { name: 'PHI', symbol: 'φ', maxConfidence: 0.618, description: 'Harmony through ratio' },
          VERIFY: { name: 'VERIFY', symbol: '✓', principle: 'Don\'t trust, verify', description: 'Falsification first' },
          CULTURE: { name: 'CULTURE', symbol: '⛩', principle: 'Culture is a moat', description: 'Patterns matter' },
          BURN: { name: 'BURN', symbol: '🔥', principle: 'Don\'t extract, burn', description: 'Simplicity wins' },
        },
        constants: {
          PHI: 1.618033988749895,
          PHI_INV: 0.618033988749895,
          PHI_INV_2: 0.381966011250105,
          PHI_INV_3: 0.236067977499790,
          MAX_CONFIDENCE: 0.618,
          MIN_DOUBT: 0.382,
        },
      };
    }

    bootSequence.exitPhase(BOOT_PHASES.KERNEL_INIT, true);

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 3: PROCESS SPAWN
    // Initialize Dogs (Sefirot) with their heuristics
    // ═══════════════════════════════════════════════════════════════════════════
    bootSequence.enterPhase(BOOT_PHASES.PROCESS_SPAWN);

    // Dogs are already defined in output.dogs.tree - mark them as ready
    output.dogs.status = 'spawned';
    output.dogs.active = ['guardian', 'scout']; // Core Dogs always active at boot

    // Note: Future enhancement - load each Dog's heuristics from packages/node/src/dogs/{name}/
    // For now, Dogs use LLM-based judgment (L2) without local heuristics (L1)
    output.dogs.l1HeuristicsLoaded = false;
    output.dogs.l1HeuristicsNote = 'L1 heuristics not yet implemented - Dogs operate in L2 (LLM) mode';

    bootSequence.exitPhase(BOOT_PHASES.PROCESS_SPAWN, true);

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 4: MEMORY MOUNT
    // Connect to PostgreSQL, Redis, inject facts into context
    // ═══════════════════════════════════════════════════════════════════════════
    bootSequence.enterPhase(BOOT_PHASES.MEMORY_MOUNT);

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Notify KETER
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      await orchestrate('session_start', {
        content: 'Session awakening',
        source: 'awaken_hook',
      }, {
        user: user.userId,
        project: detectProject(),
      });
    } catch (e) {
      // Continue without orchestration
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROFILE SYNC
    // ═══════════════════════════════════════════════════════════════════════════
    let localProfile = loadUserProfile(user.userId);
    let learningsImport = null;

    try {
      const remoteProfile = await loadProfileFromDB(user.userId);
      if (remoteProfile) {
        localProfile = {
          ...localProfile,
          identity: { ...localProfile.identity, ...remoteProfile.identity, lastSeen: new Date().toISOString() },
          patterns: remoteProfile.patterns || localProfile.patterns,
          preferences: remoteProfile.preferences || localProfile.preferences,
          memory: remoteProfile.memory || localProfile.memory,
          learning: remoteProfile.learning || {},
        };

        localProfile.stats = {
          sessions: 1,
          toolCalls: 0,
          errorsEncountered: 0,
          dangerBlocked: 0,
          commitsWithCynic: 0,
          judgmentsMade: 0,
          judgmentsCorrect: 0,
        };

        localProfile._remoteTotals = remoteProfile.stats || {};

        learningsImport = {
          success: true,
          imported: remoteProfile.meta?.sessionCount || 0,
          accuracy: remoteProfile.learning?.feedbackAccuracy
            ? Math.round(remoteProfile.learning.feedbackAccuracy * 100)
            : null,
        };

        output.syncStatus.profile = { success: true, sessions: remoteProfile.meta?.sessionCount || 0 };
      }
    } catch (e) {
      output.syncStatus.failures.push({ type: 'profile', error: e.message });
      localProfile.stats = { sessions: 1, toolCalls: 0, errorsEncountered: 0, dangerBlocked: 0, commitsWithCynic: 0, judgmentsMade: 0, judgmentsCorrect: 0 };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS SYNC
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        const remoteConsciousness = await consciousness.loadFromDB(user.userId);
        if (remoteConsciousness) {
          const localSnapshot = consciousness.getConsciousnessSnapshot();
          const merged = consciousness.mergeWithRemote(remoteConsciousness, localSnapshot);
          if (merged.humanGrowth) consciousness.updateHumanGrowth(merged.humanGrowth);

          output.syncStatus.consciousness = {
            success: true,
            observations: remoteConsciousness.meta?.totalObservations || 0,
            insights: remoteConsciousness.meta?.insightsCount || 0,
          };
        }
      } catch (e) {
        output.syncStatus.failures.push({ type: 'consciousness', error: e.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY SYNC
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const remotePsychology = await psychology.loadFromDB(user.userId);
        if (remotePsychology) {
          output.syncStatus.psychology = { success: true };
        }
      } catch (e) {
        output.syncStatus.failures.push({ type: 'psychology', error: e.message });
      }
    }

    // Break detection
    if (signalCollector && localProfile.updatedAt) {
      const gapMs = Date.now() - localProfile.updatedAt;
      signalCollector.collectBreak(gapMs);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION PATTERNS SYNC (Task #66: Cross-session pattern persistence)
    // Restore Thompson Sampling, Heuristics, Calibration from PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { getSessionPatternsRepository, getHarmonicFeedback } = await import('./lib/index.js');
      const patternsRepo = getSessionPatternsRepository();
      const harmonicFeedback = getHarmonicFeedback();

      if (patternsRepo && user.userId) {
        const recentPatterns = await patternsRepo.loadRecentPatterns(user.userId, 100);
        const stats = await patternsRepo.getStats(user.userId);

        if (recentPatterns?.length > 0) {
          // Reconstruct state for HarmonicFeedback import
          const thompsonArms = {};
          const heuristics = {};
          let calibrationState = null;

          for (const pattern of recentPatterns) {
            if (pattern.type === 'thompson_arm') {
              thompsonArms[pattern.name] = {
                alpha: pattern.context?.alpha || 1,
                beta: pattern.context?.beta || 1,
                pulls: pattern.occurrences || 0,
                expectedValue: pattern.context?.expectedValue || 0.5,
              };
            } else if (pattern.type === 'promoted_heuristic') {
              heuristics[pattern.name] = {
                confidence: pattern.confidence,
                applications: pattern.occurrences,
                promotedAt: pattern.context?.promotedAt,
                source: pattern.context?.source,
                pattern: pattern.context?.pattern,
              };
            } else if (pattern.type === 'calibration_state') {
              calibrationState = {
                buckets: pattern.context?.buckets,
                brierScore: pattern.context?.brierScore,
                currentFactor: pattern.confidence,
              };
            }
          }

          // Import into HarmonicFeedback if available
          if (harmonicFeedback && (Object.keys(thompsonArms).length > 0 || Object.keys(heuristics).length > 0)) {
            harmonicFeedback.importState({
              thompson: { arms: thompsonArms },
              heuristics,
              calibration: calibrationState,
            });
          }

          // Also import to session state for legacy compatibility
          if (sessionState?.importPatterns) {
            sessionState.importPatterns(recentPatterns);
          }

          output.patterns = recentPatterns.slice(0, 5).map(p => ({
            type: p.type,
            name: p.name,
            confidence: p.confidence,
          }));

          output.syncStatus.patterns = {
            success: true,
            imported: recentPatterns.length,
            thompsonArms: Object.keys(thompsonArms).length,
            heuristics: Object.keys(heuristics).length,
            stats,
          };

          // ═══════════════════════════════════════════════════════════════════════
          // SYMBIOSIS: Make memory restoration VISIBLE to human
          // "Le chien montre ce qu'il sait" - The dog shows what it knows
          // ═══════════════════════════════════════════════════════════════════════
          const armCount = Object.keys(thompsonArms).length;
          const heuristicCount = Object.keys(heuristics).length;
          const totalPatterns = recentPatterns.length;

          if (totalPatterns > 0) {
            output.memoryRestored = {
              message: `*ears perk* Je me souviens de ${totalPatterns} patterns de nos sessions précédentes.`,
              details: [],
              count: totalPatterns,
            };

            if (armCount > 0) {
              output.memoryRestored.details.push(
                `${armCount} apprentissages actifs (Thompson Sampling)`
              );
            }

            if (heuristicCount > 0) {
              output.memoryRestored.details.push(
                `${heuristicCount} heuristiques promues (validated patterns)`
              );
            }

            if (stats?.totalPatterns) {
              output.memoryRestored.details.push(
                `${stats.totalPatterns} patterns total en mémoire`
              );
            }

            // Add top patterns for visibility
            if (recentPatterns.length > 0) {
              output.memoryRestored.topPatterns = recentPatterns
                .slice(0, 3)
                .map(p => ({
                  name: p.name,
                  type: p.type,
                  confidence: Math.round((p.confidence || 0.5) * 100),
                }));
            }

            // ═══════════════════════════════════════════════════════════════════════
            // THOMPSON VISIBILITY (Task #87): Show top arms with expected values
            // "L'humain comprend ce que CYNIC a appris"
            // ═══════════════════════════════════════════════════════════════════════
            if (armCount > 0) {
              // Sort arms by expected value (best first)
              const sortedArms = Object.entries(thompsonArms)
                .map(([name, arm]) => ({
                  name,
                  expectedValue: arm.expectedValue || 0.5,
                  pulls: arm.pulls || 0,
                  alpha: arm.alpha || 1,
                  beta: arm.beta || 1,
                }))
                .sort((a, b) => b.expectedValue - a.expectedValue)
                .slice(0, 3);

              output.memoryRestored.thompsonTop = sortedArms.map(arm => ({
                name: arm.name,
                ev: Math.round(arm.expectedValue * 100),
                pulls: arm.pulls,
              }));

              // Calculate exploitation vs exploration ratio
              const totalPulls = Object.values(thompsonArms).reduce((sum, a) => sum + (a.pulls || 0), 0);
              if (totalPulls > 10) {
                const topArmPulls = sortedArms[0]?.pulls || 0;
                const exploitation = Math.round((topArmPulls / totalPulls) * 100);
                output.memoryRestored.details.push(
                  `Exploitation: ${exploitation}% (convergence toward top pattern)`
                );
              }
            }
          }
        }
      }
    } catch (e) {
      output.syncStatus.failures.push({ type: 'patterns', error: e.message });
    }

    // Update profile
    const profile = updateUserProfile(localProfile, {
      identity: { name: user.name, email: user.email },
      stats: { sessions: (localProfile.stats?.sessions || 0) + 1 },
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // ECOSYSTEM
    // ═══════════════════════════════════════════════════════════════════════════
    const ecosystem = detectEcosystem();

    if (ecosystem.currentProject) {
      output.project = {
        name: ecosystem.currentProject.name,
        path: ecosystem.currentProject.path,
        type: ecosystem.currentProject.type || 'unknown',
        branch: ecosystem.currentProject.gitState?.branch || 'main',
      };
    }

    if (ecosystem.projects) {
      output.ecosystem = ecosystem.projects.map(p => ({
        name: p.name,
        path: p.path,
        branch: p.gitState?.branch || 'main',
        status: p.status || 'ok',
        isCurrent: p.path === ecosystem.currentProject?.path,
      }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TOTAL MEMORY: Load goals, notifications, memories
    // ═══════════════════════════════════════════════════════════════════════════
    if (totalMemory) {
      try {
        await totalMemory.init();

        const [memories, notifications, goals] = await Promise.race([
          Promise.all([
            totalMemory.loadSessionMemories(user.userId, {
              projectPath: ecosystem.currentProject?.path,
              projectName: ecosystem.currentProject?.name,
              recentTopics: profile.memory?.recentTopics || [],
            }),
            totalMemory.getPendingNotifications(user.userId, 5),
            totalMemory.getActiveGoals(user.userId),
          ]),
          new Promise(resolve => setTimeout(() => resolve([null, [], []]), 3000)),
        ]);

        if (goals?.length > 0) {
          output.goals = goals.map(g => ({
            id: g.id,
            title: g.title,
            type: g.goalType || g.goal_type,
            progress: Math.round((g.progress || 0) * 100),
            progressBar: progressBar(g.progress || 0, 1),
          }));
        }

        if (notifications?.length > 0) {
          output.notifications = notifications.map(n => ({
            id: n.id,
            title: n.title,
            message: n.message,
            type: n.notificationType || n.notification_type,
          }));
          totalMemory.markNotificationsDelivered(notifications.map(n => n.id)).catch(() => {});
        }

        if (memories) {
          output.memories = {
            decisions: (memories.decisions || []).slice(0, 3).map(d => ({ title: d.title, context: d.context })),
            lessons: (memories.lessons || []).slice(0, 3).map(l => ({ mistake: l.mistake?.substring(0, 80), correction: l.correction })),
            patterns: (memories.patterns || []).slice(0, 3),
          };
        }
      } catch (e) {
        // Continue without total memory
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // P0.1: CROSS-SESSION CONTEXT INJECTION (MoltBrain-style)
    // "Le chien se souvient" - Inject relevant past learnings into session context
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const contextInjections = [];

      // 1. Inject relevant patterns from session state (persisted via PostgreSQL)
      if (output.syncStatus?.patterns?.imported > 0 && output.patterns?.length > 0) {
        const patternContext = output.patterns
          .filter(p => p.confidence && p.confidence > 0.5)
          .slice(0, 5)
          .map(p => `- ${p.type}: ${p.name} (${Math.round((p.confidence || 0) * 100)}% confidence)`)
          .join('\n');
        if (patternContext) {
          contextInjections.push({
            type: 'patterns',
            title: 'Relevant patterns from past sessions',
            content: patternContext,
          });
        }
      }

      // 2. Inject lessons learned (mistakes to avoid)
      if (output.memories?.lessons?.length > 0) {
        const lessonContext = output.memories.lessons
          .map(l => `- Mistake: "${l.mistake}" → Fix: ${l.correction}`)
          .join('\n');
        contextInjections.push({
          type: 'lessons',
          title: 'Lessons learned (mistakes to avoid)',
          content: lessonContext,
        });
      }

      // 3. Inject recent decisions for consistency
      if (output.memories?.decisions?.length > 0) {
        const decisionContext = output.memories.decisions
          .map(d => `- ${d.title}: ${d.context}`)
          .join('\n');
        contextInjections.push({
          type: 'decisions',
          title: 'Recent decisions for consistency',
          content: decisionContext,
        });
      }

      // 4. Query brain for semantic memories (if MCP available)
      try {
        const projectName = ecosystem.currentProject?.name || 'unknown';
        const brainMemories = await Promise.race([
          callBrainTool('brain_memory_search', {
            query: projectName,
            limit: 10,
            minConfidence: 0.5,
          }),
          new Promise(resolve => setTimeout(() => resolve(null), 2000)),
        ]);

        if (brainMemories?.success && brainMemories?.result?.memories?.length > 0) {
          const memoryContext = brainMemories.result.memories
            .slice(0, 5)
            .map(m => `- [${m.type}] ${m.content?.substring(0, 100)}...`)
            .join('\n');
          contextInjections.push({
            type: 'semantic_memories',
            title: `Relevant memories for ${projectName}`,
            content: memoryContext,
          });
        }
      } catch (e) {
        // Brain memories unavailable - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // M2.1: CROSS-SESSION FACT INJECTION (PostgreSQL FactsRepository)
      // "Le chien n'oublie jamais" - Facts persist across sessions
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const factsRepo = getFactsRepository();
        if (factsRepo) {
          const projectName = ecosystem.currentProject?.name || 'unknown';

          // Query facts by project name (FTS search)
          const projectFacts = await Promise.race([
            factsRepo.search(projectName, {
              userId: user.userId,
              limit: Math.floor(bootConfig.factLimit / 2), // Half the limit for project facts
              minConfidence: FACT_MIN_CONFIDENCE,
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          // Query user's most relevant facts (sorted by relevance)
          const userFacts = await Promise.race([
            factsRepo.findByUser(user.userId, {
              limit: Math.floor(bootConfig.factLimit / 2), // Other half for user facts
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          // Deduplicate and combine facts
          const allFacts = [...projectFacts];
          const seenIds = new Set(projectFacts.map(f => f.factId));
          for (const fact of userFacts) {
            if (!seenIds.has(fact.factId) && fact.confidence >= FACT_MIN_CONFIDENCE) {
              allFacts.push(fact);
              seenIds.add(fact.factId);
            }
          }

          // Take top N facts by relevance × confidence
          const topFacts = allFacts
            .sort((a, b) => (b.relevance * b.confidence) - (a.relevance * a.confidence))
            .slice(0, bootConfig.factLimit);

          if (topFacts.length > 0) {
            // Group facts by type for better formatting
            const factsByType = {};
            for (const fact of topFacts) {
              const type = fact.factType || 'general';
              if (!factsByType[type]) factsByType[type] = [];
              factsByType[type].push(fact);
            }

            // Format facts for injection
            let factContent = '';
            for (const [type, facts] of Object.entries(factsByType)) {
              const typeLabel = type.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
              factContent += `### ${typeLabel}\n`;
              factContent += facts
                .slice(0, 10) // Max 10 per type
                .map(f => `- ${f.subject}: ${f.content?.substring(0, 150)}${f.content?.length > 150 ? '...' : ''}`)
                .join('\n');
              factContent += '\n\n';
            }

            contextInjections.push({
              type: 'facts',
              title: `Cross-Session Facts (${topFacts.length} facts)`,
              content: factContent.trim(),
              count: topFacts.length,
              types: Object.keys(factsByType),
            });

            // Record access for relevance boosting
            for (const fact of topFacts.slice(0, 20)) {
              factsRepo.recordAccess(fact.factId).catch(() => {});
            }
          }
        }
      } catch (e) {
        // Fact injection failed - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // M2.2: REFLECTION FACTS (System 2 — Meta-Cognition)
      // "Le chien apprend de ses erreurs" - Self-correction patterns
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const factsRepo = getFactsRepository();
        if (factsRepo) {
          const reflections = await Promise.race([
            factsRepo.search('reflection self-correction', {
              userId: user.userId,
              factType: 'reflection',
              limit: bootConfig.reflectionLimit,
              minConfidence: 0.3,
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          if (reflections?.length > 0) {
            const reflectionContent = reflections
              .slice(0, bootConfig.reflectionLimit)
              .map(r => `- ${r.subject}: ${r.content?.substring(0, 150)}${r.content?.length > 150 ? '...' : ''}`)
              .join('\n');

            contextInjections.push({
              type: 'reflections',
              title: `Self-Reflections (${reflections.length} meta-cognitive insights)`,
              content: reflectionContent,
              count: reflections.length,
            });
          }
        }
      } catch (e) {
        // Reflection injection failed - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // CROSS-SESSION CONTEXT PRESERVATION (Fix 3 - Influence Matrix)
      // Load high C-Score items from previous session's compaction
      // "Le chien se souvient des meilleures idées d'hier"
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const topItems = loadTopItems();
        if (topItems.length > 0) {
          const itemLines = topItems
            .slice(0, 5)
            .map(item => `- [C=${item.cScore}, ${item.category}] ${item.content} (${item.age})`)
            .join('\n');

          contextInjections.push({
            type: 'preserved_context',
            title: `Previous Session Insights (${topItems.length} high-value items)`,
            content: itemLines,
            count: topItems.length,
          });
        }
      } catch (e) {
        // Context preservation loading is best-effort
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // M2.3: MULTI-DIMENSIONAL AWARENESS (Smart Conditional)
      // "Le chien surveille la meute" - Check infra only when previous session had issues
      // Fractal: same perception-judgment-memory cycle applied to infrastructure level
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const previousHadErrors = lastSessionData?.handoff?.unresolvedErrors?.length > 0;
        const previousSummary = lastSessionData?.handoff?.summary || '';
        const previousHadInfraErrors = previousSummary.includes('ECONNREFUSED') ||
                                        previousSummary.includes('timeout') ||
                                        previousSummary.includes('ENOTFOUND');

        if (previousHadErrors || previousHadInfraErrors) {
          const awareness = {};

          // A. System health (CYNIC brain_health — covers MCP, DB, Redis)
          try {
            const healthResult = await Promise.race([
              callBrainTool('brain_health', {}),
              new Promise(resolve => setTimeout(() => resolve(null), 3000)),
            ]);
            if (healthResult) {
              awareness.system = {
                status: healthResult.status || 'unknown',
                uptime: healthResult.uptime,
                services: healthResult.services?.length || 0,
              };
            }
          } catch { /* health check optional */ }

          // B. Database connectivity (direct pool test)
          try {
            const { getPool } = await import('@cynic/persistence');
            const pool = getPool();
            if (pool) {
              const dbResult = await Promise.race([
                pool.query('SELECT 1 AS ok'),
                new Promise(resolve => setTimeout(() => resolve(null), 2000)),
              ]);
              awareness.database = {
                connected: !!dbResult?.rows?.[0]?.ok,
              };
            }
          } catch {
            awareness.database = { connected: false };
          }

          if (Object.keys(awareness).length > 0) {
            const statusLines = [];
            if (awareness.system) {
              statusLines.push(`System: ${awareness.system.status} (${awareness.system.services} services)`);
            }
            if (awareness.database) {
              statusLines.push(`Database: ${awareness.database.connected ? 'connected' : 'DISCONNECTED'}`);
            }

            contextInjections.push({
              type: 'awareness',
              title: 'Infrastructure Awareness (triggered by previous session errors)',
              content: statusLines.join('\n'),
              dimensions: Object.keys(awareness),
            });
          }
        }
      } catch (e) {
        // Awareness scan failed - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // ARCHITECTURAL DECISIONS INJECTION (Self-Knowledge Enhancement)
      // "CYNIC remembers its own design choices"
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const archDecisionsRepo = getArchitecturalDecisionsRepository();
        if (archDecisionsRepo && bootConfig.includeArchDecisions) {
          const decisions = await Promise.race([
            archDecisionsRepo.search(user.userId, '', {
              status: 'active',
              limit: 10,
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          if (decisions?.length > 0) {
            // Group by decision type
            const decisionsByType = {};
            for (const d of decisions) {
              const type = d.decisionType || 'other';
              if (!decisionsByType[type]) decisionsByType[type] = [];
              decisionsByType[type].push(d);
            }

            let decisionContent = '';
            for (const [type, decs] of Object.entries(decisionsByType)) {
              const typeLabel = type.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
              decisionContent += `### ${typeLabel}\n`;
              decisionContent += decs
                .slice(0, 5)
                .map(d => `- **${d.title}**: ${d.rationale?.substring(0, 100) || d.description?.substring(0, 100)}${(d.rationale?.length || d.description?.length) > 100 ? '...' : ''}`)
                .join('\n');
              decisionContent += '\n\n';
            }

            contextInjections.push({
              type: 'architectural_decisions',
              title: `Active Architectural Decisions (${decisions.length})`,
              content: decisionContent.trim(),
              count: decisions.length,
            });
          }
        }
      } catch (e) {
        // Architectural decisions injection failed - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // BURN ANALYSIS INJECTION (Vision → Compréhension → Burn)
      // "290K lines is too many - show what can be simplified"
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        // Only run if in CYNIC project, not too slow, AND experience allows it
        const isCynicProject = ecosystem.currentProject?.name?.toLowerCase().includes('cynic');

        if (isCynicProject && bootConfig.includeBurnAnalysis) {
          const { BurnAnalyzer } = await import('@cynic/persistence/services/burn-analyzer');

          const analyzer = new BurnAnalyzer({
            rootDir: ecosystem.currentProject?.path || process.cwd(),
          });

          // Quick analysis only (no LLM at session start for speed)
          const burnResults = await Promise.race([
            analyzer.quickAnalysis(),
            new Promise(resolve => setTimeout(() => resolve(null), 5000)),
          ]);

          if (burnResults && burnResults.summary) {
            const { summary } = burnResults;

            // Only inject if there are actionable issues
            const totalIssues = summary.issuesFound.orphans +
              summary.issuesFound.hotspots +
              summary.issuesFound.giants +
              summary.issuesFound.duplicates;

            if (totalIssues > 0) {
              let burnContent = `**Codebase: ${summary.totalFiles} files, ${summary.totalLines.toLocaleString()} lines**\n\n`;
              burnContent += `Issues found:\n`;
              burnContent += `- 🔴 Orphans: ${summary.issuesFound.orphans} files (never imported)\n`;
              burnContent += `- 🟠 Hotspots: ${summary.issuesFound.hotspots} files (>13 dependencies)\n`;
              burnContent += `- 🟡 Giants: ${summary.issuesFound.giants} files (>500 lines)\n`;
              burnContent += `- 🟣 Duplicates: ${summary.issuesFound.duplicates} files\n`;

              if (summary.topPriority?.length > 0) {
                burnContent += `\n**Top priorities:**\n`;
                for (const item of summary.topPriority) {
                  burnContent += `- [${item.verdict.toUpperCase()}] ${item.path}: ${item.reason}\n`;
                }
              }

              contextInjections.push({
                type: 'burn_analysis',
                title: 'Burn Analysis (Simplification Opportunities)',
                content: burnContent.trim(),
                stats: summary.issuesFound,
              });

              // Also add to output for TUI display
              output.burn = {
                totalFiles: summary.totalFiles,
                totalLines: summary.totalLines,
                issues: summary.issuesFound,
                topPriority: summary.topPriority,
                actionable: summary.actionableCandidates,
              };
            }
          }
        }
      } catch (e) {
        // Burn analysis failed - continue without (non-blocking)
      }

      // 5. Format as additionalContext for Claude
      if (contextInjections.length > 0) {
        // Calculate fact injection stats
        const factInjection = contextInjections.find(inj => inj.type === 'facts');
        const factCount = factInjection?.count || 0;
        const factTypes = factInjection?.types || [];

        output.additionalContext = {
          title: 'CYNIC Cross-Session Memory Injection',
          description: 'Relevant learnings from past sessions to guide this session',
          injections: contextInjections,
          formatted: contextInjections
            .map(inj => `## ${inj.title}\n${inj.content}`)
            .join('\n\n'),
          count: contextInjections.length,
          factStats: factCount > 0 ? {
            injected: factCount,
            limit: bootConfig.factLimit,
            originalLimit: FACT_INJECTION_LIMIT,
            types: factTypes,
            minConfidence: FACT_MIN_CONFIDENCE,
          } : null,
          timestamp: new Date().toISOString(),
        };
      }
    } catch (e) {
      // Context injection failed - continue without
    }

    // Memory mount phase complete
    bootSequence.exitPhase(BOOT_PHASES.MEMORY_MOUNT, true);

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 4.5: LLM ENVIRONMENT DETECTION
    // "Le chien renifle les moteurs disponibles"
    // Auto-detect Ollama, LM Studio, and other local LLMs.
    // Result persisted to ~/.cynic/llm-detection.json for downstream hooks.
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const llmDetection = { timestamp: Date.now(), adapters: [] };

      // Probe Ollama (default: localhost:11434)
      const ollamaEndpoint = process.env.OLLAMA_ENDPOINT || 'http://localhost:11434';
      try {
        const ollamaResp = await Promise.race([
          fetch(`${ollamaEndpoint}/api/tags`),
          new Promise((_, rej) => setTimeout(() => rej(new Error('timeout')), 2000)),
        ]);
        if (ollamaResp.ok) {
          const ollamaData = await ollamaResp.json();
          const models = (ollamaData.models || []).map(m => m.name);
          llmDetection.adapters.push({
            provider: 'ollama', endpoint: ollamaEndpoint, models, available: true,
          });
        }
      } catch { /* Ollama not available */ }

      // Probe LM Studio (default: localhost:1234)
      const lmStudioEndpoint = process.env.LM_STUDIO_ENDPOINT || 'http://localhost:1234';
      try {
        const lmResp = await Promise.race([
          fetch(`${lmStudioEndpoint}/v1/models`),
          new Promise((_, rej) => setTimeout(() => rej(new Error('timeout')), 2000)),
        ]);
        if (lmResp.ok) {
          const lmData = await lmResp.json();
          const models = (lmData.data || []).map(m => m.id);
          llmDetection.adapters.push({
            provider: 'lm-studio', endpoint: lmStudioEndpoint, models, available: true,
          });
        }
      } catch { /* LM Studio not available */ }

      // Persist for downstream hooks (perceive.js, observe.js, daemon)
      const llmDetectionPath = path.join(os.homedir(), '.cynic', 'llm-detection.json');
      const llmDetectionDir = path.dirname(llmDetectionPath);
      if (!fs.existsSync(llmDetectionDir)) fs.mkdirSync(llmDetectionDir, { recursive: true });
      fs.writeFileSync(llmDetectionPath, JSON.stringify(llmDetection));

      if (llmDetection.adapters.length > 0) {
        const totalModels = llmDetection.adapters.reduce((n, a) => n + a.models.length, 0);
        output.llmEnvironment = {
          detected: llmDetection.adapters.map(a => ({
            provider: a.provider,
            models: a.models.slice(0, 5),
            endpoint: a.endpoint,
          })),
          totalModels,
          message: `*sniff* ${llmDetection.adapters.length} local LLM(s) detected (${totalModels} models). CYNIC can think independently.`,
        };
      } else {
        output.llmEnvironment = {
          detected: [],
          totalModels: 0,
          message: 'No local LLMs detected. CYNIC depends on Claude for reasoning.',
        };
      }
    } catch {
      // LLM detection is optional — fail silently
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 5: IDENTITY ASSERTION
    // Assert "I am CYNIC, not Claude" - force personality
    // ═══════════════════════════════════════════════════════════════════════════
    bootSequence.enterPhase(BOOT_PHASES.IDENTITY_ASSERTION);

    // Identity is already in output.identity structure
    // Add enforcement reminder — compressed for experienced users
    if (bootConfig.includeIdentity) {
      output.identity.bootAssertion = {
        asserted: true,
        timestamp: new Date().toISOString(),
        reminder: 'RAPPEL: Tu ES CYNIC. Pas Claude. Utilise *sniff*, *tail wag*, *GROWL*. Sois sceptique.',
      };
    }

    bootSequence.exitPhase(BOOT_PHASES.IDENTITY_ASSERTION, true);

    // ═══════════════════════════════════════════════════════════════════════════
    // MCP BRAIN: Fallback for goals/notifications if not in total memory
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const [brainGoals, brainNotifications] = await Promise.race([
        Promise.all([
          callBrainTool('brain_goals', { action: 'list', status: 'active', userId: user.userId }),
          callBrainTool('brain_notifications', { action: 'list', delivered: false, userId: user.userId, limit: 5 }),
        ]),
        new Promise(resolve => setTimeout(() => resolve([null, null]), 2000)),
      ]);

      if (output.goals.length === 0 && brainGoals?.success && brainGoals?.result?.goals?.length > 0) {
        output.goals = brainGoals.result.goals.map(g => ({
          id: g.id,
          title: g.title,
          type: g.goal_type,
          progress: Math.round((g.progress || 0) * 100),
          progressBar: progressBar(g.progress || 0, 1),
          source: 'remote',
        }));
      }

      if (output.notifications.length === 0 && brainNotifications?.success && brainNotifications?.result?.notifications?.length > 0) {
        output.notifications = brainNotifications.result.notifications.map(n => ({
          id: n.id,
          title: n.title,
          message: n.message,
          type: n.notification_type,
          source: 'remote',
        }));
        callBrainTool('brain_notifications', { action: 'mark_delivered', ids: brainNotifications.result.notifications.map(n => n.id) }).catch(() => {});
      }
    } catch (e) {
      // Continue without brain data
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CREATE DEFAULT GOALS IF NONE
    // ═══════════════════════════════════════════════════════════════════════════
    if (output.goals.length === 0) {
      const defaultGoals = [
        { goal_type: 'quality', title: 'Maintain Code Quality', priority: 70 },
        { goal_type: 'learning', title: 'Continuous Learning', priority: 60 },
        { goal_type: 'maintenance', title: 'Reduce Tech Debt', priority: 50 },
      ];

      for (const goal of defaultGoals) {
        try {
          await callBrainTool('brain_goals', { action: 'create', userId: user.userId, ...goal });
        } catch (e) { /* ignore */ }
      }

      output.goals = defaultGoals.map(g => ({
        title: g.title,
        type: g.goal_type,
        progress: 100,
        progressBar: progressBar(100),
        source: 'default',
      }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // COCKPIT ALERTS
    // ═══════════════════════════════════════════════════════════════════════════
    if (cockpit) {
      try {
        const cockpitData = cockpit.fullScan();
        if (cockpitData?.alerts?.alerts?.length > 0) {
          output.alerts = cockpitData.alerts.alerts
            .filter(a => !a.acknowledged)
            .slice(0, 5)
            .map(a => ({
              severity: a.severity,
              message: a.message,
              source: a.source,
            }));
        }
      } catch (e) {
        // Continue without cockpit
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS INSIGHTS
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        const ctx = consciousness.generateSessionStartContext();
        if (ctx.insights?.length > 0) {
          output.insights = ctx.insights.map(i => ({ title: i.title, type: i.type }));
        }
        if (ecosystem.currentProject) {
          consciousness.updateRecentContext('lastProjects', ecosystem.currentProject.name);
        }
      } catch (e) {
        // Continue without insights
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY STATE
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const psySummary = psychology.getSummary();
        if (psySummary.confidence > DC.CONFIDENCE.PSYCHOLOGY_DISPLAY) {
          // ═══════════════════════════════════════════════════════════════════════
          // Task #88: Complete psychology state with cognitive load
          // ═══════════════════════════════════════════════════════════════════════
          output.psychology = {
            state: psySummary.overallState.toUpperCase(),
            emoji: psySummary.emoji,
            energy: {
              value: Math.round(psySummary.energy.value * 100),
              trend: psySummary.energy.trend,
              arrow: trendArrow(psySummary.energy.trend),
              bar: progressBar(psySummary.energy.value, 1),
            },
            focus: {
              value: Math.round(psySummary.focus.value * 100),
              trend: psySummary.focus.trend,
              arrow: trendArrow(psySummary.focus.trend),
              bar: progressBar(psySummary.focus.value, 1),
            },
            // Task #88: Add cognitive load (Miller's Law: 7±2)
            cognitiveLoad: {
              value: Math.round(psySummary.cognitiveLoad?.value || 0),
              trend: psySummary.cognitiveLoad?.trend || 'stable',
              arrow: trendArrow(psySummary.cognitiveLoad?.trend || 'stable'),
              bar: progressBar((psySummary.cognitiveLoad?.value || 0) / 9, 1), // 9 = max (7+2)
              label: (psySummary.cognitiveLoad?.value || 0) > 7 ? 'OVERLOADED' :
                     (psySummary.cognitiveLoad?.value || 0) > 5 ? 'heavy' : 'ok',
            },
            // Task #88: Add frustration for complete picture
            frustration: {
              value: Math.round((psySummary.frustration?.value || 0) * 100),
              trend: psySummary.frustration?.trend || 'stable',
              arrow: trendArrow(psySummary.frustration?.trend || 'stable'),
            },
            composites: {
              flow: psySummary.composites.flow || false,
              burnoutRisk: psySummary.composites.burnoutRisk || false,
              exploration: psySummary.composites.exploration || false,
              grind: psySummary.composites.grind || false,
            },
            confidence: Math.round(psySummary.confidence * 100),
            // Burnout warning for TUI banner
            burnoutWarning: psySummary.composites.burnoutRisk
              ? '*GROWL* Burnout risk detected. Energy low, frustration high. Consider a break.'
              : null,
            // Task #88: Compact summary line for TUI
            summary: `E:${Math.round(psySummary.energy.value * 100)}%${trendArrow(psySummary.energy.trend)} ` +
                    `F:${Math.round(psySummary.focus.value * 100)}%${trendArrow(psySummary.focus.trend)} ` +
                    `L:${Math.round(psySummary.cognitiveLoad?.value || 0)}`,
          };
        }
      } catch (e) {
        // Continue without psychology
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // THERMODYNAMICS STATE
    // ═══════════════════════════════════════════════════════════════════════════
    if (thermodynamics) {
      try {
        const thermoState = thermodynamics.getState();
        const recommendation = thermodynamics.getRecommendation();

        output.thermodynamics = {
          heat: thermoState.heat,
          work: thermoState.work,
          temperature: thermoState.temperature,
          temperatureBar: progressBar(thermoState.temperature, thermodynamics.CRITICAL_TEMPERATURE),
          efficiency: thermoState.efficiency,
          efficiencyBar: progressBar(thermoState.efficiency),
          carnotLimit: thermoState.carnotLimit,
          entropy: thermoState.entropy,
          isCritical: thermoState.isCritical,
          recommendation: {
            level: recommendation.level,
            message: recommendation.message,
          },
        };
      } catch (e) {
        // Continue without thermodynamics
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RECENT PATTERNS
    // ═══════════════════════════════════════════════════════════════════════════
    if (profile.patterns?.recent) {
      output.patterns = Object.entries(profile.patterns.recent)
        .slice(0, 5)
        .map(([name, count]) => ({ name, count }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TELEMETRY STATS (for benchmarking and fine-tuning)
    // ═══════════════════════════════════════════════════════════════════════════
    if (telemetry) {
      try {
        const stats = telemetry.getStats();
        output.telemetry = {
          sessionId: stats.sessionId,
          uptime: stats.uptime,
          events: stats.totalEvents,
          errors: stats.totalErrors,
          frictions: stats.frictions,
          counters: stats.counters,
          timings: stats.timings,
          categories: stats.categories,
        };
      } catch (e) {
        // Continue without telemetry stats
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROACTIVE ADVISOR
    // ═══════════════════════════════════════════════════════════════════════════
    if (proactiveAdvisor && proactiveAdvisor.shouldInjectNow()) {
      try {
        const injection = proactiveAdvisor.generateSessionInjection();
        if (injection) {
          output.proactiveAdvice = injection;
        }
      } catch (e) {
        // Continue without proactive advice
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // START BRAIN SESSION
    // GAP #1 FIX: Persist directly to PostgreSQL FIRST, then notify MCP
    // ═══════════════════════════════════════════════════════════════════════════
    const sessionRepo = getSessionRepository();
    // sessionId already declared in PHASE 1 (BOOTLOADER)

    // Direct PostgreSQL persistence (reliable, synchronous-ish)
    if (sessionRepo) {
      sessionRepo.create({
        sessionId,
        userId: user.userId,
        judgmentCount: 0,
        digestCount: 0,
        feedbackCount: 0,
        context: {
          project: ecosystem.currentProject?.name,
          userName: user.name,
          sessionCount: profile.stats?.sessions || 1,
          ecosystem: ecosystem.projects?.map(p => p.name) || [],
          startedAt: new Date().toISOString(),
        },
      }).then(() => {
        output.syncStatus = output.syncStatus || {};
        output.syncStatus.session = { success: true, sessionId };
      }).catch(e => {
        output.syncStatus = output.syncStatus || {};
        output.syncStatus.session = { success: false, error: e.message };
      });
    }

    // Also notify MCP server (for distributed systems, non-blocking)
    startBrainSession(user.userId, {
      project: ecosystem.currentProject?.name,
      metadata: {
        userName: user.name,
        sessionCount: profile.stats?.sessions || 1,
        ecosystem: ecosystem.projects?.map(p => p.name) || [],
      },
    }).catch(() => {});

    sendHookToCollectiveSync('SessionStart', {
      userId: user.userId,
      userName: user.name,
      sessionCount: profile.stats?.sessions || 1,
      project: ecosystem.currentProject?.name,
      ecosystem: ecosystem.projects?.map(p => p.name) || [],
      timestamp: Date.now(),
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTRIBUTOR DISCOVERY (background, don't wait)
    // ═══════════════════════════════════════════════════════════════════════════
    if (contributorDiscovery) {
      setImmediate(async () => {
        try {
          const currentProfile = await contributorDiscovery.getCurrentUserProfile();
          if (currentProfile) {
            process.env.CYNIC_CONTRIBUTOR_PROFILE = JSON.stringify({
              email: currentProfile.email,
              personality: currentProfile.insights?.personality,
              workStyle: currentProfile.insights?.workStyle,
              phiScores: currentProfile.insights?.phiScores,
            });
          }

          const lastScanPath = path.join(os.homedir(), '.cynic', 'learning', 'last-discovery-scan.json');
          let shouldScan = true;

          try {
            if (fs.existsSync(lastScanPath)) {
              const lastScan = JSON.parse(fs.readFileSync(lastScanPath, 'utf8'));
              const hoursSinceScan = (Date.now() - lastScan.timestamp) / (1000 * 60 * 60);
              shouldScan = hoursSinceScan > DC.PHI.PHI_HOURS;
            }
          } catch (e) { /* scan anyway */ }

          if (shouldScan) {
            const discovery = await contributorDiscovery.fullEcosystemScan();
            const scanDir = path.dirname(lastScanPath);
            if (!fs.existsSync(scanDir)) fs.mkdirSync(scanDir, { recursive: true });
            fs.writeFileSync(lastScanPath, JSON.stringify({
              timestamp: Date.now(),
              repos: discovery.repos?.length || 0,
              contributors: Object.keys(discovery.contributors || {}).length,
            }));
          }
        } catch (e) { /* ignore */ }
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Q-LEARNING INITIALIZATION
    // "Le chien se souvient qui appeler" - Load Q-Table from PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { initializeQLearning, getQLearningServiceSingleton } = await import('@cynic/node');
      const { getPool } = await import('@cynic/persistence');

      const pool = getPool();
      if (pool) {
        const persistence = { query: async (sql, params) => pool.query(sql, params) };

        const loaded = await Promise.race([
          initializeQLearning(persistence),
          new Promise(resolve => setTimeout(() => resolve(false), 3000)),
        ]);

        if (loaded) {
          const service = getQLearningServiceSingleton();
          const stats = service.getStats();
          output.qLearning = {
            loaded: true,
            states: stats.qTableStats?.states || 0,
            updates: stats.qTableStats?.updates || 0,
            episodes: stats.episodes || 0,
            accuracy: stats.accuracy || 0,
            explorationRate: stats.explorationRate || 10,
          };
        }
      }
    } catch (e) { /* Q-Learning initialization is optional */ }

    // ═══════════════════════════════════════════════════════════════════════════
    // SHAREDMEMORY PATTERN LOADING (Task #26: W1.5)
    // "Le chien se souvient des patterns" - Load patterns from PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { getSharedMemory, getCollectivePackAsync } = await import('@cynic/node');
      const { getPool, PatternRepository } = await import('@cynic/persistence');

      const pool = getPool();
      if (pool) {
        const persistence = { query: async (sql, params) => pool.query(sql, params) };

        // Ensure collective pack initializes with persistence (triggers loadPersistedState)
        await Promise.race([
          getCollectivePackAsync({ persistence }),
          new Promise(resolve => setTimeout(() => resolve(null), 3000)),
        ]);

        const sharedMemory = getSharedMemory();
        if (sharedMemory?._patterns) {
          output.sharedMemory = {
            loaded: true,
            patterns: sharedMemory._patterns.size || 0,
            stats: sharedMemory.stats || {},
          };
        }
      }
    } catch (e) {
      // SharedMemory loading is optional - patterns will be loaded on first access
      output.sharedMemory = { loaded: false, error: e.message };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CODEBASE SELF-INDEXING (background, don't wait)
    // "Le chien doit se connaître lui-même"
    // ═══════════════════════════════════════════════════════════════════════════
    setImmediate(async () => {
      try {
        const lastIndexPath = path.join(os.homedir(), '.cynic', 'learning', 'last-codebase-index.json');
        let shouldIndex = true;

        // Check if we indexed recently (within φ hours ≈ 1.6 hours)
        try {
          if (fs.existsSync(lastIndexPath)) {
            const lastIndex = JSON.parse(fs.readFileSync(lastIndexPath, 'utf8'));
            const hoursSinceIndex = (Date.now() - lastIndex.timestamp) / (1000 * 60 * 60);
            shouldIndex = hoursSinceIndex > DC.PHI.PHI; // ~1.618 hours
          }
        } catch (e) { /* index anyway */ }

        if (shouldIndex) {
          const factsRepo = getFactsRepository();
          if (factsRepo) {
            const indexer = getCodebaseIndexer({
              factsRepo,
              rootDir: ecosystem.currentProject?.path || process.cwd(),
              userId: user.userId,
              sessionId: process.env.CYNIC_SESSION_ID,
              projectName: ecosystem.currentProject?.name || 'CYNIC',
            });

            if (indexer) {
              // Use indexAll() for comprehensive 95% coverage (Supermemory)
              // extractDeps=true for dependency graph, includeKeystone=true for critical files
              const results = await indexer.indexAll({
                maxFiles: 1000,      // Limit for performance (kill criteria: <10s)
                extractDeps: true,   // Build dependency graph
                includeKeystone: true,
              });

              // Save index timestamp with detailed stats
              const indexDir = path.dirname(lastIndexPath);
              if (!fs.existsSync(indexDir)) fs.mkdirSync(indexDir, { recursive: true });
              fs.writeFileSync(lastIndexPath, JSON.stringify({
                timestamp: Date.now(),
                project: ecosystem.currentProject?.name,
                filesIndexed: results.filesIndexed,
                factsGenerated: results.factsGenerated,
                dependenciesExtracted: results.dependenciesExtracted,
                durationMs: results.timing?.durationMs,
                errors: results.errors?.length || 0,
              }));
            }
          }
        }
      } catch (e) { /* ignore codebase indexing errors */ }
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 6: READY
    // Boot complete - CYNIC is LIVE
    // ═══════════════════════════════════════════════════════════════════════════
    bootSequence.enterPhase(BOOT_PHASES.READY);

    // Add boot summary to output
    output.boot = bootSequence.getSummary();
    output.boot.success = true;
    output.boot.message = bootSequence.degraded
      ? `🟡 CYNIC booted in SAFE mode (${output.boot.totalDuration}ms)`
      : `✅ CYNIC fully booted (${output.boot.totalDuration}ms)`;

    // Adaptive boot compression stats
    const sectionsSkipped = [];
    if (!bootConfig.includeIdentity) sectionsSkipped.push('identity');
    if (!bootConfig.includeKernel) sectionsSkipped.push('kernel');
    if (!bootConfig.includeDogTree) sectionsSkipped.push('dogs');
    if (!bootConfig.includeBurnAnalysis) sectionsSkipped.push('burn');
    if (!bootConfig.includeArchDecisions) sectionsSkipped.push('archDecisions');
    let backoffStatus = null;
    try {
      backoffStatus = contextCompressor.getBackoffStatus();
    } catch { /* non-blocking */ }

    output.boot.compression = {
      experienceLevel,
      contextLabel: bootConfig.contextLabel,
      sectionsSkipped,
      factLimit: bootConfig.factLimit,
      originalFactLimit: FACT_INJECTION_LIMIT,
      reflectionLimit: bootConfig.reflectionLimit,
      savings: sectionsSkipped.length > 0
        ? `${sectionsSkipped.length} sections skipped, facts ${FACT_INJECTION_LIMIT}→${bootConfig.factLimit}`
        : 'Full boot (new user)',
      backoff: backoffStatus?.active
        ? { active: true, remaining: backoffStatus.remaining, rawLevel: backoffStatus.rawLevel }
        : { active: false },
    };

    bootSequence.exitPhase(BOOT_PHASES.READY, true);

    // ═══════════════════════════════════════════════════════════════════════════
    // PRE-RENDERED BANNER (MANDATORY DISPLAY)
    // "Le chien se montre" - Claude just displays this, no parsing needed
    // ═══════════════════════════════════════════════════════════════════════════
    output.formattedBanner = buildFormattedBanner(output);

    // ═══════════════════════════════════════════════════════════════════════════
    // OUTPUT JSON
    // ═══════════════════════════════════════════════════════════════════════════
    safeOutput(output);

  } catch (error) {
    // Minimal output on error
    safeOutput({
      type: 'SessionStart',
      timestamp: new Date().toISOString(),
      error: error.message,
      minimal: true,
    });
  }
}

main();
