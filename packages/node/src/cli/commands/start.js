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
import { GossipProtocol, generateKeypair, createPeerInfo } from '@cynic/protocol';

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
  const nodeId = keypair.publicKey.slice(0, 16);

  console.log(chalk.gray('  Node ID: ') + chalk.yellow(nodeId + '...'));
  console.log(chalk.gray('  Port:    ') + chalk.white(port));
  console.log(chalk.gray('  Host:    ') + chalk.white(host));
  console.log();

  // Create transport
  const transport = new WebSocketTransport({
    port,
    host,
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    heartbeatInterval: 61800, // φ-aligned
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

  // Interactive mode
  console.log(chalk.gray('\n  ─────────────────────────────────────────────────'));
  console.log(chalk.bold('  Commands:'));
  console.log(chalk.gray('    /peers    ') + 'List connected peers');
  console.log(chalk.gray('    /stats    ') + 'Show statistics');
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
    await transport.stopServer();
    process.exit(0);
  });

  // Graceful shutdown
  process.on('SIGINT', async () => {
    console.log(chalk.yellow('\n  Received SIGINT, shutting down...'));
    await transport.stopServer();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await transport.stopServer();
    process.exit(0);
  });
}

export default { startCommand };
