/**
 * Start Command
 *
 * Start a CYNIC node using CYNICNetworkNode (Phase 2)
 *
 * Delegates all P2P, consensus, and validator logic to CYNICNetworkNode.
 * Keeps HTTP API + REPL as thin presentation layers.
 *
 * @module @cynic/node/cli/commands/start
 */

'use strict';

import fs from 'fs';
import readline from 'readline';
import chalk from 'chalk';
import { generateKeypair, hashBlock } from '@cynic/protocol';
import { PHI_INV, globalEventBus, EventType, createLogger } from '@cynic/core';
import { CYNICNetworkNode } from '../../network/network-node.js';
import { BlockStore } from '../../network/block-store.js';

const log = createLogger('StartCommand');

/**
 * Format bytes to human readable
 */
function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/**
 * Format uptime
 */
function formatUptime(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

/**
 * Load or generate keypair
 */
function loadOrGenerateKeypair(keyfile, verbose) {
  if (keyfile && fs.existsSync(keyfile)) {
    if (verbose) console.log(chalk.gray(`Loading keypair from ${keyfile}`));
    const data = JSON.parse(fs.readFileSync(keyfile, 'utf-8'));
    return {
      publicKey: data.publicKey,
      privateKey: data.privateKey,
    };
  }

  if (verbose) console.log(chalk.gray('Generating ephemeral keypair...'));
  return generateKeypair();
}

/**
 * Start command handler
 */
export async function startCommand(options) {
  // Pass anchor CLI flags to env vars before network-singleton reads them
  if (options.anchor !== undefined) {
    process.env.CYNIC_ANCHORING_ENABLED = options.anchor ? 'true' : 'false';
  }
  if (options.anchorInterval) {
    process.env.CYNIC_ANCHOR_INTERVAL = options.anchorInterval;
  }

  const port = parseInt(options.port);
  const host = options.host;
  const verbose = options.verbose;

  console.log(chalk.bold.cyan('\n  CYNIC Node Starting...\n'));

  // Load keypair
  const keypair = loadOrGenerateKeypair(options.keyfile, verbose);
  // Skip DER header (first 24 hex chars = 12 bytes) to get actual ed25519 key bytes
  const nodeId = keypair.publicKey.slice(24, 40);

  console.log(chalk.gray('  Node ID: ') + chalk.yellow(nodeId + '...'));
  console.log(chalk.gray('  Port:    ') + chalk.white(port));
  console.log(chalk.gray('  Host:    ') + chalk.white(host));
  console.log();

  // Collect peer addresses from --connect flags AND CYNIC_SEED_NODES env var
  const peerAddresses = [...(options.connect || [])];
  const envSeeds = (process.env.CYNIC_SEED_NODES || '')
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);
  for (const seed of envSeeds) {
    if (!peerAddresses.includes(seed)) {
      peerAddresses.push(seed);
    }
  }

  // We need a reference to the node inside the httpHandler closure,
  // but the node needs httpHandler at construction. Use a mutable ref.
  let node = null;
  const startTime = Date.now();

  // Build HTTP handler (closure over node ref)
  const httpHandler = (req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
    res.setHeader('Content-Type', 'application/json');

    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      return res.end();
    }

    const url = req.url.split('?')[0];

    if (url === '/') {
      res.writeHead(200);
      return res.end(JSON.stringify({
        name: 'CYNIC Node',
        version: '0.1.0',
        nodeId,
        greek: 'κυνικός',
        endpoints: ['/health', '/status', '/peers', '/consensus', '/judgment', '/propose'],
      }));
    }

    if (url === '/health') {
      const status = node ? node.getStatus() : {};
      const peers = node ? node.getConnectedPeers() : [];
      res.writeHead(200);
      return res.end(JSON.stringify({
        status: 'healthy',
        nodeId,
        uptime: Date.now() - startTime,
        peers: peers.length,
        state: node?.state || 'OFFLINE',
        phi: { maxConfidence: PHI_INV },
      }));
    }

    if (url === '/status') {
      const status = node ? node.getStatus() : {};
      res.writeHead(200);
      return res.end(JSON.stringify({
        nodeId,
        uptime: Date.now() - startTime,
        ...status,
      }));
    }

    if (url === '/peers') {
      const peers = node ? node.getConnectedPeers() : [];
      res.writeHead(200);
      return res.end(JSON.stringify({
        count: peers.length,
        peers: peers.map(p => ({ id: p.slice(0, 24) + '...' })),
      }));
    }

    if (url === '/consensus') {
      const status = node ? node.getStatus() : {};
      res.writeHead(200);
      return res.end(JSON.stringify({
        consensus: status.consensus || null,
      }));
    }

    if (url === '/judgment' && req.method === 'POST') {
      if (!node || !node.isParticipating) {
        res.writeHead(503);
        return res.end(JSON.stringify({
          error: 'Node not participating in consensus',
          state: node?.state || 'OFFLINE',
        }));
      }

      let body = '';
      req.on('data', chunk => { body += chunk; });
      req.on('end', async () => {
        try {
          const judgment = JSON.parse(body);
          const result = await node.submitJudgment(judgment);
          res.writeHead(200);
          res.end(JSON.stringify({ success: true, ...result }));
        } catch (err) {
          res.writeHead(400);
          res.end(JSON.stringify({ error: err.message }));
        }
      });
      return;
    }

    if (url === '/propose' && req.method === 'POST') {
      if (!node || !node.isParticipating) {
        res.writeHead(503);
        return res.end(JSON.stringify({
          error: 'Node not participating in consensus',
          state: node?.state || 'OFFLINE',
        }));
      }

      try {
        const testBlock = {
          type: 'JUDGMENT',
          slot: 0, // Will be set by consensus
          timestamp: Date.now(),
          previous_hash: '0'.repeat(64),
          proposer: keypair.publicKey,
          judgments: [{
            id: `jdg_test_${Date.now()}`,
            itemHash: `item_${Date.now()}`,
            globalScore: Math.round(50 + Math.random() * 50),
            verdict: 'WAG',
          }],
          merkle_root: '0'.repeat(64),
        };
        testBlock.hash = hashBlock(testBlock);

        const record = node.proposeBlock(testBlock);

        res.writeHead(200);
        return res.end(JSON.stringify({
          success: !!record,
          blockHash: testBlock.hash,
          validators: node.getValidatorCount(),
        }));
      } catch (err) {
        res.writeHead(500);
        return res.end(JSON.stringify({ error: err.message }));
      }
    }

    res.writeHead(404);
    res.end(JSON.stringify({ error: 'Not Found', path: url }));
  };

  // Create CYNICNetworkNode — replaces manual transport/gossip/consensus creation
  const anchoringEnabled = options.anchor || process.env.CYNIC_ANCHORING_ENABLED === 'true';
  node = new CYNICNetworkNode({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    port,
    host,
    httpHandler,
    seedNodes: peerAddresses.map(a => a.startsWith('ws') ? a : `wss://${a}`),
    eScore: 50,
    anchoringEnabled,
    anchorInterval: options.anchorInterval ? parseInt(options.anchorInterval) : undefined,
    solanaCluster: process.env.SOLANA_CLUSTER || 'devnet',
    dryRun: process.env.CYNIC_ANCHORING_DRY_RUN === 'true',
  });

  // ─── PERSISTENCE: Wire BlockStore for finalized block storage ────────────
  let blockStore = null;
  const dbUrl = process.env.CYNIC_DATABASE_URL || process.env.DATABASE_URL;
  if (dbUrl) {
    try {
      const pg = await import('pg');
      const pool = new pg.default.Pool({
        connectionString: dbUrl,
        ssl: dbUrl.includes('render.com') ? { rejectUnauthorized: false } : undefined,
        max: 5,
        idleTimeoutMillis: 30000,
      });
      blockStore = new BlockStore({ pool });

      // Wire for state sync (getBlocks/storeBlock)
      node.wireBlockStore(blockStore.callbacks());
      // Wire for anchoring retry sweeps
      node.wireAnchoringStore(blockStore);

      // Subscribe to BLOCK_FINALIZED → persist to PostgreSQL
      globalEventBus.subscribe(EventType.BLOCK_FINALIZED, async (event) => {
        const { blockHash, slot, block } = event.payload || event;
        if (slot === undefined) return;
        try {
          await blockStore.storeBlock({
            slot,
            hash: blockHash || block?.hash,
            proposer: block?.proposer || keypair.publicKey?.slice(0, 128) || 'unknown',
            merkle_root: block?.merkle_root || block?.judgments_root,
            judgments: block?.judgments || [],
            judgment_count: block?.judgment_count || block?.judgments?.length || 0,
            prev_hash: block?.prev_hash,
            timestamp: block?.timestamp || Date.now(),
          });
        } catch (err) {
          log.warn('Block persistence failed', { slot, error: err.message });
        }
      });

      // Subscribe to BLOCK_ANCHORED → persist anchor record
      globalEventBus.subscribe(EventType.BLOCK_ANCHORED, async (event) => {
        const { slot, signature, merkleRoot, cluster } = event.payload || event;
        if (slot === undefined) return;
        try {
          await blockStore.storeAnchor({
            slot,
            txSignature: signature,
            status: 'confirmed',
            merkleRoot,
            cluster,
          });
        } catch (err) {
          log.warn('Anchor persistence failed', { slot, error: err.message });
        }
      });

      console.log(chalk.green('  [DB]   ') + 'BlockStore wired to PostgreSQL');
    } catch (err) {
      console.log(chalk.yellow('  [DB]   ') + `PostgreSQL unavailable: ${err.message} (blocks stored in memory)`);
    }
  } else {
    console.log(chalk.gray('  [DB]   ') + 'No DATABASE_URL — blocks stored in memory only');
  }

  // ─── SOLANA WALLET: Load for production anchoring ────────────────────────
  if (anchoringEnabled && process.env.CYNIC_SOLANA_KEY) {
    try {
      const { loadWalletFromEnv } = await import('@cynic/anchor');
      const wallet = loadWalletFromEnv('CYNIC_SOLANA_KEY');
      if (wallet) {
        node.setAnchoringWallet(wallet);
        const pubkey = wallet.publicKey || 'loaded';
        console.log(chalk.green('  [SOL]  ') + `Wallet loaded: ${chalk.cyan(pubkey)}`);
      }
    } catch (err) {
      console.log(chalk.yellow('  [SOL]  ') + `Wallet not loaded: ${err.message}`);
    }
  }

  // Wire event logging
  node.on('peer:connected', ({ peerId, publicKey, address }) => {
    const id = (publicKey || peerId || '').slice(0, 12);
    console.log(chalk.green('  [PEER] ') + `Connected: ${chalk.cyan(id)}...`);
  });

  node.on('peer:disconnected', ({ peerId, code, reason }) => {
    const id = (peerId || '').slice(0, 12);
    console.log(chalk.red('  [PEER] ') + `Disconnected: ${chalk.gray(id)}... code=${code} reason=${reason || 'none'}`);
  });

  node.on('peer:error', ({ error }) => {
    console.log(chalk.red('  [ERR]  ') + error.message);
  });

  node.on('block:finalized', ({ blockHash }) => {
    console.log(chalk.green('  [FIN]  ') + `Block finalized: ${chalk.cyan(blockHash.slice(0, 16))}...`);
  });

  if (verbose) {
    node.on('block:produced', ({ block }) => {
      console.log(chalk.yellow('  [PROP] ') + `Block produced: ${chalk.cyan(block?.hash?.slice(0, 16) || '?')}...`);
    });

    node.on('consensus:started', ({ slot }) => {
      console.log(chalk.blue('  [CONS] ') + `Consensus started at slot ${slot}`);
    });

    node.on('heartbeat:received', ({ nodeId: hbNodeId, eScore }) => {
      console.log(chalk.gray('  [HB]   ') + `from ${hbNodeId?.slice(0, 8)} eScore=${eScore}`);
    });
  }

  // Start the node
  try {
    await node.start();
    console.log(chalk.green('  [OK]   ') + `Server listening on ${chalk.bold(`${host}:${port}`)}`);
    console.log(chalk.green('  [OK]   ') + `CYNICNetworkNode started (Phase 2: full features)`);
  } catch (err) {
    console.error(chalk.red('  [FAIL] ') + `Could not start node: ${err.message}`);
    process.exit(1);
  }

  // PeerDiscovery already connects to seedNodes on start().
  // We only track addresses for the auto-reconnect loop below.

  // Track required peers for auto-reconnect (normalized addresses)
  const requiredPeers = new Set();
  for (const address of peerAddresses) {
    requiredPeers.add(address.startsWith('ws') ? address : `wss://${address}`);
  }

  // Daemon mode
  if (options.daemon) {
    console.log(chalk.green('\n  [OK]   ') + `Running in daemon mode (no interactive REPL)`);
    console.log(chalk.gray('  ─────────────────────────────────────────────────\n'));

    // Verbose heartbeat logging
    if (verbose) {
      setInterval(() => {
        const info = node.getInfo();
        const status = node.getStatus();
        const peers = node.getConnectedPeers();
        console.log(
          chalk.gray(`  [♥] `) +
          `uptime=${formatUptime(info.uptime)} ` +
          `state=${info.state} ` +
          `peers=${peers.length} ` +
          `eScore=${info.eScore} ` +
          `blocks=${info.stats.blocksFinalized || 0}`
        );
      }, 61800);
    }

    // Auto-reconnect loop with FIXED address matching
    if (requiredPeers.size > 0) {
      setInterval(async () => {
        // Access the underlying transport for address-level connection check
        const transport = node._transport?.transport;
        if (!transport) return;

        for (const address of requiredPeers) {
          if (transport.hasConnectionToAddress(address)) continue;

          try {
            await node.connectToPeer({ id: `peer_${Date.now()}`, address });
            console.log(chalk.green('  [RECONN] ') + `Reconnected to ${address}`);
          } catch {
            // Unreachable - will retry next cycle
          }
        }
      }, 30000);
    }
  }

  // Interactive mode
  if (!options.daemon) {
    console.log(chalk.gray('\n  ─────────────────────────────────────────────────'));
    console.log(chalk.bold('  Commands:'));
    console.log(chalk.gray('    /peers    ') + 'List connected peers');
    console.log(chalk.gray('    /stats    ') + 'Show statistics');
    console.log(chalk.gray('    /consensus') + 'Show consensus status');
    console.log(chalk.gray('    /connect <addr>  ') + 'Connect to peer');
    console.log(chalk.gray('    /broadcast <msg> ') + 'Broadcast message');
    console.log(chalk.gray('    /quit     ') + 'Shutdown node');
    console.log(chalk.gray('  ─────────────────────────────────────────────────\n'));

    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
      prompt: chalk.cyan('cynic> '),
    });

    rl.prompt();

    rl.on('line', async (line) => {
      const input = line.trim();

      if (input.startsWith('/')) {
        const [cmd, ...args] = input.slice(1).split(/\s+/);

        switch (cmd) {
          case 'peers': {
            const peers = node.getConnectedPeers();
            console.log(chalk.bold(`\n  Connected Peers (${peers.length}):`));
            if (peers.length === 0) {
              console.log(chalk.gray('    No peers connected'));
            } else {
              peers.forEach((p, i) => {
                console.log(chalk.gray(`    ${i + 1}. `) + chalk.cyan(p.slice(0, 24) + '...'));
              });
            }
            console.log();
            break;
          }

          case 'stats': {
            const info = node.getInfo();
            const status = node.getStatus();

            console.log(chalk.bold('\n  Node Statistics:'));
            console.log(chalk.gray('    State:        ') + chalk.yellow(info.state));
            console.log(chalk.gray('    Uptime:       ') + formatUptime(info.uptime));
            console.log(chalk.gray('    E-Score:      ') + info.eScore);
            console.log(chalk.gray('    Peers:        ') + node.getConnectedPeers().length);
            console.log(chalk.gray('    Blocks:       ') + `${info.stats.blocksProposed} proposed, ${info.stats.blocksFinalized} finalized`);
            console.log(chalk.gray('    Messages:     ') + `${info.stats.messagesSent} sent, ${info.stats.messagesReceived} received`);
            if (status.transport?.stats) {
              console.log(chalk.gray('    Bandwidth:    ') + `${formatBytes(status.transport.stats.bytesOut || 0)} out, ${formatBytes(status.transport.stats.bytesIn || 0)} in`);
            }
            console.log();
            break;
          }

          case 'consensus': {
            const status = node.getStatus();
            const cons = status.consensus || {};
            console.log(chalk.bold('\n  Consensus Status:'));
            console.log(chalk.gray('    Current Slot:     ') + (cons.currentSlot ?? 'N/A'));
            console.log(chalk.gray('    Finalized Slot:   ') + (cons.lastFinalizedSlot ?? 'N/A'));
            console.log(chalk.gray('    Validators:       ') + (cons.validators ?? 'N/A'));
            console.log(chalk.gray('    Stats:            ') + JSON.stringify(cons.stats || {}).slice(0, 80));
            console.log();
            break;
          }

          case 'connect': {
            if (args.length === 0) {
              console.log(chalk.red('  Usage: /connect <address>'));
            } else {
              const address = args[0].startsWith('ws') ? args[0] : `wss://${args[0]}`;
              try {
                await node.connectToPeer({ id: `peer_${Date.now()}`, address });
                requiredPeers.add(address);
                console.log(chalk.green('  Connected to ') + chalk.cyan(address));
              } catch (err) {
                console.log(chalk.red('  Failed: ') + err.message);
              }
            }
            break;
          }

          case 'broadcast': {
            if (args.length === 0) {
              console.log(chalk.red('  Usage: /broadcast <message>'));
            } else {
              const judgment = {
                id: `jdg_${Date.now()}`,
                item: { type: 'cli', data: args.join(' ') },
                globalScore: Math.round(Math.random() * 100),
                verdict: 'WAG',
                timestamp: Date.now(),
              };
              await node.broadcastJudgment(judgment);
              console.log(chalk.green('  Broadcast sent'));
            }
            break;
          }

          case 'quit':
          case 'exit':
          case 'q': {
            console.log(chalk.yellow('\n  Shutting down...'));
            await node.stop();
            console.log(chalk.green('  Goodbye!\n'));
            process.exit(0);
            break;
          }

          default:
            console.log(chalk.red(`  Unknown command: /${cmd}`));
        }
      } else if (input.length > 0) {
        console.log(chalk.gray('  Type /help for commands'));
      }

      rl.prompt();
    });

    rl.on('close', async () => {
      console.log(chalk.yellow('\n  Shutting down...'));
      await node.stop();
      process.exit(0);
    });
  }

  // Graceful shutdown
  process.on('SIGINT', async () => {
    console.log(chalk.yellow('\n  Received SIGINT, shutting down...'));
    await node.stop();
    if (blockStore?._pool) await blockStore._pool.end().catch(() => {});
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await node.stop();
    if (blockStore?._pool) await blockStore._pool.end().catch(() => {});
    process.exit(0);
  });
}

export default { startCommand };
