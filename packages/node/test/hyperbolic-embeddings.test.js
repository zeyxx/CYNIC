/**
 * @cynic/node - Hyperbolic Embeddings Tests
 *
 * Tests for Poincaré ball model hyperbolic space operations.
 *
 * @module @cynic/node/test/hyperbolic-embeddings
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  HyperbolicSpace,
  createHyperbolicSpace,
  PoincareOperations,
  HYPERBOLIC_CONFIG,
} from '../src/embeddings/hyperbolic.js';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('HYPERBOLIC_CONFIG', () => {
  it('should have negative curvature', () => {
    assert.strictEqual(HYPERBOLIC_CONFIG.CURVATURE, -1.0);
  });

  it('should have φ-aligned thresholds', () => {
    assert.ok(Math.abs(HYPERBOLIC_CONFIG.SIMILARITY_THRESHOLD - 0.618) < 0.001);
  });

  it('should have Fibonacci-aligned default dimension', () => {
    // 8 = F(6)
    assert.strictEqual(HYPERBOLIC_CONFIG.DEFAULT_DIM, 8);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(HYPERBOLIC_CONFIG));
  });
});

// =============================================================================
// POINCARE OPERATIONS TESTS
// =============================================================================

describe('PoincareOperations', () => {
  let ops;

  beforeEach(() => {
    ops = new PoincareOperations();
  });

  describe('Construction', () => {
    it('should create with default curvature', () => {
      const o = new PoincareOperations();
      assert.strictEqual(o.c, 1.0);
    });

    it('should accept custom curvature', () => {
      const o = new PoincareOperations(-2.0);
      assert.strictEqual(o.c, 2.0); // Absolute value
    });
  });

  describe('Basic Operations', () => {
    it('should compute dot product', () => {
      const u = [1, 2, 3];
      const v = [4, 5, 6];
      assert.strictEqual(ops.dot(u, v), 32);
    });

    it('should compute norm', () => {
      const x = [3, 4];
      assert.strictEqual(ops.norm(x), 5);
    });

    it('should negate vector', () => {
      const x = [1, -2, 3];
      const neg = ops.negate(x);
      assert.deepStrictEqual(neg, [-1, 2, -3]);
    });
  });

  describe('Hyperbolic Distance', () => {
    it('should return 0 for same point', () => {
      const p = [0.1, 0.2];
      const dist = ops.distance(p, p);
      assert.ok(dist < 0.001);
    });

    it('should be symmetric', () => {
      const u = [0.1, 0.2];
      const v = [0.3, 0.1];
      const d1 = ops.distance(u, v);
      const d2 = ops.distance(v, u);
      assert.ok(Math.abs(d1 - d2) < 0.0001);
    });

    it('should be non-negative', () => {
      const u = [0.1, 0.2];
      const v = [0.3, 0.4];
      assert.ok(ops.distance(u, v) >= 0);
    });

    it('should satisfy triangle inequality', () => {
      const u = [0.1, 0.1];
      const v = [0.2, 0.3];
      const w = [0.4, 0.2];

      const duv = ops.distance(u, v);
      const dvw = ops.distance(v, w);
      const duw = ops.distance(u, w);

      assert.ok(duw <= duv + dvw + 0.0001);
    });

    it('should increase towards boundary', () => {
      const origin = [0, 0];
      const near = [0.1, 0];
      const far = [0.8, 0];

      const distNear = ops.distance(origin, near);
      const distFar = ops.distance(origin, far);

      assert.ok(distFar > distNear);
    });
  });

  describe('Möbius Addition', () => {
    it('should have origin as identity', () => {
      const origin = [0, 0];
      const v = [0.3, 0.4];
      const result = ops.mobiusAdd(origin, v);

      assert.ok(Math.abs(result[0] - v[0]) < 0.0001);
      assert.ok(Math.abs(result[1] - v[1]) < 0.0001);
    });

    it('should stay within ball', () => {
      const u = [0.5, 0.5];
      const v = [0.3, 0.4];
      const result = ops.mobiusAdd(u, v);

      const norm = ops.norm(result);
      assert.ok(norm < 1.0);
    });

    it('should have inverse', () => {
      const u = [0.3, 0.4];
      const neg = ops.negate(u);
      const result = ops.mobiusAdd(u, neg);

      assert.ok(ops.norm(result) < 0.01);
    });
  });

  describe('Exponential/Logarithmic Maps', () => {
    it('should be inverses at origin', () => {
      const origin = [0, 0];
      const v = [0.2, 0.3];

      const exp = ops.expMap(origin, v);
      const log = ops.logMap(origin, exp);

      assert.ok(Math.abs(log[0] - v[0]) < 0.1);
      assert.ok(Math.abs(log[1] - v[1]) < 0.1);
    });

    it('should keep points in ball', () => {
      const p = [0.3, 0.3];
      const v = [1.0, 1.0]; // Large tangent vector

      const result = ops.expMap(p, v);
      const norm = ops.norm(result);

      assert.ok(norm < 1.0);
    });

    it('should return zero for same point', () => {
      const p = [0.3, 0.4];
      const log = ops.logMap(p, p);

      assert.ok(ops.norm(log) < 0.0001);
    });
  });

  describe('Projection', () => {
    it('should not change valid points', () => {
      const p = [0.3, 0.4];
      const proj = ops.project(p);

      assert.ok(Math.abs(proj[0] - p[0]) < 0.0001);
      assert.ok(Math.abs(proj[1] - p[1]) < 0.0001);
    });

    it('should clip points outside ball', () => {
      const outside = [2.0, 2.0];
      const proj = ops.project(outside);

      assert.ok(ops.norm(proj) < 1.0);
    });

    it('should preserve direction', () => {
      const outside = [2.0, 4.0];
      const proj = ops.project(outside);

      // Ratio should be preserved
      assert.ok(Math.abs(proj[1] / proj[0] - 2.0) < 0.01);
    });
  });

  describe('Conformal Factor', () => {
    it('should be 2 at origin', () => {
      const origin = [0, 0];
      const lambda = ops.conformalFactor(origin);
      assert.ok(Math.abs(lambda - 2) < 0.0001);
    });

    it('should increase towards boundary', () => {
      const near = [0.1, 0];
      const far = [0.9, 0];

      const lambdaNear = ops.conformalFactor(near);
      const lambdaFar = ops.conformalFactor(far);

      assert.ok(lambdaFar > lambdaNear);
    });
  });
});

// =============================================================================
// HYPERBOLIC SPACE TESTS
// =============================================================================

describe('HyperbolicSpace', () => {
  let space;

  beforeEach(() => {
    space = createHyperbolicSpace({ dim: 4 });
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      const s = createHyperbolicSpace();
      assert.ok(s instanceof HyperbolicSpace);
    });

    it('should use default dimension', () => {
      const s = createHyperbolicSpace();
      assert.strictEqual(s.dim, 8);
    });

    it('should accept custom dimension', () => {
      const s = createHyperbolicSpace({ dim: 16 });
      assert.strictEqual(s.dim, 16);
    });

    it('should initialize stats', () => {
      const stats = space.getStats();
      assert.strictEqual(stats.embeddings, 0);
    });
  });

  describe('Adding Embeddings', () => {
    it('should add with random embedding', () => {
      const emb = space.add('node1');
      assert.ok(Array.isArray(emb));
      assert.strictEqual(emb.length, 4);
    });

    it('should add with provided embedding', () => {
      const provided = [0.1, 0.2, 0.3, 0.4];
      const emb = space.add('node1', provided);

      assert.ok(Math.abs(emb[0] - 0.1) < 0.01);
    });

    it('should project embedding into ball', () => {
      const large = [10, 10, 10, 10];
      const emb = space.add('node1', large);

      const norm = space.ops.norm(emb);
      assert.ok(norm < 1.0);
    });

    it('should track embedding count', () => {
      space.add('n1');
      space.add('n2');
      space.add('n3');

      const stats = space.getStats();
      assert.strictEqual(stats.embeddings, 3);
    });

    it('should reject wrong dimension', () => {
      assert.throws(() => {
        space.add('node1', [0.1, 0.2]); // Wrong dimension
      });
    });
  });

  describe('Getting Embeddings', () => {
    it('should get existing embedding', () => {
      space.add('node1', [0.1, 0.2, 0.3, 0.4]);
      const emb = space.get('node1');

      assert.ok(emb);
      assert.ok(Math.abs(emb[0] - 0.1) < 0.01);
    });

    it('should return null for missing', () => {
      const emb = space.get('nonexistent');
      assert.strictEqual(emb, null);
    });
  });

  describe('Removing Embeddings', () => {
    it('should remove existing', () => {
      space.add('node1');
      const removed = space.remove('node1');

      assert.strictEqual(removed, true);
      assert.strictEqual(space.get('node1'), null);
    });

    it('should return false for missing', () => {
      const removed = space.remove('nonexistent');
      assert.strictEqual(removed, false);
    });

    it('should update count', () => {
      space.add('n1');
      space.add('n2');
      space.remove('n1');

      const stats = space.getStats();
      assert.strictEqual(stats.embeddings, 1);
    });
  });

  describe('Distance & Similarity', () => {
    beforeEach(() => {
      space.add('a', [0.1, 0, 0, 0]);
      space.add('b', [0.2, 0, 0, 0]);
      space.add('c', [0.5, 0, 0, 0]);
    });

    it('should compute distance', () => {
      const dist = space.distance('a', 'b');
      assert.ok(dist > 0);
    });

    it('should compute similarity', () => {
      const sim = space.similarity('a', 'b');
      assert.ok(sim > 0 && sim <= 1);
    });

    it('should have higher similarity for closer points', () => {
      const simAB = space.similarity('a', 'b');
      const simAC = space.similarity('a', 'c');

      assert.ok(simAB > simAC);
    });

    it('should throw for missing embedding', () => {
      assert.throws(() => {
        space.distance('a', 'missing');
      });
    });
  });

  describe('K-Nearest Neighbors', () => {
    beforeEach(() => {
      space.add('center', [0, 0, 0, 0]);
      space.add('near1', [0.1, 0, 0, 0]);
      space.add('near2', [0, 0.1, 0, 0]);
      space.add('far1', [0.5, 0, 0, 0]);
      space.add('far2', [0, 0.5, 0, 0]);
    });

    it('should return k nearest', () => {
      const neighbors = space.kNearest('center', 2);

      assert.strictEqual(neighbors.length, 2);
      assert.ok(neighbors.some(n => n.id === 'near1'));
      assert.ok(neighbors.some(n => n.id === 'near2'));
    });

    it('should be sorted by distance', () => {
      const neighbors = space.kNearest('center', 4);

      for (let i = 1; i < neighbors.length; i++) {
        assert.ok(neighbors[i].distance >= neighbors[i - 1].distance);
      }
    });

    it('should not include query point', () => {
      const neighbors = space.kNearest('center', 10);
      assert.ok(!neighbors.some(n => n.id === 'center'));
    });
  });

  describe('Hierarchy', () => {
    beforeEach(() => {
      space.add('root');
      space.add('child1', null, 'root');
      space.add('child2', null, 'root');
      space.add('grandchild', null, 'child1');
    });

    it('should track parent', () => {
      const ancestors = space.getAncestors('grandchild');

      assert.ok(ancestors.includes('child1'));
      assert.ok(ancestors.includes('root'));
    });

    it('should track descendants', () => {
      const descendants = space.getDescendants('root');

      assert.ok(descendants.includes('child1'));
      assert.ok(descendants.includes('child2'));
      assert.ok(descendants.includes('grandchild'));
    });

    it('should track hierarchy depth', () => {
      const stats = space.getStats();
      assert.ok(stats.hierarchyDepth >= 2);
    });

    it('should handle orphaning on removal', () => {
      space.remove('child1');
      const descendants = space.getDescendants('root');

      // grandchild should be orphaned, not in root's descendants
      assert.ok(!descendants.includes('grandchild'));
    });
  });

  describe('Centroid', () => {
    it('should compute centroid of points', () => {
      space.add('p1', [0.1, 0, 0, 0]);
      space.add('p2', [-0.1, 0, 0, 0]);

      const cent = space.centroid(['p1', 'p2']);

      // Centroid should be near origin
      assert.ok(space.ops.norm(cent) < 0.1);
    });

    it('should return single point for one input', () => {
      space.add('p1', [0.3, 0.2, 0.1, 0]);

      const cent = space.centroid(['p1']);

      assert.ok(Math.abs(cent[0] - 0.3) < 0.01);
    });

    it('should return origin for empty input', () => {
      const cent = space.centroid([]);
      assert.ok(space.ops.norm(cent) < 0.0001);
    });
  });

  describe('Move Towards', () => {
    it('should move embedding towards target', () => {
      space.add('mover', [0.1, 0, 0, 0]);
      space.add('target', [0.5, 0, 0, 0]);

      const distBefore = space.distance('mover', 'target');
      space.moveTowards('mover', 'target', 0.5);
      const distAfter = space.distance('mover', 'target');

      assert.ok(distAfter < distBefore);
    });

    it('should stay within ball', () => {
      space.add('mover', [0.8, 0, 0, 0]);
      space.add('target', [0.9, 0, 0, 0]);

      const newEmb = space.moveTowards('mover', 'target', 1.0);
      const norm = space.ops.norm(newEmb);

      assert.ok(norm < 1.0);
    });
  });

  describe('Serialization', () => {
    it('should export to object', () => {
      space.add('n1', [0.1, 0.2, 0.3, 0.4]);
      space.add('n2', [0.5, 0.4, 0.3, 0.2]);

      const obj = space.toObject();

      assert.ok('n1' in obj);
      assert.ok('n2' in obj);
      assert.strictEqual(obj.n1.length, 4);
    });

    it('should import from object', () => {
      const data = {
        n1: [0.1, 0.2, 0.3, 0.4],
        n2: [0.5, 0.4, 0.3, 0.2],
      };

      space.fromObject(data);

      assert.ok(space.get('n1'));
      assert.ok(space.get('n2'));
      const stats = space.getStats();
      assert.strictEqual(stats.embeddings, 2);
    });
  });

  describe('Events', () => {
    it('should emit add event', () => {
      const events = [];
      space.on('add', (data) => events.push(data));

      space.add('node1');

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].id, 'node1');
    });

    it('should emit remove event', () => {
      const events = [];
      space.on('remove', (data) => events.push(data));

      space.add('node1');
      space.remove('node1');

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].id, 'node1');
    });

    it('should emit move event', () => {
      const events = [];
      space.on('move', (data) => events.push(data));

      space.add('mover');
      space.add('target');
      space.moveTowards('mover', 'target', 0.5);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].id, 'mover');
    });
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('Hyperbolic Embeddings Integration', () => {
  it('should model tree hierarchy naturally', () => {
    const space = createHyperbolicSpace({ dim: 8 });

    // Create a tree: root -> [child1, child2] -> grandchildren
    space.add('root', [0, 0, 0, 0, 0, 0, 0, 0]);
    space.add('child1', [0.3, 0, 0, 0, 0, 0, 0, 0], 'root');
    space.add('child2', [-0.3, 0, 0, 0, 0, 0, 0, 0], 'root');
    space.add('gc1', [0.5, 0.1, 0, 0, 0, 0, 0, 0], 'child1');
    space.add('gc2', [0.5, -0.1, 0, 0, 0, 0, 0, 0], 'child1');

    // Parent should be closer to children than to grandchildren
    const distRootChild = space.distance('root', 'child1');
    const distRootGC = space.distance('root', 'gc1');

    assert.ok(distRootChild < distRootGC);

    // Siblings should be equidistant from parent
    const distChild1 = space.distance('root', 'child1');
    const distChild2 = space.distance('root', 'child2');

    assert.ok(Math.abs(distChild1 - distChild2) < 0.1);
  });

  it('should find ancestors correctly', () => {
    const space = createHyperbolicSpace({ dim: 4 });

    space.add('root');
    space.add('l1', null, 'root');
    space.add('l2', null, 'l1');
    space.add('l3', null, 'l2');

    const ancestors = space.getAncestors('l3');

    assert.strictEqual(ancestors.length, 3);
    assert.strictEqual(ancestors[0], 'l2');
    assert.strictEqual(ancestors[1], 'l1');
    assert.strictEqual(ancestors[2], 'root');
  });

  it('should support Dog collective hierarchy', () => {
    const space = createHyperbolicSpace({ dim: 8 });

    // Sefirot hierarchy (simplified)
    space.add('cynic', [0, 0, 0, 0, 0, 0, 0, 0]); // Keter - at center

    // Level 1
    space.add('analyst', [0.2, 0.1, 0, 0, 0, 0, 0, 0], 'cynic');
    space.add('scholar', [0, 0.2, 0, 0, 0, 0, 0, 0], 'cynic');
    space.add('sage', [-0.2, 0.1, 0, 0, 0, 0, 0, 0], 'cynic');

    // Level 2
    space.add('guardian', [0.3, 0.3, 0, 0, 0, 0, 0, 0], 'analyst');
    space.add('oracle', [0, 0.4, 0, 0, 0, 0, 0, 0], 'scholar');
    space.add('architect', [-0.3, 0.3, 0, 0, 0, 0, 0, 0], 'sage');

    // Level 3
    space.add('scout', [0.4, 0.4, 0, 0, 0, 0, 0, 0], 'guardian');

    // CYNIC should be closest to its direct children
    const nearest = space.kNearest('cynic', 3);
    const nearestIds = nearest.map(n => n.id);

    assert.ok(nearestIds.includes('analyst') || nearestIds.includes('scholar') || nearestIds.includes('sage'));

    // Scout is deeper, should be further from CYNIC
    const distToCynic = space.distance('cynic', 'scout');
    const distToGuardian = space.distance('guardian', 'scout');

    assert.ok(distToCynic > distToGuardian);
  });
});
