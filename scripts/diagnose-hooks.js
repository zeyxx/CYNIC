#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

// Valid Claude Code hook events
const VALID_EVENTS = [
  'SessionStart',
  'SessionEnd',
  'UserPromptSubmit',
  'PreToolUse',
  'PostToolUse',
  'PreCompact',
  'Stop',
  'SubagentStart',
  'SubagentStop',
  'Error',
  'Notification'
];

const hooksFile = '.claude/hooks/hooks.json';
const data = JSON.parse(fs.readFileSync(hooksFile, 'utf8'));

console.log('=== HOOKS VALIDATION REPORT ===\n');
console.log('Defined hook events:');

const defined = Object.keys(data.hooks || {});
defined.forEach(event => {
  const valid = VALID_EVENTS.includes(event);
  const status = valid ? 'OK' : 'INVALID';
  console.log(`  [${status}] ${event}`);
});

console.log('\n=== INVALID EVENTS ===');
const invalid = defined.filter(e => !VALID_EVENTS.includes(e));
if (invalid.length === 0) {
  console.log('  None - all events are valid');
} else {
  invalid.forEach(e => console.log(`  - ${e}`));
}

console.log('\n=== STRUCTURE CHECK ===');
let hasErrors = false;

defined.forEach(event => {
  const eventHooks = data.hooks[event];
  if (Array.isArray(eventHooks)) {
    console.log(`\n${event}:`);
    eventHooks.forEach((item, idx) => {
      if (item.matcher === undefined) {
        console.log(`  ERROR Hook #${idx}: missing 'matcher' field`);
        hasErrors = true;
      }
      if (!Array.isArray(item.hooks)) {
        console.log(`  ERROR Hook #${idx}: 'hooks' is not an array`);
        hasErrors = true;
      }
      if (Array.isArray(item.hooks)) {
        item.hooks.forEach((hook, hidx) => {
          if (!hook.type) {
            console.log(`    ERROR Command #${hidx}: missing 'type' field`);
            hasErrors = true;
          }
          if (!hook.command) {
            console.log(`    ERROR Command #${hidx}: missing 'command' field`);
            hasErrors = true;
          }
          if (hook.type && !['command', 'prompt'].includes(hook.type)) {
            console.log(`    ERROR Command #${hidx}: invalid type '${hook.type}'`);
            hasErrors = true;
          }
        });
      }
    });
  }
});

if (!hasErrors) {
  console.log('\n✓ All hooks are structurally valid');
}

console.log('\n=== RECOMMENDATIONS ===');
if (invalid.length > 0) {
  console.log(`1. Remove or rename invalid events: ${invalid.join(', ')}`);
}
if (hasErrors) {
  console.log('2. Fix structural errors listed above');
}
console.log('3. If still seeing errors, try using hooks-minimal.json');
