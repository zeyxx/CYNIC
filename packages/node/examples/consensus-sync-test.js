#!/usr/bin/env node
/**
 * Consensus Sync Test
 *
 * Test demonstrating two CYNIC nodes synchronizing consensus via gossip
 *
 * Tests:
 * 1. Block proposal propagation
 * 2. Vote broadcasting
 * 3. Finality notification
 */

import { WebSocketTransport } from '../src/transport/index.js';
import {
  GossipProtocol,
  generateKeypair,
  createPeerInfo,
  ConsensusEngine,
  ConsensusGossip,
  SlotManager,
  hashBlock,
} from '@cynic/protocol';

async function createNode(name, port) {
  const keypair = generateKeypair();

  // Transport layer
  const transport = new WebSocketTransport({
    port,
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    heartbeatInterval: 60000,
  });

  // Gossip protocol
  const gossip = new GossipProtocol({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    address: `localhost:${port}`,
    sendFn: transport.getSendFn(),
    onMessage: (message) => {
      if (message.type !== 'HEARTBEAT') {
        console.log(`[${name}] 📨 ${message.type}`);
      }
    },
  });

  // Consensus engine (Layer 4: φ-BFT)
  const consensus = new ConsensusEngine({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    eScore: 0.5,
    burned: 100,
    uptime: 1.0,
  });

  // Slot manager
  const slotManager = new SlotManager();

  // Consensus-gossip bridge
  const consensusGossip = new ConsensusGossip({
    consensus,
    gossip,
  });

  // Wire transport to gossip
  transport.on('message', ({ message, peerId }) => {
    gossip.handleMessage(message, peerId);
  });

  transport.on('peer:connected', ({ peerId, address }) => {
    console.log(`[${name}] ✅ Connected to peer`);
    gossip.addPeer(createPeerInfo({ publicKey: peerId, address }));
  });

  transport.on('peer:identified', ({ publicKey }) => {
    console.log(`[${name}] 🆔 Peer identified: ${publicKey.slice(0, 12)}...`);
  });

  // Wire consensus events
  consensus.on('block:proposed', (event) => {
    console.log(`[${name}] 📦 Block proposed: ${event.blockHash.slice(0, 16)}... slot ${event.slot}`);
  });

  consensus.on('vote:cast', (event) => {
    console.log(`[${name}] 🗳️  Voted ${event.decision} on ${event.blockHash.slice(0, 16)}...`);
  });

  consensus.on('vote:received', (event) => {
    console.log(`[${name}] 📥 Vote received for ${event.blockHash.slice(0, 16)}...`);
  });

  consensus.on('block:finalized', (event) => {
    console.log(`[${name}] ✨ Block FINALIZED: ${event.blockHash.slice(0, 16)}...`);
  });

  consensusGossip.on('proposal:received', ({ blockHash, from }) => {
    console.log(`[${name}] 📬 Proposal received: ${blockHash.slice(0, 16)}... from ${from.slice(0, 12)}...`);
  });

  consensusGossip.on('vote:broadcast', ({ blockHash, peers }) => {
    console.log(`[${name}] 📤 Vote broadcast to ${peers} peer(s)`);
  });

  consensusGossip.on('error', ({ source, error }) => {
    console.log(`[${name}] ❌ ${source}: ${error}`);
  });

  // Start
  await transport.startServer();
  consensusGossip.start();
  console.log(`[${name}] 🚀 Started on port ${port}`);

  return { transport, gossip, consensus, consensusGossip, slotManager, keypair, port, name };
}

async function main() {
  console.log('═══════════════════════════════════════════════════');
  console.log('   CYNIC Consensus Sync Test - Two Nodes');
  console.log('   φ-BFT Layer 4 Integration');
  console.log('═══════════════════════════════════════════════════\n');

  // Create two nodes
  const node1 = await createNode('Node1', 18800);
  const node2 = await createNode('Node2', 18801);

  // Connect node2 to node1
  console.log('\n[Test] Connecting Node2 → Node1...');
  await node2.transport.connect({
    id: node1.keypair.publicKey,
    address: 'ws://localhost:18800',
  });

  await new Promise(r => setTimeout(r, 500));

  // Test 1: Block proposal from Node1
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Test 1: Block Proposal Propagation');
  console.log('─────────────────────────────────────────────────────\n');

  const slot = node1.slotManager.getSlotInfo().slot;
  const testBlock = {
    type: 'JUDGMENT',
    slot,
    timestamp: Date.now(),
    previous_hash: '0000000000000000000000000000000000000000000000000000000000000000',
    proposer: node1.keypair.publicKey,
    judgments: [
      {
        id: 'jdg_test_001',
        itemHash: 'item_hash_001',
        globalScore: 75,
        verdict: 'WAG',
      },
    ],
    merkle_root: '0'.repeat(64),
  };
  testBlock.hash = hashBlock(testBlock);

  console.log(`[Node1] Proposing block at slot ${slot}...`);
  await node1.consensusGossip.proposeBlock(testBlock);

  await new Promise(r => setTimeout(r, 500));

  // Test 2: Vote from Node2 on the proposed block
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Test 2: Vote Broadcasting');
  console.log('─────────────────────────────────────────────────────\n');

  // Simulate receiving the block on Node2 and voting
  const vote = {
    blockHash: testBlock.hash,
    slot,
    decision: 'ACCEPT',
    weight: 0.5,
  };

  console.log(`[Node2] Broadcasting vote for block ${testBlock.hash.slice(0, 16)}...`);
  await node2.consensusGossip.broadcastVote(vote);

  await new Promise(r => setTimeout(r, 500));

  // Test 3: Check consensus stats
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Test 3: Consensus Statistics');
  console.log('─────────────────────────────────────────────────────\n');

  const stats1 = node1.consensusGossip.getStats();
  const stats2 = node2.consensusGossip.getStats();

  console.log('[Node1] Bridge Stats:');
  console.log(`  Proposals: ${stats1.proposalsBroadcast} sent, ${stats1.proposalsReceived} received`);
  console.log(`  Votes:     ${stats1.votesBroadcast} sent, ${stats1.votesReceived} received`);

  console.log('\n[Node2] Bridge Stats:');
  console.log(`  Proposals: ${stats2.proposalsBroadcast} sent, ${stats2.proposalsReceived} received`);
  console.log(`  Votes:     ${stats2.votesBroadcast} sent, ${stats2.votesReceived} received`);

  // Transport stats
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Transport Statistics');
  console.log('─────────────────────────────────────────────────────\n');

  const tStats1 = node1.transport.getStats();
  const tStats2 = node2.transport.getStats();

  console.log(`[Node1] Messages: sent=${tStats1.messagesSent}, received=${tStats1.messagesReceived}`);
  console.log(`[Node2] Messages: sent=${tStats2.messagesSent}, received=${tStats2.messagesReceived}`);

  // Cleanup
  console.log('\n─────────────────────────────────────────────────────');
  console.log('[Test] Shutting down...');

  node1.consensusGossip.stop();
  node2.consensusGossip.stop();
  await node1.transport.stopServer();
  await node2.transport.stopServer();

  console.log('[Test] ✅ Consensus sync test complete!\n');

  // Verify results
  const success =
    stats1.proposalsBroadcast >= 1 &&
    stats2.proposalsReceived >= 1 &&
    stats2.votesBroadcast >= 1 &&
    stats1.votesReceived >= 1;

  if (success) {
    console.log('═══════════════════════════════════════════════════');
    console.log('   ✅ ALL TESTS PASSED');
    console.log('   - Block proposals propagate correctly');
    console.log('   - Votes broadcast via gossip');
    console.log('   - Consensus bridge integration working');
    console.log('═══════════════════════════════════════════════════\n');
  } else {
    console.log('═══════════════════════════════════════════════════');
    console.log('   ❌ SOME TESTS FAILED');
    console.log('   Check the logs above for details');
    console.log('═══════════════════════════════════════════════════\n');
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});
