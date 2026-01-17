/**
 * Merkle DAG Tests
 *
 * @module @cynic/persistence/test/dag
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

import {
  createCID,
  parseCID,
  isValidCID,
  shardCID,
  DAGNode,
  DAGLink,
  NodeType,
  createJudgmentNode,
  BlockStore,
  HAMTIndex,
  MerkleDAG,
} from '../src/dag/index.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TEST_DATA_DIR = path.join(__dirname, '../.test-data');

describe('CID Generation', () => {
  it('should create a valid CID from data', () => {
    const data = Buffer.from('hello world');
    const cid = createCID(data);

    assert.ok(cid.startsWith('b'), 'CID should start with base32 prefix');
    assert.ok(cid.length > 10, 'CID should be reasonably long');
  });

  it('should create deterministic CIDs', () => {
    const data = Buffer.from('test data');
    const cid1 = createCID(data);
    const cid2 = createCID(data);

    assert.strictEqual(cid1, cid2, 'Same data should produce same CID');
  });

  it('should parse a CID', () => {
    const data = Buffer.from('test');
    const cid = createCID(data);
    const parsed = parseCID(cid);

    assert.strictEqual(parsed.version, 1, 'Should be CID v1');
    assert.ok(parsed.digest, 'Should have digest');
  });

  it('should validate CIDs', () => {
    const data = Buffer.from('test');
    const cid = createCID(data);

    assert.ok(isValidCID(cid), 'Valid CID should validate');
    assert.ok(!isValidCID('invalid'), 'Invalid string should not validate');
    assert.ok(!isValidCID(''), 'Empty string should not validate');
  });

  it('should shard CIDs for storage', () => {
    const data = Buffer.from('test');
    const cid = createCID(data);
    const { prefix, suffix } = shardCID(cid);

    assert.strictEqual(prefix.length, 2, 'Prefix should be 2 chars');
    assert.ok(suffix.length > 0, 'Suffix should not be empty');
    assert.strictEqual('b' + prefix + suffix, cid, 'Should reconstruct CID');
  });
});

describe('DAG Node', () => {
  it('should create a node with data and links', () => {
    const node = new DAGNode({
      type: NodeType.JUDGMENT,
      data: { qScore: 75, verdict: 'WAG' },
      links: [],
    });

    assert.strictEqual(node.type, NodeType.JUDGMENT);
    assert.strictEqual(node.data.qScore, 75);
    assert.ok(node.cid, 'Node should have a CID');
  });

  it('should encode and decode nodes', () => {
    const node = new DAGNode({
      type: NodeType.ENTITY,
      data: { name: 'test', value: 42 },
    });

    const encoded = node.encode();
    assert.ok(Buffer.isBuffer(encoded), 'Encoded should be a buffer');

    const decoded = DAGNode.decode(encoded);
    assert.strictEqual(decoded.type, node.type);
    assert.deepStrictEqual(decoded.data, node.data);
  });

  it('should add links to nodes', () => {
    const child = new DAGNode({
      type: NodeType.ENTITY,
      data: { name: 'child' },
    });

    const parent = new DAGNode({
      type: NodeType.ENTITY,
      data: { name: 'parent' },
    });

    parent.addLink(child, 'child');

    assert.strictEqual(parent.links.length, 1);
    assert.strictEqual(parent.links[0].name, 'child');
    assert.strictEqual(parent.links[0].cid, child.cid);
  });

  it('should create judgment nodes', () => {
    const judgment = {
      id: 'jdg_test123',
      itemType: 'token',
      qScore: 85,
      verdict: 'WAG',
      confidence: 0.618,
      dimensions: { coherence: 90, clarity: 80 },
    };

    const node = createJudgmentNode(judgment);

    assert.strictEqual(node.type, NodeType.JUDGMENT);
    assert.strictEqual(node.data.id, 'jdg_test123');
    assert.strictEqual(node.data.qScore, 85);
  });
});

describe('Block Store', async () => {
  let store;

  before(async () => {
    await fs.mkdir(TEST_DATA_DIR, { recursive: true });
    store = new BlockStore({ basePath: path.join(TEST_DATA_DIR, 'blocks') });
    await store.init();
  });

  after(async () => {
    await fs.rm(TEST_DATA_DIR, { recursive: true, force: true });
  });

  it('should store and retrieve blocks', async () => {
    const node = new DAGNode({
      type: NodeType.ENTITY,
      data: { test: true },
    });

    const cid = await store.putNode(node);
    const retrieved = await store.getNode(cid);

    assert.strictEqual(retrieved.type, node.type);
    assert.deepStrictEqual(retrieved.data, node.data);
  });

  it('should check if blocks exist', async () => {
    const node = new DAGNode({
      type: NodeType.PATTERN,
      data: { name: 'test-pattern' },
    });

    const cid = await store.putNode(node);

    assert.ok(await store.has(cid), 'Should find stored block');
    assert.ok(!(await store.has('binvalidcid')), 'Should not find invalid block');
  });

  it('should delete blocks', async () => {
    const node = new DAGNode({
      type: NodeType.PATTERN,
      data: { name: 'delete-me' },
    });

    const cid = await store.putNode(node);
    assert.ok(await store.has(cid));

    await store.delete(cid);
    assert.ok(!(await store.has(cid)));
  });

  it('should get store statistics', async () => {
    const stats = await store.stats();

    assert.ok('totalBlocks' in stats);
    assert.ok('totalBytes' in stats);
    assert.ok('shardCount' in stats);
  });
});

describe('HAMT Index', async () => {
  let store;
  let index;

  before(async () => {
    await fs.mkdir(TEST_DATA_DIR, { recursive: true });
    store = new BlockStore({ basePath: path.join(TEST_DATA_DIR, 'hamt-blocks') });
    await store.init();
    index = new HAMTIndex(store);
    await index.init();
  });

  after(async () => {
    await fs.rm(TEST_DATA_DIR, { recursive: true, force: true });
  });

  it('should set and get values', async () => {
    const key = 'test-key';
    const cid = 'bafytest123456789';

    await index.set(key, cid);
    const result = await index.get(key);

    assert.strictEqual(result, cid);
  });

  it('should return null for missing keys', async () => {
    const result = await index.get('nonexistent-key');
    assert.strictEqual(result, null);
  });

  it('should check key existence', async () => {
    const key = 'exists-key';
    const cid = 'bafyexists123';

    await index.set(key, cid);

    assert.ok(await index.has(key));
    assert.ok(!(await index.has('missing-key')));
  });

  it('should delete keys', async () => {
    const key = 'delete-key';
    const cid = 'bafydelete123';

    await index.set(key, cid);
    assert.ok(await index.has(key));

    await index.delete(key);
    assert.ok(!(await index.has(key)));
  });

  it('should handle many entries', async () => {
    for (let i = 0; i < 50; i++) {
      await index.set(`key-${i}`, `bafycid${i}`);
    }

    for (let i = 0; i < 50; i++) {
      const result = await index.get(`key-${i}`);
      assert.strictEqual(result, `bafycid${i}`);
    }
  });
});

describe('Merkle DAG', async () => {
  let dag;

  before(async () => {
    await fs.mkdir(TEST_DATA_DIR, { recursive: true });
    dag = new MerkleDAG({ basePath: path.join(TEST_DATA_DIR, 'dag') });
    await dag.init();
  });

  after(async () => {
    await fs.rm(TEST_DATA_DIR, { recursive: true, force: true });
  });

  it('should have a root CID after initialization', () => {
    assert.ok(dag.rootCid, 'Should have root CID');
  });

  it('should put and get judgments', async () => {
    const judgment = {
      id: 'jdg_dag_test',
      itemType: 'token',
      qScore: 72,
      verdict: 'WAG',
      confidence: 0.55,
      dimensions: { coherence: 75, clarity: 70 },
    };

    const cid = await dag.putJudgment(judgment);
    const node = await dag.getJudgment('jdg_dag_test');

    assert.ok(cid);
    assert.strictEqual(node.data.qScore, 72);
    assert.strictEqual(node.data.verdict, 'WAG');
  });

  it('should put and get entities', async () => {
    const entity = {
      entityType: 'token',
      identifier: 'So11111111111111111111111111111111111111112',
      name: 'Wrapped SOL',
      attributes: { symbol: 'SOL' },
    };

    const cid = await dag.putEntity(entity);
    const node = await dag.getEntity('token', entity.identifier);

    assert.ok(cid);
    assert.strictEqual(node.data.name, 'Wrapped SOL');
  });

  it('should resolve paths in nodes', async () => {
    const node = new DAGNode({
      type: NodeType.ENTITY,
      data: {
        nested: {
          value: 42,
        },
      },
    });

    const cid = await dag.put(node);
    const result = await dag.resolve(cid, 'data/nested/value');

    assert.strictEqual(result, 42);
  });

  it('should get DAG statistics', async () => {
    const stats = await dag.stats();

    assert.ok(stats.rootCid);
    assert.ok(stats.store);
    assert.ok(stats.indices);
  });
});

console.log('*tail wag* DAG tests ready to run.');
