#!/usr/bin/env node
/**
 * CYNIC Training Pipeline Orchestrator
 *
 * Single command to run the complete training pipeline:
 *   node scripts/training/train.mjs
 *
 * Modes:
 *   --full        Full pipeline: export → split → sft → grpo → eval → deploy
 *   --quick       Quick: export → split only (for data inspection)
 *   --sft-only    SFT warm-up only (no GRPO)
 *   --grpo-only   GRPO only (requires existing SFT checkpoint)
 *   --eval-only   Evaluate existing model
 *   --dpo         Use DPO instead of GRPO (requires preference pairs)
 *
 * Options:
 *   --profile     local|cloud (default: local)
 *   --force       Bypass minimum data requirements
 *   --dry-run     Show what would run without executing
 *
 * Examples:
 *   node scripts/training/train.mjs --full
 *   node scripts/training/train.mjs --quick --profile cloud
 *   node scripts/training/train.mjs --sft-only --force
 *   node scripts/training/train.mjs --dpo --full
 *
 * "φ distrusts φ" — even training pipelines are bounded
 *
 * @module cynic/training/train
 */

import { spawn } from 'child_process';
import { existsSync, readdirSync, statSync } from 'fs';
import { pipeline, data } from './training-config.mjs';

// ═══════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════

const PHI_INV = 0.618;

// ═══════════════════════════════════════════════════════════════════════════
// CLI Arguments
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);

const mode = args.includes('--full') ? 'full'
           : args.includes('--quick') ? 'quick'
           : args.includes('--sft-only') ? 'sft-only'
           : args.includes('--grpo-only') ? 'grpo-only'
           : args.includes('--eval-only') ? 'eval-only'
           : 'full';

const useDpo = args.includes('--dpo');
const force = args.includes('--force');
const dryRun = args.includes('--dry-run');
const profile = getArg(args, '--profile') || process.env.CYNIC_TRAIN_PROFILE || 'local';

function getArg(args, flag) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : null;
}

// ═══════════════════════════════════════════════════════════════════════════
// Stage Definitions
// ═══════════════════════════════════════════════════════════════════════════

const stages = {
  export: {
    name: 'export',
    script: 'scripts/training/export-training-data.mjs',
    args: [],
    description: 'Export judgments + feedback + trajectories to JSONL',
    check: () => true, // Always can export
  },
  split: {
    name: 'split',
    script: 'scripts/training/split-data.mjs',
    args: [],
    description: 'Split into train/val/test (φ-aligned ratios)',
    check: () => {
      const files = readdirSync('.').filter(f => f.startsWith('training-data-') && f.endsWith('.jsonl'));
      return files.length > 0;
    },
  },
  dpo_convert: {
    name: 'dpo-convert',
    script: 'scripts/training/convert-to-dpo.mjs',
    args: ['--synthetic', 'true'],
    description: 'Convert to DPO preference pairs (chosen/rejected)',
    check: () => {
      const files = readdirSync('.').filter(f => f.startsWith('training-data-') && f.endsWith('.jsonl'));
      return files.length > 0;
    },
  },
  sft: {
    name: 'sft',
    script: profile === 'cloud' ? 'scripts/training/run-sft.sh' : 'scripts/training/run-sft-local.sh',
    args: [],
    description: profile === 'cloud' ? 'SFT warm-up (Unsloth QLoRA)' : 'SFT warm-up (llama.cpp LoRA)',
    check: () => {
      const trainFile = 'training-splits/train.jsonl';
      if (!existsSync(trainFile)) return false;
      const stat = statSync(trainFile);
      return stat.size > 1000; // At least ~10 records
    },
  },
  grpo: {
    name: 'grpo',
    script: profile === 'cloud' ? 'scripts/training/run-grpo.sh' : 'scripts/training/run-grpo-local.mjs',
    args: [],
    description: profile === 'cloud' ? 'GRPO reinforcement (GPU)' : 'Rejection sampling + SFT (CPU)',
    check: () => {
      const checkpointDir = profile === 'cloud' ? 'training-checkpoints/sft-cloud' : 'training-checkpoints/sft-local';
      return existsSync(checkpointDir);
    },
  },
  eval: {
    name: 'eval',
    script: 'scripts/training/evaluate.mjs',
    args: [],
    description: 'Evaluate model against deployment gate',
    check: () => {
      const testFile = 'training-splits/test.jsonl';
      return existsSync(testFile);
    },
  },
  deploy: {
    name: 'deploy',
    script: 'scripts/training/deploy-ollama.sh',
    args: [],
    description: 'Register model in Ollama as cynic-dog0',
    check: () => {
      // Check for GGUF or merged weights
      const ggufFiles = readdirSync('training-checkpoints').filter(f => f.endsWith('.gguf'));
      return ggufFiles.length > 0 || existsSync('training-checkpoints/merged');
    },
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline Definitions
// ═══════════════════════════════════════════════════════════════════════════

const pipelines = {
  full: useDpo
    ? ['export', 'dpo_convert', 'split', 'sft', 'eval', 'deploy']
    : ['export', 'split', 'sft', 'grpo', 'eval', 'deploy'],
  quick: ['export', 'split'],
  'sft-only': ['export', 'split', 'sft'],
  'grpo-only': ['grpo'],
  'eval-only': ['eval'],
};

// ═══════════════════════════════════════════════════════════════════════════
// Execution
// ═══════════════════════════════════════════════════════════════════════════

function runStage(stage) {
  return new Promise((resolve, reject) => {
    const stageConfig = stages[stage];
    if (!stageConfig) {
      reject(new Error(`Unknown stage: ${stage}`));
      return;
    }

    console.log(`\n${'═'.repeat(60)}`);
    console.log(`  ${stageConfig.name.toUpperCase()}: ${stageConfig.description}`);
    console.log(`${'═'.repeat(60)}\n`);

    if (dryRun) {
      console.log(`[dry-run] Would run: ${stageConfig.script} ${stageConfig.args.join(' ')}`);
      resolve();
      return;
    }

    if (!force && !stageConfig.check()) {
      console.log(`[skip] Prerequisites not met for ${stageConfig.name}`);
      console.log(`[skip] Use --force to bypass checks`);
      reject(new Error(`Prerequisites not met for ${stageConfig.name}`));
      return;
    }

    const ext = stageConfig.script.split('.').pop();
    const cmd = ext === 'mjs' || ext === 'js' ? 'node' : 'bash';
    const proc = spawn(cmd, [stageConfig.script, ...stageConfig.args], {
      stdio: 'inherit',
      env: { ...process.env, CYNIC_TRAIN_PROFILE: profile },
    });

    proc.on('close', code => {
      if (code === 0) {
        console.log(`\n✓ ${stageConfig.name} completed successfully\n`);
        resolve();
      } else {
        reject(new Error(`${stageConfig.name} failed with code ${code}`));
      }
    });

    proc.on('error', reject);
  });
}

async function runPipeline(stageList) {
  const startTime = Date.now();

  console.log(`\n${'╔' + '═'.repeat(58) + '╗'}`);
  console.log(`${'║'}  CYNIC TRAINING PIPELINE${' '.repeat(33)}${'║'}`);
  console.log(`${'║'}  Mode: ${mode.padEnd(49)}${'║'}`);
  console.log(`${'║'}  Profile: ${profile.padEnd(47)}${'║'}`);
  console.log(`${'║'}  Method: ${(useDpo ? 'DPO' : 'GRPO').padEnd(48)}${'║'}`);
  console.log(`${'║'}  Stages: ${stageList.length.toString().padEnd(47)}${'║'}`);
  console.log(`${'╚' + '═'.repeat(58) + '╝'}\n`);

  for (const stage of stageList) {
    try {
      await runStage(stage);
    } catch (err) {
      console.error(`\n✗ Pipeline failed at stage: ${stage}`);
      console.error(`  Error: ${err.message}`);
      process.exit(1);
    }
  }

  const duration = (Date.now() - startTime) / 1000 / 60;

  console.log(`\n${'╔' + '═'.repeat(58) + '╗'}`);
  console.log(`${'║'}  TRAINING COMPLETE${' '.repeat(39)}${'║'}`);
  console.log(`${'║'}  Duration: ${duration.toFixed(1)} minutes${' '.repeat(43 - duration.toFixed(1).length)}${'║'}`);
  console.log(`${'║'}  φ distrusts φ — max confidence 61.8%${' '.repeat(19)}${'║'}`);
  console.log(`${'╚' + '═'.repeat(58) + '╝'}\n`);
}

// ═══════════════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════════════

const stageList = pipelines[mode];
if (!stageList) {
  console.error(`Unknown mode: ${mode}`);
  process.exit(1);
}

console.log(`CYNIC Training Pipeline`);
console.log(`Mode: ${mode}`);
console.log(`Profile: ${profile}`);
console.log(`Stages: ${stageList.join(' → ')}`);
console.log(`DPO: ${useDpo}`);
console.log(`Force: ${force}`);
console.log(`Dry run: ${dryRun}`);

runPipeline(stageList).catch(err => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
