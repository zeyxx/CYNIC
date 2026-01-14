#!/usr/bin/env node
/**
 * φ-BFT Consensus Stress Test
 *
 * Tests consensus with larger validator sets:
 * - 7 validators (can tolerate 2 Byzantine)
 * - Multiple concurrent block proposals
 * - High message throughput
 * - Memory and timing metrics
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
  selectLeader,
} from '@cynic/protocol';

const NUM_VALIDATORS = 7;
const BASE_PORT = 23000;
const NUM_BLOCKS = 10;

// Generate node names
const NODE_NAMES = Array.from({ length: NUM_VALIDATORS }, (_, i) => `Node${i + 1}`);

async function createNode(index, allKeypairs) {
  const name = NODE_NAMES[index];
  const port = BASE_PORT + index;
  const keypair = allKeypairs[index];

  const transport = new WebSocketTransport({
    port,
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    heartbeatInterval: 60000,
  });

  const gossip = new GossipProtocol({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    address: `localhost:${port}`,
    sendFn: transport.getSendFn(),
    onMessage: () => {},
  });

  const consensus = new ConsensusEngine({
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    eScore: 0.6,
    burned: 100 + index * 10, // Varying weights
    uptime: 1.0,
    confirmationsForFinality: 3,
  });

  // Register all validators
  for (let i = 0; i < allKeypairs.length; i++) {
    consensus.registerValidator({
      publicKey: allKeypairs[i].publicKey,
      eScore: 0.6,
      burned: 100 + i * 10,
      uptime: 1.0,
    });
  }

  // Create validator list for leader selection
  const validatorList = allKeypairs.map((kp, i) => ({
    id: kp.publicKey,
    weight: 100 + i * 10,
  }));

  const slotManager = new SlotManager({ genesisTime: Date.now() - 10000 });
  slotManager.setValidators(validatorList);

  const consensusGossip = new ConsensusGossip({
    consensus,
    gossip,
    autoSync: false,
  });

  // Wire transport
  transport.on('message', ({ message, peerId }) => {
    gossip.handleMessage(message, peerId);
  });

  transport.on('peer:connected', ({ publicKey }) => {
    if (publicKey) {
      gossip.addPeer(createPeerInfo({ publicKey, address: '' }));
    }
  });

  transport.on('peer:identified', ({ publicKey }) => {
    gossip.addPeer(createPeerInfo({ publicKey, address: '' }));
  });

  await transport.startServer();
  consensusGossip.start();
  consensus.start();

  return {
    transport,
    gossip,
    consensus,
    consensusGossip,
    slotManager,
    keypair,
    port,
    name,
    index,
    validatorList,
  };
}

function createTestBlock(proposer, slot, blockNum) {
  const block = {
    type: 'JUDGMENT',
    slot,
    timestamp: Date.now(),
    previous_hash: '0'.repeat(64),
    proposer: proposer.publicKey,
    judgments: [{
      id: `jdg_stress_${blockNum}`,
      itemHash: `item_${blockNum}`,
      globalScore: 70 + (blockNum % 30),
      verdict: 'HOWL',
    }],
    merkle_root: '0'.repeat(64),
  };
  block.hash = hashBlock(block);
  return block;
}

async function main() {
  console.log('═══════════════════════════════════════════════════');
  console.log('   CYNIC φ-BFT Stress Test');
  console.log(`   ${NUM_VALIDATORS} Validators, ${NUM_BLOCKS} Blocks`);
  console.log('═══════════════════════════════════════════════════\n');

  const startTime = Date.now();
  const memBefore = process.memoryUsage();

  // Generate all keypairs
  console.log(`[Setup] Generating ${NUM_VALIDATORS} validator keypairs...`);
  const allKeypairs = Array.from({ length: NUM_VALIDATORS }, () => generateKeypair());

  // Create all nodes
  console.log(`[Setup] Starting ${NUM_VALIDATORS} nodes...`);
  const nodes = [];
  for (let i = 0; i < NUM_VALIDATORS; i++) {
    const node = await createNode(i, allKeypairs);
    nodes.push(node);
    process.stdout.write(`  ${node.name} (port ${node.port})\r`);
  }
  console.log(`\n[Setup] All ${NUM_VALIDATORS} nodes started`);

  // Connect in full mesh
  console.log('[Setup] Connecting full mesh...');
  let connectionCount = 0;
  for (let i = 0; i < NUM_VALIDATORS; i++) {
    for (let j = i + 1; j < NUM_VALIDATORS; j++) {
      await nodes[i].transport.connect({
        id: nodes[j].keypair.publicKey,
        address: `ws://localhost:${BASE_PORT + j}`,
      });
      connectionCount++;
    }
  }
  await new Promise(r => setTimeout(r, 1000));
  console.log(`[Setup] Mesh connected (${connectionCount} connections)\n`);

  // Stats tracking
  const blockStats = [];
  let totalVotes = 0;
  let finalizedCount = 0;
  let confirmedCount = 0;

  // Event tracking
  for (const node of nodes) {
    node.consensus.on('block:finalized', () => finalizedCount++);
    node.consensus.on('block:confirmed', () => confirmedCount++);
    node.consensus.on('vote:cast', () => totalVotes++);
  }

  // ═══════════════════════════════════════════════════════════════
  // Stress Test: Multiple Block Proposals
  // ═══════════════════════════════════════════════════════════════
  console.log('─────────────────────────────────────────────────────');
  console.log('Stress Test: Block Proposals');
  console.log('─────────────────────────────────────────────────────\n');

  const baseSlot = nodes[0].slotManager.getCurrentSlot();
  const proposedBlocks = [];

  for (let i = 0; i < NUM_BLOCKS; i++) {
    const slot = baseSlot + i;

    // Select leader for this slot
    const leaderId = selectLeader(slot, nodes[0].validatorList);
    const leaderIndex = allKeypairs.findIndex(kp => kp.publicKey === leaderId);
    const leaderNode = nodes[leaderIndex];

    const blockStart = Date.now();
    const block = createTestBlock(leaderNode.keypair, slot, i + 1);

    // Propose block
    leaderNode.consensus.proposeBlock(block);
    await leaderNode.consensusGossip.proposeBlock(block);

    proposedBlocks.push({
      slot,
      hash: block.hash,
      leader: leaderNode.name,
      startTime: blockStart,
    });

    // Brief delay between blocks
    await new Promise(r => setTimeout(r, 400));
  }

  // Wait for consensus to settle
  console.log(`[Test] Waiting for consensus on ${NUM_BLOCKS} blocks...`);
  await new Promise(r => setTimeout(r, 3000));

  // ═══════════════════════════════════════════════════════════════
  // Verification
  // ═══════════════════════════════════════════════════════════════
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Verification');
  console.log('─────────────────────────────────────────────────────\n');

  let allFinalized = true;
  let totalLatency = 0;
  const leaderDistribution = {};

  for (const pb of proposedBlocks) {
    // Check status on first node
    const blockRecord = nodes[0].consensus.getBlock(pb.hash);
    const status = blockRecord?.status || 'unknown';
    const votes = blockRecord?.votes?.size || 0;
    const finalized = status === 'FINALIZED';
    const latency = Date.now() - pb.startTime;

    if (!finalized) allFinalized = false;
    totalLatency += latency;

    // Track leader distribution
    leaderDistribution[pb.leader] = (leaderDistribution[pb.leader] || 0) + 1;

    blockStats.push({ slot: pb.slot, status, votes, latency, leader: pb.leader });

    const statusIcon = finalized ? '✅' : status === 'CONFIRMED' ? '⚠️' : '❌';
    console.log(`Slot ${pb.slot} [${pb.leader}]: ${status} (${votes}/${NUM_VALIDATORS} votes) ${statusIcon}`);
  }

  // ═══════════════════════════════════════════════════════════════
  // Statistics
  // ═══════════════════════════════════════════════════════════════
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Performance Statistics');
  console.log('─────────────────────────────────────────────────────\n');

  const endTime = Date.now();
  const memAfter = process.memoryUsage();

  const avgLatency = totalLatency / NUM_BLOCKS;
  const blocksPerSecond = NUM_BLOCKS / ((endTime - startTime) / 1000);
  const memUsedMB = (memAfter.heapUsed - memBefore.heapUsed) / 1024 / 1024;

  console.log(`Validators:          ${NUM_VALIDATORS}`);
  console.log(`Blocks proposed:     ${NUM_BLOCKS}`);
  console.log(`Total runtime:       ${((endTime - startTime) / 1000).toFixed(2)}s`);
  console.log(`Avg block latency:   ${avgLatency.toFixed(0)}ms`);
  console.log(`Throughput:          ${blocksPerSecond.toFixed(2)} blocks/sec`);
  console.log(`Total votes cast:    ${totalVotes}`);
  console.log(`Memory delta:        ${memUsedMB.toFixed(2)} MB`);

  console.log('\nLeader Distribution:');
  for (const [leader, count] of Object.entries(leaderDistribution)) {
    const pct = ((count / NUM_BLOCKS) * 100).toFixed(0);
    console.log(`  ${leader}: ${count} blocks (${pct}%)`);
  }

  // Check consensus across all nodes
  console.log('\n─────────────────────────────────────────────────────');
  console.log('Node Consensus State');
  console.log('─────────────────────────────────────────────────────\n');

  let consensusAgreement = true;
  const firstNodeBlocks = new Set();
  for (const pb of proposedBlocks) {
    const record = nodes[0].consensus.getBlock(pb.hash);
    if (record?.status === 'FINALIZED') {
      firstNodeBlocks.add(pb.hash);
    }
  }

  for (const node of nodes) {
    const state = node.consensus.getState();
    const finalizedHashes = new Set();
    for (const pb of proposedBlocks) {
      const record = node.consensus.getBlock(pb.hash);
      if (record?.status === 'FINALIZED') {
        finalizedHashes.add(pb.hash);
      }
    }

    const matches = [...firstNodeBlocks].every(h => finalizedHashes.has(h));
    if (!matches) consensusAgreement = false;

    console.log(`[${node.name}] State: ${state.state}, Finalized: ${finalizedHashes.size}/${NUM_BLOCKS} ${matches ? '✅' : '❌'}`);
  }

  // Cleanup
  console.log('\n─────────────────────────────────────────────────────');
  console.log('[Test] Shutting down...');

  for (const node of nodes) {
    node.consensus.stop();
    node.consensusGossip.stop();
    await node.transport.stopServer();
  }

  // Final result
  const success = allFinalized && consensusAgreement;

  if (success) {
    console.log('\n═══════════════════════════════════════════════════');
    console.log('   ✅ STRESS TEST PASSED');
    console.log(`   - ${NUM_VALIDATORS} validators in full mesh`);
    console.log(`   - ${NUM_BLOCKS} blocks finalized`);
    console.log(`   - ${blocksPerSecond.toFixed(2)} blocks/sec throughput`);
    console.log(`   - All nodes agree on finalized set`);
    console.log('═══════════════════════════════════════════════════\n');
  } else {
    const finalizedPct = (blockStats.filter(b => b.status === 'FINALIZED').length / NUM_BLOCKS * 100).toFixed(0);
    console.log('\n═══════════════════════════════════════════════════');
    console.log('   ⚠️  PARTIAL SUCCESS');
    console.log(`   - Finalized: ${finalizedPct}%`);
    console.log(`   - Agreement: ${consensusAgreement ? 'YES' : 'NO'}`);
    console.log('═══════════════════════════════════════════════════\n');
  }
}

main().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});
