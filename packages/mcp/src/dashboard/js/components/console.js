/**
 * CYNIC Dashboard - Console Component
 * REPL console (collapsible)
 */

import { Utils } from '../lib/utils.js';
import { Formulas } from '../lib/formulas.js';

export class Console {
  constructor(options = {}) {
    this.api = options.api;
    this.onCommand = options.onCommand || (() => {});
    this.container = null;
    this.outputEl = null;
    this.inputEl = null;
    this.history = [];
    this.historyIndex = -1;
    this.isOpen = false;

    // Built-in commands
    this.commands = {
      help: { description: 'Show available commands', execute: () => this._showHelp() },
      clear: { description: 'Clear console output', execute: () => this._clear() },
      phi: { description: 'Show PHI constants', execute: () => this._showPhi() },
      axioms: { description: 'Show CYNIC axioms', execute: () => this._showAxioms() },
      dimensions: { description: 'Show 25 dimensions', execute: () => this._showDimensions() },
      health: { description: 'System health check', execute: () => this._health() },
      dogs: { description: 'Show 11 Dogs status', execute: () => this._dogs() },
      chain: { description: 'PoJ chain status', execute: (args) => this._chain(args) },
      judge: { description: 'Judge an item', execute: (args) => this._judge(args) },
      patterns: { description: 'Show patterns', execute: (args) => this._patterns(args) },
    };
  }

  /**
   * Render console
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);

    // Header
    const header = Utils.createElement('div', { className: 'console-header' }, [
      Utils.createElement('div', { className: 'console-title' }, [
        Utils.createElement('span', { className: 'console-title-icon' }, ['>']),
        Utils.createElement('span', {}, ['CYNIC Console']),
      ]),
      Utils.createElement('div', { className: 'console-actions' }, [
        Utils.createElement('button', {
          className: 'console-action-btn',
          onClick: () => this._clear(),
          title: 'Clear',
        }, ['⌫']),
        Utils.createElement('button', {
          className: 'console-action-btn',
          onClick: () => this.toggle(false),
          title: 'Close',
        }, ['✕']),
      ]),
    ]);

    // Output area
    this.outputEl = Utils.createElement('div', { className: 'console-output', id: 'console-output' });

    // Input area
    const inputContainer = Utils.createElement('div', { className: 'console-input-container' }, [
      Utils.createElement('span', { className: 'console-prompt' }, ['>']),
    ]);

    this.inputEl = Utils.createElement('input', {
      className: 'console-input',
      id: 'console-input',
      placeholder: 'Type a command...',
    });

    this.inputEl.addEventListener('keydown', (e) => this._handleKeyDown(e));
    inputContainer.appendChild(this.inputEl);

    container.appendChild(header);
    container.appendChild(this.outputEl);
    container.appendChild(inputContainer);

    // Welcome message
    this.log('CYNIC Console v1.0', 'system');
    this.log('Type "help" for commands', 'system');
    this.log('─'.repeat(40), 'system');
  }

  /**
   * Handle keyboard input
   */
  _handleKeyDown(e) {
    if (e.key === 'Enter') {
      const input = this.inputEl.value.trim();
      if (input) {
        this._execute(input);
        this.history.push(input);
        this.historyIndex = this.history.length;
      }
      this.inputEl.value = '';
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (this.historyIndex > 0) {
        this.historyIndex--;
        this.inputEl.value = this.history[this.historyIndex];
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (this.historyIndex < this.history.length - 1) {
        this.historyIndex++;
        this.inputEl.value = this.history[this.historyIndex];
      } else {
        this.historyIndex = this.history.length;
        this.inputEl.value = '';
      }
    }
  }

  /**
   * Execute command
   */
  async _execute(input) {
    this.log('> ' + input, 'input');

    try {
      // Parse command
      const match = input.match(/^(\w+)(?:\((.*)\))?$/);

      if (!match) {
        // Try as math expression
        if (this._tryMath(input)) return;
        this.log('Invalid command. Type "help" for commands.', 'error');
        return;
      }

      const [, cmd, argsStr] = match;
      const args = argsStr ? this._parseArgs(argsStr) : null;

      // Execute command
      if (this.commands[cmd]) {
        const result = await this.commands[cmd].execute(args);
        if (result !== undefined && result !== null) {
          if (typeof result === 'object') {
            this.log(JSON.stringify(result, null, 2), 'output');
          } else {
            this.log(String(result), 'output');
          }
        }
      } else {
        this.log('Unknown command: ' + cmd, 'error');
        this.log('Type "help" for available commands', 'system');
      }

      // Notify callback
      this.onCommand(input);
    } catch (err) {
      this.log('Error: ' + err.message, 'error');
    }
  }

  /**
   * Try to evaluate as math (safe parser, no eval/Function)
   */
  _tryMath(expr) {
    // Replace constants
    const sanitized = expr
      .replace(/PHI_INV/g, String(Formulas.PHI_INV))
      .replace(/PHI/g, String(Formulas.PHI));

    // Only allow safe characters
    if (/^[\d\s+\-*/().,]+$/.test(sanitized)) {
      try {
        const result = this._safeMathEval(sanitized);
        if (result !== null) {
          this.log(String(result), 'output');
          return true;
        }
      } catch {
        return false;
      }
    }
    return false;
  }

  /**
   * Safe math evaluator using recursive descent parsing
   * Handles: +, -, *, /, parentheses, and numbers
   * @private
   */
  _safeMathEval(expr) {
    let pos = 0;
    const str = expr.replace(/\s+/g, '');

    const parseNumber = () => {
      let numStr = '';
      while (pos < str.length && /[\d.]/.test(str[pos])) {
        numStr += str[pos++];
      }
      return parseFloat(numStr);
    };

    const parseFactor = () => {
      if (str[pos] === '(') {
        pos++; // skip '('
        const result = parseExpr();
        pos++; // skip ')'
        return result;
      }
      if (str[pos] === '-') {
        pos++;
        return -parseFactor();
      }
      return parseNumber();
    };

    const parseTerm = () => {
      let left = parseFactor();
      while (pos < str.length && (str[pos] === '*' || str[pos] === '/')) {
        const op = str[pos++];
        const right = parseFactor();
        left = op === '*' ? left * right : left / right;
      }
      return left;
    };

    const parseExpr = () => {
      let left = parseTerm();
      while (pos < str.length && (str[pos] === '+' || str[pos] === '-')) {
        const op = str[pos++];
        const right = parseTerm();
        left = op === '+' ? left + right : left - right;
      }
      return left;
    };

    try {
      const result = parseExpr();
      return isNaN(result) ? null : result;
    } catch {
      return null;
    }
  }

  /**
   * Parse command arguments
   */
  _parseArgs(argsStr) {
    const num = parseFloat(argsStr);
    if (!isNaN(num)) return num;

    try {
      return JSON.parse(argsStr.replace(/'/g, '"'));
    } catch {
      return argsStr.replace(/['"]/g, '');
    }
  }

  /**
   * Log message to output
   */
  log(message, type = 'output') {
    if (!this.outputEl) return;

    const line = Utils.createElement('div', { className: `console-line ${type}` }, [message]);
    this.outputEl.appendChild(line);
    this.outputEl.scrollTop = this.outputEl.scrollHeight;
  }

  /**
   * Clear console
   */
  _clear() {
    if (this.outputEl) {
      Utils.clearElement(this.outputEl);
    }
    this.log('Console cleared', 'system');
  }

  /**
   * Show help
   */
  _showHelp() {
    this.log('─── CYNIC Console Commands ───', 'system');
    for (const [name, cmd] of Object.entries(this.commands)) {
      this.log(`  ${name} - ${cmd.description}`, 'system');
    }
    this.log('─'.repeat(30), 'system');
    this.log('Math expressions: PHI, PHI_INV, 1+2*3', 'system');
  }

  /**
   * Show PHI constants
   */
  _showPhi() {
    this.log('─── PHI Constants ───', 'system');
    this.log(`  φ (PHI)     = ${Formulas.PHI.toFixed(15)}`, 'output');
    this.log(`  φ⁻¹         = ${Formulas.PHI_INV.toFixed(15)}`, 'output');
    this.log(`  φ⁻²         = ${Formulas.PHI_INV_SQ.toFixed(15)}`, 'output');
    this.log('', 'system');
    this.log(`  Max confidence: ${(Formulas.PHI_INV * 100).toFixed(2)}%`, 'output');
  }

  /**
   * Show axioms
   */
  _showAxioms() {
    this.log('─── CYNIC Axioms ───', 'system');
    for (const [name, axiom] of Object.entries(Formulas.AXIOMS)) {
      this.log(`  ${name} (w=${axiom.weight.toFixed(3)})`, 'output');
      this.log(`    ${axiom.description}`, 'system');
    }
  }

  /**
   * Show dimensions
   */
  _showDimensions() {
    this.log('─── 25 Dimensions ───', 'system');
    let count = 0;
    for (const [axiom, dims] of Object.entries(Formulas.DIMENSIONS)) {
      this.log('', 'system');
      this.log(`[${axiom}]`, 'output');
      for (const dim of dims) {
        count++;
        this.log(`  ${count}. ${dim.name}`, 'system');
      }
    }
    this.log('', 'system');
    this.log(`Total: ${count} dimensions`, 'output');
  }

  /**
   * Health check
   */
  async _health() {
    if (!this.api) {
      this.log('API not connected', 'error');
      return;
    }

    this.log('Fetching health...', 'system');
    const result = await this.api.health(true);

    if (result.success && result.result) {
      const h = result.result;
      this.log('─── System Health ───', 'system');
      this.log(`  Status: ${h.status || 'ok'}`, 'output');

      if (h.services) {
        for (const [name, svc] of Object.entries(h.services)) {
          const status = svc.status === 'ok' || svc.healthy ? '🟢' : '🔴';
          this.log(`  ${name}: ${status}`, 'output');
        }
      }
    } else {
      this.log('Error: ' + (result.error || 'Unknown'), 'error');
    }
  }

  /**
   * Dogs status
   */
  async _dogs() {
    if (!this.api) {
      this.log('API not connected', 'error');
      return;
    }

    this.log('Fetching collective status...', 'system');
    const result = await this.api.collectiveStatus(true);

    if (result.success && result.result) {
      const r = result.result;
      this.log('─── 11 Dogs (Collective) ───', 'system');

      if (r.agents) {
        for (const [name, data] of Object.entries(r.agents)) {
          const status = data.active !== false ? '🟢' : '🔴';
          this.log(`  ${status} ${name}`, 'output');
        }
      }

      if (r.summary) {
        this.log('', 'system');
        this.log(`  Active: ${r.summary.active}/${r.summary.total}`, 'output');
      }
    } else {
      this.log('Error: ' + (result.error || 'Unknown'), 'error');
    }
  }

  /**
   * Chain status
   */
  async _chain(args) {
    if (!this.api) {
      this.log('API not connected', 'error');
      return;
    }

    const action = args || 'status';
    this.log(`Fetching chain ${action}...`, 'system');
    const result = await this.api.chain(action);

    if (result.success && result.result) {
      const r = result.result;
      this.log('─── PoJ Chain ───', 'system');

      if (r.head) {
        this.log(`  Block:      #${r.head.blockNumber}`, 'output');
        this.log(`  Judgments:  ${r.head.judgmentCount}`, 'output');
        this.log(`  Hash:       ${Utils.truncateHash(r.head.hash)}`, 'output');
      }

      if (r.stats) {
        this.log(`  Total:      ${r.stats.totalBlocks} blocks`, 'output');
        this.log(`  Judgments:  ${r.stats.totalJudgments}`, 'output');
      }
    } else {
      this.log('Error: ' + (result.error || 'Unknown'), 'error');
    }
  }

  /**
   * Judge an item
   */
  async _judge(args) {
    if (!this.api) {
      this.log('API not connected', 'error');
      return;
    }

    const item = args
      ? { content: String(args), type: 'test' }
      : { type: 'test', content: 'Test judgment' };

    this.log('Judging...', 'system');
    const result = await this.api.judge(item);

    if (result.success && result.result) {
      const r = result.result;
      this.log('─── Judgment Result ───', 'system');
      this.log(`  Q-Score:    ${r.qScore}/100`, 'output');
      this.log(`  Confidence: ${(r.confidence * 100).toFixed(2)}%`, 'output');
      this.log(`  Verdict:    ${r.verdict}`, 'output');
      return r;
    } else {
      this.log('Error: ' + (result.error || 'Unknown'), 'error');
    }
  }

  /**
   * Show patterns
   */
  async _patterns(args) {
    if (!this.api) {
      this.log('API not connected', 'error');
      return;
    }

    const count = args || 5;
    this.log('Fetching patterns...', 'system');
    const result = await this.api.patterns('all', count);

    if (result.success && result.result) {
      const patterns = result.result.patterns || [];
      this.log(`─── Patterns (${patterns.length}) ───`, 'system');

      if (patterns.length === 0) {
        this.log('  No patterns detected yet', 'system');
      } else {
        patterns.forEach((p, i) => {
          this.log(`  ${i + 1}. [${(p.category || p.type || 'PATTERN').toUpperCase()}] ${p.description}`, 'output');
        });
      }
    } else {
      this.log('Error: ' + (result.error || 'Unknown'), 'error');
    }
  }

  /**
   * Toggle console visibility
   */
  toggle(open) {
    this.isOpen = open !== undefined ? open : !this.isOpen;
    this.container?.classList.toggle('open', this.isOpen);

    if (this.isOpen && this.inputEl) {
      setTimeout(() => this.inputEl.focus(), 100);
    }
  }
}

// Export to window
window.CYNICConsole = Console;
