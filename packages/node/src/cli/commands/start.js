/**
 * Start Command
 *
 * Start a CYNIC node with optional peer connections
 *
 * @module @cynic/node/cli/commands/start
 */

'use strict';

import fs from 'fs';
import path from 'path';
import readline from 'readline';
import chalk from 'chalk';
import { WebSocketTransport } from '../../transport/index.js';
import {
  GossipProtocol,
  generateKeypair,
  createPeerInfo,
  ConsensusEngine,
  ConsensusGossip,
  SlotManager,
} from '@cynic/protocol';
import { PHI_INV } from '@cynic/core';

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
  const port = parseInt(options.port);
  const host = options.host;
  const verbose = options.verbose;
  const startServer = options.server !== false;

  console.log(chalk.bold.cyan('\n  CYNIC Node Starting...\n'));

  // Load keypair
  const keypair = loadOrGenerateKeypair(options.keyfile, verbose);
  // Skip DER header (first 24 hex chars = 12 bytes) to get actual ed25519 key bytes
  // DER structure: 302a 3005 0603 2b6570 0321 00 [32 bytes of key]
  const nodeId = keypair.publicKey.slice(24, 40);

  console.log(chalk.gray('  Node ID: ') + chalk.yellow(nodeId + '...'));
  console.log(chalk.gray('  Port:    ') + chalk.white(port));
  console.log(chalk.gray('  Host:    ') + chalk.white(host));
  console.log();

  // Build HTTP handler for API endpoints
  const httpHandler = (req, res) => {
    // CORS headers
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
    res.setHeader('Content-Type', 'application/json');

    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      return res.end();
    }

    const url = req.url.split('?')[0];

    // Root
    if (url === '/') {
      res.writeHead(200);
      return res.end(JSON.stringify({
        name: 'CYNIC Node',
        version: '0.1.0',
        nodeId,
        greek: 'κυνικός',
        endpoints: ['/health', '/status', '/peers', '/consensus'],
      }));
    }

    // Health
    if (url === '/health') {
      const stats = transport.getStats();
      res.writeHead(200);
      return res.end(JSON.stringify({
        status: 'healthy',
        nodeId,
        uptime: Date.now() - startTime,
        peers: stats.connections.connected,
        phi: { maxConfidence: PHI_INV },
      }));
    }

    // Status
    if (url === '/status') {
      const stats = transport.getStats();
      const gossipStats = gossip.getStats();
      const cState = consensus.getState();
      res.writeHead(200);
      return res.end(JSON.stringify({
        nodeId,
        uptime: Date.now() - startTime,
        transport: {
          connections: stats.connections,
          messagesSent: stats.messagesSent,
          messagesReceived: stats.messagesReceived,
        },
        gossip: { total: gossipStats.total, active: gossipStats.active },
        consensus: cState,
      }));
    }

    // Peers
    if (url === '/peers') {
      const peers = transport.getConnectedPeers();
      res.writeHead(200);
      return res.end(JSON.stringify({
        count: peers.length,
        peers: peers.map(p => ({ id: p.slice(0, 24) + '...' })),
      }));
    }

    // Consensus
    if (url === '/consensus') {
      const cStats = consensusGossip.getStats();
      const cState = consensus.getState();
      res.writeHead(200);
      return res.end(JSON.stringify({ state: cState, bridge: cStats }));
    }

    // 404
    res.writeHead(404);
    res.end(JSON.stringify({ error: 'Not Found', path: url }));
  };

  // Create transport
  const transport = new WebSocketTransport({
    port,
    host,
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    heartbeatInterval: 61800, // φ-aligned
    httpHandler, // HTTP API on same port as WS
  });

  // Create gossip protocol
  const gossip = new GossipProtocol({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    address: `${host}:${port}`,
    sendFn: transport.getSendFn(),
    onMessage: (message) => {
      if (message.type !== 'HEARTBEAT' && verbose) {
        console.log(chalk.blue(`  [MSG] `) + chalk.gray(`${message.type}: ${JSON.stringify(message.payload).slice(0, 60)}...`));
      }
    },
  });

  // Create consensus engine (Layer 4: φ-BFT)
  const consensus = new ConsensusEngine({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    eScore: 0.5, // Default E-Score
    burned: 0, // Default burn
    uptime: 1.0, // Full uptime initially
  });

  // Create slot manager for leader selection
  const slotManager = new SlotManager();

  // Create consensus-gossip bridge
  const consensusGossip = new ConsensusGossip({
    consensus,
    gossip,
  });

  // Wire consensus events
  consensus.on('block:proposed', (event) => {
    if (verbose) {
      console.log(chalk.yellow(`  [PROP] `) + `Block proposed: ${chalk.cyan(event.blockHash.slice(0, 16))}... slot ${event.slot}`);
    }
  });

  consensus.on('block:finalized', (event) => {
    console.log(chalk.green(`  [FIN]  `) + `Block finalized: ${chalk.cyan(event.blockHash.slice(0, 16))}...`);
  });

  consensus.on('vote:cast', (event) => {
    if (verbose) {
      console.log(chalk.magenta(`  [VOTE] `) + `Voted ${event.decision} on ${chalk.cyan(event.blockHash.slice(0, 16))}...`);
    }
  });

  consensusGossip.on('error', ({ source, error }) => {
    console.log(chalk.red(`  [C-ERR] `) + `${source}: ${error}`);
  });

  // Wire events
  transport.on('peer:connected', ({ peerId, publicKey, inbound }) => {
    const direction = inbound ? chalk.magenta('←') : chalk.green('→');
    const id = (publicKey || peerId || '').slice(0, 12);
    console.log(chalk.green('  [PEER] ') + `${direction} Connected: ${chalk.cyan(id)}...`);
    if (publicKey) {
      gossip.addPeer(createPeerInfo({ publicKey, address: '' }));
    }
  });

  transport.on('peer:identified', ({ publicKey }) => {
    if (verbose) {
      console.log(chalk.blue('  [ID]   ') + `Verified: ${chalk.cyan(publicKey.slice(0, 12))}...`);
    }
  });

  transport.on('peer:disconnected', ({ peerId }) => {
    const id = (peerId || '').slice(0, 12);
    console.log(chalk.red('  [PEER] ') + `Disconnected: ${chalk.gray(id)}...`);
  });

  transport.on('peer:error', ({ error }) => {
    console.log(chalk.red('  [ERR]  ') + error.message);
  });

  // Start server
  if (startServer) {
    try {
      await transport.startServer();
      console.log(chalk.green('  [OK]   ') + `Server listening on ${chalk.bold(`${host}:${port}`)}`);

      // Start consensus-gossip bridge
      consensusGossip.start();
      console.log(chalk.green('  [OK]   ') + `Consensus bridge started`);
    } catch (err) {
      console.error(chalk.red('  [FAIL] ') + `Could not start server: ${err.message}`);
      process.exit(1);
    }
  }

  // Connect to initial peers
  if (options.connect && options.connect.length > 0) {
    console.log(chalk.gray('\n  Connecting to peers...'));
    for (const address of options.connect) {
      try {
        const wsAddress = address.startsWith('ws') ? address : `ws://${address}`;
        await transport.connect({
          id: `peer_${Date.now()}`,
          address: wsAddress,
        });
        console.log(chalk.green('  [OK]   ') + `Connected to ${chalk.cyan(address)}`);
      } catch (err) {
        console.log(chalk.red('  [FAIL] ') + `Could not connect to ${address}: ${err.message}`);
      }
    }
  }

  // Track start time
  const startTime = Date.now();

  // Daemon mode: keep alive without interactive REPL
  if (options.daemon) {
    console.log(chalk.green('\n  [OK]   ') + `Running in daemon mode (no interactive REPL)`);
    console.log(chalk.gray('  ─────────────────────────────────────────────────\n'));

    // Keep process alive - the server event loop handles everything
    // Log heartbeat every φ minutes (61.8 seconds) in verbose mode
    if (verbose) {
      setInterval(() => {
        const stats = transport.getStats();
        const gossipStats = gossip.getStats();
        const uptime = Date.now() - startTime;
        console.log(
          chalk.gray(`  [♥] `) +
          `uptime=${formatUptime(uptime)} ` +
          `peers=${stats.connections.connected} ` +
          `msgs=${stats.messagesSent}/${stats.messagesReceived} ` +
          `gossip=${gossipStats.active}/${gossipStats.total}`
        );
      }, 61800); // φ-aligned heartbeat
    }
  }

  // HTTP API is now served on the same port as WebSocket (via httpHandler)
  // No separate API server needed

  if (!options.daemon) {
    // Interactive mode
    console.log(chalk.gray('\n  ─────────────────────────────────────────────────'));
    console.log(chalk.bold('  Commands:'));
    console.log(chalk.gray('    /peers    ') + 'List connected peers');
    console.log(chalk.gray('    /stats    ') + 'Show statistics');
    console.log(chalk.gray('    /slot     ') + 'Show current slot info');
    console.log(chalk.gray('    /consensus') + 'Show consensus status');
    console.log(chalk.gray('    /connect <addr>  ') + 'Connect to peer');
    console.log(chalk.gray('    /broadcast <msg> ') + 'Broadcast message');
    console.log(chalk.gray('    /quit     ') + 'Shutdown node');
    console.log(chalk.gray('  ─────────────────────────────────────────────────\n'));

    // Setup readline for interactive commands
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
            const peers = transport.getConnectedPeers();
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
            const stats = transport.getStats();
            const gossipStats = gossip.getStats();
            const uptime = Date.now() - startTime;

            console.log(chalk.bold('\n  Node Statistics:'));
            console.log(chalk.gray('    Uptime:       ') + formatUptime(uptime));
            console.log(chalk.gray('    Connections:  ') + `${stats.connections.connected} active, ${stats.connections.connecting} pending`);
            console.log(chalk.gray('    Messages:     ') + `${stats.messagesSent} sent, ${stats.messagesReceived} received`);
            console.log(chalk.gray('    Bandwidth:    ') + `${formatBytes(stats.bytesOut)} out, ${formatBytes(stats.bytesIn)} in`);
            console.log(chalk.gray('    Gossip Peers: ') + `${gossipStats.total} (${gossipStats.active} active)`);
            console.log();
            break;
          }

          case 'slot': {
            const slotInfo = slotManager.getSlotInfo();
            console.log(chalk.bold('\n  Slot Information:'));
            console.log(chalk.gray('    Current Slot:     ') + chalk.yellow(slotInfo.slot));
            console.log(chalk.gray('    Epoch:            ') + slotInfo.epoch);
            console.log(chalk.gray('    Slot in Epoch:    ') + `${slotInfo.slotInEpoch}/${slotInfo.slotsPerEpoch}`);
            console.log(chalk.gray('    Time to Next:     ') + `${slotInfo.msUntilNext}ms`);
            console.log();
            break;
          }

          case 'consensus': {
            const cStats = consensusGossip.getStats();
            const cState = consensus.getState();
            console.log(chalk.bold('\n  Consensus Status:'));
            console.log(chalk.gray('    State:            ') + chalk.yellow(cState.state));
            console.log(chalk.gray('    Latest Slot:      ') + cState.latestSlot);
            console.log(chalk.gray('    Finalized Slot:   ') + cState.finalizedSlot);
            console.log(chalk.gray('    Pending Blocks:   ') + cState.pendingBlocks);
            console.log(chalk.bold('  Bridge Statistics:'));
            console.log(chalk.gray('    Proposals:        ') + `${cStats.proposalsBroadcast} sent, ${cStats.proposalsReceived} received`);
            console.log(chalk.gray('    Votes:            ') + `${cStats.votesBroadcast} sent, ${cStats.votesReceived} received`);
            console.log(chalk.gray('    Finality:         ') + `${cStats.finalityBroadcast} sent, ${cStats.finalityReceived} received`);
            console.log();
            break;
          }

          case 'connect': {
            if (args.length === 0) {
              console.log(chalk.red('  Usage: /connect <address>'));
            } else {
              const address = args[0].startsWith('ws') ? args[0] : `ws://${args[0]}`;
              try {
                await transport.connect({ id: `peer_${Date.now()}`, address });
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
              const sent = await gossip.broadcastJudgment(judgment);
              console.log(chalk.green(`  Broadcast to ${sent} peer(s)`));
            }
            break;
          }

          case 'quit':
          case 'exit':
          case 'q': {
            console.log(chalk.yellow('\n  Shutting down...'));
            consensusGossip.stop();
            await transport.stopServer();
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
      consensusGossip.stop();
      await transport.stopServer();
      process.exit(0);
    });
  }

  // Graceful shutdown
  process.on('SIGINT', async () => {
    console.log(chalk.yellow('\n  Received SIGINT, shutting down...'));
    consensusGossip.stop();
    await transport.stopServer();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    consensusGossip.stop();
    await transport.stopServer();
    process.exit(0);
  });
}

export default { startCommand };
