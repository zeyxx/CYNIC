/**
 * Graph Overlay Types
 *
 * Node types, edge types, and φ-weights for the relationship graph.
 *
 * "Relationships define truth" - κυνικός
 *
 * @module @cynic/persistence/graph/types
 */

'use strict';

// φ Constants for weighting
const PHI = 1.618033988749895;
const PHI_POWERS = {
  0: 1.0,
  1: PHI,
  2: PHI ** 2,  // 2.618
  3: PHI ** 3,  // 4.236
  4: PHI ** 4,  // 6.854
  5: PHI ** 5,  // 11.090
  6: PHI ** 6,  // 17.944
};

// ═══════════════════════════════════════════════════════════════════════════
// NODE TYPES (7)
// ═══════════════════════════════════════════════════════════════════════════

export const GraphNodeType = {
  TOKEN: 'token',
  WALLET: 'wallet',
  PROJECT: 'project',
  REPO: 'repo',
  USER: 'user',
  CONTRACT: 'contract',
  NODE: 'node',
  DOG: 'dog',     // CYNIC agent (Guardian, Analyst, Observer, EventBus)
  TOOL: 'tool',   // Claude Code tool (Bash, Read, Write, Edit, etc.)
};

// Node type schemas (required and optional fields)
export const NodeSchemas = {
  [GraphNodeType.TOKEN]: {
    required: ['mintAddress', 'symbol'],
    optional: ['name', 'decimals', 'supply', 'kScore', 'metadata'],
  },
  [GraphNodeType.WALLET]: {
    required: ['address'],
    optional: ['firstSeen', 'lastSeen', 'labels', 'reputation', 'metadata'],
  },
  [GraphNodeType.PROJECT]: {
    required: ['name'],
    optional: ['domain', 'description', 'tokens', 'eScore', 'metadata'],
  },
  [GraphNodeType.REPO]: {
    required: ['url'],
    optional: ['name', 'owner', 'stars', 'forks', 'language', 'activity', 'metadata'],
  },
  [GraphNodeType.USER]: {
    required: ['handle', 'platform'],
    optional: ['name', 'verified', 'followers', 'influence', 'metadata'],
  },
  [GraphNodeType.CONTRACT]: {
    required: ['address', 'contractType'],
    optional: ['name', 'verified', 'audited', 'deployedAt', 'metadata'],
  },
  [GraphNodeType.NODE]: {
    required: ['nodeId'],
    optional: ['endpoint', 'iScore', 'uptime', 'region', 'metadata'],
  },
  [GraphNodeType.DOG]: {
    required: ['name'],
    optional: ['role', 'decisions', 'blocks', 'metadata'],
  },
  [GraphNodeType.TOOL]: {
    required: ['name'],
    optional: ['category', 'usageCount', 'metadata'],
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// EDGE TYPES (11+)
// ═══════════════════════════════════════════════════════════════════════════

export const GraphEdgeType = {
  // Token-Wallet relationships
  HOLDS: 'holds',           // Wallet -> Token
  CREATED: 'created',       // Wallet -> Token
  TRANSFERRED: 'transferred', // Wallet -> Wallet
  BURNED: 'burned',         // Wallet -> Token

  // Project relationships
  OWNS: 'owns',             // Project -> Token
  DEVELOPS: 'develops',     // Project -> Repo
  OPERATES: 'operates',     // Node -> Project

  // Development relationships
  CONTRIBUTES: 'contributes', // User -> Repo
  REFERENCES: 'references',   // Repo -> Repo

  // Social relationships
  FOLLOWS: 'follows',       // User -> User

  // Contract relationships
  DEPLOYS: 'deploys',       // Contract -> Token

  // Judgment relationship
  JUDGED: 'judged',         // CYNIC -> Entity
};

// Edge type specifications with φ-weights
export const EdgeSpecs = {
  [GraphEdgeType.HOLDS]: {
    from: [GraphNodeType.WALLET],
    to: [GraphNodeType.TOKEN],
    weight: PHI_POWERS[2], // φ² = 2.618
    bidirectional: false,
    attributes: ['amount', 'percentage', 'since'],
  },
  [GraphEdgeType.CREATED]: {
    from: [GraphNodeType.WALLET],
    to: [GraphNodeType.TOKEN],
    weight: PHI_POWERS[3], // φ³ = 4.236
    bidirectional: false,
    attributes: ['timestamp', 'txSignature'],
  },
  [GraphEdgeType.TRANSFERRED]: {
    from: [GraphNodeType.WALLET],
    to: [GraphNodeType.WALLET],
    weight: PHI_POWERS[0], // 1.0
    bidirectional: false,
    attributes: ['amount', 'token', 'timestamp', 'txSignature'],
  },
  [GraphEdgeType.BURNED]: {
    from: [GraphNodeType.WALLET],
    to: [GraphNodeType.TOKEN],
    weight: PHI_POWERS[1], // φ = 1.618
    bidirectional: false,
    attributes: ['amount', 'timestamp', 'txSignature'],
  },
  [GraphEdgeType.OWNS]: {
    from: [GraphNodeType.PROJECT],
    to: [GraphNodeType.TOKEN],
    weight: PHI_POWERS[2], // φ² = 2.618
    bidirectional: false,
    attributes: ['role', 'since'],
  },
  [GraphEdgeType.DEVELOPS]: {
    from: [GraphNodeType.PROJECT],
    to: [GraphNodeType.REPO],
    weight: PHI_POWERS[1], // φ = 1.618
    bidirectional: false,
    attributes: ['role'],
  },
  [GraphEdgeType.CONTRIBUTES]: {
    from: [GraphNodeType.USER],
    to: [GraphNodeType.REPO],
    weight: PHI_POWERS[1], // φ = 1.618
    bidirectional: false,
    attributes: ['commits', 'prs', 'issues', 'since'],
  },
  [GraphEdgeType.FOLLOWS]: {
    from: [GraphNodeType.USER],
    to: [GraphNodeType.USER],
    weight: PHI_POWERS[0], // 1.0
    bidirectional: false,
    attributes: ['since'],
  },
  [GraphEdgeType.REFERENCES]: {
    from: [GraphNodeType.REPO],
    to: [GraphNodeType.REPO],
    weight: PHI_POWERS[1], // φ = 1.618
    bidirectional: false,
    attributes: ['type', 'version'], // dependency, fork, mention
  },
  [GraphEdgeType.DEPLOYS]: {
    from: [GraphNodeType.CONTRACT],
    to: [GraphNodeType.TOKEN],
    weight: PHI_POWERS[2], // φ² = 2.618
    bidirectional: false,
    attributes: ['timestamp', 'txSignature'],
  },
  [GraphEdgeType.OPERATES]: {
    from: [GraphNodeType.NODE],
    to: [GraphNodeType.PROJECT],
    weight: PHI_POWERS[1], // φ = 1.618
    bidirectional: false,
    attributes: ['role', 'since'],
  },
  [GraphEdgeType.JUDGED]: {
    from: [GraphNodeType.NODE], // CYNIC node
    to: Object.values(GraphNodeType), // Any entity
    weight: PHI_POWERS[3], // φ³ = 4.236
    bidirectional: false,
    attributes: ['judgmentId', 'qScore', 'verdict', 'timestamp'],
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH NODE CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Graph Node - vertex in the relationship graph
 */
export class GraphNode {
  /**
   * @param {Object} options - Node options
   */
  constructor({
    id,
    type,
    identifier,
    attributes = {},
    metadata = {},
  }) {
    // Generate ID if not provided
    this.id = id || `${type}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
    this.type = type;
    this.identifier = identifier; // Type-specific unique identifier
    this.attributes = attributes;
    this.metadata = {
      created: Date.now(),
      updated: Date.now(),
      ...metadata,
    };

    // Validate type
    if (!Object.values(GraphNodeType).includes(type)) {
      throw new Error(`Invalid node type: ${type}`);
    }
  }

  /**
   * Get canonical key for this node
   * @returns {string} Key for lookups
   */
  get key() {
    return `${this.type}:${this.identifier}`;
  }

  /**
   * Update attributes
   * @param {Object} updates - Attributes to update
   * @returns {GraphNode} This node
   */
  update(updates) {
    this.attributes = { ...this.attributes, ...updates };
    this.metadata.updated = Date.now();
    return this;
  }

  /**
   * Validate node against schema
   * @returns {{ valid: boolean, errors: string[] }} Validation result
   */
  validate() {
    const schema = NodeSchemas[this.type];
    if (!schema) {
      return { valid: false, errors: [`Unknown node type: ${this.type}`] };
    }

    const errors = [];

    // Check required fields
    for (const field of schema.required) {
      if (this.attributes[field] === undefined && this[field] === undefined) {
        errors.push(`Missing required field: ${field}`);
      }
    }

    return { valid: errors.length === 0, errors };
  }

  /**
   * Convert to JSON
   * @returns {Object} JSON representation
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      identifier: this.identifier,
      key: this.key,
      attributes: this.attributes,
      metadata: this.metadata,
    };
  }

  /**
   * Create from JSON
   * @param {Object} json - JSON data
   * @returns {GraphNode} Node instance
   */
  static fromJSON(json) {
    return new GraphNode({
      id: json.id,
      type: json.type,
      identifier: json.identifier,
      attributes: json.attributes,
      metadata: json.metadata,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH EDGE CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Graph Edge - relationship between nodes
 */
export class GraphEdge {
  /**
   * @param {Object} options - Edge options
   */
  constructor({
    id,
    type,
    sourceId,
    targetId,
    weight,
    attributes = {},
    metadata = {},
  }) {
    this.id = id || `${type}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
    this.type = type;
    this.sourceId = sourceId;
    this.targetId = targetId;

    // Use default weight from spec if not provided
    const spec = EdgeSpecs[type];
    this.weight = weight ?? spec?.weight ?? 1.0;

    this.attributes = attributes;
    this.metadata = {
      created: Date.now(),
      ...metadata,
    };

    // Validate type
    if (!Object.values(GraphEdgeType).includes(type)) {
      throw new Error(`Invalid edge type: ${type}`);
    }
  }

  /**
   * Get canonical key for this edge
   * @returns {string} Key for lookups
   */
  get key() {
    return `${this.type}:${this.sourceId}:${this.targetId}`;
  }

  /**
   * Get reverse key (for bidirectional lookup)
   * @returns {string} Reverse key
   */
  get reverseKey() {
    return `${this.type}:${this.targetId}:${this.sourceId}`;
  }

  /**
   * Check if edge is bidirectional
   * @returns {boolean} True if bidirectional
   */
  get isBidirectional() {
    return EdgeSpecs[this.type]?.bidirectional ?? false;
  }

  /**
   * Get φ-weighted value (weight × base weight)
   * @param {number} [baseWeight=1] - Base weight to apply
   * @returns {number} Weighted value
   */
  getWeightedValue(baseWeight = 1) {
    return this.weight * baseWeight;
  }

  /**
   * Validate edge against spec
   * @param {GraphNode} source - Source node
   * @param {GraphNode} target - Target node
   * @returns {{ valid: boolean, errors: string[] }} Validation result
   */
  validate(source, target) {
    const spec = EdgeSpecs[this.type];
    if (!spec) {
      return { valid: false, errors: [`Unknown edge type: ${this.type}`] };
    }

    const errors = [];

    // Validate source type
    if (!spec.from.includes(source?.type)) {
      errors.push(`Invalid source type: ${source?.type}, expected one of: ${spec.from.join(', ')}`);
    }

    // Validate target type
    if (!spec.to.includes(target?.type)) {
      errors.push(`Invalid target type: ${target?.type}, expected one of: ${spec.to.join(', ')}`);
    }

    return { valid: errors.length === 0, errors };
  }

  /**
   * Convert to JSON
   * @returns {Object} JSON representation
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      sourceId: this.sourceId,
      targetId: this.targetId,
      key: this.key,
      weight: this.weight,
      attributes: this.attributes,
      metadata: this.metadata,
    };
  }

  /**
   * Create from JSON
   * @param {Object} json - JSON data
   * @returns {GraphEdge} Edge instance
   */
  static fromJSON(json) {
    return new GraphEdge({
      id: json.id,
      type: json.type,
      sourceId: json.sourceId,
      targetId: json.targetId,
      weight: json.weight,
      attributes: json.attributes,
      metadata: json.metadata,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a token node
 */
export function createTokenNode(mintAddress, symbol, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.TOKEN,
    identifier: mintAddress,
    attributes: { mintAddress, symbol, ...attributes },
  });
}

/**
 * Create a wallet node
 */
export function createWalletNode(address, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.WALLET,
    identifier: address,
    attributes: { address, ...attributes },
  });
}

/**
 * Create a project node
 */
export function createProjectNode(name, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.PROJECT,
    identifier: name.toLowerCase().replace(/\s+/g, '-'),
    attributes: { name, ...attributes },
  });
}

/**
 * Create a repo node
 */
export function createRepoNode(url, attributes = {}) {
  const identifier = url.replace(/^https?:\/\//, '').replace(/\.git$/, '');
  return new GraphNode({
    type: GraphNodeType.REPO,
    identifier,
    attributes: { url, ...attributes },
  });
}

/**
 * Create a user node
 */
export function createUserNode(handle, platform, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.USER,
    identifier: `${platform}:${handle}`,
    attributes: { handle, platform, ...attributes },
  });
}

/**
 * Create a contract node
 */
export function createContractNode(address, contractType, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.CONTRACT,
    identifier: address,
    attributes: { address, contractType, ...attributes },
  });
}

/**
 * Create a CYNIC node
 */
export function createCynicNode(nodeId, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.NODE,
    identifier: nodeId,
    attributes: { nodeId, ...attributes },
  });
}

/**
 * Create a CYNIC dog node (agent)
 */
export function createDogNode(name, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.DOG,
    identifier: name.toLowerCase(),
    attributes: { name, ...attributes },
  });
}

/**
 * Create a tool node
 */
export function createToolNode(name, attributes = {}) {
  return new GraphNode({
    type: GraphNodeType.TOOL,
    identifier: name.toLowerCase(),
    attributes: { name, ...attributes },
  });
}

// Export φ constants
export const GRAPH_PHI = {
  PHI,
  PHI_POWERS,
};

export default {
  GraphNodeType,
  GraphEdgeType,
  NodeSchemas,
  EdgeSpecs,
  GraphNode,
  GraphEdge,
  createTokenNode,
  createWalletNode,
  createProjectNode,
  createRepoNode,
  createUserNode,
  createContractNode,
  createCynicNode,
  createDogNode,
  createToolNode,
  GRAPH_PHI,
};
