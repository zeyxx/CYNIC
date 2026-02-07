#!/usr/bin/env node
/**
 * CYNIC Platform Compatibility Fixer - SessionStart
 *
 * "Le chien s'adapte au terrain" - CYNIC adapts to the platform
 *
 * Runs at session start to detect and fix platform-specific
 * incompatibilities in plugin hooks. Fires before awaken.js.
 *
 * Known fixes:
 * - python3 → python on Windows (python3 command doesn't exist)
 * - 2>/dev/null → removed on Windows (cmd.exe doesn't support it)
 *
 * @event SessionStart
 * @behavior non-blocking
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';

const isWindows = os.platform() === 'win32';

// Only run on Windows — Linux/macOS don't have these issues
if (!isWindows) {
  process.stdout.write(JSON.stringify({ message: '' }) + '\n');
  process.exit(0);
}

const FIXES = [
  {
    name: 'python3 → python',
    // python3 doesn't exist on Windows, python does
    test: (content) => content.includes('"python3 '),
    fix: (content) => content.replace(/"python3 /g, '"python '),
  },
  {
    name: '2>/dev/null removal',
    // cmd.exe doesn't support Unix redirects — creates literal "nul" files
    test: (content) => content.includes(' 2>/dev/null'),
    fix: (content) => content.replace(/ 2>\/dev\/null/g, ''),
  },
];

function findPluginHooksFiles() {
  const homeDir = os.homedir();
  const pluginDirs = [
    path.join(homeDir, '.claude', 'plugins', 'cache'),
    path.join(homeDir, '.claude', 'plugins', 'marketplaces'),
  ];

  const results = [];

  function walk(dir) {
    try {
      const entries = fs.readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(full);
        } else if (entry.name === 'hooks.json') {
          results.push(full);
        }
      }
    } catch {
      // Directory doesn't exist or not readable — skip
    }
  }

  for (const d of pluginDirs) {
    walk(d);
  }
  return results;
}

function main() {
  const fixed = [];

  const hooksFiles = findPluginHooksFiles();

  for (const filePath of hooksFiles) {
    try {
      let content = fs.readFileSync(filePath, 'utf8');
      let modified = false;

      for (const rule of FIXES) {
        if (rule.test(content)) {
          content = rule.fix(content);
          fixed.push({ file: path.basename(path.dirname(path.dirname(filePath))), fix: rule.name });
          modified = true;
        }
      }

      if (modified) {
        fs.writeFileSync(filePath, content, 'utf8');
      }
    } catch {
      // Can't read/write — skip silently
    }
  }

  // Output for Claude Code
  if (fixed.length > 0) {
    const summary = fixed.map(f => `${f.file}: ${f.fix}`).join(', ');
    process.stdout.write(JSON.stringify({
      message: `*sniff* Platform fixes applied: ${summary}`,
    }) + '\n');
  } else {
    process.stdout.write(JSON.stringify({ message: '' }) + '\n');
  }
}

main();
