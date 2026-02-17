/**
 * CYNIC PageIndex - Reasoning-Based RAG with VectorStore Integration
 *
 * Inspired by "PageIndex" research:
 * - Tree-based index for hierarchical document organization
 * - LLM reasons about which nodes to explore
 * - 98.7% accuracy vs 85% for standard RAG
 *
 * NOW INTEGRATED WITH VECTORSTORE:
 * - Semantic search via HNSW + embeddings
 * - Hybrid retrieval (tree reasoning + vector similarity)
 * - Connects to Ollama (free) or OpenAI embeddings
 *
 * @module @cynic/llm/retrieval/page-index
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { getVectorStore, VectorStore, createVectorStore } from '@cynic/persistence';
import { createEmbedder, getEmbedder } from '@cynic/persistence';

const log = createLogger('PageIndex');

/**
 * Node in the PageIndex tree
 */
export class IndexNode {
  constructor(options = {}) {
    this.id = options.id || `node-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.content = options.content || '';
    this.summary = options.summary || '';
    this.children = [];
    this.parent = options.parent || null;
    this.metadata = options.metadata || {};
    this.level = options.level || 0;
    this.embeddings = null; // Will be computed
  }

  addChild(node) {
    node.parent = this;
    node.level = this.level + 1;
    this.children.push(node);
    return node;
  }

  isLeaf() {
    return this.children.length === 0;
  }

  getPath() {
    const path = [this.id];
    let current = this.parent;
    while (current) {
      path.unshift(current.id);
      current = current.parent;
    }
    return path;
  }

  getText(maxLength = 500) {
    if (this.content.length <= maxLength) {
      return this.content;
    }
    return this.content.slice(0, maxLength) + '...';
  }
}

/**
 * PageIndex - Tree-based RAG with VectorStore semantic search
 * 
 * NOW WITH VECTORSTORE INTEGRATION:
 * - Hybrid retrieval: tree reasoning + vector similarity
 * - Auto-detects Ollama (free local) or falls back to OpenAI/Mock
 * - Semantic search for relevant context
 */
export class PageIndex {
  constructor(options = {}) {
    this.name = options.name || 'default';
    this.root = new IndexNode({ 
      id: 'root',
      content: 'Root node',
      summary: 'All documents',
    });
    this.nodes = new Map(); // id -> node
    this.nodes.set('root', this.root);
    
    this.maxChildren = options.maxChildren || 10;
    this.maxLevel = options.maxLevel || 5;
    this.embeddingModel = options.embeddingModel || 'default';
    
    // VectorStore integration
    this.vectorStore = options.vectorStore || null;
    this.embedder = options.embedder || null;
    this._useVectorSearch = options.useVectorSearch !== false;
    this._vectorSearchWeight = options.vectorSearchWeight || 0.6; // 60% vector, 40% tree
    
    // Stats
    this.stats = {
      documentsIndexed: 0,
      queriesAnswered: 0,
      nodesVisited: [],
      avgRetrievalTime: 0,
      vectorSearches: 0,
      hybridRetrievals: 0,
    };
  }

  /**
   * Initialize VectorStore for semantic search
   */
  async initializeVectorStore(options = {}) {
    if (this.vectorStore) {
      log.info('VectorStore already initialized');
      return this.vectorStore;
    }

    log.info('Initializing VectorStore for PageIndex');
    
    try {
      // Try to get existing VectorStore singleton
      this.vectorStore = getVectorStore({
        embedder: options.embedder || 'auto',
        dimensions: options.dimensions || 768, // nomic-embed-text default
        ...options,
      });
      
      log.info('VectorStore initialized', { 
        embedder: this.vectorStore.config.embedder,
        dimensions: this.vectorStore.config.dimensions,
      });
      
      return this.vectorStore;
    } catch (e) {
      log.warn('Failed to initialize VectorStore, falling back to tree-only', { 
        error: e.message 
      });
      this._useVectorSearch = false;
      return null;
    }
  }

  /**
   * Build index from documents
   * Uses recursive splitting to create tree structure
   * NOW ALSO indexes in VectorStore for semantic search
   */
  async buildFromDocuments(documents) {
    log.info('Building PageIndex', { docCount: documents.length });
    
    // Initialize VectorStore if not already done
    if (this._useVectorSearch && !this.vectorStore) {
      await this.initializeVectorStore();
    }
    
    for (const doc of documents) {
      await this._addDocument(doc);
    }
    
    // Generate summaries bottom-up
    await this._generateSummaries(this.root);
    
    log.info('PageIndex built', { 
      nodes: this.nodes.size,
      documents: this.stats.documentsIndexed,
      vectorSearchEnabled: !!this.vectorStore,
    });
    
    return this;
  }

  /**
   * Add a single document to the index
   * @private
   */
  async _addDocument(doc) {
    const { content, metadata = {} } = doc;
    
    // Split content into chunks
    const chunks = this._splitContent(content);
    
    // Create leaf nodes
    const leafNodes = [];
    for (let i = 0; i < chunks.length; i++) {
      const node = new IndexNode({
        id: `${doc.id || 'doc'}-chunk-${i}`,
        content: chunks[i],
        metadata: { ...metadata, documentId: doc.id, chunkIndex: i },
      });
      leafNodes.push(node);
      this.nodes.set(node.id, node);
      
      // Index in VectorStore for semantic search
      if (this.vectorStore && chunks[i].trim()) {
        try {
          await this.vectorStore.store(node.id, chunks[i], {
            ...metadata,
            documentId: doc.id,
            chunkIndex: i,
            nodeLevel: 0,
          });
        } catch (e) {
          log.warn('Failed to index chunk in VectorStore', { 
            nodeId: node.id, 
            error: e.message 
          });
        }
      }
    }
    
    // Build tree hierarchy
    await this._buildTree(leafNodes);
    
    this.stats.documentsIndexed++;
  }

  /**
   * Split content into manageable chunks
   * @private
   */
  _splitContent(content, maxChunkSize = 1000) {
    const chunks = [];
    
    // Simple split by paragraphs first
    const paragraphs = content.split(/\n\n+/);
    
    let currentChunk = '';
    for (const para of paragraphs) {
      if (currentChunk.length + para.length > maxChunkSize && currentChunk.length > 0) {
        chunks.push(currentChunk.trim());
        currentChunk = '';
      }
      currentChunk += para + '\n\n';
    }
    
    if (currentChunk.trim()) {
      chunks.push(currentChunk.trim());
    }
    
    return chunks;
  }

  /**
   * Build tree hierarchy from leaf nodes
   * @private
   */
  async _buildTree(leafNodes) {
    // Group leaves and create parent nodes
    let currentLevel = leafNodes;
    
    while (currentLevel.length > this.maxChildren && this.root.level < this.maxLevel) {
      const parentNodes = [];
      
      for (let i = 0; i < currentLevel.length; i += this.maxChildren) {
        const group = currentLevel.slice(i, i + this.maxChildren);
        
        if (group.length === 1) {
          parentNodes.push(group[0]);
        } else {
          const parentContent = group.map(n => n.content).join('\n\n---\n\n');
          const parent = new IndexNode({
            content: parentContent,
            metadata: { type: 'intermediate' },
          });
          
          for (const child of group) {
            parent.addChild(child);
          }
          
          parentNodes.push(parent);
          this.nodes.set(parent.id, parent);
          
          // Also index intermediate nodes in VectorStore
          if (this.vectorStore && parentContent.trim()) {
            try {
              await this.vectorStore.store(parent.id, parentContent, {
                type: 'intermediate',
                nodeLevel: parent.level,
              });
            } catch (e) {
              // Ignore - non-critical
            }
          }
        }
      }
      
      currentLevel = parentNodes;
    }
    
    // Connect to root
    for (const node of currentLevel) {
      if (!node.parent) {
        this.root.addChild(node);
      }
    }
  }

  /**
   * Generate summaries for all nodes (bottom-up)
   * @private
   */
  async _generateSummaries(node) {
    // Process children first
    for (const child of node.children) {
      await this._generateSummaries(child);
    }
    
    // Generate summary from children
    if (node.children.length > 0) {
      const childSummaries = node.children.map(c => c.summary).join('\n');
      node.summary = this._createSummary(node.content + '\n\n' + childSummaries);
    } else {
      node.summary = this._createSummary(node.content);
    }
  }

  /**
   * Create a summary (simple extraction-based)
   * @private
   */
  _createSummary(content) {
    // Simple: take first 200 chars as summary
    // In production, would use LLM for better summaries
    const firstSentences = content.split(/[.!?]/).slice(0, 2).join('. ');
    return firstSentences.slice(0, 200) + (firstSentences.length > 200 ? '...' : '');
  }

  /**
   * Retrieve relevant context for a query
   * 
   * HYBRID RETRIEVAL: Combines tree reasoning with vector similarity
   * - If VectorStore available: uses semantic search + tree reasoning
   * - Falls back to simple keyword matching if no VectorStore
   */
  async retrieve(query, options = {}) {
    const { 
      maxNodes = 5, 
      includeSummary = true,
      llm = null, // LLM for reasoning
      useVector = this._useVectorSearch && this.vectorStore,
    } = options;

    const startTime = Date.now();
    this.stats.queriesAnswered++;

    // HYBRID RETRIEVAL: Combine tree + vector search
    let relevantNodes;
    
    if (useVector && this.vectorStore) {
      // Hybrid: vector similarity + tree reasoning
      relevantNodes = await this._hybridRetrieval(query, maxNodes, llm);
      this.stats.hybridRetrievals++;
    } else if (llm) {
      // LLM-guided retrieval (PageIndex original)
      relevantNodes = await this._llmGuidedRetrieval(query, maxNodes, llm);
    } else {
      // Fallback: simple keyword-based
      relevantNodes = await this._simpleRetrieval(query, maxNodes);
    }

    const retrievalTime = Date.now() - startTime;
    this.stats.avgRetrievalTime = 
      (this.stats.avgRetrievalTime * (this.stats.queriesAnswered - 1) + retrievalTime) 
      / this.stats.queriesAnswered;

    // Build context from relevant nodes
    const context = relevantNodes.map(node => ({
      content: node.content,
      summary: node.summary,
      path: node.getPath(),
      level: node.level,
      relevance: node.relevance || 0,
    }));

    return {
      context,
      nodes: relevantNodes,
      metadata: {
        query,
        nodesRetrieved: relevantNodes.length,
        retrievalTime,
        method: useVector ? 'hybrid' : (llm ? 'llm-guided' : 'simple'),
        vectorSearchEnabled: !!this.vectorStore,
      },
    };
  }

  /**
   * Hybrid retrieval: combines vector similarity with tree reasoning
   * 
   * Strategy:
   * 1. Vector search for semantic similarity (60% weight)
   * 2. Tree-based exploration for hierarchical context (40% weight)
   * 3. Merge and re-rank results
   * @private
   */
  async _hybridRetrieval(query, maxNodes, llm) {
    this.stats.vectorSearches++;
    
    // Step 1: Vector-based semantic search
    const vectorResults = await this.vectorStore.search(query, maxNodes * 2, {
      minScore: 0.3, // Lower threshold to get more candidates
    });
    
    // Convert vector results to nodes
    const vectorNodes = [];
    for (const result of vectorResults) {
      const node = this.nodes.get(result.id);
      if (node) {
        node.vectorScore = result.score;
        vectorNodes.push(node);
      }
    }
    
    // Step 2: Tree-based reasoning (LLM or keyword)
    let treeNodes = [];
    if (llm) {
      treeNodes = await this._llmGuidedRetrieval(query, maxNodes, llm);
    } else {
      treeNodes = await this._simpleRetrieval(query, maxNodes);
    }
    
    // Convert tree scores
    const maxTreeScore = Math.max(...treeNodes.map(n => n.treeRelevance || 0), 1);
    for (const node of treeNodes) {
      node.treeScore = (node.treeRelevance || 0) / maxTreeScore;
    }
    
    // Step 3: Merge and re-rank
    const nodeMap = new Map();
    
    // Add vector results
    for (const node of vectorNodes) {
      nodeMap.set(node.id, {
        node,
        vectorScore: node.vectorScore || 0,
        treeScore: 0,
        combinedScore: 0,
      });
    }
    
    // Add tree results
    for (const node of treeNodes) {
      if (nodeMap.has(node.id)) {
        const entry = nodeMap.get(node.id);
        entry.treeScore = node.treeScore || 0;
      } else {
        nodeMap.set(node.id, {
          node,
          vectorScore: 0,
          treeScore: node.treeScore || 0,
        });
      }
    }
    
    // Calculate combined score
    for (const [id, entry] of nodeMap) {
      entry.combinedScore = 
        (this._vectorSearchWeight * entry.vectorScore) + 
        ((1 - this._vectorSearchWeight) * entry.treeScore);
    }
    
    // Sort by combined score
    const sorted = Array.from(nodeMap.values())
      .sort((a, b) => b.combinedScore - a.combinedScore);
    
    return sorted.slice(0, maxNodes).map(e => e.node);
  }

  /**
   * LLM-guided retrieval - the key innovation of PageIndex
   * @private
   */
  async _llmGuidedRetrieval(query, maxNodes, llm) {
    // 1. Ask LLM which branches are relevant
    const prompt = `Given this query: "${query}"
    
And these document summaries at level ${this.root.level}:
${this.root.children.map((c, i) => `${i + 1}. ${c.summary}`).join('\n')}

Which branches (1-${this.root.children.length}) are most relevant? 
Respond with numbers only, comma-separated.`;

    const response = await llm.complete(prompt);
    const branchIndices = this._parseBranchIndices(response.content);
    
    // 2. Explore selected branches
    const relevantNodes = [];
    const visited = new Set();
    
    for (const idx of branchIndices) {
      const branch = this.root.children[idx];
      if (branch && !visited.has(branch.id)) {
        visited.add(branch.id);
        
        // Recursively find relevant leaves
        const leaves = this._findRelevantLeaves(branch, query, maxNodes - relevantNodes.length);
        relevantNodes.push(...leaves);
        
        if (relevantNodes.length >= maxNodes) break;
      }
    }

    // 3. Score by relevance
    for (const node of relevantNodes) {
      node.treeRelevance = this._calculateRelevance(node.content, query);
    }
    
    relevantNodes.sort((a, b) => b.treeRelevance - a.treeRelevance);
    
    return relevantNodes.slice(0, maxNodes);
  }

  /**
   * Parse LLM response to get branch indices
   * @private
   */
  _parseBranchIndices(response) {
    const numbers = response.match(/\d+/g);
    if (!numbers) return [];
    
    return numbers
      .map(n => parseInt(n, 10) - 1)
      .filter(i => i >= 0 && i < this.root.children.length);
  }

  /**
   * Find relevant leaf nodes in a subtree
   * @private
   */
  _findRelevantLeaves(node, query, maxNodes) {
    if (node.isLeaf()) {
      return [node];
    }
    
    // Recurse into children
    const results = [];
    for (const child of node.children) {
      const leaves = this._findRelevantLeaves(child, query, maxNodes - results.length);
      results.push(...leaves);
      
      if (results.length >= maxNodes) break;
    }
    
    return results;
  }

  /**
   * Simple retrieval without LLM or VectorStore
   * @private
   */
  async _simpleRetrieval(query, maxNodes) {
    // Find all leaf nodes
    const leaves = [];
    const stack = [...this.root.children];
    
    while (stack.length > 0) {
      const node = stack.pop();
      if (node.isLeaf()) {
        leaves.push(node);
      } else {
        stack.push(...node.children);
      }
    }
    
    // Score by relevance
    for (const leaf of leaves) {
      leaf.treeRelevance = this._calculateRelevance(leaf.content, query);
    }
    
    // Sort and return top N
    leaves.sort((a, b) => b.treeRelevance - a.treeRelevance);
    
    return leaves.slice(0, maxNodes);
  }

  /**
   * Calculate simple relevance score
   * @private
   */
  _calculateRelevance(content, query) {
    const queryTerms = query.toLowerCase().split(/\s+/);
    const contentLower = content.toLowerCase();
    
    let score = 0;
    for (const term of queryTerms) {
      if (contentLower.includes(term)) {
        score += 1;
      }
    }
    
    return score / queryTerms.length;
  }

  /**
   * Get index statistics
   */
  getStats() {
    return {
      ...this.stats,
      totalNodes: this.nodes.size,
      treeDepth: this._getDepth(this.root),
      vectorStoreAvailable: !!this.vectorStore,
      vectorSearchEnabled: this._useVectorSearch,
    };
  }

  /**
   * Get tree depth
   * @private
   */
  _getDepth(node) {
    if (node.isLeaf()) return node.level;
    return Math.max(...node.children.map(c => this._getDepth(c)));
  }

  /**
   * Export index for serialization
   * NOTE: VectorStore data is NOT exported (it's separate)
   */
  export() {
    return {
      name: this.name,
      config: {
        maxChildren: this.maxChildren,
        maxLevel: this.maxLevel,
        vectorSearchWeight: this._vectorSearchWeight,
      },
      nodes: Array.from(this.nodes.values()).map(n => ({
        id: n.id,
        content: n.content,
        summary: n.summary,
        parentId: n.parent?.id || null,
        metadata: n.metadata,
        level: n.level,
      })),
    };
  }

  /**
   * Import index from serialized data
   * NOTE: You need to re-index in VectorStore after import
   */
  static async import(data) {
    const index = new PageIndex({ 
      name: data.name,
      maxChildren: data.config?.maxChildren,
      maxLevel: data.config?.maxLevel,
    });
    index.nodes.clear();
    
    for (const nodeData of data.nodes) {
      const node = new IndexNode({
        id: nodeData.id,
        content: nodeData.content,
        summary: nodeData.summary,
        metadata: nodeData.metadata,
        level: nodeData.level,
      });
      index.nodes.set(node.id, node);
    }
    
    // Rebuild parent-child relationships
    for (const nodeData of data.nodes) {
      if (nodeData.parentId) {
        const node = index.nodes.get(nodeData.id);
        const parent = index.nodes.get(nodeData.parentId);
        if (parent && node) {
          parent.children.push(node);
          node.parent = parent;
        }
      } else if (nodeData.id !== 'root') {
        index.root.addChild(index.nodes.get(nodeData.id));
      }
    }
    
    return index;
  }

  /**
   * Get the VectorStore instance
   */
  getVectorStore() {
    return this.vectorStore;
  }

  /**
   * Manually trigger re-indexing in VectorStore
   * Useful after importing an index
   */
  async reindexVectorStore() {
    if (!this.vectorStore) {
      log.warn('No VectorStore available for reindexing');
      return;
    }

    log.info('Re-indexing all nodes in VectorStore');
    
    for (const [id, node] of this.nodes) {
      if (node.content.trim()) {
        try {
          await this.vectorStore.store(id, node.content, {
            ...node.metadata,
            nodeLevel: node.level,
          });
        } catch (e) {
          log.warn('Failed to index node', { id, error: e.message });
        }
      }
    }
    
    log.info('Re-indexing complete', { nodes: this.nodes.size });
  }
}

/**
 * Create PageIndex instance with VectorStore integration
 */
export function createPageIndex(options) {
  return new PageIndex(options);
}

/**
 * Create PageIndex with automatic VectorStore initialization
 */
export async function createPageIndexWithVectorStore(options = {}) {
  const index = new PageIndex(options);
  await index.initializeVectorStore(options);
  return index;
}

export default PageIndex;
