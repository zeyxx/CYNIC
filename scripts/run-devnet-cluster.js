#!/usr/bin/env node
/**
 * Run Devnet Cluster
 *
 * Spawns N in-process CYNICNetworkNode instances, connects them
 * in a mesh, emits synthetic judgments to drive block production.
 *
 * Prerequisites: node scripts/setup-devnet-validators.js
 *
 * Usage: node scripts/run-devnet-cluster.js [count=5]
 *
 * "The pack hunts together" - κυνικός
 */

import { existsSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { generateKeypair } from '@cynic/protocol';
import { loadWalletFromFile, SolanaCluster } from '@cynic/anchor';
import { globalEventBus, EventType } from '@cynic/core';
import { CYNICNetworkNode } from '@cynic/node';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, '..');

const VALIDATORS_DIR = join(ROOT, 'validators');
const BASE_PORT = 19618; // φ-aligned, avoids 8618 (prod) and 18900 (test)
const RPC_URL = process.env.HELIUS_RPC || SolanaCluster.DEVNET;
const NODE_COUNT = parseInt(process.argv[2] || '5', 10);

const nodes = [];
let running = true;
let judgmentInterval = null;

// Prevent unhandled RPC errors (429, timeout) from crashing the cluster
process.on('unhandledRejection', (err) => {
  const msg = err?.message || String(err);
  if (msg.includes('429') || msg.includes('Too Many Requests') || msg.includes('timeout')) {
    console.log(`  [RPC] Rate limited — backing off (${msg.slice(0, 60)}...)`);
  } else {
    console.error(`  [WARN] Unhandled rejection: ${msg}`);
  }
});

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
  console.log('═══════════════════════════════════════════════════════════');
  console.log('  CYNIC Multi-Validator Devnet Cluster');
  console.log(`  Nodes: ${NODE_COUNT} | Base port: ${BASE_PORT}`);
  console.log(`  RPC:   ${RPC_URL.slice(0, 50)}${RPC_URL.length > 50 ? '...' : ''}`);
  console.log('═══════════════════════════════════════════════════════════\n');

  // ── Load Solana keypairs ───────────────────────────────────────────────
  console.log('── LOADING VALIDATOR WALLETS ───────────────────────────────\n');

  const wallets = [];
  for (let i = 0; i < NODE_COUNT; i++) {
    const path = join(VALIDATORS_DIR, `validator-${i}.json`);
    if (!existsSync(path)) {
      console.error(`ERROR: Validator keypair not found: ${path}`);
      console.error('Run: node scripts/setup-devnet-validators.js');
      process.exit(1);
    }
    const wallet = loadWalletFromFile(path);
    wallets.push(wallet);
    console.log(`  [${i}] Solana: ${wallet.publicKey.slice(0, 20)}...`);
  }
  console.log('');

  // ── Generate P2P keypairs (fresh each run) ─────────────────────────────
  console.log('── GENERATING P2P KEYPAIRS ─────────────────────────────────\n');

  const p2pKeypairs = [];
  for (let i = 0; i < NODE_COUNT; i++) {
    const kp = generateKeypair();
    p2pKeypairs.push(kp);
    console.log(`  [${i}] P2P:    ${kp.publicKey.slice(0, 32)}...`);
  }
  console.log('');

  // ── Create CYNICNetworkNode instances ──────────────────────────────────
  console.log('── STARTING NODES ─────────────────────────────────────────\n');

  const seedAddress = `ws://localhost:${BASE_PORT}`;

  for (let i = 0; i < NODE_COUNT; i++) {
    const port = BASE_PORT + i;
    const seedNodes = i === 0 ? [] : [seedAddress]; // Node 0 is seed

    const node = new CYNICNetworkNode({
      publicKey: p2pKeypairs[i].publicKey,
      privateKey: p2pKeypairs[i].privateKey,
      port,
      eScore: 60,
      burned: 100,
      seedNodes,
      enabled: true,
      anchoringEnabled: true,
      dryRun: false,
      anchorInterval: 3, // Anchor every 3rd block (reduce RPC load on public devnet)
      wallet: wallets[i],
      solanaCluster: RPC_URL,
      slotDuration: 2000, // 2s slots for visible feedback
      genesisTime: Date.now(),
    });

    // Register ALL validators in consensus for each node
    for (const kp of p2pKeypairs) {
      node.addValidator({
        publicKey: kp.publicKey,
        eScore: 60,
        burned: 100,
        uptime: 1.0,
      });
    }

    // Wire events for logging
    wireNodeEvents(node, i);

    nodes.push(node);
  }

  // Start nodes sequentially with delay
  for (let i = 0; i < nodes.length; i++) {
    try {
      await nodes[i].start();
      console.log(`  [${i}] Started on port ${BASE_PORT + i}`);
    } catch (err) {
      console.error(`  [${i}] Start failed: ${err.message}`);
      if (i === 0) {
        console.error('FATAL: Seed node (node 0) failed. Aborting.');
        process.exit(1);
      }
    }
    await sleep(500);
  }
  console.log('');

  // ── Wait for peer discovery ────────────────────────────────────────────
  console.log('── WAITING FOR PEER DISCOVERY (3s) ────────────────────────\n');
  await sleep(3000);

  for (let i = 0; i < nodes.length; i++) {
    const peers = nodes[i].getConnectedPeers().length;
    const validators = nodes[i].getValidatorCount();
    console.log(`  [${i}] Peers: ${peers}, Validators: ${validators}, State: ${nodes[i].state}`);
  }
  console.log('');

  // ── Emit synthetic judgments ───────────────────────────────────────────
  console.log('── EMITTING SYNTHETIC JUDGMENTS (every 5s) ─────────────────\n');

  let judgmentCounter = 0;

  judgmentInterval = setInterval(() => {
    if (!running) return;

    judgmentCounter++;
    const judgment = {
      id: `jdg_devnet_${judgmentCounter}_${Date.now().toString(36)}`,
      payload: {
        id: `jdg_devnet_${judgmentCounter}`,
        qScore: 50 + Math.floor(Math.random() * 30),
        verdict: ['HOWL', 'WAG', 'BARK'][judgmentCounter % 3],
        score: 60,
        itemHash: `item_${judgmentCounter}`,
      },
      timestamp: Date.now(),
    };

    globalEventBus.publish(EventType.JUDGMENT_CREATED, judgment);
    console.log(`  Judgment #${judgmentCounter}: ${judgment.id.slice(0, 30)}...`);
  }, 5000);

  // ── Graceful shutdown ─────────────────────────────────────────────────
  console.log('Press Ctrl+C to stop the cluster.\n');

  process.on('SIGINT', async () => {
    if (!running) return;
    running = false;
    console.log('\n── SHUTTING DOWN ──────────────────────────────────────────\n');

    if (judgmentInterval) clearInterval(judgmentInterval);

    // Stop in reverse order
    for (let i = nodes.length - 1; i >= 0; i--) {
      try {
        await nodes[i].stop();
        console.log(`  [${i}] Stopped`);
      } catch (err) {
        console.error(`  [${i}] Stop error: ${err.message}`);
      }
    }

    // Summary
    console.log('\n── CLUSTER SUMMARY ────────────────────────────────────────\n');
    for (let i = 0; i < nodes.length; i++) {
      const info = nodes[i].getInfo();
      console.log(`  [${i}] Blocks proposed: ${info.stats.blocksProposed}, Finalized: ${info.stats.blocksFinalized}, Anchored: ${info.stats.blocksAnchored || 0}`);
    }
    console.log('\n═══════════════════════════════════════════════════════════\n');

    process.exit(0);
  });

  // Windows support for Ctrl+C
  if (process.platform === 'win32') {
    const readline = await import('readline');
    const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
    rl.on('SIGINT', () => process.emit('SIGINT'));
  }
}

function wireNodeEvents(node, index) {
  node.on('block:produced', ({ slot, hash, judgmentCount }) => {
    console.log(`  [${index}] Block produced: slot=${slot} hash=${hash?.slice(0, 16)}... judgments=${judgmentCount}`);
  });

  node.on('block:finalized', ({ blockHash, slot }) => {
    console.log(`  [${index}] Block FINALIZED: slot=${slot} hash=${blockHash?.slice(0, 16)}...`);
  });

  node.on('block:anchored', ({ slot, signature, hash }) => {
    console.log(`  [${index}] Block ANCHORED: slot=${slot} sig=${signature?.slice(0, 32)}...`);
  });

  node.on('anchor:failed', ({ slot, error }) => {
    console.log(`  [${index}] Anchor FAILED: slot=${slot} error=${error}`);
  });

  node.on('peer:connected', ({ peerId }) => {
    console.log(`  [${index}] Peer connected: ${peerId?.slice(0, 16)}...`);
  });

  node.on('block:confirmed', ({ slot, ratio }) => {
    console.log(`  [${index}] Block confirmed: slot=${slot} ratio=${(ratio * 100).toFixed(1)}%`);
  });
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
