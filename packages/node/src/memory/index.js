/**
 * Memory Module
 *
 * Implements Layers 2-4 of the 6-layer hybrid context architecture.
 *
 * Layer 2: Collective Memory (SharedMemory)
 * Layer 3: Procedural Memory (SharedMemory.procedures)
 * Layer 4: User Lab (UserLab)
 *
 * P2.2: 4-Tier Memory Architecture (TieredMemory)
 * - Vector: Dense representations, similarity retrieval
 * - Episodic: Complete interaction records
 * - Semantic: Factual knowledge, learned patterns
 * - Working: Active task focus (Miller's Law: 7±2)
 *
 * @module @cynic/node/memory
 */

'use strict';

export { SharedMemory } from './shared-memory.js';
export { UserLab, LabManager } from './user-lab.js';

// P2.2: 4-Tier Memory Architecture
export {
  TieredMemory,
  VectorMemory,
  EpisodicMemory,
  SemanticMemory,
  WorkingMemory,
  MemoryItem,
  Episode,
  MEMORY_CONFIG,
  createTieredMemory,
} from './tiered-memory.js';

// Hilbert Curve Memory Indexing (locality-preserving)
export {
  xy2d,
  d2xy,
  coordsToHilbert,
  hilbertToCoords,
  HilbertMemoryIndex,
  hilbertLSH,
  hilbertLSHSimilarity,
  HILBERT_CONFIG,
} from './hilbert.js';
