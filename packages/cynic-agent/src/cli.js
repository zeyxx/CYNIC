#!/usr/bin/env node
/**
 * @cynic/agent - CLI Entry Point
 *
 * Rich TUI for the autonomous Solana AI agent.
 * "Lache le chien!" - kynikos
 *
 * @module @cynic/agent/cli
 */

'use strict';

import 'dotenv/config';
import { createAgent, AgentState } from './index.js';
import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';

const log = createLogger('CLI');

// ═══════════════════════════════════════════════════════════════════════════════
// ANSI Colors
// ═══════════════════════════════════════════════════════════════════════════════

const C = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  cyan: '\x1b[36m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  magenta: '\x1b[35m',
  white: '\x1b[37m',
  gray: '\x1b[90m',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Parse Args
// ═══════════════════════════════════════════════════════════════════════════════

function parseArgs(argv) {
  const args = argv.slice(2);
  const opts = {
    live: false,
    devnet: false,
    name: 'cynic-agent-0',
    wallet: null,
    tokens: [],
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--live') opts.live = true;
    else if (arg === '--devnet') opts.devnet = true;
    else if (arg.startsWith('--name=')) opts.name = arg.split('=')[1];
    else if (arg === '--wallet' && args[i + 1]) opts.wallet = args[++i];
    else if (arg.startsWith('--wallet=')) opts.wallet = arg.split('=')[1];
    else if (arg === '--tokens' && args[i + 1]) opts.tokens = args[++i].split(',');
    else if (arg.startsWith('--tokens=')) opts.tokens = arg.split('=')[1].split(',');
    else if (arg === '--help' || arg === '-h') {
      printHelp();
      process.exit(0);
    }
  }

  return opts;
}

function printHelp() {
  console.log(`
${C.cyan}CYNIC Agent${C.reset} - Autonomous Solana AI Agent

${C.bold}Usage:${C.reset}
  node cli.js [options]

${C.bold}Options:${C.reset}
  --live              Enable live trading (default: dry run)
  --devnet            Use Solana devnet
  --name=<name>       Agent name (default: cynic-agent-0)
  --wallet <path>     Path to keypair JSON file
  --tokens <mints>    Comma-separated token mints to watch
  --help, -h          Show this help

${C.bold}Environment:${C.reset}
  SOLANA_RPC_URL      Custom RPC endpoint
  AGENT_KEYPAIR       Keypair as JSON array or base58
  AGENT_KEYPAIR_PATH  Path to keypair file
  DRY_RUN             Set to 'false' for live mode
  GASDF_RELAYER_URL   GASdf relayer endpoint
`);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Formatting Helpers
// ═══════════════════════════════════════════════════════════════════════════════

function ts() {
  const now = new Date();
  return `${C.gray}[${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}]${C.reset}`;
}

function progressBar(value, max = 1, width = 10) {
  const ratio = Math.min(value / max, 1);
  const filled = Math.round(ratio * width);
  const empty = width - filled;
  return `[${'█'.repeat(filled)}${'░'.repeat(empty)}]`;
}

function colorByPhi(value) {
  if (value >= PHI_INV) return C.green;
  if (value >= PHI_INV_2) return C.yellow;
  return C.red;
}

function pnlColor(pnl) {
  if (pnl > 0) return C.green;
  if (pnl < 0) return C.red;
  return C.gray;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════════════════

async function main() {
  const opts = parseArgs(process.argv);

  const dryRun = !opts.live;
  const cluster = opts.devnet ? 'devnet' : (process.env.SOLANA_CLUSTER || 'mainnet-beta');

  // Apply wallet path to env
  if (opts.wallet) {
    process.env.AGENT_KEYPAIR_PATH = opts.wallet;
  }
  if (opts.devnet) {
    process.env.SOLANA_CLUSTER = 'devnet';
  }

  // Create agent
  const agent = createAgent({
    name: opts.name,
    executor: { dryRun, cluster },
  });

  // Add extra tokens
  if (opts.tokens.length > 0) {
    for (const mint of opts.tokens) {
      agent.perceiver.addWatchedToken(mint.trim());
    }
  }

  // ═════════════════════════════════════════════════════════════════════════
  // Startup Banner
  // ═════════════════════════════════════════════════════════════════════════

  console.log('');
  console.log(`${C.cyan}═══════════════════════════════════════════════════════════${C.reset}`);
  console.log(`${C.cyan}  CYNIC AGENT${C.reset} - ${C.dim}"Le chien qui pense, juge, et agit"${C.reset}`);
  console.log(`${C.cyan}═══════════════════════════════════════════════════════════${C.reset}`);

  // Show config once agent is started (to see if CYNICJudge wired)
  const showConfig = () => {
    const deciderStatus = agent.decider.getStatus();
    const executorStatus = agent.executor.getStatus();
    const watchedCount = agent.perceiver.config.watchedTokens?.length || 0;
    const modeStr = dryRun
      ? `${C.yellow}DRY RUN${C.reset}`
      : `${C.red}${C.bold}LIVE${C.reset}`;
    const brainStr = deciderStatus.cynicJudge
      ? `${C.green}25D CYNICJudge${C.reset}`
      : `${C.yellow}basic scoring${C.reset}`;
    const guardianStr = agent.guardian
      ? `${C.green}active${C.reset}`
      : `${C.gray}unavailable${C.reset}`;
    const keypairStr = executorStatus.hasKeypair
      ? `${C.green}loaded${C.reset}`
      : `${C.gray}none${C.reset}`;

    console.log(`  Mode: ${modeStr} | Cluster: ${C.white}${cluster}${C.reset} | Tokens: ${C.white}${watchedCount}${C.reset}`);
    console.log(`  Max conf: ${C.cyan}61.8%${C.reset} (phi) | Min to act: ${C.cyan}38.2%${C.reset} (phi^2)`);
    console.log(`  Guardian: ${guardianStr} | Brain: ${brainStr}`);
    console.log(`  Keypair: ${keypairStr} | Wallet: ${C.dim}${executorStatus.walletAddress || 'none'}${C.reset}`);
    console.log(`${C.cyan}═══════════════════════════════════════════════════════════${C.reset}`);
    console.log('');
  };

  // ═════════════════════════════════════════════════════════════════════════
  // Event Handlers (compact, timestamped)
  // ═════════════════════════════════════════════════════════════════════════

  agent.on('opportunity', (opp) => {
    const stats = opp.signal?.statistics || {};
    const zStr = stats.zScore !== undefined ? ` z=${stats.zScore.toFixed(1)}` : '';
    const anomStr = stats.isRateAnomaly ? ` ${C.red}anomaly${C.reset}` : '';
    const confColor = colorByPhi(opp.confidence);
    console.log(`${ts()} ${C.dim}PERCEIVE${C.reset} ${opp.signal?.type || 'signal'} ${C.bold}${opp.token}${C.reset} ${opp.direction === 'LONG' ? C.green + '+' : C.red + '-'}${(opp.magnitude * 100).toFixed(1)}%${C.reset} (${confColor}conf=${(opp.confidence * 100).toFixed(0)}%${C.reset}${zStr}${anomStr})`);
  });

  agent.on('judgment', (judgment) => {
    const conf = judgment.confidence;
    const confColor = colorByPhi(conf);
    const qColor = judgment.qScore >= 60 ? C.green : judgment.qScore >= 40 ? C.yellow : C.red;
    const entropyStr = judgment.entropy?.category ? ` H=${judgment.entropy.category}` : '';
    const skepticStr = judgment.skepticism ? ' skeptic' : '';
    const sourceStr = judgment.source === 'CYNICJudge' ? `${C.green}CYNIC${C.reset}` : `${C.gray}basic${C.reset}`;

    // Axiom scores if available
    let axiomStr = '';
    if (judgment.axiomScores && Object.keys(judgment.axiomScores).length > 0) {
      const ax = judgment.axiomScores;
      const parts = [];
      if (ax.PHI || ax.phi) parts.push(`PHI:${ax.PHI || ax.phi}`);
      if (ax.VERIFY || ax.verify) parts.push(`VER:${ax.VERIFY || ax.verify}`);
      if (ax.CULTURE || ax.culture) parts.push(`CUL:${ax.CULTURE || ax.culture}`);
      if (ax.BURN || ax.burn) parts.push(`BRN:${ax.BURN || ax.burn}`);
      if (parts.length > 0) axiomStr = ` | ${C.dim}${parts.join(' ')}${C.reset}`;
    }

    console.log(`${ts()} ${C.cyan}JUDGE${C.reset}    ${qColor}Q=${judgment.qScore}${C.reset} ${judgment.verdict} ${confColor}conf=${(conf * 100).toFixed(1)}%${C.reset}${entropyStr}${skepticStr} [${sourceStr}]${axiomStr}`);
  });

  agent.on('decision', (decision) => {
    const emoji = decision.action === 'BUY' ? `${C.green}BUY${C.reset}` :
                  decision.action === 'SELL' ? `${C.red}SELL${C.reset}` :
                  `${C.gray}HOLD${C.reset}`;

    if (decision.action !== 'HOLD') {
      // Find top factors from reason
      const topStr = decision.reason?.includes('Top factors:')
        ? ` | Top: ${C.dim}${decision.reason.split('Top factors: ')[1]}${C.reset}`
        : '';
      console.log(`${ts()} ${C.bold}DECIDE${C.reset}   ${emoji} ${C.bold}${decision.token}${C.reset} ${(decision.size * 100).toFixed(1)}%${topStr}`);
    } else {
      console.log(`${ts()} ${C.dim}DECIDE   HOLD${C.reset} ${C.dim}${decision.reason || ''}${C.reset}`);
    }
  });

  agent.on('guardian_block', ({ decision, risk }) => {
    console.log(`${ts()} ${C.red}${C.bold}BLOCKED${C.reset}  ${C.red}Guardian blocked ${decision.action} ${decision.token}${C.reset}: ${risk.message}`);
  });

  agent.on('guardian_warning', ({ decision, risk }) => {
    console.log(`${ts()} ${C.yellow}WARN${C.reset}     Guardian: ${risk.message}`);
  });

  agent.on('action_complete', (result) => {
    if (result.success) {
      const pnlStr = result.simulatedPnL !== undefined
        ? ` ${pnlColor(result.simulatedPnL)}${result.simulatedPnL > 0 ? '+' : ''}${(result.simulatedPnL * 100).toFixed(1)}%${C.reset}`
        : '';
      console.log(`${ts()} ${C.green}ACT${C.reset}      ${result.signature || 'confirmed'}${pnlStr}`);
    } else {
      console.log(`${ts()} ${C.red}FAIL${C.reset}     ${result.error || 'execution failed'}`);
    }
  });

  agent.on('lesson', (lesson) => {
    const learnerStatus = agent.learner.getStatus();
    const pnlStr = `${pnlColor(lesson.pnl)}${lesson.pnl > 0 ? '+' : ''}${(lesson.pnl * 100).toFixed(1)}%${C.reset}`;
    const winStr = `${colorByPhi(learnerStatus.winRate)}win=${(learnerStatus.winRate * 100).toFixed(0)}%${C.reset}`;
    const totalPnlStr = `${pnlColor(learnerStatus.totalPnL)}PnL=${learnerStatus.totalPnL > 0 ? '+' : ''}${(learnerStatus.totalPnL * 100).toFixed(1)}%${C.reset}`;
    console.log(`${ts()} ${C.magenta}LEARN${C.reset}    ${pnlStr} | ${winStr} | ${totalPnlStr} | ${C.dim}${lesson.recommendation}${C.reset}`);
  });

  agent.on('error', (err) => {
    console.log(`${ts()} ${C.red}${C.bold}ERROR${C.reset}    ${err.message}`);
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Status Bar (every 30s)
  // ═════════════════════════════════════════════════════════════════════════

  const statusInterval = setInterval(() => {
    const status = agent.getStatus();
    const health = agent.getHealth();
    const learnerStatus = agent.learner.getStatus();

    const healthColor = health.status === 'healthy' ? C.green :
                        health.status === 'degraded' ? C.yellow : C.red;
    const avgConf = status.metrics.judgments > 0
      ? (status.metrics.actions > 0 ? health.successRate : 0.5)
      : 0;

    const pnlStr = `${pnlColor(status.metrics.totalPnL)}PnL ${status.metrics.totalPnL >= 0 ? '+' : ''}${(status.metrics.totalPnL * 100).toFixed(1)}%${C.reset}`;
    const winStr = learnerStatus.winRate > 0
      ? `win: ${colorByPhi(learnerStatus.winRate)}${(learnerStatus.winRate * 100).toFixed(0)}%${C.reset}`
      : `${C.gray}win: --${C.reset}`;

    console.log('');
    console.log(`${C.dim}──────────────────────────────────────────────────${C.reset}`);
    console.log(`  ${healthColor}${status.state}${C.reset} | ${status.metrics.tickCount} ticks | ${status.metrics.actions}/${status.config.maxActionsPerHour} actions | ${pnlStr}`);
    console.log(`  ${progressBar(avgConf, 1)} ${(avgConf * 100).toFixed(0)}% avg conf | ${winStr}`);
    console.log(`${C.dim}──────────────────────────────────────────────────${C.reset}`);
    console.log('');
  }, 30000);

  // ═════════════════════════════════════════════════════════════════════════
  // Graceful Shutdown
  // ═════════════════════════════════════════════════════════════════════════

  let isShuttingDown = false;

  async function shutdown(signal) {
    if (isShuttingDown) return;
    isShuttingDown = true;

    console.log('');
    console.log(`${ts()} ${C.dim}${signal} received, shutting down...${C.reset}`);

    clearInterval(statusInterval);
    await agent.stop();

    // Final statistics
    const status = agent.getStatus();
    const learnerStatus = agent.learner.getStatus();
    const deciderStatus = agent.decider.getStatus();
    const executorStatus = agent.executor.getStatus();
    const uptime = status.metrics.startedAt
      ? Math.round((Date.now() - status.metrics.startedAt) / 1000)
      : 0;
    const uptimeStr = `${Math.floor(uptime / 60)}m ${uptime % 60}s`;

    console.log('');
    console.log(`${C.cyan}═══════════════════════════════════════════════════════════${C.reset}`);
    console.log(`${C.cyan}  FINAL STATISTICS${C.reset}`);
    console.log(`${C.cyan}═══════════════════════════════════════════════════════════${C.reset}`);
    console.log(`  Uptime:        ${C.white}${uptimeStr}${C.reset}`);
    console.log(`  Ticks:         ${status.metrics.tickCount}`);
    console.log(`  Perceptions:   ${status.metrics.perceptions}`);
    console.log(`  Judgments:     ${status.metrics.judgments} (${C.green}CYNICJudge: ${deciderStatus.metrics.cynicJudgeUsed}${C.reset}, ${C.gray}fallback: ${deciderStatus.metrics.fallbackUsed}${C.reset})`);
    console.log(`  Decisions:     ${status.metrics.decisions}`);
    console.log(`  Actions:       ${status.metrics.actions} (${C.green}${status.metrics.successfulActions} ok${C.reset}, ${C.red}${status.metrics.failedActions} fail${C.reset})`);
    console.log(`  Win rate:      ${learnerStatus.winRate > 0 ? colorByPhi(learnerStatus.winRate) + (learnerStatus.winRate * 100).toFixed(1) + '%' + C.reset : C.gray + '--' + C.reset}`);
    console.log(`  Total PnL:     ${pnlColor(status.metrics.totalPnL)}${status.metrics.totalPnL >= 0 ? '+' : ''}${(status.metrics.totalPnL * 100).toFixed(2)}%${C.reset}`);
    console.log(`  Lessons:       ${learnerStatus.lessonsCount}`);
    console.log(`  Adjustments:   ${deciderStatus.weightAdjustments} dimensions`);
    console.log(`  Cluster:       ${executorStatus.cluster}`);
    console.log(`${C.cyan}═══════════════════════════════════════════════════════════${C.reset}`);
    console.log('');
    console.log(`${C.dim}*tail wag* A bientot. - CYNIC${C.reset}`);
    console.log('');

    process.exit(0);
  }

  process.on('SIGINT', () => shutdown('SIGINT'));
  process.on('SIGTERM', () => shutdown('SIGTERM'));

  // Start agent
  await agent.start();

  // Show config after start (so CYNICJudge status is known)
  showConfig();

  console.log(`${ts()} ${C.green}Agent started${C.reset}. Press Ctrl+C to stop.`);
  console.log('');
}

// Run
main().catch((err) => {
  console.error(`${C.red}Fatal error:${C.reset}`, err);
  process.exit(1);
});
