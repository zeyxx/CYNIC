# @cynic/persistence

> Persistence layer for CYNIC - PostgreSQL + Redis + Merkle DAG + PoJ Chain + Graph Overlay

**Category**: persistence | **Version**: 0.1.0 | **Quality**: 🟠 needs tests

## Installation

```bash
npm install @cynic/persistence
```

## Quick Start

```javascript
import { createMockFactory } from '@cynic/persistence';

const instance = createMockFactory();
```

## API Reference

### Classes

| Class | Description |
|-------|-------------|
| `PostgresClient` | PostgresClient implementation |
| `RedisClient` | RedisClient implementation |
| `RepositoryFactory` | RepositoryFactory implementation |
| `FallbackRepositoryFactory` | FallbackRepositoryFactory implementation |
| `BackendType` | BackendType implementation |
| `FileBackedRepo` | FileBackedRepo implementation |
| `BaseRepository` | BaseRepository implementation |
| `SessionStore` | SessionStore implementation |
| `BatchQueue` | BatchQueue implementation |
| `CODECS` | CODECS implementation |
| `DAGNode` | DAGNode implementation |
| `DAGLink` | DAGLink implementation |
| `NodeType` | NodeType implementation |
| `BlockStore` | BlockStore implementation |
| `HAMTIndex` | HAMTIndex implementation |
| `HAMTEntry` | HAMTEntry implementation |
| `HAMTBucket` | HAMTBucket implementation |
| `MerkleDAG` | MerkleDAG implementation |
| `PoJBlockHeader` | PoJBlockHeader implementation |
| `PoJBlock` | PoJBlock implementation |
| `Attestation` | Attestation implementation |
| `JudgmentRef` | JudgmentRef implementation |
| `PoJChain` | PoJChain implementation |
| `GraphNodeType` | GraphNodeType implementation |
| `GraphEdgeType` | GraphEdgeType implementation |
| `NodeSchemas` | NodeSchemas implementation |
| `EdgeSpecs` | EdgeSpecs implementation |
| `GraphNode` | GraphNode implementation |
| `GraphEdge` | GraphEdge implementation |
| `GraphStore` | GraphStore implementation |
| `GraphTraversal` | GraphTraversal implementation |
| `TraversalResult` | TraversalResult implementation |
| `GraphOverlay` | GraphOverlay implementation |
| `GraphQuery` | GraphQuery implementation |
| `HNSWIndex` | HNSWIndex implementation |
| `HNSWNode` | HNSWNode implementation |
| `VectorStore` | VectorStore implementation |
| `SemanticPatternMatcher` | SemanticPatternMatcher implementation |
| `SemanticPattern` | SemanticPattern implementation |
| `PatternCluster` | PatternCluster implementation |
| `LocalXStore` | LocalXStore implementation |
| `LocalPrivacyStore` | LocalPrivacyStore implementation |
| `SyncStatus` | SyncStatus implementation |
| `VERSION` | VERSION implementation |

### Factory Functions

| Function | Description |
|----------|-------------|
| `createMockFactory()` | Create MockFactory instance |
| `createFallbackFactory()` | Create FallbackFactory instance |
| `createFileBackedRepo()` | Create FileBackedRepo instance |
| `createTableBatchQueue()` | Create TableBatchQueue instance |
| `createCID()` | Create CID instance |
| `createRawCID()` | Create RawCID instance |
| `createJudgmentNode()` | Create JudgmentNode instance |
| `createBlockNode()` | Create BlockNode instance |
| `createEntityNode()` | Create EntityNode instance |
| `createEdgeNode()` | Create EdgeNode instance |
| `createPatternNode()` | Create PatternNode instance |
| `createIndexNode()` | Create IndexNode instance |
| `createRootNode()` | Create RootNode instance |
| `createGenesisBlock()` | Create GenesisBlock instance |
| `createBlock()` | Create Block instance |
| `createTokenNode()` | Create TokenNode instance |
| `createWalletNode()` | Create WalletNode instance |
| `createProjectNode()` | Create ProjectNode instance |
| `createRepoNode()` | Create RepoNode instance |
| `createUserNode()` | Create UserNode instance |
| `createContractNode()` | Create ContractNode instance |
| `createCynicNode()` | Create CynicNode instance |
| `createHNSWIndex()` | Create HNSWIndex instance |
| `createVectorStore()` | Create VectorStore instance |
| `createSemanticPatternMatcher()` | Create SemanticPatternMatcher instance |

### Singletons

| Function | Description |
|----------|-------------|
| `getPool()` | Get Pool singleton |
| `getRedis()` | Get Redis singleton |
| `getVectorStore()` | Get VectorStore singleton |
| `getSemanticPatternMatcher()` | Get SemanticPatternMatcher singleton |

### Constants

`DEFAULT_BATCH_CONFIG`, `CODECS`, `HASH_FUNCTIONS`, `HAMT_CONFIG`, `POJ_CONSTANTS`, `GRAPH_PHI`, `HNSW_CONFIG`, `VECTOR_STORE_CONFIG`, `SEMANTIC_PATTERN_CONFIG`, `VERSION`

### Functions

`migrate`, `parseCID`, `isValidCID`, `shardCID`, `compareCIDs`, `computeMerkleRoot`

## Dependencies

**External**: cbor-x, dotenv, ioredis, pg

## Stats

- **Source files**: 84
- **Test files**: 21
- **Test ratio**: 25%
- **Exports**: 87 named

## Dimensions (4 Axioms)

```
[███░░░░░░░] 25% φ (Confidence)
[█████░░░░░] 50% Verify
[█████░░░░░] 50% Culture
[██████░░░░] 62% Burn (Simplicity)
[████░░░░░░] 40% Emergence
```

*Max 62% (φ⁻¹) - certainty is hubris*

---

*Auto-generated by CYNIC meta-cognition. "φ distrusts φ" - κυνικός*
