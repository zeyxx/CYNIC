/**
 * Brain Memory Integration Patch
 * 
 * This file contains the modifications to integrate MemoryInjector into Brain.
 * Apply these changes to brain.js
 */

// 1. Add import at top (after existing imports):
import { MemoryInjector } from './memory-injector.js';

// 2. In Brain constructor, add after this.llmOrchestrator:
this.memoryInjector = options.memoryInjector || new MemoryInjector();

// 3. Update stats to track memory injections:
// In constructor stats object, add:
memoryInjectionsRequested: 0,

// 4. In _requestJudgment method, BEFORE calling dogOrchestrator.judge():
// Replace the existing code from line 556-567 with:

async _requestJudgment(input) {
  if (!this.dogOrchestrator) return null;

  try {
    // Get memory context for this judgment
    let memoryContext = null;
    if (this.memoryInjector) {
      try {
        memoryContext = await this.memoryInjector.getMemoryContext({
          task: input.content?.slice(0, 200),
          domain: input.type || 'general',
          tags: input.context?.tags || [],
          context: input.context?.description || input.content?.slice(0, 300),
        });
        this.stats.memoryInjectionsRequested++;
      } catch (memErr) {
        log.debug('Memory injection failed, continuing without memory', { 
          error: memErr.message 
        });
      }
    }

    const item = {
      content: input.content,
      itemType: input.type || 'general',
      context: {
        ...(input.context || {}),
        // Inject memory context
        memory: memoryContext,
      },
    };

    // 1. Get local dog judgment (now with memory context)
    const result = await this.dogOrchestrator.judge(item);
    
    // ... rest of the method remains the same
  } catch (err) {
    log.warn('Dog judgment failed', { error: err.message });
    return null;
  }
}
