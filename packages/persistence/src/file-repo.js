/**
 * File-Backed Repository - JSONL persistence without databases
 *
 * Last-resort persistence when PostgreSQL and SQLite are both unavailable.
 * Stores data in ~/.cynic/data/{name}.jsonl files.
 *
 * "Le chien sans niche dort dans la rue, mais n'oublie rien" - kunikos
 *
 * Features:
 * - Append-only JSONL (one JSON object per line)
 * - Reads all records on construction
 * - Bounded: prunes to MAX_RECORDS on write
 * - Supports the repo interface that AutonomousDaemon expects
 *
 * @module @cynic/persistence/file-repo
 */

'use strict';

import { readFileSync, writeFileSync, appendFileSync, existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const MAX_RECORDS = 500;
const DATA_DIR = join(homedir(), '.cynic', 'data');

/**
 * File-Backed Repository
 *
 * Implements the same interface as PostgreSQL repositories but stores in JSONL.
 */
export class FileBackedRepo {
  /**
   * @param {string} name - Repository name (used as filename)
   */
  constructor(name) {
    this.name = name;
    this._filePath = join(DATA_DIR, `${name}.jsonl`);
    this._items = [];
    this._nextId = 1;

    // Ensure directory exists
    if (!existsSync(DATA_DIR)) {
      mkdirSync(DATA_DIR, { recursive: true });
    }

    // Load existing data
    this._load();
  }

  /**
   * Load records from JSONL file
   * @private
   */
  _load() {
    try {
      if (!existsSync(this._filePath)) return;

      const content = readFileSync(this._filePath, 'utf8').trim();
      if (!content) return;

      const lines = content.split('\n');
      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const item = JSON.parse(line);
          this._items.push(item);
          // Track max ID for auto-increment
          const idNum = typeof item.id === 'string'
            ? parseInt(item.id.replace(/\D/g, ''), 10) || 0
            : (item.id || 0);
          if (idNum >= this._nextId) this._nextId = idNum + 1;
        } catch {
          // Skip malformed lines
        }
      }
    } catch {
      // File read error — start fresh
      this._items = [];
    }
  }

  /**
   * Save all records to JSONL file (rewrite)
   * @private
   */
  _save() {
    // Prune if over limit
    if (this._items.length > MAX_RECORDS) {
      this._items = this._items.slice(-MAX_RECORDS);
    }

    const content = this._items.map(item => JSON.stringify(item)).join('\n') + '\n';
    writeFileSync(this._filePath, content, 'utf8');
  }

  /**
   * Append a single record (faster than full rewrite)
   * @private
   */
  _append(item) {
    if (this._items.length >= MAX_RECORDS) {
      // Need to prune — do full rewrite
      this._save();
      return;
    }
    appendFileSync(this._filePath, JSON.stringify(item) + '\n', 'utf8');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CRUD INTERFACE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a new record
   * @param {Object} data
   * @returns {Object} Created record with id and timestamps
   */
  async create(data) {
    const item = {
      ...data,
      id: `file-${this.name}-${this._nextId++}`,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    this._items.push(item);
    this._append(item);
    return item;
  }

  /**
   * Find record by ID
   * @param {string} id
   * @returns {Object|null}
   */
  async findById(id) {
    return this._items.find(i => i.id === id) || null;
  }

  /**
   * Get all records
   * @returns {Object[]}
   */
  async getAll() {
    return [...this._items];
  }

  /**
   * Update a record by ID
   * @param {string} id
   * @param {Object} data - Fields to update
   * @returns {Object|null}
   */
  async update(id, data) {
    const item = this._items.find(i => i.id === id);
    if (!item) return null;
    Object.assign(item, data, { updated_at: new Date().toISOString() });
    this._save();
    return item;
  }

  /**
   * Update status field
   * @param {string} id
   * @param {string} status
   * @returns {Object|null}
   */
  async updateStatus(id, status) {
    return this.update(id, { status });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LEARNING PIPELINE INTERFACE (for feedback/judgment repos)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get unapplied feedback items (for learning cycle consumption)
   * @param {number} limit
   * @returns {Object[]}
   */
  async findUnapplied(limit = 100) {
    return this._items
      .filter(i => i.applied !== true)
      .sort((a, b) => (a.created_at || '').localeCompare(b.created_at || ''))
      .slice(0, limit);
  }

  /**
   * Mark a feedback item as applied (consumed by learning cycle)
   * @param {string} id
   * @returns {Object|null}
   */
  async markApplied(id) {
    const item = this._items.find(i => i.id === id);
    if (!item) return null;
    item.applied = true;
    item.applied_at = new Date().toISOString();
    item.updated_at = new Date().toISOString();
    this._save();
    return item;
  }

  /**
   * Find items by a linked judgment ID
   * @param {string} judgmentId
   * @returns {Object[]}
   */
  async findByJudgment(judgmentId) {
    return this._items
      .filter(i => i.judgment_id === judgmentId)
      .sort((a, b) => (b.created_at || '').localeCompare(a.created_at || ''));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERN REPO INTERFACE (for learning pattern persistence)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Upsert a record by category + name (for patterns)
   * @param {Object} data - Must include category and name
   * @returns {Object} Created or updated record
   */
  async upsert(data) {
    const existing = this._items.find(
      i => i.category === data.category && i.name === data.name
    );
    if (existing) {
      Object.assign(existing, data, { updated_at: new Date().toISOString() });
      this._save();
      return existing;
    }
    return this.create(data);
  }

  /**
   * Find items by category field
   * @param {string} category
   * @returns {Object[]}
   */
  async findByCategory(category) {
    return this._items.filter(i => i.category === category);
  }

  /**
   * Search items by keyword (for knowledge store compatibility)
   * @param {string} query
   * @param {Object} [opts]
   * @returns {Object[]}
   */
  async search(query, opts = {}) {
    const limit = opts.limit || 10;
    return this._items
      .filter(i => {
        if (opts.category && i.category !== opts.category) return false;
        const text = JSON.stringify(i).toLowerCase();
        return text.includes((query || '').toLowerCase());
      })
      .slice(0, limit);
  }

  /**
   * Find items by type field (for patternEvolution compatibility)
   * @param {string} type
   * @returns {Object[]}
   */
  async findByType(type) {
    return this._items.filter(i => i.type === type || i.item_type === type);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DOMAIN-SPECIFIC QUERIES (for daemon compatibility)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Find by type (notifications)
   * @param {string} userId
   * @param {string} type
   * @param {Object} [opts]
   * @returns {Object[]}
   */
  async findByType(userId, type, opts = {}) {
    const limit = opts.limit || 10;
    return this._items
      .filter(i => i.notification_type === type && (!userId || i.user_id === userId))
      .slice(-limit);
  }

  /**
   * Find goals due soon
   * @param {string} userId
   * @param {number} days
   * @returns {Object[]}
   */
  async findDueSoon(userId, days = 3) {
    const cutoff = new Date();
    cutoff.setDate(cutoff.getDate() + days);
    return this._items.filter(i => {
      if (userId && i.user_id !== userId) return false;
      if (i.status === 'completed') return false;
      if (!i.target_date) return false;
      return new Date(i.target_date) <= cutoff;
    });
  }

  /**
   * Find recurring items
   * @returns {Object[]}
   */
  async findRecurring() {
    return this._items.filter(i => i.recurring === true || i.is_recurring === true);
  }

  /**
   * Get pending tasks
   * @param {number} limit
   * @returns {Object[]}
   */
  async getPending(limit = 10) {
    return this._items
      .filter(i => i.status === 'pending')
      .slice(0, limit);
  }

  /**
   * Claim a task (mark as in_progress)
   * @param {string} id
   * @returns {Object|null}
   */
  async claim(id) {
    const item = this._items.find(i => i.id === id && i.status === 'pending');
    if (!item) return null;
    item.status = 'in_progress';
    item.claimed_at = new Date().toISOString();
    item.updated_at = new Date().toISOString();
    this._save();
    return item;
  }

  /**
   * Complete a task
   * @param {string} id
   * @param {*} result
   * @returns {Object|null}
   */
  async complete(id, result) {
    const item = this._items.find(i => i.id === id);
    if (!item) return null;
    item.status = 'completed';
    item.result = result;
    item.completed_at = new Date().toISOString();
    item.updated_at = new Date().toISOString();
    this._save();
    return item;
  }

  /**
   * Fail a task
   * @param {string} id
   * @param {string} reason
   * @returns {Object|null}
   */
  async fail(id, reason) {
    const item = this._items.find(i => i.id === id);
    if (!item) return null;
    item.status = 'failed';
    item.error = reason;
    item.failed_at = new Date().toISOString();
    item.updated_at = new Date().toISOString();
    this._save();
    return item;
  }

  /**
   * Reset stuck tasks (in_progress for too long)
   * @param {number} minutes
   * @returns {number} Count of reset tasks
   */
  async resetStuck(minutes = 30) {
    const cutoff = Date.now() - (minutes * 60 * 1000);
    let count = 0;

    for (const item of this._items) {
      if (item.status !== 'in_progress') continue;
      const claimedAt = item.claimed_at ? new Date(item.claimed_at).getTime() : 0;
      if (claimedAt && claimedAt < cutoff) {
        item.status = 'pending';
        item.updated_at = new Date().toISOString();
        count++;
      }
    }

    if (count > 0) this._save();
    return count;
  }

  /**
   * Cleanup expired items
   * @returns {number} Count of removed items
   */
  async cleanupExpired() {
    const now = Date.now();
    const before = this._items.length;
    this._items = this._items.filter(i => {
      if (!i.expires_at) return true;
      return new Date(i.expires_at).getTime() > now;
    });
    const removed = before - this._items.length;
    if (removed > 0) this._save();
    return removed;
  }

  /**
   * Get repo stats
   * @returns {Object}
   */
  getStats() {
    return {
      name: this.name,
      backend: 'file',
      filePath: this._filePath,
      totalRecords: this._items.length,
      maxRecords: MAX_RECORDS,
    };
  }
}

/**
 * Create a file-backed repository
 * @param {string} name
 * @returns {FileBackedRepo}
 */
export function createFileBackedRepo(name) {
  return new FileBackedRepo(name);
}

export default { FileBackedRepo, createFileBackedRepo };
