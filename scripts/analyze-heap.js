/**
 * Analyze heap snapshots to find memory leaks
 *
 * Compares two .heapsnapshot files and identifies what grew
 */

import fs from 'fs';
import path from 'path';

function parseSnapshot(filepath) {
  console.log(`Parsing ${filepath}...`);
  const raw = fs.readFileSync(filepath, 'utf8');
  const snapshot = JSON.parse(raw);

  // Extract node types and sizes
  const { nodes, strings, snapshot: meta } = snapshot;

  // Node format: [type, name, id, self_size, edge_count, trace_node_id, detachedness]
  const nodeFieldCount = meta.meta.node_fields.length;
  const nodeTypeOffset = meta.meta.node_fields.indexOf('type');
  const nodeNameOffset = meta.meta.node_fields.indexOf('name');
  const nodeSelfSizeOffset = meta.meta.node_fields.indexOf('self_size');

  const stats = new Map();

  for (let i = 0; i < nodes.length; i += nodeFieldCount) {
    const typeIdx = nodes[i + nodeTypeOffset];
    const nameIdx = nodes[i + nodeNameOffset];
    const selfSize = nodes[i + nodeSelfSizeOffset];

    const typeName = strings[typeIdx] || '(unknown)';
    const nodeName = strings[nameIdx] || '(anonymous)';

    const key = `${typeName}:${nodeName}`;

    if (!stats.has(key)) {
      stats.set(key, { count: 0, size: 0, type: typeName, name: nodeName });
    }

    const entry = stats.get(key);
    entry.count++;
    entry.size += selfSize;
  }

  return { stats, totalNodes: nodes.length / nodeFieldCount };
}

function compareSnapshots(snapshot1Path, snapshot2Path) {
  const s1 = parseSnapshot(snapshot1Path);
  const s2 = parseSnapshot(snapshot2Path);

  console.log(`\nSnapshot 1: ${s1.totalNodes} nodes`);
  console.log(`Snapshot 2: ${s2.totalNodes} nodes`);

  // Find what grew
  const growth = [];

  for (const [key, s2entry] of s2.stats.entries()) {
    const s1entry = s1.stats.get(key) || { count: 0, size: 0 };

    const countDelta = s2entry.count - s1entry.count;
    const sizeDelta = s2entry.size - s1entry.size;

    if (sizeDelta > 0) {
      growth.push({
        type: s2entry.type,
        name: s2entry.name,
        countDelta,
        sizeDelta,
        sizePercent: s1entry.size === 0 ? Infinity : (sizeDelta / s1entry.size) * 100,
        finalCount: s2entry.count,
        finalSize: s2entry.size,
      });
    }
  }

  // Sort by size delta (biggest leaks first)
  growth.sort((a, b) => b.sizeDelta - a.sizeDelta);

  console.log(`\n=== TOP 20 MEMORY GROWTH ===\n`);
  console.log('Type'.padEnd(20), 'Name'.padEnd(30), 'Count Δ'.padStart(10), 'Size Δ (MB)'.padStart(12), 'Final (MB)'.padStart(12));
  console.log('-'.repeat(100));

  for (let i = 0; i < Math.min(20, growth.length); i++) {
    const g = growth[i];
    console.log(
      g.type.padEnd(20),
      g.name.slice(0, 30).padEnd(30),
      g.countDelta.toString().padStart(10),
      (g.sizeDelta / 1024 / 1024).toFixed(2).padStart(12),
      (g.finalSize / 1024 / 1024).toFixed(2).padStart(12)
    );
  }

  // Summary
  const totalGrowth = growth.reduce((sum, g) => sum + g.sizeDelta, 0);
  console.log(`\n=== SUMMARY ===`);
  console.log(`Total memory growth: ${(totalGrowth / 1024 / 1024).toFixed(2)} MB`);
  console.log(`Top suspect: ${growth[0]?.type}:${growth[0]?.name} (+${(growth[0]?.sizeDelta / 1024 / 1024).toFixed(2)} MB)`);

  return growth;
}

// Main
const [,, snapshot1, snapshot2] = process.argv;

if (!snapshot1 || !snapshot2) {
  console.error('Usage: node analyze-heap.js <snapshot1.heapsnapshot> <snapshot2.heapsnapshot>');
  process.exit(1);
}

if (!fs.existsSync(snapshot1)) {
  console.error(`Snapshot 1 not found: ${snapshot1}`);
  process.exit(1);
}

if (!fs.existsSync(snapshot2)) {
  console.error(`Snapshot 2 not found: ${snapshot2}`);
  process.exit(1);
}

compareSnapshots(snapshot1, snapshot2);
