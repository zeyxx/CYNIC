/**
 * cynic init - Project Scaffolding
 *
 * Creates a basic CYNIC project structure with configuration files.
 *
 * Usage:
 *   cynic init [project-name]
 *   cynic init --template minimal
 *
 * @module @cynic/node/cli/commands/init
 */

'use strict';

import { mkdir, writeFile, access, readFile } from 'fs/promises';
import { join, resolve } from 'path';
import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Project templates
 */
const TEMPLATES = {
  minimal: {
    description: 'Minimal CYNIC setup (hooks only)',
    files: ['settings.json', 'hooks/'],
  },
  standard: {
    description: 'Standard CYNIC setup (hooks + dogs)',
    files: ['settings.json', 'hooks/', 'agents/', 'skills/'],
  },
  full: {
    description: 'Full CYNIC setup with all components',
    files: ['settings.json', 'hooks/', 'agents/', 'skills/', 'commands/', 'plugins/'],
  },
};

/**
 * Generate settings.json content
 */
function generateSettings(projectName) {
  return JSON.stringify({
    "$schema": "https://raw.githubusercontent.com/anthropics/claude-code/main/schemas/settings.json",
    "hooks": {
      "SessionStart": [
        {
          "type": "command",
          "command": "echo \"CYNIC awakening for ${projectName}\"",
          "async": true
        }
      ],
      "SessionEnd": [
        {
          "type": "command",
          "command": "echo \"CYNIC sleeping...\"",
          "async": true
        }
      ]
    },
    "permissions": {
      "allow": [],
      "deny": []
    }
  }, null, 2);
}

/**
 * Generate CLAUDE.md content
 */
function generateClaudeMd(projectName) {
  return `# ${projectName}

> CYNIC-powered project

## Project Identity

This project uses CYNIC for collective consciousness and φ-aligned judgment.

## Commands

- \`/status\` - Show project status
- \`/judge <item>\` - Judge an item using CYNIC's 25-dimension system

## Configuration

Settings are in \`.claude/settings.local.json\`.

## Philosophy

> "φ distrusts φ" - Maximum confidence: 61.8%
>
> *Loyal to truth, not to comfort*
`;
}

/**
 * Generate sample hook script
 */
function generateSampleHook() {
  return `#!/usr/bin/env node
/**
 * Sample CYNIC Hook
 *
 * This hook runs on specific events.
 * Modify to add your own logic.
 */

const data = JSON.parse(process.argv[2] || '{}');

// Example: Log session start
if (data.type === 'SessionStart') {
  console.log('CYNIC awakening...');
  console.log('Project:', data.project?.name);
}

// Always output valid JSON for prompt hooks
// process.stdout.write(JSON.stringify({ continue: true }));
`;
}

/**
 * Generate sample agent
 */
function generateSampleAgent() {
  return `---
name: sample-agent
description: Sample CYNIC agent for demonstration
when_to_use: Use this agent when you need to demonstrate agent capabilities
tools:
  - Read
  - Grep
  - Glob
---

You are a sample CYNIC agent.

Your role is to demonstrate how agents work in CYNIC.

## Capabilities

1. Read files using the Read tool
2. Search code using Grep
3. Find files using Glob

## Behavior

- Always be helpful and concise
- Maximum confidence: 61.8% (φ⁻¹)
- Report findings clearly

*"Loyal to truth, not to comfort"*
`;
}

/**
 * Create project directory structure
 */
async function createProjectStructure(projectPath, template, projectName) {
  const structure = [];

  // .claude directory
  const claudeDir = join(projectPath, '.claude');
  await mkdir(claudeDir, { recursive: true });
  structure.push('.claude/');

  // settings.local.json
  const settingsPath = join(claudeDir, 'settings.local.json');
  await writeFile(settingsPath, generateSettings(projectName));
  structure.push('.claude/settings.local.json');

  // CLAUDE.md
  const claudeMdPath = join(projectPath, 'CLAUDE.md');
  await writeFile(claudeMdPath, generateClaudeMd(projectName));
  structure.push('CLAUDE.md');

  // Template-specific directories
  const templateDef = TEMPLATES[template] || TEMPLATES.standard;

  for (const file of templateDef.files) {
    if (file.endsWith('/')) {
      // Directory
      const dirName = file.slice(0, -1);
      const dirPath = join(claudeDir, dirName);
      await mkdir(dirPath, { recursive: true });
      structure.push(`.claude/${dirName}/`);

      // Add sample files based on directory type
      if (dirName === 'hooks') {
        const samplePath = join(dirPath, 'sample-hook.js');
        await writeFile(samplePath, generateSampleHook());
        structure.push(`.claude/hooks/sample-hook.js`);
      } else if (dirName === 'agents') {
        const samplePath = join(dirPath, 'sample-agent.md');
        await writeFile(samplePath, generateSampleAgent());
        structure.push(`.claude/agents/sample-agent.md`);
      }
    }
  }

  return structure;
}

/**
 * Check if directory exists and is empty
 */
async function checkDirectory(path) {
  try {
    await access(path);
    // Directory exists, check if .claude already exists
    try {
      await access(join(path, '.claude'));
      return { exists: true, hasClaudeDir: true };
    } catch {
      return { exists: true, hasClaudeDir: false };
    }
  } catch {
    return { exists: false, hasClaudeDir: false };
  }
}

/**
 * Init command handler
 */
export async function initCommand(projectName, options) {
  const { template = 'standard', force = false } = options;

  console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
  console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Project Initialization          ') + chalk.yellow('║'));
  console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));

  // Determine project path
  const projectPath = projectName ? resolve(projectName) : process.cwd();
  const resolvedName = projectName || projectPath.split(/[\\/]/).pop();

  // Check directory status
  const dirStatus = await checkDirectory(projectPath);

  if (dirStatus.hasClaudeDir && !force) {
    console.log(chalk.red('✗ Error: .claude directory already exists'));
    console.log(chalk.gray('  Use --force to overwrite existing configuration'));
    process.exit(1);
  }

  // Validate template
  if (!TEMPLATES[template]) {
    console.log(chalk.red(`✗ Error: Unknown template "${template}"`));
    console.log(chalk.gray('  Available templates: ' + Object.keys(TEMPLATES).join(', ')));
    process.exit(1);
  }

  console.log(chalk.gray(`  Project:  ${resolvedName}`));
  console.log(chalk.gray(`  Path:     ${projectPath}`));
  console.log(chalk.gray(`  Template: ${template} - ${TEMPLATES[template].description}`));
  console.log();

  // Create structure
  try {
    const structure = await createProjectStructure(projectPath, template, resolvedName);

    console.log(chalk.green('✓ Created project structure:\n'));
    for (const item of structure) {
      console.log(chalk.gray('  ' + item));
    }

    console.log(chalk.yellow('\n═════════════════════════════════════════'));
    console.log(chalk.green('\n✓ CYNIC project initialized successfully!'));
    console.log(chalk.gray('\n  Next steps:'));
    console.log(chalk.gray('  1. cd ' + (projectName || '.')));
    console.log(chalk.gray('  2. Edit .claude/settings.local.json'));
    console.log(chalk.gray('  3. Run: claude'));
    console.log();
    console.log(chalk.cyan(`  *tail wag* φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));

  } catch (error) {
    console.log(chalk.red('✗ Error: ' + error.message));
    process.exit(1);
  }
}

export default { initCommand, TEMPLATES };
