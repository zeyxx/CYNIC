/**
 * Fact Extractor (Local/Brain)
 *
 * "Le chien extrait les faits" - Extract factual knowledge from tool outputs
 *
 * P0.2: MoltBrain-style fact extraction for semantic memory.
 * Facts are discrete pieces of knowledge that can be retrieved later.
 *
 * @module scripts/hooks/lib/fact-extractor-local
 */

'use strict';

import path from 'path';
import { callBrainTool } from '../../lib/index.js';

/**
 * Extract facts from tool outputs for semantic memory
 * Facts are discrete pieces of knowledge that can be retrieved later
 *
 * @param {string} toolName - Name of the tool used
 * @param {Object} toolInput - Tool input parameters
 * @param {*} toolOutput - Tool output
 * @param {boolean} isError - Whether the tool errored
 * @returns {Array<{type: string, content: string, confidence: number, context: object}>}
 */
export function extractFacts(toolName, toolInput, toolOutput, isError) {
  const facts = [];
  const outputStr = typeof toolOutput === 'string' ? toolOutput : JSON.stringify(toolOutput || {});
  const filePath = toolInput?.file_path || toolInput?.filePath || '';

  // Don't extract facts from errors (they're handled separately)
  if (isError) return facts;

  // 1. File structure facts from Read/Glob
  if (toolName === 'Read' && filePath) {
    const ext = path.extname(filePath).toLowerCase();

    // Extract function/class definitions from code files
    if (['.js', '.ts', '.tsx', '.jsx', '.py', '.rs', '.go'].includes(ext)) {
      // JavaScript/TypeScript functions
      const funcMatches = outputStr.match(/(?:function|const|let|var)\s+(\w+)\s*(?:=\s*(?:async\s*)?\(|[\(<])/g);
      if (funcMatches && funcMatches.length > 0) {
        const funcNames = funcMatches.slice(0, 5).map(m => m.match(/(?:function|const|let|var)\s+(\w+)/)?.[1]).filter(Boolean);
        if (funcNames.length > 0) {
          facts.push({
            type: 'file_structure',
            content: `File ${path.basename(filePath)} contains functions: ${funcNames.join(', ')}`,
            confidence: 0.8,
            context: { file: filePath, functions: funcNames },
          });
        }
      }

      // Class definitions
      const classMatches = outputStr.match(/class\s+(\w+)/g);
      if (classMatches && classMatches.length > 0) {
        const classNames = classMatches.map(m => m.match(/class\s+(\w+)/)?.[1]).filter(Boolean);
        facts.push({
          type: 'file_structure',
          content: `File ${path.basename(filePath)} defines classes: ${classNames.join(', ')}`,
          confidence: 0.9,
          context: { file: filePath, classes: classNames },
        });
      }

      // Export facts
      const exportMatches = outputStr.match(/export\s+(?:default\s+)?(?:const|function|class|let|var)?\s*(\w+)/g);
      if (exportMatches && exportMatches.length > 0) {
        const exports = exportMatches.slice(0, 10).map(m => m.match(/(\w+)$/)?.[1]).filter(Boolean);
        facts.push({
          type: 'module_exports',
          content: `${path.basename(filePath)} exports: ${exports.join(', ')}`,
          confidence: 0.9,
          context: { file: filePath, exports },
        });
      }
    }

    // Extract API endpoints from code
    const apiMatches = outputStr.match(/['"`](\/api\/[^'"`]+)['"`]|router\.(get|post|put|delete)\s*\(\s*['"`]([^'"`]+)['"`]/gi);
    if (apiMatches && apiMatches.length > 0) {
      const endpoints = [...new Set(apiMatches.map(m => m.match(/['"`]([^'"`]+)['"`]/)?.[1]).filter(Boolean))];
      if (endpoints.length > 0) {
        facts.push({
          type: 'api_endpoint',
          content: `API endpoints in ${path.basename(filePath)}: ${endpoints.join(', ')}`,
          confidence: 0.85,
          context: { file: filePath, endpoints },
        });
      }
    }

    // Extract environment variable references
    const envMatches = outputStr.match(/process\.env\.(\w+)|import\.meta\.env\.(\w+)|\$\{(\w+)\}/g);
    if (envMatches && envMatches.length > 0) {
      const envVars = [...new Set(envMatches.map(m => m.match(/\.(\w+)|\{(\w+)\}/)?.[1] || m.match(/\.(\w+)|\{(\w+)\}/)?.[2]).filter(Boolean))];
      if (envVars.length > 0) {
        facts.push({
          type: 'env_dependency',
          content: `${path.basename(filePath)} uses env vars: ${envVars.slice(0, 10).join(', ')}`,
          confidence: 0.9,
          context: { file: filePath, envVars },
        });
      }
    }
  }

  // 2. Git facts from Bash git commands
  if (toolName === 'Bash' && toolInput.command?.startsWith('git ')) {
    const command = toolInput.command;

    // Branch facts
    if (command.includes('git branch') && outputStr) {
      const currentBranch = outputStr.match(/\*\s+(\S+)/)?.[1];
      if (currentBranch) {
        facts.push({
          type: 'git_state',
          content: `Current branch: ${currentBranch}`,
          confidence: 1.0,
          context: { branch: currentBranch },
        });
      }
    }

    // Commit facts
    if (command.startsWith('git log') && outputStr) {
      const commitMatches = outputStr.match(/commit\s+([a-f0-9]+)/gi);
      if (commitMatches && commitMatches.length > 0) {
        const recentCommit = commitMatches[0].match(/([a-f0-9]+)/)?.[1];
        facts.push({
          type: 'git_state',
          content: `Recent commit: ${recentCommit?.substring(0, 7)}`,
          confidence: 1.0,
          context: { commit: recentCommit },
        });
      }
    }
  }

  // 3. Test facts from test outputs
  if (toolName === 'Bash' && toolInput.command?.match(/npm\s+(run\s+)?test|jest|vitest|pytest/i)) {
    // Extract test count
    const passMatch = outputStr.match(/(\d+)\s*(?:tests?\s+)?pass(?:ed|ing)?/i);
    const failMatch = outputStr.match(/(\d+)\s*(?:tests?\s+)?fail(?:ed|ing)?/i);
    if (passMatch || failMatch) {
      const passed = parseInt(passMatch?.[1] || '0', 10);
      const failed = parseInt(failMatch?.[1] || '0', 10);
      facts.push({
        type: 'test_state',
        content: `Test results: ${passed} passed, ${failed} failed`,
        confidence: 0.95,
        context: { passed, failed, total: passed + failed },
      });
    }

    // Extract failing test names
    const failingTests = outputStr.match(/(?:FAIL|✕|×)\s+(.+?)(?:\n|$)/gi);
    if (failingTests && failingTests.length > 0) {
      const testNames = failingTests.slice(0, 5).map(t => t.replace(/(?:FAIL|✕|×)\s+/, '').trim());
      facts.push({
        type: 'test_failure',
        content: `Failing tests: ${testNames.join(', ')}`,
        confidence: 0.9,
        context: { failingTests: testNames },
      });
    }
  }

  // 4. Dependency facts from package.json reads
  if (toolName === 'Read' && filePath.endsWith('package.json')) {
    try {
      const pkg = JSON.parse(outputStr);
      if (pkg.dependencies) {
        const deps = Object.keys(pkg.dependencies).slice(0, 10);
        facts.push({
          type: 'dependency',
          content: `Project dependencies: ${deps.join(', ')}${Object.keys(pkg.dependencies).length > 10 ? '...' : ''}`,
          confidence: 1.0,
          context: { dependencies: Object.keys(pkg.dependencies) },
        });
      }
      if (pkg.scripts) {
        facts.push({
          type: 'npm_scripts',
          content: `Available scripts: ${Object.keys(pkg.scripts).join(', ')}`,
          confidence: 1.0,
          context: { scripts: Object.keys(pkg.scripts) },
        });
      }
    } catch (e) {
      // Not valid JSON - ignore
    }
  }

  // 5. Write/Edit facts - what was changed
  if ((toolName === 'Write' || toolName === 'Edit') && filePath) {
    facts.push({
      type: 'file_modified',
      content: `Modified: ${path.basename(filePath)}`,
      confidence: 1.0,
      context: { file: filePath, action: toolName.toLowerCase() },
    });
  }

  return facts;
}

/**
 * Store extracted facts to brain memory for semantic retrieval
 *
 * @param {Array} facts - Array of fact objects
 * @param {Object} context - Additional context (tool, project, userId)
 */
export async function storeFacts(facts, context = {}) {
  if (!facts || facts.length === 0) return;

  for (const fact of facts) {
    try {
      // Store to brain memory with embedding for semantic search
      await callBrainTool('brain_memory_store', {
        type: fact.type,
        content: fact.content,
        confidence: fact.confidence,
        context: {
          ...fact.context,
          ...context,
          extractedAt: Date.now(),
        },
        // Flag as extracted fact (vs observed pattern)
        metadata: { source: 'fact_extraction', version: 'p0.2' },
      });
    } catch (e) {
      // Storage failed - continue with other facts
    }
  }
}

export default {
  extractFacts,
  storeFacts,
};
