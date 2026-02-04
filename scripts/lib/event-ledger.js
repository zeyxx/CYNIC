/**
 * Event Ledger — Append-only JSONL event log for session continuity
 *
 * "Le chien n'oublie jamais ce qui s'est passé"
 *
 * Stolen from Continuous-Claude concept: maintains an append-only log
 * of session events, enabling handoff summaries between sessions.
 *
 * File: ~/.cynic/ledger/events-YYYY-MM-DD.jsonl
 * Retention: 7 days (automatic rotation)
 * Events: SESSION_START, TOOL_CALL, ERROR, REFLECTION, JUDGMENT
 *
 * @module scripts/lib/event-ledger
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';

const DEFAULT_LEDGER_DIR = path.join(os.homedir(), '.cynic', 'ledger');
const RETENTION_DAYS = 7;
const MAX_HANDOFF_TOKENS = 500; // ~500 tokens max for handoff

/** Singleton instance */
let _instance = null;

export class EventLedger {
  /**
   * @param {string} [ledgerDir] - Directory for ledger files
   */
  constructor(ledgerDir = DEFAULT_LEDGER_DIR) {
    this.ledgerDir = ledgerDir;
    this._ensureDir();
  }

  /**
   * Ensure ledger directory exists
   */
  _ensureDir() {
    try {
      if (!fs.existsSync(this.ledgerDir)) {
        fs.mkdirSync(this.ledgerDir, { recursive: true });
      }
    } catch { /* ignore */ }
  }

  /**
   * Get today's ledger file path
   * @returns {string}
   */
  _todayFile() {
    const date = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
    return path.join(this.ledgerDir, `events-${date}.jsonl`);
  }

  /**
   * Append a single event to today's ledger
   * @param {Object} event - Event data
   * @param {string} event.type - Event type (SESSION_START, TOOL_CALL, ERROR, REFLECTION, JUDGMENT)
   * @param {string} [event.tool] - Tool name (for TOOL_CALL/ERROR)
   * @param {number} [event.duration] - Duration in ms
   * @param {string} [event.sessionId] - Session ID
   * @param {string} [event.summary] - Brief summary
   * @param {Object} [event.metadata] - Additional metadata
   */
  append(event) {
    try {
      const entry = {
        ts: Date.now(),
        ...event,
      };
      const line = JSON.stringify(entry) + '\n';
      fs.appendFileSync(this._todayFile(), line);
    } catch { /* ledger is optional, never block */ }
  }

  /**
   * Read last N events across recent files (newest first)
   * @param {number} [limit=50] - Max events to return
   * @returns {Array<Object>} Events
   */
  getRecentEvents(limit = 50) {
    try {
      const files = this._getLedgerFiles().reverse(); // newest first
      const events = [];

      for (const file of files) {
        if (events.length >= limit) break;
        const lines = fs.readFileSync(file, 'utf8')
          .split('\n')
          .filter(Boolean)
          .reverse(); // newest first within file

        for (const line of lines) {
          if (events.length >= limit) break;
          try {
            events.push(JSON.parse(line));
          } catch { /* skip malformed */ }
        }
      }

      return events;
    } catch {
      return [];
    }
  }

  /**
   * Read events for a specific session
   * @param {string} sessionId
   * @returns {Array<Object>}
   */
  getSessionEvents(sessionId) {
    if (!sessionId) return [];
    try {
      const allEvents = this.getRecentEvents(500);
      return allEvents.filter(e => e.sessionId === sessionId);
    } catch {
      return [];
    }
  }

  /**
   * Generate a compact handoff summary for the next session
   * ~500 tokens max, focuses on what matters for continuity
   *
   * @returns {Object} Handoff object
   */
  generateHandoff() {
    try {
      const events = this.getRecentEvents(200);
      if (events.length === 0) return null;

      // Find session boundaries
      const sessionStarts = events.filter(e => e.type === 'SESSION_START');
      const currentSessionId = sessionStarts[0]?.sessionId || events[0]?.sessionId;

      // Filter to current/last session
      const sessionEvents = currentSessionId
        ? events.filter(e => e.sessionId === currentSessionId)
        : events.slice(0, 50);

      // Aggregate tool usage
      const toolCounts = {};
      const errors = [];
      const reflections = [];
      const filesModified = new Set();

      for (const event of sessionEvents) {
        if (event.type === 'TOOL_CALL' && event.tool) {
          toolCounts[event.tool] = (toolCounts[event.tool] || 0) + 1;
          if (event.file) filesModified.add(event.file);
        }
        if (event.type === 'ERROR') {
          errors.push({
            tool: event.tool,
            error: event.summary?.substring(0, 80),
            ts: event.ts,
          });
        }
        if (event.type === 'REFLECTION') {
          reflections.push(event.summary?.substring(0, 80));
        }
      }

      // Top tools
      const topTools = Object.entries(toolCounts)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 5)
        .map(([tool, count]) => `${tool}(${count})`);

      // Unresolved errors (errors without a subsequent success on same tool)
      const resolvedTools = new Set();
      for (const event of sessionEvents) {
        if (event.type === 'TOOL_CALL' && !event.isError) {
          resolvedTools.add(event.tool);
        }
      }
      const unresolvedErrors = errors.filter(e => !resolvedTools.has(e.tool));

      // Build summary string (compact)
      const parts = [];
      if (topTools.length > 0) {
        parts.push(`Tools: ${topTools.join(', ')}`);
      }
      if (filesModified.size > 0) {
        const files = [...filesModified].slice(0, 5);
        parts.push(`Files: ${files.map(f => path.basename(f)).join(', ')}`);
      }
      if (unresolvedErrors.length > 0) {
        parts.push(`Unresolved: ${unresolvedErrors.length} error(s)`);
      }
      if (reflections.length > 0) {
        parts.push(`Reflections: ${reflections.length}`);
      }

      return {
        sessionId: currentSessionId,
        eventCount: sessionEvents.length,
        summary: parts.join(' | '),
        topTools,
        filesModified: [...filesModified].slice(0, 10),
        unresolvedErrors: unresolvedErrors.slice(0, 5),
        reflections: reflections.slice(0, 3),
        generatedAt: Date.now(),
      };
    } catch {
      return null;
    }
  }

  /**
   * Remove ledger files older than retention period
   */
  cleanup() {
    try {
      const cutoff = Date.now() - (RETENTION_DAYS * 24 * 60 * 60 * 1000);
      const files = this._getLedgerFiles();

      for (const file of files) {
        const stat = fs.statSync(file);
        if (stat.mtimeMs < cutoff) {
          fs.unlinkSync(file);
        }
      }
    } catch { /* cleanup is optional */ }
  }

  /**
   * Get all ledger files sorted by date
   * @returns {string[]}
   */
  _getLedgerFiles() {
    try {
      return fs.readdirSync(this.ledgerDir)
        .filter(f => f.startsWith('events-') && f.endsWith('.jsonl'))
        .sort()
        .map(f => path.join(this.ledgerDir, f));
    } catch {
      return [];
    }
  }
}

/**
 * Get singleton EventLedger instance
 * @returns {EventLedger}
 */
export function getEventLedger() {
  if (!_instance) {
    _instance = new EventLedger();
  }
  return _instance;
}
