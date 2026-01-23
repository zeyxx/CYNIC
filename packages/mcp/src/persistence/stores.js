/**
 * Fallback Stores - Memory and File-based storage
 *
 * Used when PostgreSQL is not configured.
 * "φ distrusts φ" - CYNIC must work even offline.
 *
 * @module @cynic/mcp/persistence/stores
 */

'use strict';

import fs from 'fs/promises';
import path from 'path';

/**
 * In-memory fallback storage
 * Used when neither PostgreSQL nor file storage is configured
 */
export class MemoryStore {
  constructor() {
    this.judgments = [];
    this.patterns = [];
    this.feedback = [];
    this.knowledge = [];
    this.pojBlocks = [];
    this.observations = [];
    this.triggersState = null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // JUDGMENTS
  // ═══════════════════════════════════════════════════════════════════════════

  async storeJudgment(judgment) {
    const id = `jdg_${Date.now().toString(36)}`;
    const stored = { ...judgment, judgment_id: id, created_at: new Date() };
    this.judgments.push(stored);
    if (this.judgments.length > 1000) this.judgments.shift();
    return stored;
  }

  async searchJudgments(query, options = {}) {
    const { limit = 10 } = options;
    if (!query) return this.judgments.slice(-limit);
    const q = query.toLowerCase();
    return this.judgments
      .filter(j => JSON.stringify(j).toLowerCase().includes(q))
      .slice(-limit);
  }

  async findRecentJudgments(limit = 10) {
    return this.judgments.slice(-limit);
  }

  async getJudgment(judgmentId) {
    return this.judgments.find(j => j.judgment_id === judgmentId) || null;
  }

  async getJudgmentStats() {
    const total = this.judgments.length;
    if (total === 0) return { total: 0, avgScore: 0, avgConfidence: 0, verdicts: {} };

    const avgScore = this.judgments.reduce((s, j) => s + (j.q_score || 0), 0) / total;
    const avgConfidence = this.judgments.reduce((s, j) => s + (j.confidence || 0), 0) / total;
    const verdicts = this.judgments.reduce((v, j) => {
      v[j.verdict] = (v[j.verdict] || 0) + 1;
      return v;
    }, {});

    return { total, avgScore, avgConfidence, verdicts };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // FEEDBACK & KNOWLEDGE
  // ═══════════════════════════════════════════════════════════════════════════

  async storeFeedback(fb) {
    const id = `fb_${Date.now().toString(36)}`;
    const stored = { ...fb, feedback_id: id, created_at: new Date() };
    this.feedback.push(stored);
    return stored;
  }

  async storeKnowledge(k) {
    const id = `kn_${Date.now().toString(36)}`;
    const stored = { ...k, knowledge_id: id, created_at: new Date() };
    this.knowledge.push(stored);
    return stored;
  }

  async searchKnowledge(query, options = {}) {
    const { limit = 10 } = options;
    if (!query) return this.knowledge.slice(-limit);
    const q = query.toLowerCase();
    return this.knowledge
      .filter(k => {
        const summary = (k.summary || '').toLowerCase();
        const content = (k.content || '').toLowerCase();
        const insights = JSON.stringify(k.insights || []).toLowerCase();
        return summary.includes(q) || content.includes(q) || insights.includes(q);
      })
      .slice(-limit);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERNS
  // ═══════════════════════════════════════════════════════════════════════════

  async upsertPattern(pattern) {
    const existing = this.patterns.find(p => p.name === pattern.name);
    if (existing) {
      Object.assign(existing, pattern, { updated_at: new Date() });
      return existing;
    }
    const id = `pat_${Date.now().toString(36)}`;
    const stored = { ...pattern, pattern_id: id, created_at: new Date() };
    this.patterns.push(stored);
    return stored;
  }

  async getPatterns(options = {}) {
    const { category, limit = 10 } = options;
    let result = this.patterns;
    if (category) {
      result = result.filter(p => p.category === category);
    }
    return result.slice(-limit);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // OBSERVATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  async storeObservation(obs) {
    const stored = { ...obs, created_at: new Date() };
    this.observations.push(stored);
    if (this.observations.length > 500) this.observations.shift();
    return stored;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PoJ CHAIN
  // ═══════════════════════════════════════════════════════════════════════════

  async storePoJBlock(block) {
    if (this.pojBlocks.length > 0) {
      const head = this.pojBlocks[this.pojBlocks.length - 1];
      const expectedPrevHash = head.hash || head.block_hash;
      if (block.prev_hash !== expectedPrevHash) {
        throw new Error(`Chain integrity violation: expected prev_hash ${expectedPrevHash}, got ${block.prev_hash}`);
      }
      if (block.slot !== head.slot + 1) {
        throw new Error(`Slot mismatch: expected ${head.slot + 1}, got ${block.slot}`);
      }
    }

    const stored = {
      ...block,
      block_hash: block.hash,
      judgment_count: block.judgments?.length || 0,
      judgment_ids: block.judgments?.map(j => j.judgment_id) || [],
      created_at: new Date(),
    };
    this.pojBlocks.push(stored);

    if (this.pojBlocks.length > 10000) {
      this.pojBlocks = this.pojBlocks.slice(-5000);
    }

    return stored;
  }

  async getPoJHead() {
    if (this.pojBlocks.length === 0) return null;
    return this.pojBlocks[this.pojBlocks.length - 1];
  }

  async getPoJStats() {
    const total = this.pojBlocks.length;
    if (total === 0) return { totalBlocks: 0, headSlot: 0, totalJudgments: 0 };

    const head = this.pojBlocks[total - 1];
    const totalJudgments = this.pojBlocks.reduce((sum, b) => sum + (b.judgment_count || 0), 0);

    return { totalBlocks: total, headSlot: head.slot, totalJudgments };
  }

  async getRecentPoJBlocks(limit = 10) {
    return this.pojBlocks.slice(-limit).reverse();
  }

  async getPoJBlockBySlot(slot) {
    return this.pojBlocks.find(b => b.slot === slot) || null;
  }

  async verifyPoJChain() {
    const errors = [];

    for (let i = 1; i < this.pojBlocks.length; i++) {
      const block = this.pojBlocks[i];
      const prevBlock = this.pojBlocks[i - 1];
      const expectedPrevHash = prevBlock.hash || prevBlock.block_hash;

      if (block.prev_hash !== expectedPrevHash) {
        errors.push({
          slot: block.slot,
          error: `Invalid prev_hash: expected ${expectedPrevHash}, got ${block.prev_hash}`,
        });
      }

      if (block.slot !== prevBlock.slot + 1) {
        errors.push({
          slot: block.slot,
          error: `Slot gap: expected ${prevBlock.slot + 1}, got ${block.slot}`,
        });
      }
    }

    return { valid: errors.length === 0, blocksChecked: this.pojBlocks.length, errors };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRIGGERS STATE
  // ═══════════════════════════════════════════════════════════════════════════

  async getTriggersState() {
    return this.triggersState;
  }

  async saveTriggersState(state) {
    this.triggersState = state;
    return state;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EXPORT / IMPORT
  // ═══════════════════════════════════════════════════════════════════════════

  async export() {
    return {
      judgments: this.judgments,
      patterns: this.patterns,
      feedback: this.feedback,
      knowledge: this.knowledge,
      pojBlocks: this.pojBlocks,
      triggersState: this.triggersState,
    };
  }

  async import(data) {
    if (data.judgments) this.judgments = data.judgments;
    if (data.patterns) this.patterns = data.patterns;
    if (data.feedback) this.feedback = data.feedback;
    if (data.knowledge) this.knowledge = data.knowledge;
    if (data.pojBlocks) this.pojBlocks = data.pojBlocks;
    if (data.triggersState) this.triggersState = data.triggersState;
  }
}

/**
 * File-based storage adapter
 * Uses MemoryStore + periodic file sync
 */
export class FileStore extends MemoryStore {
  constructor(dataDir) {
    super();
    this.dataDir = dataDir;
    this.filePath = path.join(dataDir, 'cynic-state.json');
    this._dirty = false;
  }

  async initialize() {
    try {
      await fs.mkdir(this.dataDir, { recursive: true });
      const data = await fs.readFile(this.filePath, 'utf-8');
      await this.import(JSON.parse(data));
      console.error(`   File storage: loaded from ${this.filePath}`);
    } catch (err) {
      if (err.code !== 'ENOENT') {
        console.error(`   File storage: ${err.message}`);
      } else {
        console.error(`   File storage: fresh start at ${this.filePath}`);
      }
    }
  }

  async storeJudgment(judgment) {
    const result = await super.storeJudgment(judgment);
    this._dirty = true;
    return result;
  }

  async storeFeedback(fb) {
    const result = await super.storeFeedback(fb);
    this._dirty = true;
    return result;
  }

  async storeKnowledge(k) {
    const result = await super.storeKnowledge(k);
    this._dirty = true;
    return result;
  }

  async upsertPattern(pattern) {
    const result = await super.upsertPattern(pattern);
    this._dirty = true;
    return result;
  }

  async storePoJBlock(block) {
    const result = await super.storePoJBlock(block);
    this._dirty = true;
    await this.save();
    return result;
  }

  async saveTriggersState(state) {
    const result = await super.saveTriggersState(state);
    this._dirty = true;
    return result;
  }

  async save() {
    if (!this._dirty) return;
    try {
      const data = await this.export();
      await fs.writeFile(this.filePath, JSON.stringify(data, null, 2));
      this._dirty = false;
    } catch (err) {
      console.error('Error saving state:', err.message);
    }
  }
}
